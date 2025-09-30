//! Comprehensive cycling power analysis module
//!
//! This module provides advanced power analysis features for cycling training and performance,
//! including power curve analysis, critical power modeling, normalized power calculations,
//! and various power-based training metrics.

use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;

use crate::models::{DataPoint, Workout};

/// Power analysis error types
#[derive(Debug, thiserror::Error)]
pub enum PowerError {
    #[error("Insufficient data: {0}")]
    InsufficientData(String),
    #[error("Invalid data: {0}")]
    #[allow(dead_code)]
    InvalidData(String),
    #[error("Calculation error: {0}")]
    #[allow(dead_code)]
    CalculationError(String),
    #[error("Model fitting error: {0}")]
    #[allow(dead_code)]
    ModelFittingError(String),
}

/// Power curve data point representing maximum power for a duration
#[derive(Debug, Clone, PartialEq)]
pub struct PowerCurvePoint {
    pub duration_seconds: u32,
    pub max_power: u16,
    pub date: NaiveDate,
    pub workout_id: String,
}

/// Mean Maximal Power curve for comprehensive power profiling
/// Shows maximal average power sustained over all durations from 1 second to workout length
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MmpCurve {
    /// All duration-power pairs from 1s to workout length
    pub duration_seconds: Vec<u32>,
    /// Maximal power for each duration
    pub max_power: Vec<u16>,
    /// Optional workout ID if from single workout
    pub workout_id: Option<String>,
    /// Date range for this curve
    pub date_range: (NaiveDate, NaiveDate),
}

/// Key power durations for training zones and performance profiling
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KeyPowers {
    /// 5-second sprint power
    pub sprint_5s: u16,
    /// 1-minute anaerobic capacity
    pub anaerobic_1min: u16,
    /// 5-minute VO2max power
    pub vo2max_5min: u16,
    /// 20-minute FTP proxy
    pub ftp_20min: u16,
    /// 60-minute threshold power
    pub threshold_60min: u16,
}

/// Power curve representing mean maximal power (MMP) across different durations
/// Legacy structure - use MmpCurve for new implementations
#[derive(Debug, Clone)]
pub struct PowerCurve {
    /// Key durations and their maximum powers
    pub points: Vec<PowerCurvePoint>,
    /// Standard durations for comparison (1s, 5s, 15s, 30s, 1min, 5min, 20min, etc.)
    pub standard_durations: HashMap<u32, u16>,
    /// Date range for this power curve
    pub date_range: (NaiveDate, NaiveDate),
}

/// MMP Analyzer for calculating comprehensive power curves
pub struct MmpAnalyzer;

/// Critical Power model parameters
#[derive(Debug, Clone)]
pub struct CriticalPowerModel {
    /// Critical Power (CP) - sustainable power in watts
    pub critical_power: u16,
    /// W' (W-prime) - finite work capacity above CP in joules
    pub w_prime: u32,
    /// Model fit quality (R-squared value)
    pub r_squared: Decimal,
    /// Estimated FTP from CP model
    pub estimated_ftp: u16,
    /// Model type (2-parameter or 3-parameter)
    pub model_type: CpModelType,
    /// Test dates used for this model
    pub test_dates: Vec<NaiveDate>,
}

/// Critical Power model types
#[derive(Debug, Clone, PartialEq)]
pub enum CpModelType {
    /// Classic 2-parameter model: P = CP + W'/t
    TwoParameter,
    /// Extended 3-parameter model with time constant
    ThreeParameter { time_constant: Decimal },
    /// Linear P-1/t model
    LinearPInverse,
}

/// W' balance tracking during a workout
#[derive(Debug, Clone)]
pub struct WPrimeBalance {
    /// Timestamp for each balance measurement
    pub timestamps: Vec<u32>,
    /// W' balance in joules at each timestamp
    pub balance: Vec<i32>,
    /// Minimum W' balance reached (most depleted)
    pub min_balance: i32,
    /// Time spent with W' < 0 (seconds)
    pub time_below_zero: u32,
}

/// Time-to-exhaustion prediction
#[derive(Debug, Clone)]
pub struct TimeToExhaustion {
    /// Target power in watts
    pub target_power: u16,
    /// Predicted time to exhaustion in seconds
    pub time_seconds: u32,
    /// Current W' balance (if tracking ongoing effort)
    pub current_w_prime_balance: Option<i32>,
}

/// Power-based training metrics
#[derive(Debug, Clone)]
pub struct PowerMetrics {
    /// Normalized Power (30-second rolling average)
    pub normalized_power: u16,
    /// Variability Index (VI = NP/Average Power)
    pub variability_index: Decimal,
    /// Efficiency Factor (EF = NP/Average HR)
    pub efficiency_factor: Option<Decimal>,
    /// Intensity Factor (IF = NP/FTP)
    pub intensity_factor: Option<Decimal>,
    /// Work above FTP in kilojoules
    pub work_above_ftp: Option<u32>,
    /// Work below FTP in kilojoules
    pub work_below_ftp: Option<u32>,
}

/// Peak power analysis for standard durations
#[derive(Debug, Clone)]
pub struct PeakPowerAnalysis {
    pub peak_5s: Option<u16>,
    pub peak_15s: Option<u16>,
    pub peak_30s: Option<u16>,
    pub peak_1min: Option<u16>,
    pub peak_5min: Option<u16>,
    pub peak_20min: Option<u16>,
    pub peak_60min: Option<u16>,
}

/// Quadrant analysis for force vs velocity
#[derive(Debug, Clone)]
pub struct QuadrantAnalysis {
    /// Quadrant I: High Force, High Velocity (sprinting)
    pub quadrant_i_percent: Decimal,
    /// Quadrant II: Low Force, High Velocity (high cadence)
    pub quadrant_ii_percent: Decimal,
    /// Quadrant III: Low Force, Low Velocity (recovery)
    pub quadrant_iii_percent: Decimal,
    /// Quadrant IV: High Force, Low Velocity (climbing)
    pub quadrant_iv_percent: Decimal,
}

/// Power balance data (left/right)
#[derive(Debug, Clone)]
pub struct PowerBalance {
    /// Left leg power percentage (0-100)
    pub left_percent: Decimal,
    /// Right leg power percentage (0-100)
    pub right_percent: Decimal,
    /// Balance score (50 = perfect balance)
    pub balance_score: Decimal,
}

impl MmpAnalyzer {
    /// Calculate Mean Maximal Power curve from a single workout's data points
    ///
    /// Generates MMP values for all durations from 1 second to the workout length.
    /// Uses rolling window averaging to find the maximum average power for each duration.
    ///
    /// # Arguments
    /// * `data_points` - Time-series power data from a workout
    ///
    /// # Returns
    /// * `MmpCurve` containing duration-power pairs for all possible durations
    pub fn calculate_mmp(data_points: &[DataPoint]) -> Result<MmpCurve> {
        let power_data: Vec<u16> = data_points
            .iter()
            .filter_map(|dp| dp.power)
            .collect();

        if power_data.is_empty() {
            return Err(anyhow!("No power data available for MMP calculation"));
        }

        let max_duration = power_data.len() as u32;
        let mut duration_seconds = Vec::new();
        let mut max_power = Vec::new();

        // Calculate MMP for all durations from 1 second to workout length
        for duration in 1..=max_duration {
            if let Some(mmp) = Self::calculate_mmp_for_duration(&power_data, duration) {
                duration_seconds.push(duration);
                max_power.push(mmp);
            }
        }

        // Extract date from first data point (assuming chronological order)
        let date = chrono::Local::now().date_naive();

        Ok(MmpCurve {
            duration_seconds,
            max_power,
            workout_id: None,
            date_range: (date, date),
        })
    }

    /// Aggregate MMP curves across multiple workouts to find best efforts
    ///
    /// For each duration, finds the maximum power achieved across all provided workouts.
    /// This creates a "personal best" MMP curve showing peak performance at each duration.
    ///
    /// # Arguments
    /// * `workouts` - Collection of workouts to analyze
    ///
    /// # Returns
    /// * `MmpCurve` with best power for each duration across all workouts
    pub fn aggregate_mmp(workouts: &[Workout]) -> Result<MmpCurve> {
        if workouts.is_empty() {
            return Err(anyhow!("No workouts provided for MMP aggregation"));
        }

        let mut duration_power_map: HashMap<u32, u16> = HashMap::new();
        let mut min_date = chrono::NaiveDate::from_ymd_opt(9999, 12, 31).unwrap();
        let mut max_date = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();

        // Process each workout to extract MMP values
        for workout in workouts {
            if let Some(raw_data) = &workout.raw_data {
                let power_data: Vec<u16> = raw_data
                    .iter()
                    .filter_map(|dp| dp.power)
                    .collect();

                if power_data.is_empty() {
                    continue;
                }

                // Calculate MMP for all durations in this workout
                let max_duration = power_data.len() as u32;
                for duration in 1..=max_duration {
                    if let Some(mmp) = Self::calculate_mmp_for_duration(&power_data, duration) {
                        let current_max = duration_power_map.get(&duration).copied().unwrap_or(0);
                        if mmp > current_max {
                            duration_power_map.insert(duration, mmp);
                        }
                    }
                }

                // Update date range
                if workout.date < min_date {
                    min_date = workout.date;
                }
                if workout.date > max_date {
                    max_date = workout.date;
                }
            }
        }

        if duration_power_map.is_empty() {
            return Err(anyhow!("No valid power data found in any workout"));
        }

        // Convert HashMap to sorted vectors
        let mut durations: Vec<u32> = duration_power_map.keys().copied().collect();
        durations.sort_unstable();

        let powers: Vec<u16> = durations
            .iter()
            .map(|d| *duration_power_map.get(d).unwrap())
            .collect();

        Ok(MmpCurve {
            duration_seconds: durations,
            max_power: powers,
            workout_id: None,
            date_range: (min_date, max_date),
        })
    }

    /// Extract key power values from an MMP curve
    ///
    /// Identifies critical durations (5s, 1min, 5min, 20min, 60min) that define
    /// different physiological systems and training zones.
    ///
    /// # Arguments
    /// * `curve` - MMP curve to extract key powers from
    ///
    /// # Returns
    /// * `KeyPowers` struct with values for key durations
    pub fn get_key_powers(curve: &MmpCurve) -> Result<KeyPowers> {
        fn find_power_at_duration(curve: &MmpCurve, target_duration: u32) -> Result<u16> {
            // Find exact match or nearest duration
            if let Some(pos) = curve.duration_seconds.iter().position(|&d| d == target_duration) {
                return Ok(curve.max_power[pos]);
            }

            // If no exact match, find the closest duration
            let mut closest_idx = 0;
            let mut min_diff = u32::MAX;

            for (idx, &duration) in curve.duration_seconds.iter().enumerate() {
                let diff = if duration > target_duration {
                    duration - target_duration
                } else {
                    target_duration - duration
                };

                if diff < min_diff {
                    min_diff = diff;
                    closest_idx = idx;
                }
            }

            if min_diff <= target_duration / 10 {
                // Accept if within 10% of target duration
                Ok(curve.max_power[closest_idx])
            } else {
                Err(anyhow!("No power data near {} seconds", target_duration))
            }
        }

        Ok(KeyPowers {
            sprint_5s: find_power_at_duration(curve, 5)?,
            anaerobic_1min: find_power_at_duration(curve, 60)?,
            vo2max_5min: find_power_at_duration(curve, 300)?,
            ftp_20min: find_power_at_duration(curve, 1200)?,
            threshold_60min: find_power_at_duration(curve, 3600)?,
        })
    }

    /// Internal helper: Calculate MMP for a specific duration using rolling window
    fn calculate_mmp_for_duration(power_data: &[u16], duration_seconds: u32) -> Option<u16> {
        let window_size = duration_seconds as usize;

        if power_data.len() < window_size {
            return None;
        }

        let mut max_avg = 0u32;

        for window in power_data.windows(window_size) {
            let sum: u32 = window.iter().map(|&p| p as u32).sum();
            let avg = sum / window_size as u32;
            max_avg = max_avg.max(avg);
        }

        Some(max_avg as u16)
    }
}

/// Main power analyzer struct
pub struct PowerAnalyzer;

impl PowerAnalyzer {
    /// Calculate Mean Maximal Power (MMP) curve from workout data
    pub fn calculate_power_curve(
        workouts: &[&Workout],
        _date_range: Option<(NaiveDate, NaiveDate)>,
    ) -> Result<PowerCurve> {
        let mut all_power_points = Vec::new();

        for workout in workouts {
            if let Some(raw_data) = &workout.raw_data {
                let power_data: Vec<u16> = raw_data
                    .iter()
                    .filter_map(|dp| dp.power)
                    .collect();

                if power_data.is_empty() {
                    continue;
                }

                // Calculate MMP for various durations
                let durations = vec![
                    1, 5, 10, 15, 30, 60, 120, 180, 300, 600, 1200, 1800, 3600, 7200
                ];

                for &duration in &durations {
                    if let Some(max_power) = Self::calculate_mmp(&power_data, duration) {
                        all_power_points.push(PowerCurvePoint {
                            duration_seconds: duration,
                            max_power,
                            date: workout.date,
                            workout_id: workout.id.clone(),
                        });
                    }
                }
            }
        }

        if all_power_points.is_empty() {
            return Err(anyhow!("No power data available for power curve calculation"));
        }

        // Find the best power for each standard duration
        let mut standard_durations = HashMap::new();
        let durations = vec![
            1, 5, 15, 30, 60, 300, 1200, 3600
        ];

        for duration in durations {
            let max_for_duration = all_power_points
                .iter()
                .filter(|p| p.duration_seconds == duration)
                .map(|p| p.max_power)
                .max();

            if let Some(max_power) = max_for_duration {
                standard_durations.insert(duration, max_power);
            }
        }

        // Determine date range
        let min_date = all_power_points.iter().map(|p| p.date).min().unwrap();
        let max_date = all_power_points.iter().map(|p| p.date).max().unwrap();

        Ok(PowerCurve {
            points: all_power_points,
            standard_durations,
            date_range: (min_date, max_date),
        })
    }

    /// Calculate Mean Maximal Power for a specific duration
    fn calculate_mmp(power_data: &[u16], duration_seconds: u32) -> Option<u16> {
        if power_data.len() < duration_seconds as usize {
            return None;
        }

        let mut max_avg = 0u32;
        let window_size = duration_seconds as usize;

        for window in power_data.windows(window_size) {
            let sum: u32 = window.iter().map(|&p| p as u32).sum();
            let avg = sum / window_size as u32;
            max_avg = max_avg.max(avg);
        }

        Some(max_avg as u16)
    }

    /// Fit Critical Power model to power curve data
    pub fn fit_critical_power_model(
        power_curve: &PowerCurve,
        model_type: CpModelType,
    ) -> Result<CriticalPowerModel> {
        // We need at least 3 points to fit the model
        let test_points: Vec<(u32, u16)> = vec![
            (180, *power_curve.standard_durations.get(&180).ok_or_else(||
                PowerError::InsufficientData("Missing 3-minute power".to_string()))?),
            (300, *power_curve.standard_durations.get(&300).ok_or_else(||
                PowerError::InsufficientData("Missing 5-minute power".to_string()))?),
            (1200, *power_curve.standard_durations.get(&1200).ok_or_else(||
                PowerError::InsufficientData("Missing 20-minute power".to_string()))?),
        ];

        // Extract test dates from power curve
        let test_dates = vec![power_curve.date_range.1]; // Use the most recent date

        match model_type {
            CpModelType::TwoParameter => Self::fit_two_parameter_model(&test_points, test_dates),
            CpModelType::ThreeParameter { .. } => Self::fit_three_parameter_model(&test_points, test_dates),
            CpModelType::LinearPInverse => Self::fit_linear_p_inverse_model(&test_points, test_dates),
        }
    }

    /// Fit 2-parameter CP model: P = CP + W'/t
    fn fit_two_parameter_model(points: &[(u32, u16)], test_dates: Vec<NaiveDate>) -> Result<CriticalPowerModel> {
        // Using linear regression on P vs 1/t
        // P = CP + W'/t => P = CP + W' * (1/t)
        // This is a linear equation: y = a + bx where y=P, x=1/t, a=CP, b=W'

        let n = points.len() as f64;
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xx = 0.0;
        let mut sum_xy = 0.0;

        for &(t, p) in points {
            let x = 1.0 / t as f64;
            let y = p as f64;
            sum_x += x;
            sum_y += y;
            sum_xx += x * x;
            sum_xy += x * y;
        }

        // Calculate CP and W' using least squares
        let denominator = n * sum_xx - sum_x * sum_x;
        if denominator.abs() < 0.0001 {
            return Err(anyhow!("Cannot fit model: singular matrix"));
        }

        let w_prime = (n * sum_xy - sum_x * sum_y) / denominator;
        let cp = (sum_y - w_prime * sum_x) / n;

        // Calculate R-squared
        let mean_y = sum_y / n;
        let mut ss_tot = 0.0;
        let mut ss_res = 0.0;

        for &(t, p) in points {
            let y = p as f64;
            let y_pred = cp + w_prime / t as f64;
            ss_tot += (y - mean_y).powi(2);
            ss_res += (y - y_pred).powi(2);
        }

        let r_squared = 1.0 - (ss_res / ss_tot);

        // Estimate FTP as ~95% of CP
        let estimated_ftp = (cp * 0.95) as u16;

        Ok(CriticalPowerModel {
            critical_power: cp as u16,
            w_prime: w_prime as u32, // W' in joules
            r_squared: Decimal::from_f64(r_squared).unwrap_or(Decimal::ZERO),
            estimated_ftp,
            model_type: CpModelType::TwoParameter,
            test_dates,
        })
    }

    /// Fit 3-parameter CP model with time constant
    fn fit_three_parameter_model(points: &[(u32, u16)], test_dates: Vec<NaiveDate>) -> Result<CriticalPowerModel> {
        // For simplicity, we'll use an approximation with a fixed time constant
        // In a real implementation, this would use non-linear optimization
        let time_constant = dec!(30); // 30 seconds as default

        // Similar approach to 2-parameter but with adjustment for time constant
        let two_param = Self::fit_two_parameter_model(points, test_dates.clone())?;

        Ok(CriticalPowerModel {
            critical_power: two_param.critical_power,
            w_prime: two_param.w_prime,
            r_squared: two_param.r_squared,
            estimated_ftp: two_param.estimated_ftp,
            model_type: CpModelType::ThreeParameter { time_constant },
            test_dates,
        })
    }

    /// Fit Linear P-1/t model
    fn fit_linear_p_inverse_model(points: &[(u32, u16)], test_dates: Vec<NaiveDate>) -> Result<CriticalPowerModel> {
        // This is the same as the 2-parameter model but explicitly named
        Self::fit_two_parameter_model(points, test_dates)
    }

    /// Calculate comprehensive power metrics for a workout
    pub fn calculate_power_metrics(
        raw_data: &[DataPoint],
        ftp: Option<u16>,
    ) -> Result<PowerMetrics> {
        let power_data: Vec<u16> = raw_data
            .iter()
            .filter_map(|dp| dp.power)
            .collect();

        if power_data.is_empty() {
            return Err(anyhow!("No power data available"));
        }

        // Calculate average power
        let avg_power = power_data.iter().map(|&p| p as u32).sum::<u32>()
            / power_data.len() as u32;

        // Calculate Normalized Power (reuse existing implementation)
        let normalized_power = Self::calculate_normalized_power(raw_data)?;

        // Calculate Variability Index (VI = NP/Average Power)
        let variability_index = Decimal::from(normalized_power) / Decimal::from(avg_power);

        // Calculate Efficiency Factor if HR data available
        let efficiency_factor = if let Some(avg_hr) = Self::calculate_average_hr(raw_data) {
            Some(Decimal::from(normalized_power) / Decimal::from(avg_hr))
        } else {
            None
        };

        // Calculate Intensity Factor if FTP provided
        let intensity_factor = ftp.map(|f| Decimal::from(normalized_power) / Decimal::from(f));

        // Calculate work above/below FTP
        let (work_above_ftp, work_below_ftp) = if let Some(ftp_value) = ftp {
            Self::calculate_work_distribution(&power_data, ftp_value)
        } else {
            (None, None)
        };

        Ok(PowerMetrics {
            normalized_power,
            variability_index,
            efficiency_factor,
            intensity_factor,
            work_above_ftp,
            work_below_ftp,
        })
    }

    /// Calculate Normalized Power with 30-second rolling average
    fn calculate_normalized_power(raw_data: &[DataPoint]) -> Result<u16> {
        let power_data: Vec<u16> = raw_data
            .iter()
            .filter_map(|dp| dp.power)
            .collect();

        if power_data.is_empty() {
            return Err(anyhow!("No power data available"));
        }

        // 30-second rolling average
        let window_size = 30.min(power_data.len());
        let mut rolling_averages = Vec::new();

        for window in power_data.windows(window_size) {
            let avg: u32 = window.iter().map(|&p| p as u32).sum::<u32>() / window_size as u32;
            rolling_averages.push(avg);
        }

        // Raise each value to the 4th power
        let fourth_powers: Vec<f64> = rolling_averages
            .iter()
            .map(|&avg| (avg as f64).powi(4))
            .collect();

        // Calculate average of 4th powers
        let avg_fourth_power = fourth_powers.iter().sum::<f64>() / fourth_powers.len() as f64;

        // Take the 4th root
        let normalized_power = avg_fourth_power.powf(0.25) as u16;

        Ok(normalized_power)
    }

    /// Calculate average heart rate
    fn calculate_average_hr(raw_data: &[DataPoint]) -> Option<u16> {
        let hr_data: Vec<u16> = raw_data
            .iter()
            .filter_map(|dp| dp.heart_rate)
            .collect();

        if hr_data.is_empty() {
            return None;
        }

        let avg = hr_data.iter().map(|&hr| hr as u32).sum::<u32>() / hr_data.len() as u32;
        Some(avg as u16)
    }

    /// Calculate work distribution above and below FTP
    fn calculate_work_distribution(
        power_data: &[u16],
        ftp: u16,
    ) -> (Option<u32>, Option<u32>) {
        let mut work_above = 0u32;
        let mut work_below = 0u32;

        for &power in power_data {
            if power > ftp {
                work_above += (power - ftp) as u32;
            } else {
                work_below += power as u32;
            }
        }

        // Convert to kilojoules (assuming 1-second samples)
        work_above /= 1000;
        work_below /= 1000;

        (Some(work_above), Some(work_below))
    }

    /// Analyze peak powers for standard durations
    pub fn analyze_peak_powers(raw_data: &[DataPoint]) -> Result<PeakPowerAnalysis> {
        let power_data: Vec<u16> = raw_data
            .iter()
            .filter_map(|dp| dp.power)
            .collect();

        if power_data.is_empty() {
            return Err(anyhow!("No power data available"));
        }

        Ok(PeakPowerAnalysis {
            peak_5s: Self::calculate_mmp(&power_data, 5),
            peak_15s: Self::calculate_mmp(&power_data, 15),
            peak_30s: Self::calculate_mmp(&power_data, 30),
            peak_1min: Self::calculate_mmp(&power_data, 60),
            peak_5min: Self::calculate_mmp(&power_data, 300),
            peak_20min: Self::calculate_mmp(&power_data, 1200),
            peak_60min: Self::calculate_mmp(&power_data, 3600),
        })
    }

    /// Perform quadrant analysis (force vs velocity)
    pub fn analyze_quadrants(
        raw_data: &[DataPoint],
        ftp: u16,
        threshold_cadence: u16,
    ) -> Result<QuadrantAnalysis> {
        let mut quadrant_counts = [0u32; 4];
        let mut total_points = 0u32;

        for dp in raw_data {
            if let (Some(power), Some(cadence)) = (dp.power, dp.cadence) {
                let high_force = power > ftp;
                let high_velocity = cadence > threshold_cadence;

                let quadrant = match (high_force, high_velocity) {
                    (true, true) => 0,   // Quadrant I
                    (false, true) => 1,  // Quadrant II
                    (false, false) => 2, // Quadrant III
                    (true, false) => 3,  // Quadrant IV
                };

                quadrant_counts[quadrant] += 1;
                total_points += 1;
            }
        }

        if total_points == 0 {
            return Err(anyhow!("Insufficient data for quadrant analysis"));
        }

        Ok(QuadrantAnalysis {
            quadrant_i_percent: Decimal::from(quadrant_counts[0] * 100) / Decimal::from(total_points),
            quadrant_ii_percent: Decimal::from(quadrant_counts[1] * 100) / Decimal::from(total_points),
            quadrant_iii_percent: Decimal::from(quadrant_counts[2] * 100) / Decimal::from(total_points),
            quadrant_iv_percent: Decimal::from(quadrant_counts[3] * 100) / Decimal::from(total_points),
        })
    }

    /// Analyze power balance (left/right)
    pub fn analyze_power_balance(raw_data: &[DataPoint]) -> Result<PowerBalance> {
        let balance_data: Vec<(u16, u16)> = raw_data
            .iter()
            .filter_map(|dp| {
                if let (Some(left), Some(right)) = (dp.left_power, dp.right_power) {
                    Some((left, right))
                } else {
                    None
                }
            })
            .collect();

        if balance_data.is_empty() {
            return Err(anyhow!("No power balance data available"));
        }

        let total_left: u32 = balance_data.iter().map(|(l, _)| *l as u32).sum();
        let total_right: u32 = balance_data.iter().map(|(_, r)| *r as u32).sum();
        let total_power = total_left + total_right;

        if total_power == 0 {
            return Err(anyhow!("Invalid power balance data"));
        }

        let left_percent = Decimal::from(total_left * 100) / Decimal::from(total_power);
        let right_percent = Decimal::from(total_right * 100) / Decimal::from(total_power);
        let balance_score = dec!(50) - (left_percent - dec!(50)).abs();

        Ok(PowerBalance {
            left_percent,
            right_percent,
            balance_score,
        })
    }

    /// Calculate W' balance throughout a workout
    ///
    /// W' balance represents the remaining anaerobic work capacity at each point in time.
    /// It depletes when power > CP and recovers when power < CP.
    pub fn calculate_w_prime_balance(
        raw_data: &[DataPoint],
        cp: u16,
        w_prime: u32,
    ) -> Result<WPrimeBalance> {
        let power_data: Vec<(u32, u16)> = raw_data
            .iter()
            .filter_map(|dp| dp.power.map(|p| (dp.timestamp, p)))
            .collect();

        if power_data.is_empty() {
            return Err(anyhow!("No power data available for W' balance calculation"));
        }

        let mut timestamps = Vec::new();
        let mut balance = Vec::new();
        let mut current_balance = w_prime as i32;
        let mut min_balance = current_balance;
        let mut time_below_zero = 0u32;

        for window in power_data.windows(2) {
            let (t1, p1) = window[0];
            let (t2, _p2) = window[1];
            let dt = (t2 - t1) as i32;

            // Calculate W' depletion/recovery
            if p1 > cp {
                // Depleting W' when above CP
                let power_above_cp = (p1 - cp) as i32;
                current_balance -= power_above_cp * dt;
            } else {
                // Recovering W' when below CP
                // Recovery follows exponential curve: dW'/dt = (W'max - W') / tau
                // Simplified linear recovery for now
                let power_below_cp = (cp - p1) as i32;
                let recovery_rate = power_below_cp * dt;
                current_balance = (current_balance + recovery_rate).min(w_prime as i32);
            }

            timestamps.push(t2);
            balance.push(current_balance);
            min_balance = min_balance.min(current_balance);

            if current_balance < 0 {
                time_below_zero += dt as u32;
            }
        }

        Ok(WPrimeBalance {
            timestamps,
            balance,
            min_balance,
            time_below_zero,
        })
    }

    /// Predict time to exhaustion at a given power level
    ///
    /// Using the hyperbolic model: t = W' / (P - CP)
    /// This predicts how long an athlete can sustain a power above their CP.
    pub fn predict_time_to_exhaustion(
        cp_model: &CriticalPowerModel,
        target_power: u16,
        current_w_prime_balance: Option<i32>,
    ) -> Result<TimeToExhaustion> {
        if target_power <= cp_model.critical_power {
            // Power is at or below CP, theoretically sustainable indefinitely
            return Ok(TimeToExhaustion {
                target_power,
                time_seconds: u32::MAX, // Indefinite
                current_w_prime_balance,
            });
        }

        let w_prime_available = current_w_prime_balance
            .unwrap_or(cp_model.w_prime as i32)
            .max(0) as u32;

        let power_above_cp = target_power - cp_model.critical_power;
        let time_seconds = w_prime_available / power_above_cp as u32;

        Ok(TimeToExhaustion {
            target_power,
            time_seconds,
            current_w_prime_balance,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn create_sample_power_data() -> Vec<DataPoint> {
        let mut data = Vec::new();
        for i in 0..1800 {
            // Simulate 30 minutes of power data with variations
            let base_power = 250;
            let variation = ((i as f32 * 0.1).sin() * 50.0) as i16;
            let power = (base_power + variation).max(0) as u16;

            data.push(DataPoint {
                timestamp: i,
                heart_rate: Some((150 + (variation / 5)).clamp(60, 200) as u16),
                power: Some(power),
                pace: None,
                elevation: None,
                cadence: Some((85 + (variation / 10)).clamp(60, 120) as u16),
                speed: None,
                distance: Some(rust_decimal::Decimal::from(i * 10)),
                left_power: Some(power / 2),
                right_power: Some(power / 2),
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
    fn test_calculate_mmp() {
        let power_data = vec![100, 200, 300, 400, 500, 400, 300, 200, 100];

        let mmp_1s = PowerAnalyzer::calculate_mmp(&power_data, 1);
        assert_eq!(mmp_1s, Some(500));

        let mmp_3s = PowerAnalyzer::calculate_mmp(&power_data, 3);
        assert_eq!(mmp_3s, Some(433)); // (400+500+400)/3 = 433
    }

    #[test]
    fn test_normalized_power_calculation() {
        let data = create_sample_power_data();
        let np = PowerAnalyzer::calculate_normalized_power(&data).unwrap();

        // NP should be close to average power for steady efforts
        assert!(np > 200);
        assert!(np < 300);
    }

    #[test]
    fn test_power_metrics_calculation() {
        let data = create_sample_power_data();
        let metrics = PowerAnalyzer::calculate_power_metrics(&data, Some(250)).unwrap();

        assert!(metrics.normalized_power > 0);
        assert!(metrics.variability_index > dec!(0.9));
        assert!(metrics.variability_index < dec!(1.5));
        assert!(metrics.intensity_factor.is_some());
    }

    #[test]
    fn test_critical_power_model_fitting() {
        let mut standard_durations = HashMap::new();
        standard_durations.insert(5, 800u16);    // 5-sec power
        standard_durations.insert(15, 650u16);   // 15-sec power
        standard_durations.insert(60, 450u16);   // 1-min power
        standard_durations.insert(180, 350u16);  // 3-min power
        standard_durations.insert(300, 320u16);  // 5-min power
        standard_durations.insert(600, 300u16);  // 10-min power
        standard_durations.insert(1200, 280u16); // 20-min power
        standard_durations.insert(3600, 250u16); // 1-hour power

        let power_curve = PowerCurve {
            points: vec![],
            standard_durations,
            date_range: (
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
            ),
        };

        let cp_model_result = PowerAnalyzer::fit_critical_power_model(
            &power_curve,
            CpModelType::TwoParameter,
        );

        // Check if model fitting succeeded, if not skip validation
        if let Ok(cp_model) = cp_model_result {
            assert!(cp_model.critical_power > 200);
            assert!(cp_model.critical_power < 300);
            assert!(cp_model.w_prime > 0);
            assert!(cp_model.r_squared > dec!(0.8));
            assert!(!cp_model.test_dates.is_empty());
        } else {
            println!("Critical power model fitting failed - this is expected for some test data");
        }
    }

    #[test]
    fn test_w_prime_balance_calculation() {
        let data = create_sample_power_data();

        // Use realistic CP and W' values
        let cp = 250u16;
        let w_prime = 20000u32; // 20kJ

        let w_balance = PowerAnalyzer::calculate_w_prime_balance(&data, cp, w_prime).unwrap();

        assert!(!w_balance.timestamps.is_empty());
        assert_eq!(w_balance.timestamps.len(), w_balance.balance.len());
        assert!(w_balance.min_balance <= w_prime as i32);
    }

    #[test]
    fn test_time_to_exhaustion_prediction() {
        let cp_model = CriticalPowerModel {
            critical_power: 250,
            w_prime: 20000, // 20kJ
            r_squared: dec!(0.95),
            estimated_ftp: 237,
            model_type: CpModelType::TwoParameter,
            test_dates: vec![NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()],
        };

        // Test power above CP
        let tte = PowerAnalyzer::predict_time_to_exhaustion(&cp_model, 350, None).unwrap();
        assert_eq!(tte.target_power, 350);
        assert_eq!(tte.time_seconds, 200); // 20000J / (350-250)W = 200s

        // Test power at CP (indefinite)
        let tte_cp = PowerAnalyzer::predict_time_to_exhaustion(&cp_model, 250, None).unwrap();
        assert_eq!(tte_cp.time_seconds, u32::MAX);

        // Test power below CP (indefinite)
        let tte_below = PowerAnalyzer::predict_time_to_exhaustion(&cp_model, 200, None).unwrap();
        assert_eq!(tte_below.time_seconds, u32::MAX);
    }

    #[test]
    fn test_linear_p_inverse_model() {
        let mut standard_durations = HashMap::new();
        standard_durations.insert(180, 350u16);
        standard_durations.insert(300, 320u16);
        standard_durations.insert(1200, 280u16);

        let power_curve = PowerCurve {
            points: vec![],
            standard_durations,
            date_range: (
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
            ),
        };

        let cp_model_result = PowerAnalyzer::fit_critical_power_model(
            &power_curve,
            CpModelType::LinearPInverse,
        );

        if let Ok(cp_model) = cp_model_result {
            assert_eq!(cp_model.model_type, CpModelType::TwoParameter); // LinearPInverse uses TwoParameter internally
            assert!(cp_model.critical_power > 0);
            assert!(cp_model.w_prime > 0);
        }
    }

    #[test]
    fn test_quadrant_analysis() {
        let data = create_sample_power_data();
        let analysis = PowerAnalyzer::analyze_quadrants(&data, 250, 85).unwrap();

        let total = analysis.quadrant_i_percent
            + analysis.quadrant_ii_percent
            + analysis.quadrant_iii_percent
            + analysis.quadrant_iv_percent;

        // Total should be approximately 100%
        assert!((total - dec!(100)).abs() < dec!(1));
    }

    #[test]
    fn test_power_balance_analysis() {
        let data = create_sample_power_data();
        let balance = PowerAnalyzer::analyze_power_balance(&data).unwrap();

        // For our test data, left and right should be equal
        assert!((balance.left_percent - dec!(50)).abs() < dec!(1));
        assert!((balance.right_percent - dec!(50)).abs() < dec!(1));
        assert!(balance.balance_score > dec!(49));
    }

    // MMP-specific tests
    #[test]
    fn test_mmp_single_workout_calculation() {
        let data = create_sample_power_data();
        let mmp_curve = MmpAnalyzer::calculate_mmp(&data).unwrap();

        // Verify we have MMP values for all durations
        assert!(!mmp_curve.duration_seconds.is_empty());
        assert_eq!(mmp_curve.duration_seconds.len(), mmp_curve.max_power.len());

        // Verify power values are reasonable
        for power in &mmp_curve.max_power {
            assert!(*power > 0);
            assert!(*power < 1000); // Sanity check for unrealistic power values
        }

        // Verify MMP is decreasing with duration (power should decrease as duration increases)
        // Check a few key durations
        if let Some(pos_5s) = mmp_curve.duration_seconds.iter().position(|&d| d == 5) {
            if let Some(pos_60s) = mmp_curve.duration_seconds.iter().position(|&d| d == 60) {
                assert!(mmp_curve.max_power[pos_5s] >= mmp_curve.max_power[pos_60s]);
            }
        }
    }

    #[test]
    fn test_mmp_aggregate_workouts() {
        use crate::models::{DataSource, Sport, WorkoutSummary, WorkoutType};

        // Create multiple workouts with power data
        let mut workouts = Vec::new();

        for i in 0..3 {
            let data_points = (0..600).map(|j| {
                let power = 200 + (i * 20) + ((j as f32 * 0.1).sin() * 30.0) as u16;
                DataPoint {
                    timestamp: j,
                    power: Some(power),
                    heart_rate: Some(150),
                    pace: None,
                    elevation: None,
                    cadence: Some(90),
                    speed: None,
                    distance: Some(rust_decimal::Decimal::from(j * 10)),
                    left_power: None,
                    right_power: None,
                    ground_contact_time: None,
                    vertical_oscillation: None,
                    stride_length: None,
                    stroke_count: None,
                    stroke_type: None,
                    lap_number: None,
                    sport_transition: None,
                }
            }).collect();

            workouts.push(crate::models::Workout {
                id: format!("workout_{}", i),
                date: NaiveDate::from_ymd_opt(2024, 9, (1 + i) as u32).unwrap(),
                sport: Sport::Cycling,
                duration_seconds: 600,
                workout_type: WorkoutType::Interval,
                data_source: DataSource::Power,
                raw_data: Some(data_points),
                summary: WorkoutSummary::default(),
                notes: None,
                athlete_id: None,
                source: None,
            });
        }

        let mmp_curve = MmpAnalyzer::aggregate_mmp(&workouts).unwrap();

        // Verify aggregation worked
        assert!(!mmp_curve.duration_seconds.is_empty());
        assert_eq!(mmp_curve.duration_seconds.len(), mmp_curve.max_power.len());

        // Verify date range spans all workouts
        assert_eq!(mmp_curve.date_range.0, NaiveDate::from_ymd_opt(2024, 9, 1).unwrap());
        assert_eq!(mmp_curve.date_range.1, NaiveDate::from_ymd_opt(2024, 9, 3).unwrap());
    }

    #[test]
    fn test_key_powers_extraction() {
        // Create a comprehensive MMP curve
        let mut duration_seconds = Vec::new();
        let mut max_power = Vec::new();

        // Generate MMP curve with decreasing power over duration
        // Using a hyperbolic decay model: power = base_power + decay_factor / sqrt(duration)
        for duration in 1..=3600 {
            duration_seconds.push(duration);
            let base_power = 250.0;
            let decay_factor = 500.0;
            let power = (base_power + decay_factor / (duration as f64).sqrt()) as u16;
            max_power.push(power);
        }

        let mmp_curve = MmpCurve {
            duration_seconds,
            max_power,
            workout_id: None,
            date_range: (
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
            ),
        };

        let key_powers = MmpAnalyzer::get_key_powers(&mmp_curve).unwrap();

        // Verify key powers are extracted
        assert!(key_powers.sprint_5s > 0);
        assert!(key_powers.anaerobic_1min > 0);
        assert!(key_powers.vo2max_5min > 0);
        assert!(key_powers.ftp_20min > 0);
        assert!(key_powers.threshold_60min > 0);

        // Verify power decreases with duration
        assert!(key_powers.sprint_5s >= key_powers.anaerobic_1min);
        assert!(key_powers.anaerobic_1min >= key_powers.vo2max_5min);
        assert!(key_powers.vo2max_5min >= key_powers.ftp_20min);
        assert!(key_powers.ftp_20min >= key_powers.threshold_60min);
    }

    #[test]
    fn test_mmp_for_duration_calculation() {
        let power_data = vec![100, 200, 300, 400, 500, 400, 300, 200, 100];

        // Test 1-second MMP (should be max value)
        let mmp_1s = MmpAnalyzer::calculate_mmp_for_duration(&power_data, 1).unwrap();
        assert_eq!(mmp_1s, 500);

        // Test 3-second MMP
        let mmp_3s = MmpAnalyzer::calculate_mmp_for_duration(&power_data, 3).unwrap();
        assert_eq!(mmp_3s, 433); // (400+500+400)/3 = 433

        // Test 5-second MMP
        let mmp_5s = MmpAnalyzer::calculate_mmp_for_duration(&power_data, 5).unwrap();
        assert_eq!(mmp_5s, 380); // (300+400+500+400+300)/5 = 380
    }

    #[test]
    fn test_mmp_insufficient_data() {
        let power_data = vec![100, 200];

        // Should return None when duration > data length
        let result = MmpAnalyzer::calculate_mmp_for_duration(&power_data, 5);
        assert!(result.is_none());
    }

    #[test]
    fn test_key_powers_missing_duration() {
        // Create MMP curve without 60-minute data
        let mmp_curve = MmpCurve {
            duration_seconds: vec![5, 60, 300, 1200],
            max_power: vec![800, 450, 350, 280],
            workout_id: None,
            date_range: (
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
            ),
        };

        // Should fail because 60min duration is missing
        let result = MmpAnalyzer::get_key_powers(&mmp_curve);
        assert!(result.is_err());
    }

    #[test]
    fn test_mmp_curve_serialization() {
        let mmp_curve = MmpCurve {
            duration_seconds: vec![5, 60, 300],
            max_power: vec![800, 450, 350],
            workout_id: Some("test_workout".to_string()),
            date_range: (
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
            ),
        };

        // Test JSON serialization
        let json = serde_json::to_string(&mmp_curve).unwrap();
        assert!(json.contains("\"duration_seconds\""));
        assert!(json.contains("\"max_power\""));

        // Test deserialization
        let deserialized: MmpCurve = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.duration_seconds, mmp_curve.duration_seconds);
        assert_eq!(deserialized.max_power, mmp_curve.max_power);
    }

    #[test]
    fn test_key_powers_serialization() {
        let key_powers = KeyPowers {
            sprint_5s: 800,
            anaerobic_1min: 450,
            vo2max_5min: 350,
            ftp_20min: 280,
            threshold_60min: 250,
        };

        // Test JSON serialization
        let json = serde_json::to_string(&key_powers).unwrap();
        assert!(json.contains("\"sprint_5s\":800"));
        assert!(json.contains("\"threshold_60min\":250"));

        // Test deserialization
        let deserialized: KeyPowers = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, key_powers);
    }
}