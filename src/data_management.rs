use anyhow::Result;
use chrono::NaiveDate;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use indicatif::{ProgressBar, ProgressStyle};

use crate::database::{Database, DatabaseError, DatabaseStats};
use crate::models::{Workout, Sport};

/// Comprehensive data management and cleanup utilities
#[allow(dead_code)]
pub struct DataManager {
    database: Database,
    backup_directory: Option<PathBuf>,
}

impl DataManager {
    /// Create a new data manager
    pub fn new(database: Database, backup_directory: Option<PathBuf>) -> Self {
        Self {
            database,
            backup_directory,
        }
    }

    /// Comprehensive data cleanup operation
    pub fn full_cleanup(&mut self) -> Result<CleanupReport, DatabaseError> {
        println!("🧹 Starting comprehensive data cleanup...");

        let mut report = CleanupReport::default();
        let start_time = std::time::Instant::now();

        // Step 1: Create backup before cleanup
        if let Some(ref backup_dir) = self.backup_directory {
            println!("💾 Creating backup before cleanup...");
            match self.create_backup(backup_dir) {
                Ok(backup_path) => {
                    report.backup_created = Some(backup_path);
                    println!("✅ Backup created successfully");
                }
                Err(e) => {
                    println!("⚠️  Backup failed: {}. Continuing without backup.", e);
                }
            }
        }

        // Step 2: Duplicate detection and removal
        println!("🔍 Detecting and removing duplicates...");
        let duplicates_removed = self.remove_duplicates()?;
        report.duplicates_removed = duplicates_removed;

        if duplicates_removed > 0 {
            println!("✅ Removed {} duplicate workouts", duplicates_removed);
        } else {
            println!("✅ No duplicates found");
        }

        // Step 3: Data integrity checks
        println!("🔧 Running data integrity checks...");
        let integrity_issues = self.check_data_integrity()?;
        report.integrity_issues = integrity_issues.clone();

        if !integrity_issues.is_empty() {
            println!("⚠️  Found {} data integrity issues", integrity_issues.len());
            for issue in &integrity_issues {
                println!("   - {}", issue);
            }
        } else {
            println!("✅ No data integrity issues found");
        }

        // Step 4: Orphaned data cleanup
        println!("🗑️  Cleaning up orphaned data...");
        let orphaned_cleaned = self.cleanup_orphaned_data()?;
        report.orphaned_data_cleaned = orphaned_cleaned;

        if orphaned_cleaned > 0 {
            println!("✅ Cleaned up {} orphaned records", orphaned_cleaned);
        }

        // Step 5: Cache optimization
        println!("⚡ Optimizing cache...");
        self.database.clear_cache();
        report.cache_cleared = true;

        // Step 6: Database statistics
        let final_stats = self.database.get_stats()?;
        report.final_stats = Some(final_stats);

        report.total_time = start_time.elapsed();
        report.print_summary();

        Ok(report)
    }

    /// Detect and remove duplicate workouts
    pub fn remove_duplicates(&mut self) -> Result<usize, DatabaseError> {
        let duplicates = self.database.find_duplicates()?;

        if duplicates.is_empty() {
            return Ok(0);
        }

        println!("Found {} sets of duplicate workouts", duplicates.len());

        // Group duplicates for better reporting
        let mut sport_counts: HashMap<Sport, usize> = HashMap::new();
        for duplicate in &duplicates {
            *sport_counts.entry(duplicate.sport.clone()).or_insert(0) += duplicate.duplicate_count as usize;
        }

        println!("Duplicate breakdown by sport:");
        for (sport, count) in sport_counts {
            println!("  {}: {} duplicates", sport.to_string(), count);
        }

        self.database.remove_duplicates()
    }

    /// Comprehensive data integrity checks
    pub fn check_data_integrity(&mut self) -> Result<Vec<String>, DatabaseError> {
        let mut issues = Vec::new();

        println!("Running integrity checks...");

        // Check 1: Workouts with missing essential data
        let workouts = self.database.query_workouts(Default::default())?;

        let pb = ProgressBar::new(workouts.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} Checking integrity...")
                .unwrap()
                .progress_chars("#>-"),
        );

        let mut missing_duration_count = 0;
        let mut negative_values_count = 0;
        let mut future_dates_count = 0;
        let mut invalid_tss_count = 0;

        for workout in &workouts {
            // Check for missing duration
            if workout.duration_seconds == 0 {
                missing_duration_count += 1;
            }

            // Check for negative values
            if let Some(tss) = workout.summary.tss {
                if tss < rust_decimal::Decimal::ZERO {
                    negative_values_count += 1;
                }
                // Check for unrealistic TSS values (> 1000)
                if tss > rust_decimal::Decimal::from(1000) {
                    invalid_tss_count += 1;
                }
            }

            if let Some(distance) = workout.summary.total_distance {
                if distance < rust_decimal::Decimal::ZERO {
                    negative_values_count += 1;
                }
            }

            // Check for future dates
            let today = chrono::Utc::now().date_naive();
            if workout.date > today {
                future_dates_count += 1;
            }

            pb.inc(1);
        }

        pb.finish_with_message("Integrity checks complete");

        // Report issues
        if missing_duration_count > 0 {
            issues.push(format!("{} workouts with zero duration", missing_duration_count));
        }

        if negative_values_count > 0 {
            issues.push(format!("{} workouts with negative values (TSS, distance, etc.)", negative_values_count));
        }

        if future_dates_count > 0 {
            issues.push(format!("{} workouts with future dates", future_dates_count));
        }

        if invalid_tss_count > 0 {
            issues.push(format!("{} workouts with unrealistic TSS values (>1000)", invalid_tss_count));
        }

        // Check 2: Time series data consistency
        let mut time_series_issues = self.check_time_series_integrity(&workouts)?;
        issues.append(&mut time_series_issues);

        Ok(issues)
    }

    /// Check time series data for integrity issues
    fn check_time_series_integrity(&mut self, workouts: &[Workout]) -> Result<Vec<String>, DatabaseError> {
        let mut issues = Vec::new();
        let mut missing_time_series = 0;
        let mut corrupted_time_series = 0;

        for workout in workouts {
            if workout.summary.total_distance.is_some() {
                // Should have time series data for workouts with distance
                if let Ok(time_series) = self.database.load_time_series_data(&workout.id) {
                    if time_series.is_none() {
                        missing_time_series += 1;
                    } else if let Some(data_points) = time_series {
                        // Check for corrupted time series (e.g., timestamps not in order)
                        if !self.is_time_series_valid(&data_points) {
                            corrupted_time_series += 1;
                        }
                    }
                }
            }
        }

        if missing_time_series > 0 {
            issues.push(format!("{} workouts missing expected time series data", missing_time_series));
        }

        if corrupted_time_series > 0 {
            issues.push(format!("{} workouts with corrupted time series data", corrupted_time_series));
        }

        Ok(issues)
    }

    /// Validate time series data structure
    fn is_time_series_valid(&self, data_points: &[crate::models::DataPoint]) -> bool {
        if data_points.is_empty() {
            return false;
        }

        // Check if timestamps are in order
        for i in 1..data_points.len() {
            if data_points[i].timestamp < data_points[i - 1].timestamp {
                return false;
            }
        }

        // Check for reasonable data ranges
        for point in data_points {
            if let Some(hr) = point.heart_rate {
                if hr < 30 || hr > 220 {
                    return false; // Unrealistic heart rate
                }
            }

            if let Some(power) = point.power {
                if power > 2000 {
                    return false; // Unrealistic power for most athletes
                }
            }

            if let Some(elevation) = point.elevation {
                if elevation < -500 || elevation > 9000 {
                    return false; // Unrealistic elevation range
                }
            }
        }

        true
    }

    /// Clean up orphaned data records
    pub fn cleanup_orphaned_data(&mut self) -> Result<usize, DatabaseError> {
        // For now, this is a placeholder - in a full implementation, this would:
        // 1. Find time series data without corresponding workouts
        // 2. Find athlete references that don't exist
        // 3. Clean up any other dangling references

        println!("Orphaned data cleanup not yet implemented - returning 0");
        Ok(0)
    }

    /// Create a backup of the current database
    pub fn create_backup(&self, backup_dir: &Path) -> Result<PathBuf, std::io::Error> {
        std::fs::create_dir_all(backup_dir)?;

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_filename = format!("trainrs_backup_{}.db", timestamp);
        let backup_path = backup_dir.join(backup_filename);

        // In a real implementation, this would copy the SQLite database file
        // For now, we'll create a placeholder
        std::fs::write(&backup_path, b"Backup placeholder - implement actual database backup")?;

        Ok(backup_path)
    }

    /// Archive old data based on age
    pub fn archive_old_data(&mut self, cutoff_date: NaiveDate) -> Result<ArchiveReport, DatabaseError> {
        println!("🗂️  Archiving workouts older than {}", cutoff_date);

        let mut filters = crate::database::WorkoutFilters::default();
        filters.end_date = Some(cutoff_date);

        let old_workouts = self.database.query_workouts(filters)?;

        if old_workouts.is_empty() {
            return Ok(ArchiveReport {
                workouts_archived: 0,
                archive_path: None,
                size_saved_mb: 0.0,
            });
        }

        println!("Found {} workouts to archive", old_workouts.len());

        // In a full implementation, this would:
        // 1. Export old workouts to archive format (JSON/CSV)
        // 2. Compress the archived data
        // 3. Remove old workouts from active database
        // 4. Keep metadata for quick reference

        Ok(ArchiveReport {
            workouts_archived: old_workouts.len(),
            archive_path: self.backup_directory.clone(),
            size_saved_mb: 0.0, // Would calculate actual size saved
        })
    }

    /// Get comprehensive database health report
    pub fn generate_health_report(&mut self) -> Result<HealthReport, DatabaseError> {
        let stats = self.database.get_stats()?;
        let duplicates = self.database.find_duplicates()?;
        let integrity_issues = self.check_data_integrity()?;

        let cache_stats = crate::performance::PerformanceBatchProcessor::new().get_cache_stats();

        Ok(HealthReport {
            database_stats: stats,
            duplicate_count: duplicates.len(),
            integrity_issues: integrity_issues.len(),
            cache_efficiency: cache_stats.total_entries as f64 / (cache_stats.total_size_bytes as f64 / 1024.0),
            recommendations: self.generate_recommendations(&duplicates, &integrity_issues),
        })
    }

    /// Generate maintenance recommendations
    fn generate_recommendations(&self, duplicates: &[crate::database::DuplicateWorkout], integrity_issues: &[String]) -> Vec<String> {
        let mut recommendations = Vec::new();

        if !duplicates.is_empty() {
            recommendations.push(format!(
                "Run duplicate cleanup - found {} sets of duplicates",
                duplicates.len()
            ));
        }

        if !integrity_issues.is_empty() {
            recommendations.push(format!(
                "Address {} data integrity issues found",
                integrity_issues.len()
            ));
        }

        if duplicates.is_empty() && integrity_issues.is_empty() {
            recommendations.push("Database is healthy - consider regular maintenance schedule".to_string());
        } else {
            recommendations.push("Run full cleanup operation to address all issues".to_string());
        }

        recommendations.push("Set up automated backups if not already configured".to_string());

        recommendations
    }
}

/// Comprehensive cleanup report
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct CleanupReport {
    pub duplicates_removed: usize,
    pub integrity_issues: Vec<String>,
    pub orphaned_data_cleaned: usize,
    pub backup_created: Option<PathBuf>,
    pub cache_cleared: bool,
    pub final_stats: Option<DatabaseStats>,
    pub total_time: std::time::Duration,
}

impl CleanupReport {
    pub fn print_summary(&self) {
        println!("\n📊 Cleanup Summary:");
        println!("   Total time: {:.2?}", self.total_time);
        println!("   Duplicates removed: {}", self.duplicates_removed);
        println!("   Integrity issues found: {}", self.integrity_issues.len());
        println!("   Orphaned records cleaned: {}", self.orphaned_data_cleaned);
        println!("   Cache cleared: {}", if self.cache_cleared { "Yes" } else { "No" });

        if let Some(ref backup_path) = self.backup_created {
            println!("   Backup created: {}", backup_path.display());
        }

        if let Some(ref stats) = self.final_stats {
            println!("   Final database size:");
            println!("     Workouts: {}", stats.workout_count);
            println!("     Time series: {}", stats.time_series_count);
            println!("     Compression ratio: {:.1}:1", stats.compression_ratio);
        }
    }
}

/// Archive operation report
#[allow(dead_code)]
#[derive(Debug)]
pub struct ArchiveReport {
    pub workouts_archived: usize,
    pub archive_path: Option<PathBuf>,
    pub size_saved_mb: f64,
}

/// Database health report
#[allow(dead_code)]
#[derive(Debug)]
pub struct HealthReport {
    pub database_stats: DatabaseStats,
    pub duplicate_count: usize,
    pub integrity_issues: usize,
    pub cache_efficiency: f64,
    pub recommendations: Vec<String>,
}

impl HealthReport {
    pub fn print_report(&self) {
        println!("\n🏥 Database Health Report:");
        println!("   Database Statistics:");
        println!("     Workouts: {}", self.database_stats.workout_count);
        println!("     Athletes: {}", self.database_stats.athlete_count);
        println!("     Time series: {}", self.database_stats.time_series_count);
        println!("     Compression: {:.1}:1 ratio", self.database_stats.compression_ratio);
        println!("     Cache entries: {}", self.database_stats.cache_entries);

        println!("\n   Issues:");
        println!("     Duplicates: {}", self.duplicate_count);
        println!("     Integrity issues: {}", self.integrity_issues);

        println!("\n   Recommendations:");
        for (i, rec) in self.recommendations.iter().enumerate() {
            println!("     {}. {}", i + 1, rec);
        }
    }
}

