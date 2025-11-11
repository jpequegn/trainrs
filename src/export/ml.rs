//! Machine Learning Export Module
//!
//! Exports training data in ML-ready formats with engineered features for
//! external analysis, predictive modeling, and data science workflows.

use crate::models::{AthleteProfile, Workout};
use chrono::{Datelike, NaiveDate};
use csv::Writer;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::path::Path;

/// ML export errors
#[derive(Debug)]
pub enum MlExportError {
    /// IO error during export
    IoError(std::io::Error),
    /// CSV serialization error
    CsvError(csv::Error),
    /// Invalid data for ML export
    InvalidData(String),
    /// Missing required athlete profile data
    MissingProfile(String),
}

impl fmt::Display for MlExportError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MlExportError::IoError(e) => write!(f, "IO error: {}", e),
            MlExportError::CsvError(e) => write!(f, "CSV error: {}", e),
            MlExportError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            MlExportError::MissingProfile(msg) => write!(f, "Missing profile: {}", msg),
        }
    }
}

impl Error for MlExportError {}

impl From<std::io::Error> for MlExportError {
    fn from(error: std::io::Error) -> Self {
        MlExportError::IoError(error)
    }
}

impl From<csv::Error> for MlExportError {
    fn from(error: csv::Error) -> Self {
        MlExportError::CsvError(error)
    }
}

/// Data split type for ML training
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitType {
    /// Training set (default 70%)
    Train,
    /// Validation set (default 15%)
    Validation,
    /// Test set (default 15%)
    Test,
}

/// Configuration for data splitting
#[derive(Debug, Clone)]
pub struct SplitConfig {
    /// Training set percentage (0.0-1.0)
    pub train_pct: f64,
    /// Validation set percentage (0.0-1.0)
    pub val_pct: f64,
    /// Test set percentage (0.0-1.0)
    pub test_pct: f64,
    /// Random seed for reproducibility
    pub seed: Option<u64>,
}

impl Default for SplitConfig {
    fn default() -> Self {
        Self {
            train_pct: 0.70,
            val_pct: 0.15,
            test_pct: 0.15,
            seed: Some(42),
        }
    }
}

impl SplitConfig {
    /// Validate split percentages sum to 1.0
    pub fn validate(&self) -> Result<(), MlExportError> {
        let sum = self.train_pct + self.val_pct + self.test_pct;
        if (sum - 1.0).abs() > 0.001 {
            return Err(MlExportError::InvalidData(format!(
                "Split percentages must sum to 1.0, got {}",
                sum
            )));
        }
        Ok(())
    }

    /// Determine split type for a workout based on index
    pub fn determine_split(&self, index: usize, total: usize) -> SplitType {
        let pct = (index as f64) / (total as f64);
        if pct < self.train_pct {
            SplitType::Train
        } else if pct < self.train_pct + self.val_pct {
            SplitType::Validation
        } else {
            SplitType::Test
        }
    }
}

/// Engineered features for a workout
#[derive(Debug, Clone)]
pub struct WorkoutFeatures {
    // Basic workout metadata
    pub workout_id: String,
    pub date: NaiveDate,
    pub sport: String,
    pub duration_seconds: u32,
    pub distance_meters: Option<f64>,

    // Temporal features
    pub year: i32,
    pub month: u32,
    pub day_of_week: u32,
    pub day_of_year: u32,
    pub week_of_year: u32,

    // Training load metrics
    pub tss: Option<f64>,
    pub intensity_factor: Option<f64>,
    pub normalized_power: Option<f64>,
    pub average_power: Option<f64>,
    pub max_power: Option<f64>,

    // Heart rate metrics
    pub average_hr: Option<f64>,
    pub max_hr: Option<f64>,
    pub hr_zone_1_pct: Option<f64>,
    pub hr_zone_2_pct: Option<f64>,
    pub hr_zone_3_pct: Option<f64>,
    pub hr_zone_4_pct: Option<f64>,
    pub hr_zone_5_pct: Option<f64>,

    // Training effect metrics
    pub aerobic_te: Option<f64>,
    pub anaerobic_te: Option<f64>,
    pub epoc: Option<f64>,
    pub recovery_hours: Option<f64>,

    // Pace metrics (running/swimming)
    pub average_pace_min_per_km: Option<f64>,
    pub best_pace_min_per_km: Option<f64>,

    // Rolling statistics (7-day windows)
    pub rolling_7d_tss: Option<f64>,
    pub rolling_7d_duration: Option<f64>,
    pub rolling_7d_distance: Option<f64>,

    // Rolling statistics (28-day windows)
    pub rolling_28d_tss: Option<f64>,
    pub rolling_28d_duration: Option<f64>,
    pub rolling_28d_distance: Option<f64>,

    // Cumulative metrics
    pub cumulative_tss: Option<f64>,
    pub cumulative_duration: Option<f64>,
    pub cumulative_distance: Option<f64>,

    // Data split assignment
    pub split: String,
}

/// ML-optimized CSV exporter
pub struct MlCsvExporter;

impl MlCsvExporter {
    /// Create a new ML CSV exporter
    pub fn new() -> Self {
        Self
    }

    /// Export workouts to ML-ready CSV with engineered features
    pub fn export_with_features<P: AsRef<Path>>(
        &self,
        workouts: &[Workout],
        athlete: &AthleteProfile,
        output_path: P,
        split_config: Option<SplitConfig>,
    ) -> Result<(), MlExportError> {
        let config = split_config.unwrap_or_default();
        config.validate()?;

        // Sort workouts by date
        let mut sorted_workouts = workouts.to_vec();
        sorted_workouts.sort_by_key(|w| w.date);

        // Calculate rolling statistics and cumulative metrics
        let features = self.engineer_features(&sorted_workouts, athlete, &config)?;

        // Write to CSV
        let file = File::create(output_path)?;
        let mut writer = Writer::from_writer(file);

        // Write header
        writer.write_record(&[
            "workout_id",
            "date",
            "sport",
            "duration_seconds",
            "distance_meters",
            // Temporal features
            "year",
            "month",
            "day_of_week",
            "day_of_year",
            "week_of_year",
            // Training load
            "tss",
            "intensity_factor",
            "normalized_power",
            "average_power",
            "max_power",
            // Heart rate
            "average_hr",
            "max_hr",
            "hr_zone_1_pct",
            "hr_zone_2_pct",
            "hr_zone_3_pct",
            "hr_zone_4_pct",
            "hr_zone_5_pct",
            // Training effect
            "aerobic_te",
            "anaerobic_te",
            "epoc",
            "recovery_hours",
            // Pace
            "average_pace_min_per_km",
            "best_pace_min_per_km",
            // Rolling 7-day
            "rolling_7d_tss",
            "rolling_7d_duration",
            "rolling_7d_distance",
            // Rolling 28-day
            "rolling_28d_tss",
            "rolling_28d_duration",
            "rolling_28d_distance",
            // Cumulative
            "cumulative_tss",
            "cumulative_duration",
            "cumulative_distance",
            // Split
            "split",
        ])?;

        // Write data rows
        for feature in features {
            writer.write_record(&[
                feature.workout_id,
                feature.date.to_string(),
                feature.sport,
                feature.duration_seconds.to_string(),
                format_optional_f64(feature.distance_meters),
                // Temporal
                feature.year.to_string(),
                feature.month.to_string(),
                feature.day_of_week.to_string(),
                feature.day_of_year.to_string(),
                feature.week_of_year.to_string(),
                // Training load
                format_optional_f64(feature.tss),
                format_optional_f64(feature.intensity_factor),
                format_optional_f64(feature.normalized_power),
                format_optional_f64(feature.average_power),
                format_optional_f64(feature.max_power),
                // Heart rate
                format_optional_f64(feature.average_hr),
                format_optional_f64(feature.max_hr),
                format_optional_f64(feature.hr_zone_1_pct),
                format_optional_f64(feature.hr_zone_2_pct),
                format_optional_f64(feature.hr_zone_3_pct),
                format_optional_f64(feature.hr_zone_4_pct),
                format_optional_f64(feature.hr_zone_5_pct),
                // Training effect
                format_optional_f64(feature.aerobic_te),
                format_optional_f64(feature.anaerobic_te),
                format_optional_f64(feature.epoc),
                format_optional_f64(feature.recovery_hours),
                // Pace
                format_optional_f64(feature.average_pace_min_per_km),
                format_optional_f64(feature.best_pace_min_per_km),
                // Rolling 7-day
                format_optional_f64(feature.rolling_7d_tss),
                format_optional_f64(feature.rolling_7d_duration),
                format_optional_f64(feature.rolling_7d_distance),
                // Rolling 28-day
                format_optional_f64(feature.rolling_28d_tss),
                format_optional_f64(feature.rolling_28d_duration),
                format_optional_f64(feature.rolling_28d_distance),
                // Cumulative
                format_optional_f64(feature.cumulative_tss),
                format_optional_f64(feature.cumulative_duration),
                format_optional_f64(feature.cumulative_distance),
                // Split
                feature.split,
            ])?;
        }

        writer.flush()?;
        Ok(())
    }

    /// Engineer features from workouts
    fn engineer_features(
        &self,
        workouts: &[Workout],
        athlete: &AthleteProfile,
        config: &SplitConfig,
    ) -> Result<Vec<WorkoutFeatures>, MlExportError> {
        let mut features = Vec::new();
        let mut cumulative_tss = 0.0;
        let mut cumulative_duration = 0.0;
        let mut cumulative_distance = 0.0;

        for (idx, workout) in workouts.iter().enumerate() {
            // Get existing summary metrics
            let summary = &workout.summary;
            let training_effect = crate::training_effect::TrainingEffectAnalyzer::calculate_training_effect(workout, athlete)
                .ok();

            // Extract temporal features
            let date = workout.date;
            let year = date.year();
            let month = date.month();
            let day_of_week = date.weekday().num_days_from_monday();
            let day_of_year = date.ordinal();
            let week_of_year = date.iso_week().week();

            // Calculate HR zone percentages
            let hr_zones = self.calculate_hr_zone_percentages(workout, athlete);

            // Calculate pace metrics
            let (avg_pace, best_pace) = self.calculate_pace_metrics(workout);

            // Calculate rolling statistics
            let rolling_7d = self.calculate_rolling_stats(workouts, idx, 7);
            let rolling_28d = self.calculate_rolling_stats(workouts, idx, 28);

            // Update cumulative metrics
            if let Some(tss) = summary.tss {
                cumulative_tss += tss.to_string().parse::<f64>().unwrap_or(0.0);
            }
            cumulative_duration += workout.duration_seconds as f64;
            if let Some(dist) = summary.total_distance {
                cumulative_distance += dist.to_string().parse::<f64>().unwrap_or(0.0);
            }

            // Determine data split
            let split = config.determine_split(idx, workouts.len());
            let split_str = match split {
                SplitType::Train => "train",
                SplitType::Validation => "validation",
                SplitType::Test => "test",
            };

            // Extract distance in meters
            let distance_meters = summary.total_distance.map(|d| d.to_string().parse::<f64>().unwrap_or(0.0));

            features.push(WorkoutFeatures {
                workout_id: workout.id.clone(),
                date,
                sport: format!("{:?}", workout.sport),
                duration_seconds: workout.duration_seconds,
                distance_meters,
                year,
                month,
                day_of_week,
                day_of_year,
                week_of_year,
                tss: summary.tss.map(|d| d.to_string().parse().unwrap_or(0.0)),
                intensity_factor: summary
                    .intensity_factor
                    .map(|d| d.to_string().parse().unwrap_or(0.0)),
                normalized_power: summary.normalized_power.map(|d| d as f64),
                average_power: summary.avg_power.map(|d| d as f64),
                max_power: None, // Not available in WorkoutSummary
                average_hr: summary.avg_heart_rate.map(|d| d as f64),
                max_hr: summary.max_heart_rate.map(|d| d as f64),
                hr_zone_1_pct: hr_zones.get(&1).copied(),
                hr_zone_2_pct: hr_zones.get(&2).copied(),
                hr_zone_3_pct: hr_zones.get(&3).copied(),
                hr_zone_4_pct: hr_zones.get(&4).copied(),
                hr_zone_5_pct: hr_zones.get(&5).copied(),
                aerobic_te: training_effect
                    .as_ref()
                    .map(|te| te.aerobic_te.to_string().parse().unwrap_or(0.0)),
                anaerobic_te: training_effect
                    .as_ref()
                    .map(|te| te.anaerobic_te.to_string().parse().unwrap_or(0.0)),
                epoc: training_effect
                    .as_ref()
                    .map(|te| te.epoc.to_string().parse().unwrap_or(0.0)),
                recovery_hours: training_effect.as_ref().map(|te| te.recovery_time_hours as f64),
                average_pace_min_per_km: avg_pace,
                best_pace_min_per_km: best_pace,
                rolling_7d_tss: rolling_7d.0,
                rolling_7d_duration: rolling_7d.1,
                rolling_7d_distance: rolling_7d.2,
                rolling_28d_tss: rolling_28d.0,
                rolling_28d_duration: rolling_28d.1,
                rolling_28d_distance: rolling_28d.2,
                cumulative_tss: Some(cumulative_tss),
                cumulative_duration: Some(cumulative_duration),
                cumulative_distance: Some(cumulative_distance),
                split: split_str.to_string(),
            });
        }

        Ok(features)
    }

    /// Calculate HR zone percentages from raw data
    fn calculate_hr_zone_percentages(
        &self,
        workout: &Workout,
        athlete: &AthleteProfile,
    ) -> HashMap<u8, f64> {
        // Return empty if no raw data available
        let raw_data = match &workout.raw_data {
            Some(data) if !data.is_empty() => data,
            _ => return HashMap::new(),
        };

        let mut zone_counts: HashMap<u8, usize> = HashMap::new();
        let total_points = raw_data.len();

        for point in raw_data {
            if let Some(hr) = point.heart_rate {
                let zone = self.get_hr_zone(hr, athlete);
                *zone_counts.entry(zone).or_insert(0) += 1;
            }
        }

        zone_counts
            .into_iter()
            .map(|(zone, count)| (zone, (count as f64 / total_points as f64) * 100.0))
            .collect()
    }

    /// Determine HR zone for a given heart rate
    fn get_hr_zone(&self, hr: u16, athlete: &AthleteProfile) -> u8 {
        let lthr = athlete.lthr.unwrap_or(170);
        let hr_pct = (hr as f64 / lthr as f64) * 100.0;

        if hr_pct < 65.0 {
            1
        } else if hr_pct < 75.0 {
            2
        } else if hr_pct < 85.0 {
            3
        } else if hr_pct < 95.0 {
            4
        } else {
            5
        }
    }

    /// Calculate pace metrics from workout summary
    fn calculate_pace_metrics(&self, workout: &Workout) -> (Option<f64>, Option<f64>) {
        // Use summary average pace if available
        let avg_pace = workout.summary.avg_pace.map(|p| p.to_string().parse::<f64>().unwrap_or(0.0));

        // For best pace, we'd need raw data which may not be available
        // For now, just return average pace as best pace estimate
        (avg_pace, avg_pace)
    }

    /// Calculate rolling statistics for a workout
    fn calculate_rolling_stats(
        &self,
        workouts: &[Workout],
        current_idx: usize,
        window_days: i64,
    ) -> (Option<f64>, Option<f64>, Option<f64>) {
        let current_workout = &workouts[current_idx];
        let cutoff_date = current_workout.date - chrono::Duration::days(window_days);

        let mut total_tss = 0.0;
        let mut total_duration = 0.0;
        let mut total_distance = 0.0;
        let mut count = 0;

        for workout in &workouts[0..=current_idx] {
            if workout.date >= cutoff_date {
                if let Some(tss) = workout.summary.tss {
                    total_tss += tss.to_string().parse::<f64>().unwrap_or(0.0);
                }
                total_duration += workout.duration_seconds as f64;
                if let Some(dist) = workout.summary.total_distance {
                    total_distance += dist.to_string().parse::<f64>().unwrap_or(0.0);
                }
                count += 1;
            }
        }

        if count > 0 {
            (
                Some(total_tss),
                Some(total_duration),
                Some(total_distance),
            )
        } else {
            (None, None, None)
        }
    }
}

impl Default for MlCsvExporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Format optional f64 for CSV output
fn format_optional_f64(value: Option<f64>) -> String {
    value.map_or_else(String::new, |v| format!("{:.2}", v))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DataPoint, Sport};
    use rust_decimal_macros::dec;

    fn create_test_athlete() -> AthleteProfile {
        use crate::models::{TrainingZones, Units};
        use rust_decimal_macros::dec;

        AthleteProfile {
            id: "test_athlete".to_string(),
            name: "Test Athlete".to_string(),
            date_of_birth: None,
            weight: Some(dec!(70)),
            height: Some(175),
            ftp: Some(250),
            lthr: Some(170),
            threshold_pace: Some(dec!(4.0)),
            max_hr: Some(190),
            resting_hr: Some(50),
            training_zones: TrainingZones::default(),
            preferred_units: Units::Metric,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn create_test_workout(id: &str, date: NaiveDate) -> Workout {
        use crate::models::{DataSource, WorkoutSummary, WorkoutType};

        Workout {
            id: id.to_string(),
            athlete_id: Some("test_athlete".to_string()),
            date,
            sport: Sport::Cycling,
            duration_seconds: 3600,
            workout_type: WorkoutType::Endurance,
            data_source: DataSource::Power,
            raw_data: Some(vec![
                DataPoint {
                    timestamp: 0,
                    power: Some(200),
                    heart_rate: Some(140),
                    cadence: Some(90),
                    speed: Some(dec!(8.33)),
                    distance: Some(dec!(0.0)),
                    pace: None,
                    elevation: None,
                    left_power: None,
                    right_power: None,
                    ground_contact_time: None,
                    vertical_oscillation: None,
                    stride_length: None,
                    stroke_count: None,
                    stroke_type: None,
                    lap_number: Some(1),
                    sport_transition: Some(false),
                },
                DataPoint {
                    timestamp: 1800,
                    power: Some(250),
                    heart_rate: Some(160),
                    cadence: Some(95),
                    speed: Some(dec!(8.33)),
                    distance: Some(dec!(15000.0)),
                    pace: None,
                    elevation: None,
                    left_power: None,
                    right_power: None,
                    ground_contact_time: None,
                    vertical_oscillation: None,
                    stride_length: None,
                    stroke_count: None,
                    stroke_type: None,
                    lap_number: Some(1),
                    sport_transition: Some(false),
                },
            ]),
            summary: WorkoutSummary {
                avg_heart_rate: Some(150),
                max_heart_rate: Some(160),
                avg_power: Some(225),
                normalized_power: Some(230),
                avg_pace: None,
                intensity_factor: Some(dec!(0.92)),
                tss: Some(dec!(75.0)),
                total_distance: Some(dec!(30000.0)),
                elevation_gain: None,
                avg_cadence: None,
                calories: None,
            },
            notes: None,
            source: None,
        }
    }

    #[test]
    fn test_split_config_default() {
        let config = SplitConfig::default();
        assert_eq!(config.train_pct, 0.70);
        assert_eq!(config.val_pct, 0.15);
        assert_eq!(config.test_pct, 0.15);
        assert_eq!(config.seed, Some(42));
    }

    #[test]
    fn test_split_config_validation() {
        let config = SplitConfig {
            train_pct: 0.70,
            val_pct: 0.15,
            test_pct: 0.15,
            seed: Some(42),
        };
        assert!(config.validate().is_ok());

        let invalid_config = SplitConfig {
            train_pct: 0.70,
            val_pct: 0.20,
            test_pct: 0.15,
            seed: Some(42),
        };
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_determine_split() {
        let config = SplitConfig::default();
        assert_eq!(config.determine_split(0, 100), SplitType::Train);
        assert_eq!(config.determine_split(69, 100), SplitType::Train);
        assert_eq!(config.determine_split(70, 100), SplitType::Validation);
        assert_eq!(config.determine_split(84, 100), SplitType::Validation);
        assert_eq!(config.determine_split(85, 100), SplitType::Test);
        assert_eq!(config.determine_split(99, 100), SplitType::Test);
    }

    #[test]
    fn test_ml_csv_exporter_creation() {
        let _exporter = MlCsvExporter::new();
        // Just verify it constructs
    }

    #[test]
    fn test_hr_zone_calculation() {
        let exporter = MlCsvExporter::new();
        let athlete = create_test_athlete();

        assert_eq!(exporter.get_hr_zone(100, &athlete), 1);
        assert_eq!(exporter.get_hr_zone(120, &athlete), 2);
        assert_eq!(exporter.get_hr_zone(140, &athlete), 3);
        assert_eq!(exporter.get_hr_zone(160, &athlete), 4);
        assert_eq!(exporter.get_hr_zone(180, &athlete), 5);
    }

    #[test]
    fn test_pace_metrics_calculation() {
        let exporter = MlCsvExporter::new();
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let workout = create_test_workout("test", date);

        let (_avg_pace, _best_pace) = exporter.calculate_pace_metrics(&workout);
        // Pace calculation just returns from summary, which is None for this cycling workout
        // Test passes if it doesn't panic
    }

    #[test]
    fn test_export_with_features() {
        let exporter = MlCsvExporter::new();
        let athlete = create_test_athlete();
        let base_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let workouts = vec![
            create_test_workout("workout1", base_date),
            create_test_workout("workout2", base_date + chrono::Duration::days(1)),
            create_test_workout("workout3", base_date + chrono::Duration::days(2)),
        ];

        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_ml_export.csv");

        let result = exporter.export_with_features(&workouts, &athlete, &output_path, None);
        assert!(result.is_ok());

        // Verify file was created
        assert!(output_path.exists());

        // Cleanup
        let _ = std::fs::remove_file(output_path);
    }
}
