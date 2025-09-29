//! Swimming-specific training analysis module
//!
//! This module provides comprehensive swimming analysis including stroke analysis,
//! SWOLF calculation, pool lap detection, and swimming efficiency metrics.

#![allow(dead_code)]

use crate::models::{DataPoint, Sport, Workout};
use anyhow::{anyhow, Result};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum SwimmingError {
    #[error("Insufficient data: {0}")]
    InsufficientData(String),
    #[error("Invalid stroke data: {0}")]
    InvalidStrokeData(String),
    #[error("Calculation error: {0}")]
    CalculationError(String),
}

/// Swimming analysis results
#[derive(Debug, Clone)]
pub struct SwimmingAnalysis {
    /// Total stroke count for the workout
    pub total_strokes: u32,
    /// Average strokes per length/lap
    pub avg_strokes_per_lap: Option<Decimal>,
    /// SWOLF analysis (Stroke + Time efficiency measure)
    pub swolf_analysis: SwolfAnalysis,
    /// Stroke type distribution
    pub stroke_distribution: StrokeDistribution,
    /// Lap analysis for pool swimming
    pub lap_analysis: Vec<LapAnalysis>,
    /// Swimming pace analysis
    pub pace_analysis: SwimmingPaceAnalysis,
    /// Efficiency metrics
    pub efficiency_metrics: SwimmingEfficiencyMetrics,
}

/// SWOLF (Swimming Golf) analysis - measures efficiency
#[derive(Debug, Clone)]
pub struct SwolfAnalysis {
    /// Average SWOLF score (strokes + time per length)
    pub avg_swolf: Option<Decimal>,
    /// Best SWOLF score in the workout
    pub best_swolf: Option<Decimal>,
    /// SWOLF consistency (coefficient of variation)
    pub swolf_variability: Option<u8>,
}

/// Distribution of different stroke types throughout the workout
#[derive(Debug, Clone)]
pub struct StrokeDistribution {
    /// Time/distance spent in each stroke type
    pub stroke_times: HashMap<SwimStrokeType, u32>, // seconds
    /// Percentage of workout in each stroke type
    pub stroke_percentages: HashMap<SwimStrokeType, Decimal>,
}

/// Analysis for individual swimming laps
#[derive(Debug, Clone)]
pub struct LapAnalysis {
    /// Lap number
    pub lap_number: u16,
    /// Duration of the lap in seconds
    pub duration_seconds: u32,
    /// Stroke count for this lap
    pub stroke_count: u16,
    /// SWOLF score for this lap (strokes + time)
    pub swolf_score: Decimal,
    /// Average pace for this lap
    pub pace: Option<Decimal>,
    /// Stroke type used in this lap
    pub stroke_type: Option<SwimStrokeType>,
    /// Distance covered in this lap (meters)
    pub distance_meters: Option<Decimal>,
}

/// Swimming pace analysis
#[derive(Debug, Clone)]
pub struct SwimmingPaceAnalysis {
    /// Average pace per 100m
    pub avg_pace_per_100m: Option<Decimal>,
    /// Best pace per 100m
    pub best_pace_per_100m: Option<Decimal>,
    /// Pace variability
    pub pace_consistency: Option<u8>,
}

/// Swimming efficiency metrics
#[derive(Debug, Clone)]
pub struct SwimmingEfficiencyMetrics {
    /// Stroke efficiency rating
    pub stroke_efficiency: Option<EfficiencyRating>,
    /// Distance per stroke
    pub distance_per_stroke: Option<Decimal>,
    /// Stroke rate (strokes per minute)
    pub stroke_rate: Option<Decimal>,
}

/// Swim stroke types supported by FIT format
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SwimStrokeType {
    Freestyle = 0,
    Backstroke = 1,
    Breaststroke = 2,
    Butterfly = 3,
    Drill = 4,
    Mixed = 5,
    IM = 6, // Individual Medley
}

impl From<u8> for SwimStrokeType {
    fn from(value: u8) -> Self {
        match value {
            0 => SwimStrokeType::Freestyle,
            1 => SwimStrokeType::Backstroke,
            2 => SwimStrokeType::Breaststroke,
            3 => SwimStrokeType::Butterfly,
            4 => SwimStrokeType::Drill,
            5 => SwimStrokeType::Mixed,
            6 => SwimStrokeType::IM,
            _ => SwimStrokeType::Freestyle, // Default fallback
        }
    }
}

/// Efficiency rating scale for swimming metrics
#[derive(Debug, Clone, PartialEq)]
pub enum EfficiencyRating {
    Excellent,
    Good,
    Average,
    BelowAverage,
    Poor,
}

/// Swimming analyzer for comprehensive swimming workout analysis
pub struct SwimmingAnalyzer;

impl SwimmingAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Analyze comprehensive swimming metrics
    pub fn analyze_swimming(workout: &Workout) -> Result<SwimmingAnalysis> {
        if workout.sport != Sport::Swimming {
            return Err(anyhow!("Workout must be a swimming activity"));
        }

        let raw_data = workout.raw_data.as_ref()
            .ok_or_else(|| SwimmingError::InsufficientData("No raw data available".to_string()))?;

        if raw_data.is_empty() {
            return Err(SwimmingError::InsufficientData("No data points available".to_string()).into());
        }

        // Filter data points with swimming data
        let swimming_data: Vec<&DataPoint> = raw_data.iter()
            .filter(|dp| dp.stroke_count.is_some() || dp.stroke_type.is_some())
            .collect();

        let total_strokes = Self::calculate_total_strokes(&swimming_data);
        let avg_strokes_per_lap = Self::calculate_avg_strokes_per_lap(&swimming_data);
        let swolf_analysis = Self::analyze_swolf(&swimming_data, raw_data)?;
        let stroke_distribution = Self::analyze_stroke_distribution(&swimming_data, workout.duration_seconds);
        let lap_analysis = Self::analyze_laps(&swimming_data, raw_data)?;
        let pace_analysis = Self::analyze_pace(&swimming_data, raw_data)?;
        let efficiency_metrics = Self::calculate_efficiency_metrics(&swimming_data, &lap_analysis)?;

        Ok(SwimmingAnalysis {
            total_strokes,
            avg_strokes_per_lap,
            swolf_analysis,
            stroke_distribution,
            lap_analysis,
            pace_analysis,
            efficiency_metrics,
        })
    }

    /// Calculate total stroke count for the workout
    fn calculate_total_strokes(swimming_data: &[&DataPoint]) -> u32 {
        swimming_data.iter()
            .filter_map(|dp| dp.stroke_count)
            .map(|count| count as u32)
            .sum()
    }

    /// Calculate average strokes per lap
    fn calculate_avg_strokes_per_lap(swimming_data: &[&DataPoint]) -> Option<Decimal> {
        // Group by lap and calculate average
        let mut lap_strokes: HashMap<u16, Vec<u16>> = HashMap::new();

        for dp in swimming_data {
            if let (Some(lap), Some(strokes)) = (dp.lap_number, dp.stroke_count) {
                lap_strokes.entry(lap).or_insert_with(Vec::new).push(strokes);
            }
        }

        if lap_strokes.is_empty() {
            return None;
        }

        let lap_averages: Vec<Decimal> = lap_strokes.values()
            .map(|strokes| {
                let sum: u16 = strokes.iter().sum();
                Decimal::from(sum) / Decimal::from(strokes.len())
            })
            .collect();

        if lap_averages.is_empty() {
            None
        } else {
            Some(lap_averages.iter().sum::<Decimal>() / Decimal::from(lap_averages.len()))
        }
    }

    /// Analyze SWOLF (Swimming Golf) scores
    fn analyze_swolf(swimming_data: &[&DataPoint], all_data: &[DataPoint]) -> Result<SwolfAnalysis> {
        // SWOLF = Strokes + Time (for a standard distance, typically 25m or 50m)
        // We'll calculate SWOLF per lap when possible

        let mut lap_swolf_scores: Vec<Decimal> = Vec::new();
        let mut current_lap_data: HashMap<u16, Vec<&DataPoint>> = HashMap::new();

        // Group data by lap
        for dp in swimming_data {
            if let Some(lap) = dp.lap_number {
                current_lap_data.entry(lap).or_insert_with(Vec::new).push(dp);
            }
        }

        // Calculate SWOLF for each lap
        for (lap_num, lap_data) in current_lap_data {
            if let Some(swolf) = Self::calculate_lap_swolf(&lap_data, all_data, lap_num) {
                lap_swolf_scores.push(swolf);
            }
        }

        let avg_swolf = if !lap_swolf_scores.is_empty() {
            Some(lap_swolf_scores.iter().sum::<Decimal>() / Decimal::from(lap_swolf_scores.len()))
        } else {
            None
        };

        let best_swolf = lap_swolf_scores.iter().min().copied();

        // Calculate SWOLF variability (coefficient of variation)
        let swolf_variability = if lap_swolf_scores.len() > 1 && avg_swolf.is_some() {
            let mean = avg_swolf.unwrap();
            let variance = lap_swolf_scores.iter()
                .map(|&score| {
                    let diff = score - mean;
                    diff * diff
                })
                .sum::<Decimal>() / Decimal::from(lap_swolf_scores.len());

            let std_dev = Decimal::from_f64_retain(variance.to_f64().unwrap_or(0.0).sqrt()).unwrap_or_default();
            let cv = if mean > Decimal::ZERO {
                (std_dev / mean * dec!(100)).to_u8().unwrap_or(0)
            } else {
                0
            };
            Some(cv)
        } else {
            None
        };

        Ok(SwolfAnalysis {
            avg_swolf,
            best_swolf,
            swolf_variability,
        })
    }

    /// Calculate SWOLF score for a specific lap
    fn calculate_lap_swolf(lap_data: &[&DataPoint], all_data: &[DataPoint], lap_num: u16) -> Option<Decimal> {
        // Find total strokes for this lap
        let total_strokes: u16 = lap_data.iter()
            .filter_map(|dp| dp.stroke_count)
            .sum();

        // Find lap duration by looking at timestamp range for this lap
        let lap_timestamps: Vec<u32> = all_data.iter()
            .filter(|dp| dp.lap_number == Some(lap_num))
            .map(|dp| dp.timestamp)
            .collect();

        if lap_timestamps.is_empty() {
            return None;
        }

        let lap_start = *lap_timestamps.iter().min()?;
        let lap_end = *lap_timestamps.iter().max()?;
        let lap_duration = lap_end - lap_start;

        if total_strokes > 0 && lap_duration > 0 {
            Some(Decimal::from(total_strokes) + Decimal::from(lap_duration))
        } else {
            None
        }
    }

    /// Analyze stroke type distribution
    fn analyze_stroke_distribution(swimming_data: &[&DataPoint], total_duration: u32) -> StrokeDistribution {
        let mut stroke_times: HashMap<SwimStrokeType, u32> = HashMap::new();

        for dp in swimming_data {
            if let Some(stroke_type_raw) = dp.stroke_type {
                let stroke_type = SwimStrokeType::from(stroke_type_raw);
                *stroke_times.entry(stroke_type).or_insert(0) += 1; // Count data points
            }
        }

        // Convert counts to approximate time (assuming 1 data point per second)
        let stroke_percentages = if total_duration > 0 {
            stroke_times.iter()
                .map(|(stroke_type, &time)| {
                    let percentage = Decimal::from(time) / Decimal::from(total_duration) * dec!(100);
                    (stroke_type.clone(), percentage)
                })
                .collect()
        } else {
            HashMap::new()
        };

        StrokeDistribution {
            stroke_times,
            stroke_percentages,
        }
    }

    /// Analyze individual lap performance
    fn analyze_laps(swimming_data: &[&DataPoint], all_data: &[DataPoint]) -> Result<Vec<LapAnalysis>> {
        let mut lap_analysis = Vec::new();
        let mut lap_data: HashMap<u16, Vec<&DataPoint>> = HashMap::new();

        // Group swimming data by lap
        for dp in swimming_data {
            if let Some(lap) = dp.lap_number {
                lap_data.entry(lap).or_insert_with(Vec::new).push(dp);
            }
        }

        for (lap_num, lap_points) in lap_data {
            // Calculate lap metrics
            let stroke_count: u16 = lap_points.iter()
                .filter_map(|dp| dp.stroke_count)
                .sum();

            let stroke_type = lap_points.iter()
                .filter_map(|dp| dp.stroke_type)
                .next()
                .map(SwimStrokeType::from);

            // Find lap duration and distance
            let lap_timestamps: Vec<u32> = all_data.iter()
                .filter(|dp| dp.lap_number == Some(lap_num))
                .map(|dp| dp.timestamp)
                .collect();

            let duration_seconds = if !lap_timestamps.is_empty() {
                let lap_start = *lap_timestamps.iter().min().unwrap();
                let lap_end = *lap_timestamps.iter().max().unwrap();
                lap_end - lap_start
            } else {
                0
            };

            // Calculate distance for this lap (if available)
            let distance_meters = all_data.iter()
                .filter(|dp| dp.lap_number == Some(lap_num))
                .filter_map(|dp| dp.distance)
                .max(); // Take the maximum distance value in the lap

            // Calculate SWOLF score
            let swolf_score = if stroke_count > 0 && duration_seconds > 0 {
                Decimal::from(stroke_count) + Decimal::from(duration_seconds)
            } else {
                Decimal::ZERO
            };

            // Calculate pace (time per 100m if distance is available)
            let pace = if let Some(distance) = distance_meters {
                if distance > Decimal::ZERO && duration_seconds > 0 {
                    let pace_per_100m = Decimal::from(duration_seconds) / distance * dec!(100);
                    Some(pace_per_100m)
                } else {
                    None
                }
            } else {
                None
            };

            lap_analysis.push(LapAnalysis {
                lap_number: lap_num,
                duration_seconds,
                stroke_count,
                swolf_score,
                pace,
                stroke_type,
                distance_meters,
            });
        }

        // Sort by lap number
        lap_analysis.sort_by_key(|lap| lap.lap_number);

        Ok(lap_analysis)
    }

    /// Analyze swimming pace metrics
    fn analyze_pace(_swimming_data: &[&DataPoint], all_data: &[DataPoint]) -> Result<SwimmingPaceAnalysis> {
        // Calculate pace per 100m from speed and distance data
        let pace_values: Vec<Decimal> = all_data.iter()
            .filter_map(|dp| {
                if let Some(speed) = dp.speed {
                    if speed > Decimal::ZERO {
                        // Convert speed (m/s) to pace per 100m (seconds per 100m)
                        Some(dec!(100) / speed)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        let avg_pace_per_100m = if !pace_values.is_empty() {
            Some(pace_values.iter().sum::<Decimal>() / Decimal::from(pace_values.len()))
        } else {
            None
        };

        let best_pace_per_100m = pace_values.iter().min().copied();

        // Calculate pace consistency
        let pace_consistency = if pace_values.len() > 1 && avg_pace_per_100m.is_some() {
            let mean = avg_pace_per_100m.unwrap();
            let variance = pace_values.iter()
                .map(|&pace| {
                    let diff = pace - mean;
                    diff * diff
                })
                .sum::<Decimal>() / Decimal::from(pace_values.len());

            let std_dev = Decimal::from_f64_retain(variance.to_f64().unwrap_or(0.0).sqrt()).unwrap_or_default();
            let cv = if mean > Decimal::ZERO {
                (std_dev / mean * dec!(100)).to_u8().unwrap_or(0)
            } else {
                0
            };
            Some(cv)
        } else {
            None
        };

        Ok(SwimmingPaceAnalysis {
            avg_pace_per_100m,
            best_pace_per_100m,
            pace_consistency,
        })
    }

    /// Calculate swimming efficiency metrics
    fn calculate_efficiency_metrics(
        _swimming_data: &[&DataPoint],
        lap_analysis: &[LapAnalysis]
    ) -> Result<SwimmingEfficiencyMetrics> {

        // Calculate average distance per stroke
        let distance_per_stroke = if !lap_analysis.is_empty() {
            let valid_laps: Vec<&LapAnalysis> = lap_analysis.iter()
                .filter(|lap| lap.distance_meters.is_some() && lap.stroke_count > 0)
                .collect();

            if !valid_laps.is_empty() {
                let total_distance: Decimal = valid_laps.iter()
                    .filter_map(|lap| lap.distance_meters)
                    .sum();
                let total_strokes: u16 = valid_laps.iter()
                    .map(|lap| lap.stroke_count)
                    .sum();

                if total_strokes > 0 {
                    Some(total_distance / Decimal::from(total_strokes))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Calculate stroke rate (strokes per minute)
        let stroke_rate = if !lap_analysis.is_empty() {
            let valid_laps: Vec<&LapAnalysis> = lap_analysis.iter()
                .filter(|lap| lap.stroke_count > 0 && lap.duration_seconds > 0)
                .collect();

            if !valid_laps.is_empty() {
                let total_strokes: u16 = valid_laps.iter()
                    .map(|lap| lap.stroke_count)
                    .sum();
                let total_time: u32 = valid_laps.iter()
                    .map(|lap| lap.duration_seconds)
                    .sum();

                if total_time > 0 {
                    Some(Decimal::from(total_strokes) * dec!(60) / Decimal::from(total_time))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Rate stroke efficiency based on distance per stroke
        let stroke_efficiency = distance_per_stroke.map(|dps| {
            // These are general guidelines - actual efficiency varies by stroke type and pool length
            if dps >= dec!(2.5) {
                EfficiencyRating::Excellent
            } else if dps >= dec!(2.0) {
                EfficiencyRating::Good
            } else if dps >= dec!(1.5) {
                EfficiencyRating::Average
            } else if dps >= dec!(1.0) {
                EfficiencyRating::BelowAverage
            } else {
                EfficiencyRating::Poor
            }
        });

        Ok(SwimmingEfficiencyMetrics {
            stroke_efficiency,
            distance_per_stroke,
            stroke_rate,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{WorkoutSummary, WorkoutType, DataSource};
    use chrono::NaiveDate;
    use uuid::Uuid;

    fn create_sample_swimming_data() -> Vec<DataPoint> {
        vec![
            // Start of lap 1
            DataPoint {
                timestamp: 0,
                heart_rate: Some(130),
                power: None,
                pace: None,
                elevation: None,
                cadence: None,
                speed: Some(dec!(1.5)), // 1.5 m/s swimming speed
                distance: Some(dec!(0.0)), // Start of lap
                left_power: None,
                right_power: None,
                ground_contact_time: None,
                vertical_oscillation: None,
                stride_length: None,
                stroke_count: Some(0), // Start of lap
                stroke_type: Some(0), // Freestyle
                lap_number: Some(1),
                sport_transition: None,
            },
            // End of lap 1 / 25m mark
            DataPoint {
                timestamp: 20, // 20 seconds for 25m
                heart_rate: Some(132),
                power: None,
                pace: None,
                elevation: None,
                cadence: None,
                speed: Some(dec!(1.5)),
                distance: Some(dec!(25.0)), // 25m pool length
                left_power: None,
                right_power: None,
                ground_contact_time: None,
                vertical_oscillation: None,
                stride_length: None,
                stroke_count: Some(20), // 20 strokes for 25m
                stroke_type: Some(0), // Freestyle
                lap_number: Some(1),
                sport_transition: None,
            },
            // Start of lap 2
            DataPoint {
                timestamp: 20, // Same time as lap 1 end
                heart_rate: Some(134),
                power: None,
                pace: None,
                elevation: None,
                cadence: None,
                speed: Some(dec!(1.4)),
                distance: Some(dec!(25.0)), // Reset for new lap
                left_power: None,
                right_power: None,
                ground_contact_time: None,
                vertical_oscillation: None,
                stride_length: None,
                stroke_count: Some(0), // Start of lap 2
                stroke_type: Some(0), // Freestyle
                lap_number: Some(2),
                sport_transition: None,
            },
            // End of lap 2 / 50m mark
            DataPoint {
                timestamp: 42, // 22 seconds for second 25m
                heart_rate: Some(135),
                power: None,
                pace: None,
                elevation: None,
                cadence: None,
                speed: Some(dec!(1.4)),
                distance: Some(dec!(50.0)), // 50m total
                left_power: None,
                right_power: None,
                ground_contact_time: None,
                vertical_oscillation: None,
                stride_length: None,
                stroke_count: Some(22), // 22 strokes for second 25m
                stroke_type: Some(0), // Freestyle
                lap_number: Some(2),
                sport_transition: None,
            },
        ]
    }

    #[test]
    fn test_swimming_analysis() {
        let swimming_data = create_sample_swimming_data();

        let workout = Workout {
            id: Uuid::new_v4().to_string(),
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            sport: Sport::Swimming,
            duration_seconds: 60,
            workout_type: WorkoutType::Endurance,
            data_source: DataSource::Pace,
            raw_data: Some(swimming_data),
            summary: WorkoutSummary {
                avg_heart_rate: Some(132),
                max_heart_rate: Some(135),
                avg_power: None,
                normalized_power: None,
                avg_pace: Some(dec!(18.5)), // Average pace
                intensity_factor: None,
                tss: None,
                total_distance: Some(dec!(50.0)),
                elevation_gain: None,
                avg_cadence: None,
                calories: None,
            },
            notes: None,
            athlete_id: None,
            source: None,
        };

        let analysis = SwimmingAnalyzer::analyze_swimming(&workout).unwrap();

        // Test total strokes
        assert_eq!(analysis.total_strokes, 42); // 20 + 22

        // Test lap analysis
        assert_eq!(analysis.lap_analysis.len(), 2);
        assert_eq!(analysis.lap_analysis[0].stroke_count, 20);
        assert_eq!(analysis.lap_analysis[1].stroke_count, 22);

        // Test stroke distribution
        assert!(analysis.stroke_distribution.stroke_times.contains_key(&SwimStrokeType::Freestyle));

        // Test efficiency metrics
        assert!(analysis.efficiency_metrics.distance_per_stroke.is_some());
        assert!(analysis.efficiency_metrics.stroke_rate.is_some());
    }

    #[test]
    fn test_swolf_calculation() {
        let swimming_data = create_sample_swimming_data();
        let all_data = swimming_data.clone();

        let swolf_analysis = SwimmingAnalyzer::analyze_swolf(
            &swimming_data.iter().collect::<Vec<_>>(),
            &all_data
        ).unwrap();

        assert!(swolf_analysis.avg_swolf.is_some());
        assert!(swolf_analysis.best_swolf.is_some());
    }

    #[test]
    fn test_stroke_type_conversion() {
        assert_eq!(SwimStrokeType::from(0), SwimStrokeType::Freestyle);
        assert_eq!(SwimStrokeType::from(1), SwimStrokeType::Backstroke);
        assert_eq!(SwimStrokeType::from(2), SwimStrokeType::Breaststroke);
        assert_eq!(SwimStrokeType::from(3), SwimStrokeType::Butterfly);
        assert_eq!(SwimStrokeType::from(99), SwimStrokeType::Freestyle); // Default fallback
    }

    #[test]
    fn test_efficiency_rating() {
        let metrics = SwimmingAnalyzer::calculate_efficiency_metrics(
            &vec![], // Empty swimming data
            &vec![], // Empty lap analysis
        ).unwrap();

        // With no data, efficiency metrics should be None
        assert!(metrics.stroke_efficiency.is_none());
        assert!(metrics.distance_per_stroke.is_none());
        assert!(metrics.stroke_rate.is_none());
    }
}