#![allow(dead_code)]

use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::database::{Database, DatabaseError, WorkoutFilters};
use crate::models::{Workout, AthleteProfile};
use crate::pmc::{PmcCalculator, PmcMetrics};
use crate::tss::TssCalculator;

/// Performance optimized batch operations for large datasets
#[allow(dead_code)]
pub struct PerformanceBatchProcessor {
    /// Cache for frequently accessed data
    cache: Arc<Mutex<HashMap<String, CachedResult>>>,
    /// Number of parallel threads to use
    _thread_count: usize,
}

/// Cached calculation results with timestamps
#[derive(Debug, Clone)]
pub struct CachedResult {
    pub data: Vec<u8>, // Serialized result
    pub timestamp: std::time::SystemTime,
    pub cache_type: CacheType,
}

#[derive(Debug, Clone)]
pub enum CacheType {
    PmcMetrics,
    TssCalculation,
    WorkoutSummary,
    ZoneAnalysis,
}

/// Batch processing results
#[derive(Debug)]
pub struct BatchProcessingResult<T> {
    pub results: Vec<T>,
    pub processing_time: std::time::Duration,
    pub items_processed: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

impl PerformanceBatchProcessor {
    /// Create a new batch processor with optimal thread count
    pub fn new() -> Self {
        let thread_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4); // Fallback to 4 threads

        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            _thread_count: thread_count,
        }
    }

    /// Create with custom thread count
    pub fn with_thread_count(thread_count: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            _thread_count: thread_count,
        }
    }

    /// Sequential PMC calculation for multiple athletes (simplified for thread safety)
    pub fn batch_calculate_pmc(
        &self,
        database: &Database,
        athlete_ids: &[String],
        filters: Option<WorkoutFilters>,
    ) -> Result<BatchProcessingResult<(String, Vec<PmcMetrics>)>, DatabaseError> {
        let start_time = Instant::now();
        let mut cache_hits = 0;
        let mut cache_misses = 0;

        let mut processed_results = Vec::new();

        // Process each athlete sequentially
        for athlete_id in athlete_ids {
            let cache_key = format!("pmc_{}_{:?}", athlete_id, filters);

            // Check cache first
            let cached_result = if let Ok(cache) = self.cache.lock() {
                if let Some(cached) = cache.get(&cache_key) {
                    // Check if cache is still valid (1 hour)
                    if cached.timestamp.elapsed().unwrap_or(std::time::Duration::MAX)
                        < std::time::Duration::from_secs(3600) {

                        if let Ok(metrics) = bincode::deserialize::<Vec<PmcMetrics>>(&cached.data) {
                            Some(metrics)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            let metrics = if let Some(cached_metrics) = cached_result {
                cache_hits += 1;
                cached_metrics
            } else {
                // Cache miss - calculate PMC metrics
                cache_misses += 1;
                let mut athlete_filters = filters.clone().unwrap_or_default();
                athlete_filters.athlete_id = Some(athlete_id.clone());

                let workouts = database.query_workouts(athlete_filters)?;
                let pmc_calculator = PmcCalculator::new();

                // Build daily TSS data from workouts
                let daily_tss = pmc_calculator.aggregate_daily_tss(&workouts);

                // Calculate PMC series for the entire date range
                let metrics = if let (Some(start_date), Some(end_date)) = (
                    workouts.iter().map(|w| w.date).min(),
                    workouts.iter().map(|w| w.date).max()
                ) {
                    match pmc_calculator.calculate_pmc_series(&daily_tss, start_date, end_date) {
                        Ok(series) => series,
                        Err(_) => Vec::new(),
                    }
                } else {
                    Vec::new()
                };

                // Cache the result
                if let Ok(serialized) = bincode::serialize(&metrics) {
                    let cached_result = CachedResult {
                        data: serialized,
                        timestamp: std::time::SystemTime::now(),
                        cache_type: CacheType::PmcMetrics,
                    };

                    if let Ok(mut cache) = self.cache.lock() {
                        cache.insert(cache_key, cached_result);
                    }
                }

                metrics
            };

            processed_results.push((athlete_id.clone(), metrics));
        }

        Ok(BatchProcessingResult {
            results: processed_results,
            processing_time: start_time.elapsed(),
            items_processed: athlete_ids.len(),
            cache_hits,
            cache_misses,
        })
    }

    /// Batch calculate TSS for workouts that are missing TSS values
    pub fn batch_calculate_missing_tss(
        &self,
        database: &mut Database,
        athlete_profile: &AthleteProfile,
        workout_ids: &[String],
    ) -> Result<BatchProcessingResult<String>, DatabaseError> {
        let start_time = Instant::now();

        // Load workouts sequentially (SQLite connection is not thread-safe)
        let mut workouts = Vec::new();
        for workout_id in workout_ids {
            workouts.push(database.load_workout(workout_id));
        }

        let mut updated_workouts = Vec::new();
        let mut processing_count = 0;

        for (_idx, workout_result) in workouts.into_iter().enumerate() {
            match workout_result {
                Ok(Some(mut workout)) => {
                    if workout.summary.tss.is_none() {
                        // Calculate TSS using TssCalculator static methods
                        let tss_result = TssCalculator::calculate_tss(&workout, athlete_profile);

                        match tss_result {
                            Ok(result) => {
                                workout.summary.tss = Some(result.tss);
                                updated_workouts.push(workout.id.clone());
                                processing_count += 1;
                            }
                            Err(_) => {
                                // Skip workouts that can't have TSS calculated
                                continue;
                            }
                        }

                        // Store the updated workout
                        database.store_workout(&workout)?;
                    }
                }
                Ok(None) => {
                    // Workout not found - skip
                }
                Err(e) => return Err(e),
            }
        }

        Ok(BatchProcessingResult {
            results: updated_workouts,
            processing_time: start_time.elapsed(),
            items_processed: processing_count,
            cache_hits: 0, // TSS calculation doesn't use cache in this implementation
            cache_misses: processing_count,
        })
    }

    /// Sequential zone analysis for multiple workouts (simplified for thread safety)
    pub fn batch_zone_analysis(
        &self,
        database: &mut Database,
        workout_ids: &[String],
        athlete_profile: &AthleteProfile,
    ) -> Result<BatchProcessingResult<WorkoutZoneAnalysis>, DatabaseError> {
        let start_time = Instant::now();

        // Process workouts sequentially to avoid thread safety issues
        let mut successful_results = Vec::new();
        for workout_id in workout_ids {
            // Load workout
            let workout = match database.load_workout(workout_id)? {
                Some(w) => w,
                None => {
                    eprintln!("Workout not found: {}", workout_id);
                    continue;
                }
            };

            // Load time-series data if available
            let time_series_data = if workout.summary.total_distance.is_some() {
                database.load_time_series_data(workout_id).unwrap_or(None)
            } else {
                None
            };

            // Perform zone analysis
            match self.analyze_workout_zones(&workout, &time_series_data, athlete_profile) {
                Ok(analysis) => successful_results.push(analysis),
                Err(e) => {
                    eprintln!("Zone analysis error for {}: {}", workout_id, e);
                    // Continue with other workouts
                }
            }
        }

        let result_count = successful_results.len();
        Ok(BatchProcessingResult {
            results: successful_results,
            processing_time: start_time.elapsed(),
            items_processed: result_count,
            cache_hits: 0, // Zone analysis is computed fresh each time
            cache_misses: result_count,
        })
    }

    /// Analyze zones for a single workout
    fn analyze_workout_zones(
        &self,
        workout: &Workout,
        time_series_data: &Option<Vec<crate::models::DataPoint>>,
        athlete_profile: &AthleteProfile,
    ) -> Result<WorkoutZoneAnalysis, DatabaseError> {
        let mut zone_analysis = WorkoutZoneAnalysis {
            workout_id: workout.id.clone(),
            sport: workout.sport.clone(),
            duration_seconds: workout.duration_seconds,
            heart_rate_zones: None,
            power_zones: None,
            pace_zones: None,
        };

        if let Some(ref data_points) = time_series_data {
            // Analyze heart rate zones
            if let Some(lthr) = athlete_profile.lthr {
                let hr_zones = self.calculate_hr_zone_distribution(data_points, lthr);
                zone_analysis.heart_rate_zones = Some(hr_zones);
            }

            // Analyze power zones
            if let Some(ftp) = athlete_profile.ftp {
                let power_zones = self.calculate_power_zone_distribution(data_points, ftp);
                zone_analysis.power_zones = Some(power_zones);
            }

            // Analyze pace zones (for running)
            if workout.sport == crate::models::Sport::Running {
                if let Some(threshold_pace) = athlete_profile.threshold_pace {
                    let pace_zones = self.calculate_pace_zone_distribution(data_points, threshold_pace);
                    zone_analysis.pace_zones = Some(pace_zones);
                }
            }
        }

        Ok(zone_analysis)
    }

    /// Calculate heart rate zone distribution
    fn calculate_hr_zone_distribution(
        &self,
        data_points: &[crate::models::DataPoint],
        lthr: u16,
    ) -> ZoneDistribution {
        let mut zone_times = HashMap::new();

        for point in data_points {
            if let Some(hr) = point.heart_rate {
                let zone = match hr {
                    hr if hr < (lthr as f32 * 0.68) as u16 => 1, // Zone 1: < 68% LTHR
                    hr if hr < (lthr as f32 * 0.83) as u16 => 2, // Zone 2: 68-82% LTHR
                    hr if hr < (lthr as f32 * 0.94) as u16 => 3, // Zone 3: 83-93% LTHR
                    hr if hr < (lthr as f32 * 1.05) as u16 => 4, // Zone 4: 94-104% LTHR
                    _ => 5, // Zone 5: > 105% LTHR
                };

                *zone_times.entry(zone).or_insert(0) += 1; // Assuming 1-second intervals
            }
        }

        let total_time: u32 = zone_times.values().sum();
        let zone_percentages = zone_times
            .iter()
            .map(|(&zone, &time)| (zone, (time as f64 / total_time as f64) * 100.0))
            .collect();

        ZoneDistribution {
            zone_times,
            zone_percentages,
        }
    }

    /// Calculate power zone distribution
    fn calculate_power_zone_distribution(
        &self,
        data_points: &[crate::models::DataPoint],
        ftp: u16,
    ) -> ZoneDistribution {
        let mut zone_times = HashMap::new();

        for point in data_points {
            if let Some(power) = point.power {
                let zone = match power {
                    p if p < (ftp as f32 * 0.55) as u16 => 1, // Zone 1: < 55% FTP
                    p if p < (ftp as f32 * 0.75) as u16 => 2, // Zone 2: 56-75% FTP
                    p if p < (ftp as f32 * 0.90) as u16 => 3, // Zone 3: 76-90% FTP
                    p if p < (ftp as f32 * 1.05) as u16 => 4, // Zone 4: 91-105% FTP
                    p if p < (ftp as f32 * 1.20) as u16 => 5, // Zone 5: 106-120% FTP
                    p if p < (ftp as f32 * 1.50) as u16 => 6, // Zone 6: 121-150% FTP
                    _ => 7, // Zone 7: > 150% FTP
                };

                *zone_times.entry(zone).or_insert(0) += 1;
            }
        }

        let total_time: u32 = zone_times.values().sum();
        let zone_percentages = zone_times
            .iter()
            .map(|(&zone, &time)| (zone, (time as f64 / total_time as f64) * 100.0))
            .collect();

        ZoneDistribution {
            zone_times,
            zone_percentages,
        }
    }

    /// Calculate pace zone distribution (for running)
    fn calculate_pace_zone_distribution(
        &self,
        data_points: &[crate::models::DataPoint],
        threshold_pace: rust_decimal::Decimal,
    ) -> ZoneDistribution {
        let mut zone_times = HashMap::new();
        let threshold_pace_f64: f64 = threshold_pace.try_into().unwrap_or(6.0); // minutes per km

        for point in data_points {
            if let Some(pace) = point.pace {
                let pace_f64: f64 = pace.try_into().unwrap_or(10.0);
                let zone = if pace_f64 > threshold_pace_f64 * 1.29 {
                    1 // Zone 1: Slower than 129% threshold pace (easy)
                } else if pace_f64 > threshold_pace_f64 * 1.14 {
                    2 // Zone 2: 114-129% threshold pace (aerobic)
                } else if pace_f64 > threshold_pace_f64 * 1.07 {
                    3 // Zone 3: 107-114% threshold pace (tempo)
                } else if pace_f64 > threshold_pace_f64 * 0.97 {
                    4 // Zone 4: 97-107% threshold pace (threshold)
                } else {
                    5 // Zone 5: Faster than 97% threshold pace (VO2 max)
                };

                *zone_times.entry(zone).or_insert(0) += 1;
            }
        }

        let total_time: u32 = zone_times.values().sum();
        let zone_percentages = zone_times
            .iter()
            .map(|(&zone, &time)| (zone, (time as f64 / total_time as f64) * 100.0))
            .collect();

        ZoneDistribution {
            zone_times,
            zone_percentages,
        }
    }

    /// Clear expired cache entries
    pub fn cleanup_cache(&self, max_age_seconds: u64) {
        if let Ok(mut cache) = self.cache.lock() {
            let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(max_age_seconds);
            cache.retain(|_, cached| {
                cached.timestamp > cutoff
            });
        }
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> CacheStats {
        if let Ok(cache) = self.cache.lock() {
            let total_entries = cache.len();
            let mut type_counts = HashMap::new();
            let mut total_size = 0;

            for cached in cache.values() {
                total_size += cached.data.len();
                *type_counts.entry(format!("{:?}", cached.cache_type)).or_insert(0) += 1;
            }

            CacheStats {
                total_entries,
                total_size_bytes: total_size,
                type_counts,
            }
        } else {
            CacheStats::default()
        }
    }
}

/// Workout zone analysis results
#[derive(Debug)]
pub struct WorkoutZoneAnalysis {
    pub workout_id: String,
    pub sport: crate::models::Sport,
    pub duration_seconds: u32,
    pub heart_rate_zones: Option<ZoneDistribution>,
    pub power_zones: Option<ZoneDistribution>,
    pub pace_zones: Option<ZoneDistribution>,
}

/// Zone distribution data
#[derive(Debug)]
pub struct ZoneDistribution {
    pub zone_times: HashMap<u8, u32>, // Zone number -> time in seconds
    pub zone_percentages: HashMap<u8, f64>, // Zone number -> percentage of total time
}

/// Cache statistics
#[derive(Debug, Default)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_size_bytes: usize,
    pub type_counts: HashMap<String, usize>,
}

