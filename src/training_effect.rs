//! Training Effect calculation module
//!
//! Implements aerobic and anaerobic training effect metrics similar to Garmin/Firstbeat.
//! Training Effect quantifies workout impact on physiological fitness based on EPOC
//! (Excess Post-Exercise Oxygen Consumption) and high-intensity interval analysis.
//!
//! ## Training Effect Scale
//! - 0.0-1.0: None - No significant impact
//! - 1.0-2.0: Minor - Minor impact on fitness
//! - 2.0-3.0: Maintaining - Maintains current fitness level
//! - 3.0-4.0: Improving - Improves fitness level
//! - 4.0-5.0: Highly Improving - Significant fitness improvement
//! - >5.0: Overreaching - Risk of overtraining

use crate::models::{AthleteProfile, DataPoint, Workout};
use anyhow::Result;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Training Effect calculation errors
#[derive(Error, Debug)]
pub enum TrainingEffectError {
    #[error("Missing required data: {0}")]
    MissingData(String),
    #[error("Invalid athlete profile: {0}")]
    InvalidProfile(String),
    #[error("Calculation error: {0}")]
    CalculationError(String),
    #[error("Insufficient workout data: {0}")]
    InsufficientData(String),
}

/// Training Effect level classification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainingEffectLevel {
    None,           // 0.0-1.0
    Minor,          // 1.0-2.0
    Maintaining,    // 2.0-3.0
    Improving,      // 3.0-4.0
    HighlyImproving,// 4.0-5.0
    Overreaching,   // >5.0
}

impl TrainingEffectLevel {
    /// Get level from numeric value
    pub fn from_value(value: f64) -> Self {
        match value {
            v if v < 1.0 => TrainingEffectLevel::None,
            v if v < 2.0 => TrainingEffectLevel::Minor,
            v if v < 3.0 => TrainingEffectLevel::Maintaining,
            v if v < 4.0 => TrainingEffectLevel::Improving,
            v if v < 5.0 => TrainingEffectLevel::HighlyImproving,
            _ => TrainingEffectLevel::Overreaching,
        }
    }

    /// Get description of training effect level
    pub fn description(&self) -> &str {
        match self {
            TrainingEffectLevel::None => "No significant impact on fitness",
            TrainingEffectLevel::Minor => "Minor impact on fitness",
            TrainingEffectLevel::Maintaining => "Maintains current fitness level",
            TrainingEffectLevel::Improving => "Improves fitness level",
            TrainingEffectLevel::HighlyImproving => "Significant fitness improvement",
            TrainingEffectLevel::Overreaching => "Risk of overtraining - consider recovery",
        }
    }
}

/// Training Effect result with aerobic and anaerobic components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingEffect {
    /// Aerobic Training Effect (0.0-5.0+)
    pub aerobic_te: f64,

    /// Anaerobic Training Effect (0.0-5.0+)
    pub anaerobic_te: f64,

    /// Estimated EPOC in ml O2/kg
    pub epoc: f64,

    /// Workout identifier
    pub workout_id: String,

    /// Date of workout
    pub date: NaiveDate,

    /// Aerobic TE level classification
    pub aerobic_level: TrainingEffectLevel,

    /// Anaerobic TE level classification
    pub anaerobic_level: TrainingEffectLevel,

    /// Recommended recovery time in hours
    pub recovery_time_hours: u32,
}

impl TrainingEffect {
    /// Get total training effect (combined aerobic and anaerobic)
    pub fn total_effect(&self) -> f64 {
        // Weighted combination: aerobic typically contributes more to total effect
        self.aerobic_te * 0.7 + self.anaerobic_te * 0.3
    }

    /// Check if workout was high intensity
    pub fn is_high_intensity(&self) -> bool {
        self.anaerobic_te > 2.0
    }

    /// Check if workout was endurance focused
    pub fn is_endurance_focused(&self) -> bool {
        self.aerobic_te > 2.0 && self.anaerobic_te < 2.0
    }

    /// Get recommended recovery days
    pub fn recovery_days(&self) -> u8 {
        (self.recovery_time_hours / 24) as u8
    }
}

/// Heart rate zone distribution for EPOC calculation
#[derive(Debug, Clone)]
struct ZoneDistribution {
    /// Time in zone 1 (recovery) in seconds
    zone1_time: u32,
    /// Time in zone 2 (aerobic) in seconds
    zone2_time: u32,
    /// Time in zone 3 (tempo) in seconds
    zone3_time: u32,
    /// Time in zone 4 (threshold) in seconds
    zone4_time: u32,
    /// Time in zone 5 (VO2max) in seconds
    zone5_time: u32,
}

impl ZoneDistribution {
    /// Calculate total time in zones
    fn total_time(&self) -> u32 {
        self.zone1_time + self.zone2_time + self.zone3_time + self.zone4_time + self.zone5_time
    }

    /// Calculate percentage in high intensity zones (4-5)
    fn high_intensity_percentage(&self) -> f64 {
        let total = self.total_time();
        if total == 0 {
            return 0.0;
        }
        ((self.zone4_time + self.zone5_time) as f64 / total as f64) * 100.0
    }
}

/// Training Effect analyzer
pub struct TrainingEffectAnalyzer;

impl TrainingEffectAnalyzer {
    /// Calculate training effect for a workout
    ///
    /// Combines EPOC estimation with time-in-zone analysis to determine
    /// aerobic and anaerobic training effects.
    pub fn calculate_training_effect(
        workout: &Workout,
        athlete: &AthleteProfile,
    ) -> Result<TrainingEffect, TrainingEffectError> {
        // Validate athlete profile has required data
        let max_hr = athlete.max_hr.ok_or_else(|| {
            TrainingEffectError::InvalidProfile("Max heart rate required".to_string())
        })?;

        let resting_hr = athlete.resting_hr.ok_or_else(|| {
            TrainingEffectError::InvalidProfile("Resting heart rate required".to_string())
        })?;

        // Get raw data points
        let data = workout.raw_data.as_ref().ok_or_else(|| {
            TrainingEffectError::MissingData("Raw heart rate data required".to_string())
        })?;

        if data.is_empty() {
            return Err(TrainingEffectError::InsufficientData(
                "No data points in workout".to_string(),
            ));
        }

        // Calculate EPOC from heart rate response
        let epoc = Self::estimate_epoc(data, max_hr, resting_hr)?;

        // Analyze time in zones
        let zone_dist = Self::calculate_zone_distribution(data, max_hr, resting_hr);

        // Calculate aerobic training effect from EPOC
        let aerobic_te = Self::calculate_aerobic_te(epoc, workout.duration_seconds);

        // Calculate anaerobic training effect from high-intensity intervals
        let anaerobic_te = Self::calculate_anaerobic_te(&zone_dist, data);

        // Calculate recovery time recommendation
        let recovery_time_hours = Self::calculate_recovery_time(&TrainingEffect {
            aerobic_te,
            anaerobic_te,
            epoc,
            workout_id: workout.id.clone(),
            date: workout.date,
            aerobic_level: TrainingEffectLevel::from_value(aerobic_te),
            anaerobic_level: TrainingEffectLevel::from_value(anaerobic_te),
            recovery_time_hours: 0, // Placeholder
        }, athlete.lthr.unwrap_or(max_hr - 20));

        Ok(TrainingEffect {
            aerobic_te,
            anaerobic_te,
            epoc,
            workout_id: workout.id.clone(),
            date: workout.date,
            aerobic_level: TrainingEffectLevel::from_value(aerobic_te),
            anaerobic_level: TrainingEffectLevel::from_value(anaerobic_te),
            recovery_time_hours,
        })
    }

    /// Estimate EPOC from heart rate data
    ///
    /// Uses a simplified model based on Firstbeat methodology:
    /// - EPOC accumulates based on exercise intensity (%HRmax)
    /// - Higher intensities (>75% HRmax) accumulate EPOC faster
    /// - Low intensities (<65% HRmax) accumulate slowly
    ///
    /// Formula approximation: EPOC(t) = f(EPOC(t-1), intensity(t), dt)
    pub fn estimate_epoc(
        data: &[DataPoint],
        max_hr: u16,
        resting_hr: u16,
    ) -> Result<f64, TrainingEffectError> {
        if data.is_empty() {
            return Err(TrainingEffectError::InsufficientData(
                "No data points provided".to_string(),
            ));
        }

        let mut cumulative_epoc = 0.0;
        let hr_reserve = (max_hr - resting_hr) as f64;

        for i in 1..data.len() {
            let prev_point = &data[i - 1];
            let curr_point = &data[i];

            // Get heart rate at current point
            let hr = match curr_point.heart_rate {
                Some(h) => h as f64,
                None => continue, // Skip points without HR data
            };

            // Calculate time delta in seconds
            let dt = (curr_point.timestamp - prev_point.timestamp) as f64;

            // Calculate heart rate reserve percentage (Karvonen formula)
            let hr_reserve_pct = ((hr - resting_hr as f64) / hr_reserve) * 100.0;

            // Calculate VO2max percentage (approximate from HR reserve)
            // VO2R% ≈ HRR% (Swain formula)
            let vo2max_pct = hr_reserve_pct;

            // EPOC accumulation rate based on intensity (ml O2/kg per minute)
            // Based on research showing EPOC accumulates faster at higher intensities
            let epoc_rate = if vo2max_pct < 30.0 {
                // Very low intensity: minimal EPOC accumulation
                0.05
            } else if vo2max_pct < 50.0 {
                // Low intensity: slow EPOC accumulation
                0.2 + (vo2max_pct - 30.0) * 0.02
            } else if vo2max_pct < 70.0 {
                // Moderate intensity: steady EPOC accumulation
                0.6 + (vo2max_pct - 50.0) * 0.03
            } else if vo2max_pct < 85.0 {
                // High-moderate intensity: increasing EPOC accumulation
                1.2 + (vo2max_pct - 70.0) * 0.05
            } else {
                // High intensity: rapid EPOC accumulation
                2.0 + (vo2max_pct - 85.0) * 0.08
            };

            // Add EPOC for this time segment
            // EPOC in ml O2/kg accumulates over time
            let epoc_increment = epoc_rate * (dt / 60.0); // Normalize to per-minute
            cumulative_epoc += epoc_increment;
        }

        Ok(cumulative_epoc)
    }

    /// Calculate zone distribution from heart rate data
    fn calculate_zone_distribution(
        data: &[DataPoint],
        max_hr: u16,
        resting_hr: u16,
    ) -> ZoneDistribution {
        let mut dist = ZoneDistribution {
            zone1_time: 0,
            zone2_time: 0,
            zone3_time: 0,
            zone4_time: 0,
            zone5_time: 0,
        };

        let hr_reserve = max_hr - resting_hr;

        for i in 1..data.len() {
            let prev_point = &data[i - 1];
            let curr_point = &data[i];

            if let Some(hr) = curr_point.heart_rate {
                let dt = curr_point.timestamp - prev_point.timestamp;

                // Calculate zone based on % HRR (Karvonen zones)
                let hr_reserve_pct = if hr > resting_hr {
                    ((hr - resting_hr) as f64 / hr_reserve as f64) * 100.0
                } else {
                    0.0
                };

                // Assign to zone
                if hr_reserve_pct < 60.0 {
                    dist.zone1_time += dt;
                } else if hr_reserve_pct < 70.0 {
                    dist.zone2_time += dt;
                } else if hr_reserve_pct < 80.0 {
                    dist.zone3_time += dt;
                } else if hr_reserve_pct < 90.0 {
                    dist.zone4_time += dt;
                } else {
                    dist.zone5_time += dt;
                }
            }
        }

        dist
    }

    /// Calculate aerobic training effect from EPOC
    ///
    /// Based on Firstbeat research: Aerobic TE correlates with peak EPOC
    /// - EPOC < 20: TE < 1.0 (None)
    /// - EPOC 20-40: TE 1.0-2.0 (Minor)
    /// - EPOC 40-60: TE 2.0-3.0 (Maintaining)
    /// - EPOC 60-100: TE 3.0-4.0 (Improving)
    /// - EPOC > 100: TE 4.0-5.0+ (Highly Improving)
    fn calculate_aerobic_te(epoc: f64, duration_seconds: u32) -> f64 {
        // Normalize EPOC by duration (longer workouts accumulate more EPOC)
        let duration_hours = duration_seconds as f64 / 3600.0;
        let normalized_epoc = epoc / duration_hours.max(0.5); // Avoid division by very small durations

        // Map normalized EPOC to training effect scale
        let te = if normalized_epoc < 20.0 {
            0.5 + (normalized_epoc / 20.0) * 0.5
        } else if normalized_epoc < 40.0 {
            1.0 + ((normalized_epoc - 20.0) / 20.0) * 1.0
        } else if normalized_epoc < 60.0 {
            2.0 + ((normalized_epoc - 40.0) / 20.0) * 1.0
        } else if normalized_epoc < 100.0 {
            3.0 + ((normalized_epoc - 60.0) / 40.0) * 1.0
        } else {
            4.0 + ((normalized_epoc - 100.0) / 50.0).min(1.5)
        };

        te.min(5.5) // Cap at 5.5 (overreaching territory)
    }

    /// Calculate anaerobic training effect from high-intensity intervals
    ///
    /// Analyzes time in high-intensity zones and interval structure:
    /// - High-intensity time percentage
    /// - Number and duration of high-intensity bouts
    /// - Recovery between intervals
    fn calculate_anaerobic_te(zone_dist: &ZoneDistribution, data: &[DataPoint]) -> f64 {
        // Calculate percentage in high-intensity zones (4-5)
        let high_intensity_pct = zone_dist.high_intensity_percentage();

        // Count high-intensity intervals (Z4-Z5 bouts)
        let interval_count = Self::count_high_intensity_intervals(data);

        // Base anaerobic TE on time percentage in high zones
        let base_te = if high_intensity_pct < 5.0 {
            0.0 + high_intensity_pct * 0.1
        } else if high_intensity_pct < 15.0 {
            0.5 + (high_intensity_pct - 5.0) * 0.1
        } else if high_intensity_pct < 25.0 {
            1.5 + (high_intensity_pct - 15.0) * 0.08
        } else {
            2.3 + (high_intensity_pct - 25.0) * 0.06
        };

        // Boost based on interval structure (more intervals = more anaerobic stimulus)
        let interval_bonus = (interval_count as f64 * 0.15).min(1.5);

        let total_te = base_te + interval_bonus;
        total_te.min(5.5) // Cap at 5.5
    }

    /// Count high-intensity interval bouts
    fn count_high_intensity_intervals(data: &[DataPoint]) -> usize {
        let mut intervals = 0;
        let mut in_high_intensity = false;
        let mut high_intensity_duration = 0;

        for point in data {
            if let Some(hr) = point.heart_rate {
                // Simple threshold: >85% of typical max (approximate)
                let is_high = hr > 160; // Simplified threshold

                if is_high && !in_high_intensity {
                    in_high_intensity = true;
                    high_intensity_duration = 0;
                } else if is_high {
                    high_intensity_duration += 1;
                } else if in_high_intensity {
                    // End of high-intensity bout
                    if high_intensity_duration >= 10 { // At least 10 seconds
                        intervals += 1;
                    }
                    in_high_intensity = false;
                }
            }
        }

        // Count final bout if still in high intensity
        if in_high_intensity && high_intensity_duration >= 10 {
            intervals += 1;
        }

        intervals
    }

    /// Calculate recovery time recommendation
    ///
    /// Based on training effect magnitude and current fitness level:
    /// - Higher TE = longer recovery
    /// - Consider both aerobic and anaerobic components
    pub fn calculate_recovery_time(te: &TrainingEffect, fitness_level: u16) -> u32 {
        // Total effect weighted by component
        let total_effect = te.total_effect();

        // Base recovery hours based on total effect
        let base_recovery = if total_effect < 1.0 {
            12
        } else if total_effect < 2.0 {
            24
        } else if total_effect < 3.0 {
            36
        } else if total_effect < 4.0 {
            48
        } else {
            72
        };

        // Adjust for fitness level (higher fitness = faster recovery)
        // fitness_level typically represents LTHR or similar metric
        let fitness_factor = if fitness_level > 170 {
            0.8 // Well-trained athlete
        } else if fitness_level > 160 {
            0.9 // Trained athlete
        } else {
            1.0 // Average athlete
        };

        // Extra recovery for high anaerobic load
        let anaerobic_penalty = if te.anaerobic_te > 3.0 {
            12
        } else if te.anaerobic_te > 2.0 {
            6
        } else {
            0
        };

        (base_recovery as f64 * fitness_factor) as u32 + anaerobic_penalty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DataPoint, Sport, Workout, WorkoutSummary, WorkoutType, DataSource};
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn create_test_athlete() -> AthleteProfile {
        AthleteProfile {
            id: "test_athlete".to_string(),
            name: "Test Athlete".to_string(),
            date_of_birth: Some(NaiveDate::from_ymd_opt(1990, 1, 1).unwrap()),
            weight: Some(dec!(75.0)),
            height: Some(180),
            ftp: Some(250),
            lthr: Some(165),
            threshold_pace: Some(dec!(6.0)),
            max_hr: Some(190),
            resting_hr: Some(50),
            training_zones: Default::default(),
            preferred_units: crate::models::Units::Metric,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn create_test_data_steady_zone2() -> Vec<DataPoint> {
        (0..3600)
            .step_by(1)
            .map(|t| DataPoint {
                timestamp: t,
                heart_rate: Some(130), // Steady Zone 2
                power: None,
                pace: None,
                elevation: None,
                cadence: Some(85),
                speed: None,
                distance: None,
                left_power: None,
                right_power: None,
                ground_contact_time: None,
                vertical_oscillation: None,
                stride_length: None,
                stroke_count: None,
                stroke_type: None,
                lap_number: None,
                sport_transition: None,
            })
            .collect()
    }

    fn create_test_data_intervals() -> Vec<DataPoint> {
        let mut data = Vec::new();

        // Warm-up: 10 minutes at 130 bpm
        for t in 0..600 {
            data.push(DataPoint {
                timestamp: t,
                heart_rate: Some(130),
                power: None,
                pace: None,
                elevation: None,
                cadence: Some(85),
                speed: None,
                distance: None,
                left_power: None,
                right_power: None,
                ground_contact_time: None,
                vertical_oscillation: None,
                stride_length: None,
                stroke_count: None,
                stroke_type: None,
                lap_number: None,
                sport_transition: None,
            });
        }

        // 5x (3min @ 180bpm, 2min @ 120bpm) = 25 minutes
        for interval in 0..5 {
            let base_time = 600 + interval * 300;

            // 3 minutes high intensity
            for t in 0..180 {
                data.push(DataPoint {
                    timestamp: base_time + t,
                    heart_rate: Some(180),
                    power: None,
                    pace: None,
                    elevation: None,
                    cadence: Some(95),
                    speed: None,
                    distance: None,
                    left_power: None,
                    right_power: None,
                    ground_contact_time: None,
                    vertical_oscillation: None,
                    stride_length: None,
                    stroke_count: None,
                    stroke_type: None,
                    lap_number: None,
                    sport_transition: None,
                });
            }

            // 2 minutes recovery
            for t in 180..300 {
                data.push(DataPoint {
                    timestamp: base_time + t,
                    heart_rate: Some(120),
                    power: None,
                    pace: None,
                    elevation: None,
                    cadence: Some(80),
                    speed: None,
                    distance: None,
                    left_power: None,
                    right_power: None,
                    ground_contact_time: None,
                    vertical_oscillation: None,
                    stride_length: None,
                    stroke_count: None,
                    stroke_type: None,
                    lap_number: None,
                    sport_transition: None,
                });
            }
        }

        // Cool-down: 5 minutes at 120 bpm
        let cool_start = 600 + 5 * 300;
        for t in 0..300 {
            data.push(DataPoint {
                timestamp: cool_start + t,
                heart_rate: Some(120),
                power: None,
                pace: None,
                elevation: None,
                cadence: Some(80),
                speed: None,
                distance: None,
                left_power: None,
                right_power: None,
                ground_contact_time: None,
                vertical_oscillation: None,
                stride_length: None,
                stroke_count: None,
                stroke_type: None,
                lap_number: None,
                sport_transition: None,
            });
        }

        data
    }

    #[test]
    fn test_training_effect_level_classification() {
        assert_eq!(TrainingEffectLevel::from_value(0.5), TrainingEffectLevel::None);
        assert_eq!(TrainingEffectLevel::from_value(1.5), TrainingEffectLevel::Minor);
        assert_eq!(TrainingEffectLevel::from_value(2.5), TrainingEffectLevel::Maintaining);
        assert_eq!(TrainingEffectLevel::from_value(3.5), TrainingEffectLevel::Improving);
        assert_eq!(TrainingEffectLevel::from_value(4.5), TrainingEffectLevel::HighlyImproving);
        assert_eq!(TrainingEffectLevel::from_value(5.5), TrainingEffectLevel::Overreaching);
    }

    #[test]
    fn test_epoc_estimation_steady_state() {
        let data = create_test_data_steady_zone2();
        let athlete = create_test_athlete();

        let epoc = TrainingEffectAnalyzer::estimate_epoc(
            &data,
            athlete.max_hr.unwrap(),
            athlete.resting_hr.unwrap(),
        ).unwrap();

        // Steady Zone 2 should produce moderate EPOC
        assert!(epoc > 10.0, "EPOC should be > 10 for 1hr Zone 2");
        assert!(epoc < 60.0, "EPOC should be < 60 for steady Zone 2");
    }

    #[test]
    fn test_epoc_estimation_intervals() {
        let data = create_test_data_intervals();
        let athlete = create_test_athlete();

        let epoc = TrainingEffectAnalyzer::estimate_epoc(
            &data,
            athlete.max_hr.unwrap(),
            athlete.resting_hr.unwrap(),
        ).unwrap();

        // Interval workout should produce high EPOC
        assert!(epoc > 30.0, "EPOC should be > 30 for interval workout");
    }

    #[test]
    fn test_training_effect_steady_endurance() {
        let athlete = create_test_athlete();
        let data = create_test_data_steady_zone2();

        let workout = Workout {
            id: "steady_endurance".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            sport: Sport::Running,
            duration_seconds: 3600,
            workout_type: WorkoutType::Endurance,
            data_source: DataSource::HeartRate,
            raw_data: Some(data),
            summary: WorkoutSummary::default(),
            notes: None,
            athlete_id: Some("test_athlete".to_string()),
            source: None,
        };

        let te = TrainingEffectAnalyzer::calculate_training_effect(&workout, &athlete).unwrap();

        // Steady endurance: moderate aerobic, low anaerobic
        assert!(te.aerobic_te >= 2.0, "Aerobic TE should be >= 2.0 for 1hr Zone 2");
        assert!(te.aerobic_te <= 3.5, "Aerobic TE should be <= 3.5 for Zone 2");
        assert!(te.anaerobic_te < 1.5, "Anaerobic TE should be low for steady Zone 2");
        assert!(te.is_endurance_focused());
    }

    #[test]
    fn test_training_effect_intervals() {
        let athlete = create_test_athlete();
        let data = create_test_data_intervals();

        let workout = Workout {
            id: "interval_workout".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            sport: Sport::Running,
            duration_seconds: 2400, // 40 minutes
            workout_type: WorkoutType::Interval,
            data_source: DataSource::HeartRate,
            raw_data: Some(data),
            summary: WorkoutSummary::default(),
            notes: None,
            athlete_id: Some("test_athlete".to_string()),
            source: None,
        };

        let te = TrainingEffectAnalyzer::calculate_training_effect(&workout, &athlete).unwrap();

        // Interval workout: high aerobic and significant anaerobic
        assert!(te.aerobic_te >= 2.5, "Aerobic TE should be >= 2.5 for intervals");
        assert!(te.anaerobic_te >= 2.0, "Anaerobic TE should be >= 2.0 for intervals");
        assert!(te.is_high_intensity());
    }

    #[test]
    fn test_recovery_time_calculation() {
        let te = TrainingEffect {
            aerobic_te: 3.5,
            anaerobic_te: 2.5,
            epoc: 75.0,
            workout_id: "test".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            aerobic_level: TrainingEffectLevel::Improving,
            anaerobic_level: TrainingEffectLevel::Maintaining,
            recovery_time_hours: 0,
        };

        let recovery = TrainingEffectAnalyzer::calculate_recovery_time(&te, 165);

        // Improving workout should require ~36-48 hours recovery
        assert!(recovery >= 36, "Recovery should be >= 36 hours for TE 3.0");
        assert!(recovery <= 72, "Recovery should be <= 72 hours");
    }

    #[test]
    fn test_missing_heart_rate_data() {
        let athlete = create_test_athlete();
        let workout = Workout {
            id: "no_data".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            sport: Sport::Running,
            duration_seconds: 3600,
            workout_type: WorkoutType::Endurance,
            data_source: DataSource::HeartRate,
            raw_data: None, // No data
            summary: WorkoutSummary::default(),
            notes: None,
            athlete_id: Some("test_athlete".to_string()),
            source: None,
        };

        let result = TrainingEffectAnalyzer::calculate_training_effect(&workout, &athlete);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_athlete_thresholds() {
        let mut athlete = create_test_athlete();
        athlete.max_hr = None; // Missing max HR

        let data = create_test_data_steady_zone2();
        let workout = Workout {
            id: "test".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            sport: Sport::Running,
            duration_seconds: 3600,
            workout_type: WorkoutType::Endurance,
            data_source: DataSource::HeartRate,
            raw_data: Some(data),
            summary: WorkoutSummary::default(),
            notes: None,
            athlete_id: Some("test_athlete".to_string()),
            source: None,
        };

        let result = TrainingEffectAnalyzer::calculate_training_effect(&workout, &athlete);
        assert!(result.is_err());
    }
}
