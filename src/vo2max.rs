//! VO2max estimation module
//!
//! Provides multiple methods for estimating VO2max (maximal oxygen uptake) from
//! heart rate, power, and pace data using validated sports science models.
//!
//! VO2max represents the maximum rate of oxygen consumption during incremental exercise
//! and is the gold standard measure of aerobic fitness.

use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

/// VO2max estimation methods
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vo2MaxMethod {
    /// ACSM running metabolic equation
    AcsmRunning,
    /// ACSM cycling metabolic equation
    AcsmCycling,
    /// Power-based estimation from critical power
    PowerBased,
    /// Heart rate reserve method
    HeartRateReserve,
    /// Combined multi-signal estimation
    Combined,
}

/// VO2max estimate with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vo2MaxEstimate {
    /// VO2max in ml/kg/min
    pub vo2max_ml_kg_min: f64,
    /// Estimation method used
    pub estimation_method: Vo2MaxMethod,
    /// Confidence level (0.0-1.0)
    pub confidence: f64,
    /// Date of estimation
    pub date: NaiveDate,
    /// Contributing workout IDs
    pub contributing_workouts: Vec<String>,
}

/// VO2max trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vo2MaxTrend {
    /// Linear regression slope (ml/kg/min per day)
    pub trend_slope: f64,
    /// Average VO2max over period
    pub average_vo2max: f64,
    /// Minimum VO2max in period
    pub min_vo2max: f64,
    /// Maximum VO2max in period
    pub max_vo2max: f64,
    /// Number of estimates in trend
    pub estimate_count: usize,
    /// Date range for trend
    pub date_range: (NaiveDate, NaiveDate),
}

/// VO2max estimation errors
#[derive(Debug, thiserror::Error)]
pub enum Vo2MaxError {
    #[error("Insufficient data: {0}")]
    InsufficientData(String),
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    #[error("Calculation error: {0}")]
    CalculationError(String),
}

/// VO2max analyzer for estimation and trend tracking
pub struct Vo2MaxAnalyzer;

impl Vo2MaxAnalyzer {
    /// Estimate VO2max from critical power (cycling)
    ///
    /// Uses the relationship: VO2max ≈ (CP × 10.8) / body_mass + 7
    /// Based on research showing CP correlates strongly with VO2max
    ///
    /// # Arguments
    /// * `cp` - Critical power in watts
    /// * `body_mass_kg` - Athlete body mass in kilograms
    ///
    /// # Returns
    /// VO2max estimate with confidence score
    pub fn estimate_from_power(cp: u16, body_mass_kg: f64) -> Result<Vo2MaxEstimate> {
        if body_mass_kg <= 0.0 {
            return Err(anyhow!(Vo2MaxError::InvalidParameter(
                "Body mass must be positive".to_string()
            )));
        }

        if cp == 0 {
            return Err(anyhow!(Vo2MaxError::InvalidParameter(
                "Critical power must be positive".to_string()
            )));
        }

        // VO2max ≈ (CP × 10.8) / body_mass + 7
        let vo2max = (cp as f64 * 10.8) / body_mass_kg + 7.0;

        // Confidence based on typical CP measurement accuracy
        // Higher CP values relative to body mass = higher confidence
        let power_to_mass_ratio = cp as f64 / body_mass_kg;
        let confidence = if power_to_mass_ratio > 3.0 {
            0.85
        } else if power_to_mass_ratio > 2.0 {
            0.75
        } else {
            0.65
        };

        Ok(Vo2MaxEstimate {
            vo2max_ml_kg_min: vo2max,
            estimation_method: Vo2MaxMethod::PowerBased,
            confidence,
            date: chrono::Local::now().date_naive(),
            contributing_workouts: Vec::new(),
        })
    }

    /// Estimate VO2max from running pace using ACSM metabolic equation
    ///
    /// ACSM Running Equation: VO2 = 0.2×speed + 0.9×speed×grade + 3.5
    /// For flat ground (grade=0): VO2 = 0.2×speed + 3.5
    /// Speed in m/min, VO2 in ml/kg/min
    ///
    /// # Arguments
    /// * `pace` - Running pace in min/km
    /// * `hr` - Heart rate during run
    /// * `max_hr` - Maximum heart rate
    ///
    /// # Returns
    /// VO2max estimate assuming effort was at ~90% max HR
    pub fn estimate_from_running(
        pace: Decimal,
        hr: u16,
        max_hr: u16,
    ) -> Result<Vo2MaxEstimate> {
        if pace <= dec!(0) {
            return Err(anyhow!(Vo2MaxError::InvalidParameter(
                "Pace must be positive".to_string()
            )));
        }

        if hr == 0 || max_hr == 0 {
            return Err(anyhow!(Vo2MaxError::InvalidParameter(
                "Heart rates must be positive".to_string()
            )));
        }

        // Convert pace (min/km) to speed (m/min)
        // pace in min/km → speed = 1000 / pace m/min
        let pace_f64 = pace.to_string().parse::<f64>().unwrap_or(0.0);
        let speed_m_per_min = 1000.0 / pace_f64;

        // ACSM equation for flat running: VO2 = 0.2×speed + 3.5
        let vo2_at_pace = 0.2 * speed_m_per_min + 3.5;

        // Adjust to VO2max based on HR% (assuming linear relationship)
        let hr_percent = hr as f64 / max_hr as f64;
        let vo2max = vo2_at_pace / hr_percent;

        // Confidence based on HR zone (higher HR = more accurate)
        let confidence = if hr_percent > 0.85 {
            0.85
        } else if hr_percent > 0.75 {
            0.70
        } else {
            0.55
        };

        Ok(Vo2MaxEstimate {
            vo2max_ml_kg_min: vo2max,
            estimation_method: Vo2MaxMethod::AcsmRunning,
            confidence,
            date: chrono::Local::now().date_naive(),
            contributing_workouts: Vec::new(),
        })
    }

    /// Estimate VO2max from cycling power using ACSM metabolic equation
    ///
    /// ACSM Cycling Equation: VO2 = (12×power_watts)/body_mass + 3.5
    /// Where power is in watts and body mass in kg
    ///
    /// # Arguments
    /// * `power` - Power output in watts
    /// * `hr` - Heart rate during effort
    /// * `body_mass_kg` - Athlete body mass in kg
    ///
    /// # Returns
    /// VO2max estimate
    pub fn estimate_from_cycling(
        power: u16,
        hr: u16,
        max_hr: u16,
        body_mass_kg: f64,
    ) -> Result<Vo2MaxEstimate> {
        if power == 0 {
            return Err(anyhow!(Vo2MaxError::InvalidParameter(
                "Power must be positive".to_string()
            )));
        }

        if body_mass_kg <= 0.0 {
            return Err(anyhow!(Vo2MaxError::InvalidParameter(
                "Body mass must be positive".to_string()
            )));
        }

        if hr == 0 || max_hr == 0 {
            return Err(anyhow!(Vo2MaxError::InvalidParameter(
                "Heart rates must be positive".to_string()
            )));
        }

        // ACSM cycling equation: VO2 = (12×power)/body_mass + 3.5
        let vo2_at_power = (12.0 * power as f64) / body_mass_kg + 3.5;

        // Adjust to VO2max based on HR%
        let hr_percent = hr as f64 / max_hr as f64;
        let vo2max = vo2_at_power / hr_percent;

        // Confidence based on HR zone
        let confidence = if hr_percent > 0.85 {
            0.80
        } else if hr_percent > 0.75 {
            0.65
        } else {
            0.50
        };

        Ok(Vo2MaxEstimate {
            vo2max_ml_kg_min: vo2max,
            estimation_method: Vo2MaxMethod::AcsmCycling,
            confidence,
            date: chrono::Local::now().date_naive(),
            contributing_workouts: Vec::new(),
        })
    }

    /// Estimate VO2max from heart rate reserve
    ///
    /// Uses Swain formula: VO2R% = HRR%
    /// Where VO2R = VO2max - VO2rest and HRR = HRmax - HRrest
    ///
    /// # Arguments
    /// * `current_hr` - Current heart rate during exercise
    /// * `resting_hr` - Resting heart rate
    /// * `max_hr` - Maximum heart rate
    /// * `estimated_vo2_at_current` - Estimated VO2 at current HR
    ///
    /// # Returns
    /// VO2max estimate using heart rate reserve method
    pub fn estimate_from_hr_reserve(
        current_hr: u16,
        resting_hr: u16,
        max_hr: u16,
        estimated_vo2_at_current: f64,
    ) -> Result<Vo2MaxEstimate> {
        if current_hr <= resting_hr || max_hr <= resting_hr {
            return Err(anyhow!(Vo2MaxError::InvalidParameter(
                "Invalid heart rate values".to_string()
            )));
        }

        // Calculate HRR% = (current_hr - resting) / (max - resting)
        let hrr_percent = (current_hr - resting_hr) as f64 / (max_hr - resting_hr) as f64;

        // Assuming VO2rest ≈ 3.5 ml/kg/min (1 MET)
        let vo2_rest = 3.5;

        // VO2R% = HRR% → (VO2 - VO2rest) / (VO2max - VO2rest) = HRR%
        // Solving for VO2max: VO2max = (VO2 - VO2rest) / HRR% + VO2rest
        let vo2max = (estimated_vo2_at_current - vo2_rest) / hrr_percent + vo2_rest;

        // Confidence based on HRR zone
        let confidence = if hrr_percent > 0.80 {
            0.75
        } else if hrr_percent > 0.60 {
            0.65
        } else {
            0.50
        };

        Ok(Vo2MaxEstimate {
            vo2max_ml_kg_min: vo2max,
            estimation_method: Vo2MaxMethod::HeartRateReserve,
            confidence,
            date: chrono::Local::now().date_naive(),
            contributing_workouts: Vec::new(),
        })
    }

    /// Track VO2max trends over time
    ///
    /// Analyzes multiple VO2max estimates to identify training adaptations
    ///
    /// # Arguments
    /// * `estimates` - Collection of VO2max estimates over time
    ///
    /// # Returns
    /// Trend analysis with slope and statistics
    pub fn track_vo2max_trends(estimates: &[Vo2MaxEstimate]) -> Result<Vo2MaxTrend> {
        if estimates.is_empty() {
            return Err(anyhow!(Vo2MaxError::InsufficientData(
                "No VO2max estimates provided".to_string()
            )));
        }

        if estimates.len() < 2 {
            // Can't calculate trend with less than 2 points
            let estimate = &estimates[0];
            return Ok(Vo2MaxTrend {
                trend_slope: 0.0,
                average_vo2max: estimate.vo2max_ml_kg_min,
                min_vo2max: estimate.vo2max_ml_kg_min,
                max_vo2max: estimate.vo2max_ml_kg_min,
                estimate_count: 1,
                date_range: (estimate.date, estimate.date),
            });
        }

        // Sort by date
        let mut sorted_estimates = estimates.to_vec();
        sorted_estimates.sort_by_key(|e| e.date);

        // Calculate statistics
        let vo2max_values: Vec<f64> = sorted_estimates
            .iter()
            .map(|e| e.vo2max_ml_kg_min)
            .collect();

        let average_vo2max = vo2max_values.iter().sum::<f64>() / vo2max_values.len() as f64;
        let min_vo2max = vo2max_values
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let max_vo2max = vo2max_values
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        // Calculate linear regression slope (VO2max change per day)
        let first_date = sorted_estimates.first().unwrap().date;
        let last_date = sorted_estimates.last().unwrap().date;

        // Convert dates to days from first date
        let x_values: Vec<f64> = sorted_estimates
            .iter()
            .map(|e| (e.date - first_date).num_days() as f64)
            .collect();

        let y_values = vo2max_values;

        // Simple linear regression: y = mx + b
        let n = x_values.len() as f64;
        let sum_x: f64 = x_values.iter().sum();
        let sum_y: f64 = y_values.iter().sum();
        let sum_xy: f64 = x_values.iter().zip(y_values.iter()).map(|(x, y)| x * y).sum();
        let sum_xx: f64 = x_values.iter().map(|x| x * x).sum();

        let slope = if (n * sum_xx - sum_x * sum_x).abs() > 0.0001 {
            (n * sum_xy - sum_x * sum_y) / (n * sum_xx - sum_x * sum_x)
        } else {
            0.0
        };

        Ok(Vo2MaxTrend {
            trend_slope: slope,
            average_vo2max,
            min_vo2max,
            max_vo2max,
            estimate_count: estimates.len(),
            date_range: (first_date, last_date),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_power_based_vo2max_estimation() {
        // Typical cyclist: 250W CP, 75kg
        let estimate = Vo2MaxAnalyzer::estimate_from_power(250, 75.0).unwrap();

        // Expected: (250 × 10.8) / 75 + 7 = 43 ml/kg/min
        assert!((estimate.vo2max_ml_kg_min - 43.0).abs() < 1.0);
        assert_eq!(estimate.estimation_method, Vo2MaxMethod::PowerBased);
        assert!(estimate.confidence > 0.6);
    }

    #[test]
    fn test_running_vo2max_estimation() {
        // Runner at 4:00 min/km pace, 165 bpm, max HR 180
        let estimate = Vo2MaxAnalyzer::estimate_from_running(dec!(4.0), 165, 180).unwrap();

        // Speed = 1000/4 = 250 m/min
        // VO2 at pace = 0.2×250 + 3.5 = 53.5 ml/kg/min
        // HR% = 165/180 = 0.917
        // VO2max ≈ 53.5 / 0.917 ≈ 58.3 ml/kg/min
        assert!((estimate.vo2max_ml_kg_min - 58.3).abs() < 2.0);
        assert_eq!(estimate.estimation_method, Vo2MaxMethod::AcsmRunning);
    }

    #[test]
    fn test_cycling_vo2max_estimation() {
        // Cyclist at 300W, 170 bpm, max HR 190, 75kg
        let estimate = Vo2MaxAnalyzer::estimate_from_cycling(300, 170, 190, 75.0).unwrap();

        // VO2 at power = (12×300)/75 + 3.5 = 51.5 ml/kg/min
        // HR% = 170/190 = 0.895
        // VO2max ≈ 51.5 / 0.895 ≈ 57.5 ml/kg/min
        assert!((estimate.vo2max_ml_kg_min - 57.5).abs() < 2.0);
        assert_eq!(estimate.estimation_method, Vo2MaxMethod::AcsmCycling);
    }

    #[test]
    fn test_hr_reserve_vo2max_estimation() {
        // Current HR 160, resting 50, max 180
        // Estimated VO2 at current = 45 ml/kg/min
        let estimate = Vo2MaxAnalyzer::estimate_from_hr_reserve(160, 50, 180, 45.0).unwrap();

        // HRR% = (160-50)/(180-50) = 110/130 = 0.846
        // VO2max = (45 - 3.5) / 0.846 + 3.5 ≈ 52.5 ml/kg/min
        assert!((estimate.vo2max_ml_kg_min - 52.5).abs() < 2.0);
        assert_eq!(estimate.estimation_method, Vo2MaxMethod::HeartRateReserve);
    }

    #[test]
    fn test_vo2max_trend_tracking() {
        let estimates = vec![
            Vo2MaxEstimate {
                vo2max_ml_kg_min: 50.0,
                estimation_method: Vo2MaxMethod::PowerBased,
                confidence: 0.8,
                date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                contributing_workouts: vec![],
            },
            Vo2MaxEstimate {
                vo2max_ml_kg_min: 52.0,
                estimation_method: Vo2MaxMethod::PowerBased,
                confidence: 0.8,
                date: NaiveDate::from_ymd_opt(2024, 2, 1).unwrap(),
                contributing_workouts: vec![],
            },
            Vo2MaxEstimate {
                vo2max_ml_kg_min: 54.0,
                estimation_method: Vo2MaxMethod::PowerBased,
                confidence: 0.8,
                date: NaiveDate::from_ymd_opt(2024, 3, 1).unwrap(),
                contributing_workouts: vec![],
            },
        ];

        let trend = Vo2MaxAnalyzer::track_vo2max_trends(&estimates).unwrap();

        assert!((trend.average_vo2max - 52.0).abs() < 0.1);
        assert_eq!(trend.min_vo2max, 50.0);
        assert_eq!(trend.max_vo2max, 54.0);
        assert_eq!(trend.estimate_count, 3);
        assert!(trend.trend_slope > 0.0); // Improving fitness
    }

    #[test]
    fn test_invalid_parameters() {
        // Zero body mass
        assert!(Vo2MaxAnalyzer::estimate_from_power(250, 0.0).is_err());

        // Zero pace
        assert!(Vo2MaxAnalyzer::estimate_from_running(dec!(0), 160, 180).is_err());

        // Invalid HR values
        assert!(Vo2MaxAnalyzer::estimate_from_cycling(300, 0, 190, 75.0).is_err());

        // Invalid HR reserve
        assert!(Vo2MaxAnalyzer::estimate_from_hr_reserve(150, 160, 180, 45.0).is_err());
    }

    #[test]
    fn test_confidence_levels() {
        // High power-to-mass ratio = higher confidence
        let high_pm = Vo2MaxAnalyzer::estimate_from_power(300, 70.0).unwrap();
        let low_pm = Vo2MaxAnalyzer::estimate_from_power(150, 80.0).unwrap();
        assert!(high_pm.confidence > low_pm.confidence);

        // High HR% = higher confidence for running
        let high_hr = Vo2MaxAnalyzer::estimate_from_running(dec!(4.0), 170, 180).unwrap();
        let low_hr = Vo2MaxAnalyzer::estimate_from_running(dec!(4.0), 140, 180).unwrap();
        assert!(high_hr.confidence > low_hr.confidence);
    }

    #[test]
    fn test_single_estimate_trend() {
        let estimates = vec![Vo2MaxEstimate {
            vo2max_ml_kg_min: 55.0,
            estimation_method: Vo2MaxMethod::PowerBased,
            confidence: 0.8,
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            contributing_workouts: vec![],
        }];

        let trend = Vo2MaxAnalyzer::track_vo2max_trends(&estimates).unwrap();
        assert_eq!(trend.trend_slope, 0.0);
        assert_eq!(trend.average_vo2max, 55.0);
        assert_eq!(trend.estimate_count, 1);
    }

    #[test]
    fn test_vo2max_serialization() {
        let estimate = Vo2MaxEstimate {
            vo2max_ml_kg_min: 55.0,
            estimation_method: Vo2MaxMethod::Combined,
            confidence: 0.85,
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            contributing_workouts: vec!["workout_1".to_string()],
        };

        let json = serde_json::to_string(&estimate).unwrap();
        assert!(json.contains("\"vo2max_ml_kg_min\":55"));
        assert!(json.contains("\"Combined\""));

        let deserialized: Vo2MaxEstimate = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.vo2max_ml_kg_min, 55.0);
        assert_eq!(deserialized.estimation_method, Vo2MaxMethod::Combined);
    }
}
