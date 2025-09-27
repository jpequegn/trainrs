//! Running-specific training analysis module
//!
//! This module provides comprehensive running analysis including pace zones,
//! elevation adjustment, performance prediction, and training metrics.

#![allow(dead_code)]

use crate::models::{DataPoint, Sport, Workout};
use anyhow::{anyhow, Result};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum RunningError {
    #[error("Insufficient data: {0}")]
    InsufficientData(String),
    #[error("Invalid pace: {0}")]
    InvalidPace(String),
    #[error("Calculation error: {0}")]
    CalculationError(String),
}

/// Pace Analysis results
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PaceAnalysis {
    /// Average pace in minutes per kilometer or mile
    pub avg_pace: Decimal,
    /// Normalized Graded Pace (accounting for elevation changes)
    pub normalized_graded_pace: Decimal,
    /// Grade Adjusted Pace (GAP)
    pub grade_adjusted_pace: Decimal,
    /// Pace distribution by zones
    pub pace_distribution: PaceDistribution,
    /// Split analysis for different segments
    pub splits: Vec<SplitAnalysis>,
    /// Efficiency factor (normalized pace / average heart rate)
    pub efficiency_factor: Option<Decimal>,
}

/// Pace distribution across zones
#[derive(Debug, Clone)]
pub struct PaceDistribution {
    /// Time spent in each pace zone (in seconds)
    pub zone_times: HashMap<u8, u32>,
    /// Percentage of time in each zone
    pub zone_percentages: HashMap<u8, Decimal>,
}

/// Analysis for a segment/split of the run
#[derive(Debug, Clone)]
pub struct SplitAnalysis {
    /// Distance of the split in meters
    pub distance_meters: Decimal,
    /// Duration in seconds
    pub duration_seconds: u32,
    /// Average pace for this split
    pub avg_pace: Decimal,
    /// Grade adjusted pace for this split
    pub gap: Decimal,
    /// Elevation change in this split
    pub elevation_change: i16,
    /// Average gradient percentage
    pub avg_gradient: Decimal,
}

/// Elevation analysis results
#[derive(Debug, Clone)]
pub struct ElevationAnalysis {
    /// Total elevation gain in meters
    pub total_gain: u16,
    /// Total elevation loss in meters
    pub total_loss: u16,
    /// Vertical Ascent Meters per hour
    pub vam: Decimal,
    /// Average gradient percentage
    pub avg_gradient: Decimal,
    /// Maximum gradient
    pub max_gradient: Decimal,
    /// Gradient-adjusted training stress
    pub gradient_adjusted_stress: Decimal,
    /// Time spent at different gradient categories
    pub gradient_distribution: GradientDistribution,
}

/// Distribution of time spent at different gradients
#[derive(Debug, Clone)]
pub struct GradientDistribution {
    /// Steep descent (< -10%)
    pub steep_descent: u32,
    /// Descent (-10% to -3%)
    pub descent: u32,
    /// Flat (-3% to 3%)
    pub flat: u32,
    /// Ascent (3% to 10%)
    pub ascent: u32,
    /// Steep ascent (> 10%)
    pub steep_ascent: u32,
}

/// VDOT and running performance metrics
#[derive(Debug, Clone)]
pub struct PerformancePrediction {
    /// VDOT score (Jack Daniels' VO2max estimate)
    pub vdot: Decimal,
    /// Predicted race times for various distances
    pub race_predictions: RacePredictions,
    /// Training paces based on VDOT
    pub training_paces: TrainingPaces,
    /// Equivalent performances across distances
    pub equivalent_performances: HashMap<String, Decimal>,
}

/// Predicted race times for standard distances
#[derive(Debug, Clone)]
pub struct RacePredictions {
    /// 5K race time in minutes
    pub time_5k: Decimal,
    /// 10K race time in minutes
    pub time_10k: Decimal,
    /// Half marathon time in minutes
    pub time_half_marathon: Decimal,
    /// Marathon time in minutes
    pub time_marathon: Decimal,
}

/// Training paces based on performance level
#[derive(Debug, Clone)]
pub struct TrainingPaces {
    /// Easy pace (minutes per km)
    pub easy_pace: Decimal,
    /// Marathon pace
    pub marathon_pace: Decimal,
    /// Threshold/Tempo pace
    pub threshold_pace: Decimal,
    /// Interval pace (VO2max pace)
    pub interval_pace: Decimal,
    /// Repetition pace (speed work)
    pub repetition_pace: Decimal,
}

/// Running-specific training zones
#[derive(Debug, Clone)]
pub struct RunningZones {
    /// Zone 1: Recovery/Easy
    pub zone1: PaceRange,
    /// Zone 2: Aerobic Base
    pub zone2: PaceRange,
    /// Zone 3: Tempo/Marathon
    pub zone3: PaceRange,
    /// Zone 4: Threshold
    pub zone4: PaceRange,
    /// Zone 5: VO2max/Interval
    pub zone5: PaceRange,
    /// Zone 6: Neuromuscular/Sprint
    pub zone6: PaceRange,
}

/// Pace range for a training zone
#[derive(Debug, Clone)]
pub struct PaceRange {
    /// Minimum pace (faster) in min/km
    pub min_pace: Decimal,
    /// Maximum pace (slower) in min/km
    pub max_pace: Decimal,
    /// Heart rate range if available
    pub hr_range: Option<(u16, u16)>,
}

/// Main running analysis struct
pub struct RunningAnalyzer;

impl RunningAnalyzer {
    pub fn new() -> Self {
        Self
    }
    /// Calculate comprehensive pace analysis
    pub fn analyze_pace(workout: &Workout) -> Result<PaceAnalysis> {
        if workout.sport != Sport::Running {
            return Err(anyhow!("Workout must be a running activity"));
        }

        let raw_data = workout.raw_data.as_ref()
            .ok_or_else(|| RunningError::InsufficientData("No raw data available".to_string()))?;

        if raw_data.len() < 2 {
            return Err(RunningError::InsufficientData("Insufficient data points".to_string()).into());
        }

        let avg_pace = Self::calculate_average_pace(raw_data)?;
        let ngp = Self::calculate_normalized_graded_pace(raw_data)?;
        let gap = Self::calculate_grade_adjusted_pace(raw_data)?;
        let pace_distribution = Self::analyze_pace_distribution(raw_data)?;
        let splits = Self::analyze_splits(raw_data, 1000.0)?; // 1km splits
        let efficiency_factor = Self::calculate_efficiency_factor(raw_data)?;

        Ok(PaceAnalysis {
            avg_pace,
            normalized_graded_pace: ngp,
            grade_adjusted_pace: gap,
            pace_distribution,
            splits,
            efficiency_factor,
        })
    }

    /// Calculate average pace from raw data
    fn calculate_average_pace(raw_data: &[DataPoint]) -> Result<Decimal> {
        let total_distance = raw_data.last()
            .and_then(|dp| dp.distance)
            .unwrap_or(dec!(0));

        if total_distance == dec!(0) {
            return Err(RunningError::InsufficientData("No distance data".to_string()).into());
        }

        let total_time = raw_data.last()
            .map(|dp| dp.timestamp)
            .unwrap_or(0) as i64;

        if total_time == 0 {
            return Err(RunningError::InsufficientData("No time data".to_string()).into());
        }

        // Convert to minutes per kilometer
        let minutes = Decimal::from(total_time) / dec!(60);
        let kilometers = total_distance / dec!(1000);

        Ok(minutes / kilometers)
    }

    /// Calculate Normalized Graded Pace accounting for elevation changes
    pub fn calculate_normalized_graded_pace(raw_data: &[DataPoint]) -> Result<Decimal> {
        let mut ngp_sum = dec!(0);
        let mut valid_points = 0;

        for i in 1..raw_data.len() {
            let prev = &raw_data[i - 1];
            let curr = &raw_data[i];

            if let (Some(prev_dist), Some(curr_dist), Some(prev_elev), Some(curr_elev)) =
                (prev.distance, curr.distance, prev.elevation, curr.elevation) {

                let distance = curr_dist - prev_dist;
                if distance <= dec!(0) {
                    continue;
                }

                let time_diff = (curr.timestamp - prev.timestamp) as i64;
                if time_diff <= 0 {
                    continue;
                }

                let gradient = Self::calculate_gradient(
                    distance,
                    Decimal::from(curr_elev - prev_elev)
                );

                let actual_pace = Decimal::from(time_diff) / dec!(60) / (distance / dec!(1000));
                let adjusted_pace = Self::apply_grade_adjustment(actual_pace, gradient);

                ngp_sum += adjusted_pace;
                valid_points += 1;
            }
        }

        if valid_points == 0 {
            return Err(RunningError::InsufficientData("No valid pace data".to_string()).into());
        }

        Ok(ngp_sum / Decimal::from(valid_points))
    }

    /// Calculate Grade Adjusted Pace (GAP)
    pub fn calculate_grade_adjusted_pace(raw_data: &[DataPoint]) -> Result<Decimal> {
        let mut total_adjusted_time = dec!(0);
        let mut total_distance = dec!(0);

        for i in 1..raw_data.len() {
            let prev = &raw_data[i - 1];
            let curr = &raw_data[i];

            if let (Some(prev_dist), Some(curr_dist), Some(prev_elev), Some(curr_elev)) =
                (prev.distance, curr.distance, prev.elevation, curr.elevation) {

                let segment_distance = curr_dist - prev_dist;
                if segment_distance <= dec!(0) {
                    continue;
                }

                let time_diff = Decimal::from((curr.timestamp - prev.timestamp) as i64);
                if time_diff <= dec!(0) {
                    continue;
                }

                let gradient = Self::calculate_gradient(
                    segment_distance,
                    Decimal::from(curr_elev - prev_elev)
                );

                // Apply grade adjustment to time
                let adjusted_time = Self::apply_gap_adjustment(time_diff, gradient);

                total_adjusted_time += adjusted_time;
                total_distance += segment_distance;
            }
        }

        if total_distance == dec!(0) {
            return Err(RunningError::InsufficientData("No distance data".to_string()).into());
        }

        // Convert to minutes per kilometer
        Ok((total_adjusted_time / dec!(60)) / (total_distance / dec!(1000)))
    }

    /// Calculate gradient as a percentage
    fn calculate_gradient(horizontal_distance: Decimal, vertical_distance: Decimal) -> Decimal {
        if horizontal_distance == dec!(0) {
            return dec!(0);
        }
        (vertical_distance / horizontal_distance) * dec!(100)
    }

    /// Apply grade adjustment to pace using standard formula
    fn apply_grade_adjustment(pace: Decimal, gradient: Decimal) -> Decimal {
        // Standard formula: Adjusted Pace = Actual Pace × (1 + (Grade × 0.033))
        // For grades > 10%, additional factors apply

        let adjustment_factor = if gradient.abs() <= dec!(10) {
            dec!(1) + (gradient * dec!(0.033))
        } else if gradient > dec!(10) {
            // Steeper uphill requires more aggressive adjustment
            dec!(1) + (gradient * dec!(0.05))
        } else {
            // Steep downhill has diminishing returns
            dec!(1) + (gradient * dec!(0.02))
        };

        pace * adjustment_factor
    }

    /// Apply GAP adjustment to time
    fn apply_gap_adjustment(time: Decimal, gradient: Decimal) -> Decimal {
        // Inverse of pace adjustment for time
        let adjustment_factor = if gradient.abs() <= dec!(10) {
            dec!(1) / (dec!(1) + (gradient * dec!(0.033)))
        } else if gradient > dec!(10) {
            dec!(1) / (dec!(1) + (gradient * dec!(0.05)))
        } else {
            dec!(1) / (dec!(1) + (gradient * dec!(0.02)))
        };

        time * adjustment_factor
    }

    /// Analyze pace distribution across zones
    fn analyze_pace_distribution(raw_data: &[DataPoint]) -> Result<PaceDistribution> {
        let mut zone_times: HashMap<u8, u32> = HashMap::new();

        // Initialize zones
        for zone in 1..=6 {
            zone_times.insert(zone, 0);
        }

        for dp in raw_data {
            if let Some(pace) = dp.pace {
                let zone = Self::pace_to_zone(pace);
                *zone_times.entry(zone).or_insert(0) += 1;
            }
        }

        let total_time: u32 = zone_times.values().sum();
        let mut zone_percentages = HashMap::new();

        if total_time > 0 {
            for (zone, time) in &zone_times {
                let percentage = Decimal::from(*time) / Decimal::from(total_time) * dec!(100);
                zone_percentages.insert(*zone, percentage);
            }
        }

        Ok(PaceDistribution {
            zone_times,
            zone_percentages,
        })
    }

    /// Map pace to training zone (simplified - should use athlete's zones)
    fn pace_to_zone(pace: Decimal) -> u8 {
        // Simplified zone mapping based on typical threshold pace
        // In real implementation, this would use athlete's specific zones
        if pace > dec!(6.5) { 1 }      // Recovery
        else if pace > dec!(5.5) { 2 } // Easy
        else if pace > dec!(5.0) { 3 } // Marathon
        else if pace > dec!(4.5) { 4 } // Threshold
        else if pace > dec!(4.0) { 5 } // VO2max
        else { 6 }                     // Sprint
    }

    /// Analyze splits for given distance
    fn analyze_splits(raw_data: &[DataPoint], split_distance: f64) -> Result<Vec<SplitAnalysis>> {
        let mut splits = Vec::new();
        let mut current_split_start = 0;
        let mut current_distance = dec!(0);

        for (i, dp) in raw_data.iter().enumerate() {
            if let Some(dist) = dp.distance {
                if dist - current_distance >= Decimal::from_f64(split_distance).unwrap() {
                    // Complete split
                    let split = Self::calculate_split(
                        &raw_data[current_split_start..=i],
                        current_distance,
                        dist
                    )?;
                    splits.push(split);
                    current_split_start = i;
                    current_distance = dist;
                }
            }
        }

        // Add final partial split if there's remaining data
        if current_split_start < raw_data.len() - 1 {
            if let Some(final_dist) = raw_data.last().and_then(|dp| dp.distance) {
                let split = Self::calculate_split(
                    &raw_data[current_split_start..],
                    current_distance,
                    final_dist
                )?;
                splits.push(split);
            }
        }

        Ok(splits)
    }

    /// Calculate metrics for a single split
    fn calculate_split(data: &[DataPoint], start_dist: Decimal, end_dist: Decimal) -> Result<SplitAnalysis> {
        let distance_meters = end_dist - start_dist;
        let duration_seconds = (data.last().unwrap().timestamp - data.first().unwrap().timestamp) as u32;

        let avg_pace = if distance_meters > dec!(0) && duration_seconds > 0 {
            (Decimal::from(duration_seconds) / dec!(60)) / (distance_meters / dec!(1000))
        } else {
            dec!(0)
        };

        let gap = Self::calculate_grade_adjusted_pace(data).unwrap_or(avg_pace);

        let elevation_change = if let (Some(start_elev), Some(end_elev)) =
            (data.first().and_then(|dp| dp.elevation), data.last().and_then(|dp| dp.elevation)) {
            end_elev - start_elev
        } else {
            0
        };

        let avg_gradient = if distance_meters > dec!(0) {
            Decimal::from(elevation_change) / distance_meters * dec!(100)
        } else {
            dec!(0)
        };

        Ok(SplitAnalysis {
            distance_meters,
            duration_seconds,
            avg_pace,
            gap,
            elevation_change,
            avg_gradient,
        })
    }

    /// Calculate efficiency factor (normalized pace / average heart rate)
    fn calculate_efficiency_factor(raw_data: &[DataPoint]) -> Result<Option<Decimal>> {
        let ngp = Self::calculate_normalized_graded_pace(raw_data)?;

        let avg_hr = Self::calculate_average_hr(raw_data);
        if let Some(hr) = avg_hr {
            if hr > 0 {
                // Convert pace to speed for efficiency factor
                // EF = speed / heart rate, where speed = 1000 / (pace * 60) m/s
                let speed = dec!(1000) / (ngp * dec!(60));
                return Ok(Some(speed / Decimal::from(hr)));
            }
        }

        Ok(None)
    }

    /// Calculate average heart rate
    fn calculate_average_hr(raw_data: &[DataPoint]) -> Option<u16> {
        let hr_values: Vec<u16> = raw_data.iter()
            .filter_map(|dp| dp.heart_rate)
            .collect();

        if hr_values.is_empty() {
            return None;
        }

        let sum: u32 = hr_values.iter().map(|&hr| hr as u32).sum();
        Some((sum / hr_values.len() as u32) as u16)
    }

    /// Analyze elevation data
    pub fn analyze_elevation(workout: &Workout) -> Result<ElevationAnalysis> {
        let raw_data = workout.raw_data.as_ref()
            .ok_or_else(|| RunningError::InsufficientData("No raw data available".to_string()))?;

        let (total_gain, total_loss) = Self::calculate_elevation_changes(raw_data)?;
        let vam = Self::calculate_vam(total_gain, workout.duration_seconds)?;
        let (avg_gradient, max_gradient) = Self::calculate_gradient_stats(raw_data)?;
        let gradient_distribution = Self::analyze_gradient_distribution(raw_data)?;
        let gradient_adjusted_stress = Self::calculate_gradient_adjusted_stress(
            total_gain,
            total_loss,
            workout.duration_seconds
        )?;

        Ok(ElevationAnalysis {
            total_gain,
            total_loss,
            vam,
            avg_gradient,
            max_gradient,
            gradient_adjusted_stress,
            gradient_distribution,
        })
    }

    /// Calculate total elevation gain and loss
    fn calculate_elevation_changes(raw_data: &[DataPoint]) -> Result<(u16, u16)> {
        let mut total_gain = 0i32;
        let mut total_loss = 0i32;

        for i in 1..raw_data.len() {
            if let (Some(prev_elev), Some(curr_elev)) =
                (raw_data[i - 1].elevation, raw_data[i].elevation) {

                let change = curr_elev - prev_elev;
                if change > 0 {
                    total_gain += change as i32;
                } else {
                    total_loss += (-change) as i32;
                }
            }
        }

        Ok((total_gain.max(0) as u16, total_loss.max(0) as u16))
    }

    /// Calculate Vertical Ascent Meters per hour
    fn calculate_vam(total_gain: u16, duration_seconds: u32) -> Result<Decimal> {
        if duration_seconds == 0 {
            return Ok(dec!(0));
        }

        let hours = Decimal::from(duration_seconds) / dec!(3600);
        Ok(Decimal::from(total_gain) / hours)
    }

    /// Calculate gradient statistics
    fn calculate_gradient_stats(raw_data: &[DataPoint]) -> Result<(Decimal, Decimal)> {
        let mut gradients = Vec::new();

        for i in 1..raw_data.len() {
            if let (Some(prev_dist), Some(curr_dist), Some(prev_elev), Some(curr_elev)) =
                (raw_data[i - 1].distance, raw_data[i].distance,
                 raw_data[i - 1].elevation, raw_data[i].elevation) {

                let distance = curr_dist - prev_dist;
                if distance > dec!(0) {
                    let gradient = Self::calculate_gradient(
                        distance,
                        Decimal::from(curr_elev - prev_elev)
                    );
                    gradients.push(gradient);
                }
            }
        }

        if gradients.is_empty() {
            return Ok((dec!(0), dec!(0)));
        }

        let sum: Decimal = gradients.iter().sum();
        let avg = sum / Decimal::from(gradients.len());
        let max = gradients.iter().max().cloned().unwrap_or(dec!(0));

        Ok((avg, max))
    }

    /// Analyze time spent at different gradient categories
    fn analyze_gradient_distribution(raw_data: &[DataPoint]) -> Result<GradientDistribution> {
        let mut distribution = GradientDistribution {
            steep_descent: 0,
            descent: 0,
            flat: 0,
            ascent: 0,
            steep_ascent: 0,
        };

        for i in 1..raw_data.len() {
            if let (Some(prev_dist), Some(curr_dist), Some(prev_elev), Some(curr_elev)) =
                (raw_data[i - 1].distance, raw_data[i].distance,
                 raw_data[i - 1].elevation, raw_data[i].elevation) {

                let distance = curr_dist - prev_dist;
                if distance > dec!(0) {
                    let gradient = Self::calculate_gradient(
                        distance,
                        Decimal::from(curr_elev - prev_elev)
                    );

                    let time_diff = (raw_data[i].timestamp - raw_data[i - 1].timestamp) as u32;

                    if gradient < dec!(-10) {
                        distribution.steep_descent += time_diff;
                    } else if gradient < dec!(-3) {
                        distribution.descent += time_diff;
                    } else if gradient < dec!(3) {
                        distribution.flat += time_diff;
                    } else if gradient < dec!(10) {
                        distribution.ascent += time_diff;
                    } else {
                        distribution.steep_ascent += time_diff;
                    }
                }
            }
        }

        Ok(distribution)
    }

    /// Calculate gradient-adjusted training stress
    fn calculate_gradient_adjusted_stress(
        total_gain: u16,
        total_loss: u16,
        duration_seconds: u32
    ) -> Result<Decimal> {
        // Simplified formula: stress increases with elevation change relative to time
        // More sophisticated calculation would use intensity factor and FTP
        let total_change = Decimal::from(total_gain + total_loss);
        let hours = Decimal::from(duration_seconds) / dec!(3600);

        if hours == dec!(0) {
            return Ok(dec!(0));
        }

        // Base stress + elevation factor
        let base_stress = hours * dec!(50); // Base TSS per hour
        let elevation_factor = (total_change / dec!(100)) * dec!(10); // Extra stress per 100m elevation

        Ok(base_stress + elevation_factor)
    }

    /// Calculate VDOT and performance predictions
    pub fn predict_performance(recent_race_time: Decimal, race_distance_km: Decimal) -> Result<PerformancePrediction> {
        let vdot = Self::calculate_vdot(recent_race_time, race_distance_km)?;
        let race_predictions = Self::predict_race_times(vdot)?;
        let training_paces = Self::calculate_training_paces(vdot)?;
        let equivalent_performances = Self::calculate_equivalent_performances(vdot)?;

        Ok(PerformancePrediction {
            vdot,
            race_predictions,
            training_paces,
            equivalent_performances,
        })
    }

    /// Calculate VDOT from race performance using Jack Daniels' formula
    fn calculate_vdot(time_minutes: Decimal, distance_km: Decimal) -> Result<Decimal> {
        // Simplified VDOT calculation
        // Real implementation would use Jack Daniels' full formula

        if distance_km <= dec!(0) || time_minutes <= dec!(0) {
            return Err(RunningError::InvalidPace("Invalid race data".to_string()).into());
        }

        // Convert to velocity in m/min
        let velocity = (distance_km * dec!(1000)) / time_minutes;

        // Simplified VDOT formula (approximation)
        // VDOT ≈ 0.000104 * v^2 + 0.182 * v - 4.6
        let v = velocity;
        let vdot = dec!(0.000104) * v * v + dec!(0.182) * v - dec!(4.6);

        Ok(vdot.max(dec!(20)).min(dec!(85))) // Clamp to reasonable range
    }

    /// Predict race times based on VDOT
    fn predict_race_times(vdot: Decimal) -> Result<RacePredictions> {
        // Using simplified prediction formulas
        // Real implementation would use Jack Daniels' tables

        Ok(RacePredictions {
            time_5k: Self::predict_time_for_distance(vdot, dec!(5))?,
            time_10k: Self::predict_time_for_distance(vdot, dec!(10))?,
            time_half_marathon: Self::predict_time_for_distance(vdot, dec!(21.0975))?,
            time_marathon: Self::predict_time_for_distance(vdot, dec!(42.195))?,
        })
    }

    /// Predict time for a specific distance based on VDOT
    fn predict_time_for_distance(vdot: Decimal, distance_km: Decimal) -> Result<Decimal> {
        // Simplified prediction based on VDOT using empirical relationships
        // For testing, we'll use simple scaling based on distance

        // Base pace from VDOT (roughly 4 min/km for VDOT 50)
        let base_pace_per_km = dec!(240) / vdot; // 240/vdot gives reasonable pace in min/km

        // Apply distance multipliers (endurance factor)
        let endurance_factor = if distance_km <= dec!(5) {
            dec!(1.00)
        } else if distance_km <= dec!(10) {
            dec!(1.05) // 5% slower per km for 10K
        } else if distance_km <= dec!(21.1) {
            dec!(1.12) // 12% slower per km for half marathon
        } else {
            dec!(1.20) // 20% slower per km for marathon
        };

        let adjusted_pace = base_pace_per_km * endurance_factor;
        Ok(distance_km * adjusted_pace)
    }

    /// Calculate training paces based on VDOT
    fn calculate_training_paces(vdot: Decimal) -> Result<TrainingPaces> {
        // Calculate paces as percentage of VDOT velocity
        let base_velocity = (vdot + dec!(4.6) - dec!(0.182)) / dec!(0.000104);
        let base_velocity = Decimal::from_f64(base_velocity.to_f64().unwrap_or(0.0).sqrt())
            .unwrap_or(dec!(0));

        // Convert velocity (m/min) to pace (min/km)
        let threshold_pace = dec!(1000) / base_velocity;

        Ok(TrainingPaces {
            easy_pace: threshold_pace * dec!(1.25),        // 75-80% of threshold
            marathon_pace: threshold_pace * dec!(1.07),    // 93% of threshold
            threshold_pace,                                // 100%
            interval_pace: threshold_pace * dec!(0.94),    // 106% of threshold
            repetition_pace: threshold_pace * dec!(0.88),  // 113% of threshold
        })
    }

    /// Calculate equivalent performances across distances
    fn calculate_equivalent_performances(vdot: Decimal) -> Result<HashMap<String, Decimal>> {
        let mut performances = HashMap::new();

        performances.insert("1500m".to_string(), Self::predict_time_for_distance(vdot, dec!(1.5))?);
        performances.insert("1_mile".to_string(), Self::predict_time_for_distance(vdot, dec!(1.60934))?);
        performances.insert("3000m".to_string(), Self::predict_time_for_distance(vdot, dec!(3))?);
        performances.insert("2_miles".to_string(), Self::predict_time_for_distance(vdot, dec!(3.21869))?);
        performances.insert("5000m".to_string(), Self::predict_time_for_distance(vdot, dec!(5))?);
        performances.insert("10000m".to_string(), Self::predict_time_for_distance(vdot, dec!(10))?);
        performances.insert("15k".to_string(), Self::predict_time_for_distance(vdot, dec!(15))?);
        performances.insert("half_marathon".to_string(), Self::predict_time_for_distance(vdot, dec!(21.0975))?);
        performances.insert("marathon".to_string(), Self::predict_time_for_distance(vdot, dec!(42.195))?);

        Ok(performances)
    }

    /// Calculate running-specific training zones
    pub fn calculate_running_zones(threshold_pace: Decimal, lthr: Option<u16>) -> Result<RunningZones> {
        // Zone 1: Recovery (slower than easy)
        let zone1 = PaceRange {
            min_pace: threshold_pace * dec!(1.35),
            max_pace: threshold_pace * dec!(1.50),
            hr_range: lthr.map(|hr| ((hr as f64 * 0.60) as u16, (hr as f64 * 0.70) as u16)),
        };

        // Zone 2: Easy/Aerobic Base
        let zone2 = PaceRange {
            min_pace: threshold_pace * dec!(1.20),
            max_pace: threshold_pace * dec!(1.35),
            hr_range: lthr.map(|hr| ((hr as f64 * 0.70) as u16, (hr as f64 * 0.80) as u16)),
        };

        // Zone 3: Marathon/Tempo
        let zone3 = PaceRange {
            min_pace: threshold_pace * dec!(1.05),
            max_pace: threshold_pace * dec!(1.20),
            hr_range: lthr.map(|hr| ((hr as f64 * 0.80) as u16, (hr as f64 * 0.90) as u16)),
        };

        // Zone 4: Threshold
        let zone4 = PaceRange {
            min_pace: threshold_pace * dec!(0.97),
            max_pace: threshold_pace * dec!(1.05),
            hr_range: lthr.map(|hr| ((hr as f64 * 0.90) as u16, (hr as f64 * 0.95) as u16)),
        };

        // Zone 5: VO2max/Interval
        let zone5 = PaceRange {
            min_pace: threshold_pace * dec!(0.90),
            max_pace: threshold_pace * dec!(0.97),
            hr_range: lthr.map(|hr| ((hr as f64 * 0.95) as u16, (hr as f64 * 1.02) as u16)),
        };

        // Zone 6: Neuromuscular/Sprint
        let zone6 = PaceRange {
            min_pace: threshold_pace * dec!(0.80),
            max_pace: threshold_pace * dec!(0.90),
            hr_range: lthr.map(|hr| ((hr as f64 * 1.02) as u16, hr + 20)),
        };

        Ok(RunningZones {
            zone1,
            zone2,
            zone3,
            zone4,
            zone5,
            zone6,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn create_sample_running_data() -> Vec<DataPoint> {
        let mut data = Vec::new();

        // Simulate a 5km run with some elevation changes
        for i in 0..300 {
            let timestamp = i * 6; // 6 second intervals for 30 minutes total
            let distance = Decimal::from(i) * dec!(16.67); // ~5km in 30 minutes

            // Simulate elevation changes
            let elevation = 100 + ((i as f32 * 0.05).sin() * 20.0) as i16;

            // Simulate pace variations
            let base_pace = dec!(6.0); // 6 min/km base pace
            let pace_variation = ((i as f32 * 0.1).sin() * 0.5) as f64;
            let pace = base_pace + Decimal::from_f64(pace_variation).unwrap();

            // Simulate heart rate
            let hr = 140 + ((i as f32 * 0.08).sin() * 20.0) as u16;

            data.push(DataPoint {
                timestamp,
                heart_rate: Some(hr.clamp(120, 180)),
                power: None,
                pace: Some(pace),
                elevation: Some(elevation),
                cadence: Some(170 + (i % 20) as u16),
                speed: Some(dec!(1000) / (pace * dec!(60))),
                distance: Some(distance),
                left_power: None,
                right_power: None,
            });
        }

        data
    }

    #[test]
    fn test_pace_analysis() {
        let workout = Workout {
            id: "test_run".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            sport: Sport::Running,
            duration_seconds: 1800,
            workout_type: crate::models::WorkoutType::Endurance,
            data_source: crate::models::DataSource::Pace,
            raw_data: Some(create_sample_running_data()),
            summary: Default::default(),
            notes: None,
            athlete_id: None,
            source: None,
        };

        let analysis = RunningAnalyzer::analyze_pace(&workout).unwrap();

        assert!(analysis.avg_pace > dec!(5.5) && analysis.avg_pace < dec!(6.5));
        assert!(analysis.normalized_graded_pace > dec!(0));
        assert!(analysis.grade_adjusted_pace > dec!(0));
        assert!(!analysis.splits.is_empty());
    }

    #[test]
    fn test_elevation_analysis() {
        let workout = Workout {
            id: "hill_run".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            sport: Sport::Running,
            duration_seconds: 3600,
            workout_type: crate::models::WorkoutType::Endurance,
            data_source: crate::models::DataSource::Pace,
            raw_data: Some(create_sample_running_data()),
            summary: Default::default(),
            notes: None,
            athlete_id: None,
            source: None,
        };

        let elevation = RunningAnalyzer::analyze_elevation(&workout).unwrap();

        assert!(elevation.total_gain > 0);
        assert!(elevation.total_loss > 0);
        assert!(elevation.vam > dec!(0));
        assert!(elevation.gradient_adjusted_stress > dec!(0));
    }

    #[test]
    fn test_vdot_calculation() {
        // Test with a 20-minute 5K time
        let vdot = RunningAnalyzer::calculate_vdot(dec!(20), dec!(5)).unwrap();

        // VDOT should be around 49-51 for a 20-minute 5K
        assert!(vdot > dec!(45) && vdot < dec!(55));
    }

    #[test]
    fn test_performance_prediction() {
        // Predict based on 20-minute 5K
        let prediction = RunningAnalyzer::predict_performance(dec!(20), dec!(5)).unwrap();

        assert!(prediction.vdot > dec!(45));
        assert!(prediction.race_predictions.time_10k > dec!(40));
        assert!(prediction.race_predictions.time_half_marathon > dec!(90));
        assert!(prediction.race_predictions.time_marathon > dec!(180));
    }

    #[test]
    fn test_running_zones() {
        let threshold_pace = dec!(4.5); // 4:30/km threshold pace
        let zones = RunningAnalyzer::calculate_running_zones(threshold_pace, Some(170)).unwrap();

        // Zone 1 should be slowest
        assert!(zones.zone1.min_pace > zones.zone2.min_pace);
        // Zone 6 should be fastest
        assert!(zones.zone6.max_pace < zones.zone5.max_pace);
        // HR ranges should be present
        assert!(zones.zone4.hr_range.is_some());
    }

    #[test]
    fn test_grade_adjustment() {
        // Test uphill adjustment
        let uphill_adjusted = RunningAnalyzer::apply_grade_adjustment(dec!(6.0), dec!(5.0));
        assert!(uphill_adjusted > dec!(6.0)); // Should be slower

        // Test downhill adjustment
        let downhill_adjusted = RunningAnalyzer::apply_grade_adjustment(dec!(6.0), dec!(-5.0));
        assert!(downhill_adjusted < dec!(6.0)); // Should be faster

        // Test flat adjustment
        let flat_adjusted = RunningAnalyzer::apply_grade_adjustment(dec!(6.0), dec!(0));
        assert_eq!(flat_adjusted, dec!(6.0)); // Should be unchanged
    }

    #[test]
    fn test_gradient_calculation() {
        let gradient = RunningAnalyzer::calculate_gradient(dec!(100), dec!(10));
        assert_eq!(gradient, dec!(10)); // 10% gradient

        let negative_gradient = RunningAnalyzer::calculate_gradient(dec!(100), dec!(-5));
        assert_eq!(negative_gradient, dec!(-5)); // -5% gradient
    }

    #[test]
    fn test_vam_calculation() {
        let vam = RunningAnalyzer::calculate_vam(300, 3600).unwrap();
        assert_eq!(vam, dec!(300)); // 300m gain in 1 hour = 300 VAM

        let vam_half_hour = RunningAnalyzer::calculate_vam(150, 1800).unwrap();
        assert_eq!(vam_half_hour, dec!(300)); // 150m gain in 0.5 hour = 300 VAM
    }

    #[test]
    fn test_split_analysis() {
        let data = create_sample_running_data();
        let splits = RunningAnalyzer::analyze_splits(&data, 1000.0).unwrap();

        assert!(!splits.is_empty());
        for split in splits {
            assert!(split.distance_meters > dec!(0));
            assert!(split.duration_seconds > 0);
            assert!(split.avg_pace > dec!(0));
        }
    }
}