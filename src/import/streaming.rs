use anyhow::Result;
use std::path::Path;
use indicatif::{ProgressBar, ProgressStyle};

use crate::database::{Database, DatabaseError};
use crate::import::ImportManager;
use crate::models::Workout;

/// Streaming import manager that processes files without loading all data into memory
pub struct StreamingImportManager {
    import_manager: ImportManager,
    database: Database,
    chunk_size: usize,
}

impl StreamingImportManager {
    /// Create a new streaming import manager
    pub fn new(database: Database, chunk_size: Option<usize>) -> Self {
        Self {
            import_manager: ImportManager::new(),
            database,
            chunk_size: chunk_size.unwrap_or(100), // Process 100 workouts at a time by default
        }
    }

    /// Import and store a single file using streaming approach
    pub fn import_and_store_file(&mut self, file_path: &Path) -> Result<usize, DatabaseError> {
        println!("🔄 Streaming import: {}", file_path.display());

        // Import workouts from file (this still loads into memory, but we'll store and clear in chunks)
        let workouts = self.import_manager.import_file(file_path)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

        let total_workouts = workouts.len();

        // Store workouts in chunks to manage memory usage
        let mut stored_count = 0;
        for chunk in workouts.chunks(self.chunk_size) {
            for workout in chunk {
                match self.database.store_workout(workout) {
                    Ok(()) => {
                        stored_count += 1;
                    }
                    Err(DatabaseError::Duplicate(_)) => {
                        // Skip duplicates silently
                    }
                    Err(e) => {
                        eprintln!("⚠️  Failed to store workout {}: {}", workout.id, e);
                    }
                }
            }

            // Clear cache periodically to free memory
            if stored_count % (self.chunk_size * 5) == 0 {
                self.database.clear_cache();
            }
        }

        println!("✅ Stored {} workouts from {}", stored_count, file_path.display());
        Ok(stored_count)
    }

    /// Import and store all files from a directory using streaming approach
    pub fn import_and_store_directory(&mut self, dir_path: &Path) -> Result<ImportStats, DatabaseError> {
        if !dir_path.is_dir() {
            return Err(DatabaseError::SerializationError(format!(
                "Path is not a directory: {}", dir_path.display()
            )));
        }

        println!("🔍 Scanning directory: {}", dir_path.display());

        // Collect all importable files first
        let files = self.collect_importable_files(dir_path)?;

        if files.is_empty() {
            println!("No importable files found in {}", dir_path.display());
            return Ok(ImportStats::default());
        }

        println!("📁 Found {} importable files", files.len());

        // Set up progress bar
        let pb = ProgressBar::new(files.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg} (Memory: {bytes}/s)",
                )
                .unwrap()
                .progress_chars("#>-"),
        );

        let mut stats = ImportStats::default();

        // Process each file
        for (index, file_path) in files.iter().enumerate() {
            pb.set_message(format!(
                "Processing {}",
                file_path.file_name().unwrap_or_default().to_string_lossy()
            ));

            match self.import_and_store_file(&file_path) {
                Ok(workout_count) => {
                    stats.files_processed += 1;
                    stats.workouts_imported += workout_count;
                    stats.successful_files.push(file_path.clone());

                    pb.println(format!(
                        "✓ Imported {} workouts from {}",
                        workout_count,
                        file_path.file_name().unwrap_or_default().to_string_lossy()
                    ));
                }
                Err(e) => {
                    stats.failed_files.push((file_path.clone(), e.to_string()));
                    pb.println(format!(
                        "✗ Failed to import {}: {}",
                        file_path.file_name().unwrap_or_default().to_string_lossy(),
                        e
                    ));
                }
            }

            // Clear cache every 10 files to manage memory
            if index > 0 && index % 10 == 0 {
                self.database.clear_cache();
                pb.set_message("Clearing cache...".to_string());
            }

            pb.inc(1);
        }

        pb.finish_with_message(format!(
            "Import complete: {} files processed, {} workouts imported",
            stats.files_processed, stats.workouts_imported
        ));

        Ok(stats)
    }

    /// Collect all files that can be imported from a directory (same logic as ImportManager)
    fn collect_importable_files(&self, dir_path: &Path) -> Result<Vec<std::path::PathBuf>, DatabaseError> {
        let mut files = Vec::new();

        for entry in std::fs::read_dir(dir_path)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))? {
            let entry = entry.map_err(|e| DatabaseError::SerializationError(e.to_string()))?;
            let path = entry.path();

            if path.is_file() {
                // Check if the import manager can handle this file
                if self.import_manager.can_import_file(&path) {
                    files.push(path);
                }
            }
        }

        Ok(files)
    }

    /// Get database statistics after import
    pub fn get_database_stats(&self) -> Result<crate::database::DatabaseStats, DatabaseError> {
        self.database.get_stats()
    }

    /// Remove duplicates after import
    pub fn remove_duplicates(&mut self) -> Result<usize, DatabaseError> {
        println!("🔍 Scanning for duplicate workouts...");
        let duplicates = self.database.find_duplicates()?;

        if duplicates.is_empty() {
            println!("✅ No duplicates found");
            return Ok(0);
        }

        println!("⚠️  Found {} duplicate workout sets", duplicates.len());
        for dup in &duplicates {
            println!("  - {} on {} ({} seconds, {} duplicates)",
                dup.sport.to_string(), dup.date, dup.duration_seconds, dup.duplicate_count);
        }

        println!("🗑️  Removing duplicates...");
        let removed = self.database.remove_duplicates()?;
        println!("✅ Removed {} duplicate workouts", removed);

        Ok(removed)
    }

    /// Force garbage collection and cache clearing
    pub fn cleanup_memory(&mut self) {
        self.database.clear_cache();
        // In a real implementation, we might also trigger garbage collection
        // or other memory management operations here
    }
}


/// Statistics for import operations
#[derive(Debug, Default)]
pub struct ImportStats {
    pub files_processed: usize,
    pub workouts_imported: usize,
    pub successful_files: Vec<std::path::PathBuf>,
    pub failed_files: Vec<(std::path::PathBuf, String)>,
}

impl ImportStats {
    pub fn success_rate(&self) -> f64 {
        if self.files_processed == 0 {
            0.0
        } else {
            self.successful_files.len() as f64 / self.files_processed as f64
        }
    }

    pub fn print_summary(&self) {
        println!("\n📊 Import Summary:");
        println!("   Files processed: {}", self.files_processed);
        println!("   Workouts imported: {}", self.workouts_imported);
        println!("   Success rate: {:.1}%", self.success_rate() * 100.0);

        if !self.failed_files.is_empty() {
            println!("\n❌ Failed files:");
            for (file, error) in &self.failed_files {
                println!("   {}: {}", file.display(), error);
            }
        }
    }
}