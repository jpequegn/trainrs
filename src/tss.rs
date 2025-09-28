use crate::models::{AthleteProfile, DataPoint, Sport, Workout, WorkoutSummary};
use anyhow::Result;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use thiserror::Error;

/// TSS calculation errors
#[derive(Error, Debug)]
pub enum TssError {
    #[error("Missing required threshold: {0}")]
    MissingThreshold(String),
    #[error("Invalid workout data: {0}")]
    InvalidData(String),
    #[error("Calculation error: {0}")]
    CalculationError(String),
    #[error("Unsupported sport for TSS calculation: {0:?}")]
    UnsupportedSport(Sport),
}

/// TSS calculation result with method used
#[derive(Debug, Clone, PartialEq)]
pub struct TssResult {
    pub tss: Decimal,
    pub method: TssMethod,
    pub intensity_factor: Option<Decimal>,
    pub normalized_power: Option<u16>,
}

/// Methods used for TSS calculation
#[derive(Debug, Clone, PartialEq)]
pub enum TssMethod {
    PowerBased,     // Cycling with power data
    HeartRateBased, // hrTSS from heart rate zones
    PaceBased,      // rTSS for running, sTSS for swimming
    Estimated,      // Fallback estimation from available data
}

/// Core TSS calculation engine
pub struct TssCalculator;

impl TssCalculator {
    /// Calculate TSS for a workout using the best available method
    pub fn calculate_tss(
        workout: &Workout,
        athlete: &AthleteProfile,
    ) -> Result<TssResult, TssError> {
        // Try power-based TSS first for cycling
        if workout.sport == Sport::Cycling {
            if let Ok(result) = Self::calculate_power_tss(workout, athlete) {
                return Ok(result);
            }
        }

        // Try pace-based TSS for running and swimming
        if matches!(workout.sport, Sport::Running | Sport::Swimming) {
            if let Ok(result) = Self::calculate_pace_tss(workout, athlete) {
                return Ok(result);
            }
        }

        // Try heart rate TSS as fallback
        if let Ok(result) = Self::calculate_heart_rate_tss(workout, athlete) {
            return Ok(result);
        }

        // Final fallback: estimated TSS
        Self::estimate_tss(workout, athlete)
    }

    /// Calculate power-based TSS for cycling
    /// TSS = (duration_hours × IF²) × 100
    pub fn calculate_power_tss(
        workout: &Workout,
        athlete: &AthleteProfile,
    ) -> Result<TssResult, TssError> {
        let ftp = athlete
            .ftp
            .ok_or_else(|| TssError::MissingThreshold("FTP required for power-based TSS".to_string()))?;

        let raw_data = workout
            .raw_data
            .as_ref()
            .ok_or_else(|| TssError::InvalidData("Raw power data required".to_string()))?;

        // Calculate Normalized Power with 30-second rolling average
        let normalized_power = Self::calculate_normalized_power(raw_data)?;

        // Calculate Intensity Factor (IF = NP/FTP)
        let intensity_factor = Decimal::from(normalized_power) / Decimal::from(ftp);

        // Calculate duration in hours
        let duration_hours = Decimal::from(workout.duration_seconds) / Decimal::from(3600);

        // TSS = (duration_hours × IF²) × 100
        let tss = (duration_hours * intensity_factor * intensity_factor) * Decimal::from(100);

        Ok(TssResult {
            tss,
            method: TssMethod::PowerBased,
            intensity_factor: Some(intensity_factor),
            normalized_power: Some(normalized_power),
        })
    }

    /// Calculate heart rate-based TSS (hrTSS)
    /// hrTSS = duration_hours × avg_intensity_factor² × 100
    pub fn calculate_heart_rate_tss(
        workout: &Workout,
        athlete: &AthleteProfile,
    ) -> Result<TssResult, TssError> {
        let lthr = athlete
            .lthr
            .ok_or_else(|| TssError::MissingThreshold("LTHR required for heart rate TSS".to_string()))?;

        let raw_data = workout
            .raw_data
            .as_ref()
            .ok_or_else(|| TssError::InvalidData("Raw heart rate data required".to_string()))?;

        // Calculate time-weighted average intensity factor
        let avg_intensity_factor = Self::calculate_hr_intensity_factor(raw_data, lthr)?;

        // Calculate duration in hours
        let duration_hours = Decimal::from(workout.duration_seconds) / Decimal::from(3600);

        // hrTSS = duration_hours × avg_intensity_factor² × 100
        let tss = duration_hours * avg_intensity_factor * avg_intensity_factor * Decimal::from(100);

        Ok(TssResult {
            tss,
            method: TssMethod::HeartRateBased,
            intensity_factor: Some(avg_intensity_factor),
            normalized_power: None,
        })
    }

    /// Calculate pace-based TSS for running (rTSS) and swimming (sTSS)
    pub fn calculate_pace_tss(
        workout: &Workout,
        athlete: &AthleteProfile,
    ) -> Result<TssResult, TssError> {
        let threshold_pace = athlete
            .threshold_pace
            .ok_or_else(|| TssError::MissingThreshold("Threshold pace required for pace-based TSS".to_string()))?;

        let raw_data = workout
            .raw_data
            .as_ref()
            .ok_or_else(|| TssError::InvalidData("Raw pace data required".to_string()))?;

        match workout.sport {
            Sport::Running => Self::calculate_running_tss(raw_data, threshold_pace, workout.duration_seconds),
            Sport::Swimming => Self::calculate_swimming_tss(raw_data, threshold_pace, workout.duration_seconds),
            _ => Err(TssError::UnsupportedSport(workout.sport.clone())),
        }
    }

    /// Calculate running TSS with elevation adjustment
    fn calculate_running_tss(
        raw_data: &[DataPoint],
        threshold_pace: Decimal,
        duration_seconds: u32,
    ) -> Result<TssResult, TssError> {
        let mut total_intensity_factor = Decimal::ZERO;
        let mut valid_points = 0;
        let mut elevation_gain = 0i32;
        let mut prev_elevation: Option<i16> = None;

        for point in raw_data {
            if let Some(pace) = point.pace {
                // Calculate intensity factor for this pace point
                // IF = threshold_pace / current_pace (faster pace = higher IF)
                let intensity_factor = threshold_pace / pace;
                total_intensity_factor += intensity_factor;
                valid_points += 1;
            }

            // Track elevation gain for gradient adjustment
            if let Some(elevation) = point.elevation {
                if let Some(prev_elev) = prev_elevation {
                    let gain = elevation - prev_elev;
                    if gain > 0 {
                        elevation_gain += gain as i32;
                    }
                }
                prev_elevation = Some(elevation);
            }
        }

        if valid_points == 0 {
            return Err(TssError::InvalidData("No valid pace data points".to_string()));
        }

        let avg_intensity_factor = total_intensity_factor / Decimal::from(valid_points);

        // Apply elevation gradient factor (1% grade ≈ 1.02 multiplier)
        let gradient_factor = if elevation_gain > 0 {
            let avg_gradient = Decimal::from(elevation_gain) / Decimal::from(duration_seconds);
            Decimal::ONE + (avg_gradient * Decimal::from_f32(0.02).unwrap())
        } else {
            Decimal::ONE
        };

        let duration_hours = Decimal::from(duration_seconds) / Decimal::from(3600);

        // rTSS = duration_hours × avg_intensity_factor² × gradient_factor × 100
        let tss = duration_hours * avg_intensity_factor * avg_intensity_factor * gradient_factor * Decimal::from(100);

        Ok(TssResult {
            tss,
            method: TssMethod::PaceBased,
            intensity_factor: Some(avg_intensity_factor),
            normalized_power: None,
        })
    }

    /// Calculate swimming TSS based on Critical Swim Speed (CSS)
    fn calculate_swimming_tss(
        raw_data: &[DataPoint],
        css_pace: Decimal,
        duration_seconds: u32,
    ) -> Result<TssResult, TssError> {
        let mut total_intensity_factor = Decimal::ZERO;
        let mut valid_points = 0;

        for point in raw_data {
            if let Some(pace) = point.pace {
                // For swimming, faster pace (lower time) = higher intensity
                let intensity_factor = css_pace / pace;
                total_intensity_factor += intensity_factor;
                valid_points += 1;
            }
        }

        if valid_points == 0 {
            return Err(TssError::InvalidData("No valid pace data points".to_string()));
        }

        let avg_intensity_factor = total_intensity_factor / Decimal::from(valid_points);
        let duration_hours = Decimal::from(duration_seconds) / Decimal::from(3600);

        // sTSS = duration_hours × avg_intensity_factor² × 100
        let tss = duration_hours * avg_intensity_factor * avg_intensity_factor * Decimal::from(100);

        Ok(TssResult {
            tss,
            method: TssMethod::PaceBased,
            intensity_factor: Some(avg_intensity_factor),
            normalized_power: None,
        })
    }

    /// Estimate TSS when primary metrics are unavailable
    fn estimate_tss(
        workout: &Workout,
        _athlete: &AthleteProfile,
    ) -> Result<TssResult, TssError> {
        // Simple estimation based on duration and sport
        let duration_hours = Decimal::from(workout.duration_seconds) / Decimal::from(3600);

        let base_tss_per_hour = match workout.sport {
            Sport::Cycling => Decimal::from(60),      // Moderate cycling intensity
            Sport::Running => Decimal::from(70),      // Moderate running intensity
            Sport::Swimming => Decimal::from(80),     // Higher due to full-body engagement
            Sport::Rowing => Decimal::from(75),       // Full-body, high intensity
            Sport::Triathlon => Decimal::from(65),    // Mixed activity average
            Sport::CrossTraining => Decimal::from(50), // Variable intensity
        };

        let tss = duration_hours * base_tss_per_hour;

        Ok(TssResult {
            tss,
            method: TssMethod::Estimated,
            intensity_factor: None,
            normalized_power: None,
        })
    }

    /// Calculate Normalized Power with 30-second rolling average
    fn calculate_normalized_power(raw_data: &[DataPoint]) -> Result<u16, TssError> {
        if raw_data.is_empty() {
            return Err(TssError::InvalidData("No power data available".to_string()));
        }

        let mut rolling_powers: Vec<Decimal> = Vec::new();
        let window_size = 30; // 30-second rolling window

        // Calculate 30-second rolling averages
        for window_start in 0..raw_data.len() {
            let window_end = (window_start + window_size).min(raw_data.len());
            let mut sum = Decimal::ZERO;
            let mut count = 0;

            for i in window_start..window_end {
                if let Some(power) = raw_data[i].power {
                    sum += Decimal::from(power);
                    count += 1;
                }
            }

            if count > 0 {
                rolling_powers.push(sum / Decimal::from(count));
            }
        }

        if rolling_powers.is_empty() {
            return Err(TssError::InvalidData("No valid power data points".to_string()));
        }

        // Calculate the fourth power of each rolling average, then take the fourth root
        // Use f64 for intermediate calculation to avoid overflow
        let sum_fourth_powers: f64 = rolling_powers
            .iter()
            .map(|&power| {
                let power_f64 = power.to_f64().unwrap_or(0.0);
                power_f64.powi(4)
            })
            .sum();

        let avg_fourth_power = sum_fourth_powers / rolling_powers.len() as f64;

        // Take fourth root (sqrt of sqrt)
        let normalized_power_f64 = avg_fourth_power.sqrt().sqrt();
        let normalized_power = Decimal::from_f64(normalized_power_f64).unwrap_or(Decimal::ZERO);

        normalized_power
            .to_u16()
            .ok_or_else(|| TssError::CalculationError("Invalid normalized power calculation".to_string()))
    }

    /// Calculate heart rate intensity factor
    fn calculate_hr_intensity_factor(
        raw_data: &[DataPoint],
        lthr: u16,
    ) -> Result<Decimal, TssError> {
        let mut total_intensity_factor = Decimal::ZERO;
        let mut valid_points = 0;

        for point in raw_data {
            if let Some(hr) = point.heart_rate {
                // Basic intensity factor calculation: HR / LTHR
                let intensity_factor = Decimal::from(hr) / Decimal::from(lthr);
                total_intensity_factor += intensity_factor;
                valid_points += 1;
            }
        }

        if valid_points == 0 {
            return Err(TssError::InvalidData("No valid heart rate data points".to_string()));
        }

        Ok(total_intensity_factor / Decimal::from(valid_points))
    }

    /// Validate TSS result for sanity check
    pub fn validate_tss(tss: Decimal, duration_seconds: u32) -> Result<Decimal, TssError> {
        let duration_hours = Decimal::from(duration_seconds) / Decimal::from(3600);

        // Sanity checks
        if tss < Decimal::ZERO {
            return Err(TssError::CalculationError("TSS cannot be negative".to_string()));
        }

        // Maximum reasonable TSS per hour is about 300 (very high intensity)
        let max_reasonable_tss = duration_hours * Decimal::from(300);
        if tss > max_reasonable_tss {
            return Err(TssError::CalculationError(format!(
                "TSS {} seems unreasonably high for duration", tss
            )));
        }

        // Minimum reasonable TSS for active workouts (very easy pace = ~20 TSS/hour)
        let min_reasonable_tss = duration_hours * Decimal::from(10);
        if tss < min_reasonable_tss && duration_seconds > 600 {
            return Err(TssError::CalculationError(format!(
                "TSS {} seems unreasonably low for duration", tss
            )));
        }

        Ok(tss)
    }
}

/// Integration with WorkoutSummary
impl WorkoutSummary {
    /// Calculate and update TSS fields in the summary
    pub fn calculate_tss(&mut self, workout: &Workout, athlete: &AthleteProfile) -> Result<(), TssError> {
        let tss_result = TssCalculator::calculate_tss(workout, athlete)?;

        // Validate the calculated TSS
        let validated_tss = TssCalculator::validate_tss(tss_result.tss, workout.duration_seconds)?;

        // Update summary fields
        self.tss = Some(validated_tss);
        self.intensity_factor = tss_result.intensity_factor;
        self.normalized_power = tss_result.normalized_power;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, Utc};
    use rust_decimal_macros::dec;
    use crate::models::{DataSource, Sport, Units, WorkoutType, TrainingZones};

    fn create_test_athlete() -> AthleteProfile {
        AthleteProfile {
            id: "test_athlete".to_string(),
            name: "Test Athlete".to_string(),
            date_of_birth: Some(NaiveDate::from_ymd_opt(1990, 1, 1).unwrap()),
            weight: Some(dec!(70.0)),
            height: Some(175),
            ftp: Some(250),
            lthr: Some(165),
            threshold_pace: Some(dec!(6.0)), // 6 min/mile threshold pace
            max_hr: Some(190),
            resting_hr: Some(50),
            training_zones: TrainingZones::default(),
            preferred_units: Units::Metric,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_cycling_workout_with_power() -> Workout {
        let power_data = vec![
            DataPoint {
                timestamp: 0,
                heart_rate: Some(120),
                power: Some(200),
                pace: None,
                elevation: Some(100),
                cadence: Some(90),
                speed: Some(dec!(8.5)),
                distance: Some(dec!(0.0)),
                left_power: Some(100),
                right_power: Some(100),
            },
            DataPoint {
                timestamp: 30,
                heart_rate: Some(140),
                power: Some(250),
                pace: None,
                elevation: Some(105),
                cadence: Some(95),
                speed: Some(dec!(9.2)),
                distance: Some(dec!(250.0)),
                left_power: Some(125),
                right_power: Some(125),
            },
            DataPoint {
                timestamp: 60,
                heart_rate: Some(160),
                power: Some(300),
                pace: None,
                elevation: Some(110),
                cadence: Some(100),
                speed: Some(dec!(10.0)),
                distance: Some(dec!(550.0)),
                left_power: Some(150),
                right_power: Some(150),
            },
        ];

        Workout {
            id: "cycling_workout".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 9, 23).unwrap(),
            sport: Sport::Cycling,
            duration_seconds: 3600, // 1 hour
            workout_type: WorkoutType::Endurance,
            data_source: DataSource::Power,
            raw_data: Some(power_data),
            summary: WorkoutSummary::default(),
            notes: None,
            athlete_id: Some("test_athlete".to_string()),
            source: None,
        }
    }

    fn create_running_workout_with_pace() -> Workout {
        let pace_data = vec![
            DataPoint {
                timestamp: 0,
                heart_rate: Some(130),
                power: None,
                pace: Some(dec!(7.0)), // 7 min/mile
                elevation: Some(100),
                cadence: Some(85),
                speed: Some(dec!(3.8)),
                distance: Some(dec!(0.0)),
                left_power: None,
                right_power: None,
            },
            DataPoint {
                timestamp: 300,
                heart_rate: Some(145),
                power: None,
                pace: Some(dec!(6.5)), // 6.5 min/mile
                elevation: Some(120),
                cadence: Some(88),
                speed: Some(dec!(4.1)),
                distance: Some(dec!(1200.0)),
                left_power: None,
                right_power: None,
            },
            DataPoint {
                timestamp: 600,
                heart_rate: Some(155),
                power: None,
                pace: Some(dec!(6.0)), // 6 min/mile (threshold)
                elevation: Some(140),
                cadence: Some(90),
                speed: Some(dec!(4.5)),
                distance: Some(dec!(2500.0)),
                left_power: None,
                right_power: None,
            },
        ];

        Workout {
            id: "running_workout".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 9, 23).unwrap(),
            sport: Sport::Running,
            duration_seconds: 1800, // 30 minutes
            workout_type: WorkoutType::Tempo,
            data_source: DataSource::Pace,
            raw_data: Some(pace_data),
            summary: WorkoutSummary::default(),
            notes: None,
            athlete_id: Some("test_athlete".to_string()),
            source: None,
        }
    }

    #[test]
    fn test_power_based_tss_calculation() {
        let athlete = create_test_athlete();
        let workout = create_cycling_workout_with_power();

        let result = TssCalculator::calculate_power_tss(&workout, &athlete).unwrap();

        assert_eq!(result.method, TssMethod::PowerBased);
        assert!(result.tss > dec!(0));
        assert!(result.tss < dec!(300)); // Reasonable upper bound
        assert!(result.intensity_factor.is_some());
        assert!(result.normalized_power.is_some());
    }

    #[test]
    fn test_running_tss_with_elevation() {
        let athlete = create_test_athlete();
        let workout = create_running_workout_with_pace();

        let result = TssCalculator::calculate_pace_tss(&workout, &athlete).unwrap();

        assert_eq!(result.method, TssMethod::PaceBased);
        assert!(result.tss > dec!(0));
        assert!(result.intensity_factor.is_some());
        assert!(result.normalized_power.is_none());
    }

    #[test]
    fn test_heart_rate_tss_calculation() {
        let athlete = create_test_athlete();
        let mut workout = create_cycling_workout_with_power();

        // Remove power data to force heart rate calculation
        for point in workout.raw_data.as_mut().unwrap() {
            point.power = None;
        }
        workout.data_source = DataSource::HeartRate;

        let result = TssCalculator::calculate_heart_rate_tss(&workout, &athlete).unwrap();

        assert_eq!(result.method, TssMethod::HeartRateBased);
        assert!(result.tss > dec!(0));
        assert!(result.intensity_factor.is_some());
        assert!(result.normalized_power.is_none());
    }

    #[test]
    fn test_estimated_tss_fallback() {
        let athlete = create_test_athlete();
        let workout = Workout {
            id: "minimal_workout".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 9, 23).unwrap(),
            sport: Sport::Running,
            duration_seconds: 3600, // 1 hour
            workout_type: WorkoutType::Endurance,
            data_source: DataSource::Rpe,
            raw_data: None, // No raw data forces estimation
            summary: WorkoutSummary::default(),
            notes: None,
            athlete_id: Some("test_athlete".to_string()),
            source: None,
        };

        let result = TssCalculator::estimate_tss(&workout, &athlete).unwrap();

        assert_eq!(result.method, TssMethod::Estimated);
        assert_eq!(result.tss, dec!(70)); // 1 hour × 70 TSS/hour for running
        assert!(result.intensity_factor.is_none());
        assert!(result.normalized_power.is_none());
    }

    #[test]
    fn test_normalized_power_calculation() {
        let power_data = vec![
            DataPoint {
                timestamp: 0,
                power: Some(200),
                heart_rate: None,
                pace: None,
                elevation: None,
                cadence: None,
                speed: None,
                distance: None,
                left_power: Some(100),
                right_power: Some(100),
            },
            DataPoint {
                timestamp: 30,
                power: Some(250),
                heart_rate: None,
                pace: None,
                elevation: None,
                cadence: None,
                speed: None,
                distance: None,
                left_power: Some(125),
                right_power: Some(125),
            },
            DataPoint {
                timestamp: 60,
                power: Some(300),
                heart_rate: None,
                pace: None,
                elevation: None,
                cadence: None,
                speed: None,
                distance: None,
                left_power: Some(150),
                right_power: Some(150),
            },
        ];

        let np = TssCalculator::calculate_normalized_power(&power_data).unwrap();

        assert!(np > 200);
        assert!(np < 300);
        // Normalized power should be higher than average due to variability
        assert!(np > 250); // Average is 250, NP should be higher
    }

    #[test]
    fn test_tss_validation() {
        // Valid TSS
        assert!(TssCalculator::validate_tss(dec!(100), 3600).is_ok());

        // Negative TSS should fail
        assert!(TssCalculator::validate_tss(dec!(-10), 3600).is_err());

        // Unreasonably high TSS should fail
        assert!(TssCalculator::validate_tss(dec!(1000), 3600).is_err());

        // Very low TSS for long workout should fail
        assert!(TssCalculator::validate_tss(dec!(5), 7200).is_err());
    }

    #[test]
    fn test_workout_summary_integration() {
        let athlete = create_test_athlete();
        let workout = create_cycling_workout_with_power();
        let mut summary = WorkoutSummary::default();

        summary.calculate_tss(&workout, &athlete).unwrap();

        assert!(summary.tss.is_some());
        assert!(summary.tss.unwrap() > dec!(0));
        assert!(summary.intensity_factor.is_some());
        assert!(summary.normalized_power.is_some());
    }

    #[test]
    fn test_missing_threshold_errors() {
        let mut athlete = create_test_athlete();
        athlete.ftp = None; // Remove FTP

        let workout = create_cycling_workout_with_power();

        let result = TssCalculator::calculate_power_tss(&workout, &athlete);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TssError::MissingThreshold(_)));
    }

    #[test]
    fn test_swimming_tss_calculation() {
        let athlete = create_test_athlete();

        let pace_data = vec![
            DataPoint {
                timestamp: 0,
                heart_rate: Some(130),
                power: None,
                pace: Some(dec!(2.0)), // 2 min/100m
                elevation: None,
                cadence: Some(60),
                speed: Some(dec!(0.8)),
                distance: Some(dec!(0.0)),
                left_power: None,
                right_power: None,
            },
            DataPoint {
                timestamp: 120,
                heart_rate: Some(140),
                power: None,
                pace: Some(dec!(1.8)), // Faster: 1.8 min/100m
                elevation: None,
                cadence: Some(65),
                speed: Some(dec!(0.9)),
                distance: Some(dec!(100.0)),
                left_power: None,
                right_power: None,
            },
        ];

        let workout = Workout {
            id: "swimming_workout".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 9, 23).unwrap(),
            sport: Sport::Swimming,
            duration_seconds: 1800, // 30 minutes
            workout_type: WorkoutType::Endurance,
            data_source: DataSource::Pace,
            raw_data: Some(pace_data),
            summary: WorkoutSummary::default(),
            notes: None,
            athlete_id: Some("test_athlete".to_string()),
            source: None,
        };

        let result = TssCalculator::calculate_pace_tss(&workout, &athlete).unwrap();

        assert_eq!(result.method, TssMethod::PaceBased);
        assert!(result.tss > dec!(0));
        assert!(result.intensity_factor.is_some());
    }

    // Property-based tests using proptest
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_power_tss_properties(
            ftp in 150u16..350u16,
            avg_power in 120u16..400u16,
            duration in 1800u32..7200u32 // 30 minutes to 2 hours
        ) {
            let athlete = create_test_athlete_with_ftp(ftp);
            let workout = create_test_power_workout(avg_power, duration);

            let result = TssCalculator::calculate_power_tss(&workout, &athlete);

            // TSS should always be calculated successfully for valid inputs
            prop_assert!(result.is_ok());
            let tss_result = result.unwrap();

            // TSS should be positive
            prop_assert!(tss_result.tss > dec!(0));

            // TSS should be reasonable (typically 1-600 for normal workouts, allowing for intense sessions)
            prop_assert!(tss_result.tss <= dec!(600));

            // Intensity factor should be reasonable (0.3-2.0)
            if let Some(if_value) = tss_result.intensity_factor {
                prop_assert!(if_value >= dec!(0.3) && if_value <= dec!(2.0));
            }

            // Higher power should generally result in higher TSS for same duration
            if avg_power > ftp {
                prop_assert!(tss_result.tss >= dec!(100.0) * Decimal::from(duration) / dec!(3600));
            }
        }

        #[test]
        fn test_hr_tss_properties(
            lthr in 140u16..180u16,
            avg_hr in 120u16..200u16,
            duration in 1800u32..7200u32
        ) {
            let athlete = create_test_athlete_with_lthr(lthr);
            let workout = create_test_hr_workout(avg_hr, duration);

            let result = TssCalculator::calculate_heart_rate_tss(&workout, &athlete);

            prop_assert!(result.is_ok());
            let tss_result = result.unwrap();

            prop_assert!(tss_result.tss > dec!(0));
            prop_assert!(tss_result.tss <= dec!(500));

            if let Some(if_value) = tss_result.intensity_factor {
                prop_assert!(if_value >= dec!(0.5) && if_value <= dec!(1.5));
            }
        }

        #[test]
        fn test_pace_tss_properties(
            threshold_pace_seconds in 300u32..420u32, // 5-7 minutes per mile
            avg_pace_seconds in 300u32..500u32, // 5-8.33 minutes per mile
            duration in 1800u32..7200u32
        ) {
            let threshold_pace = Decimal::from(threshold_pace_seconds) / dec!(60); // Convert to minutes
            let avg_pace = Decimal::from(avg_pace_seconds) / dec!(60);

            let athlete = create_test_athlete_with_pace(threshold_pace);
            let workout = create_test_pace_workout(avg_pace, duration);

            let result = TssCalculator::calculate_pace_tss(&workout, &athlete);

            prop_assert!(result.is_ok());
            let tss_result = result.unwrap();

            prop_assert!(tss_result.tss > dec!(0));
            prop_assert!(tss_result.tss <= dec!(400));
        }

        #[test]
        fn test_tss_scales_with_duration(
            power in 200u16..300u16,
            duration1 in 1800u32..3600u32,
        ) {
            let duration2 = duration1 * 2; // Double the duration
            let athlete = create_test_athlete_with_ftp(250);

            let workout1 = create_test_power_workout(power, duration1);
            let workout2 = create_test_power_workout(power, duration2);

            let tss1 = TssCalculator::calculate_power_tss(&workout1, &athlete).unwrap().tss;
            let tss2 = TssCalculator::calculate_power_tss(&workout2, &athlete).unwrap().tss;

            // TSS should roughly double with double duration (within 20% tolerance)
            let ratio = tss2 / tss1;
            prop_assert!(ratio >= dec!(1.8) && ratio <= dec!(2.2));
        }

        #[test]
        fn test_normalized_power_properties(
            powers in prop::collection::vec(150u16..350u16, 30..300)
        ) {
            // Create DataPoint structures with power values
            let data_points: Vec<DataPoint> = powers.iter().enumerate().map(|(i, &p)| DataPoint {
                timestamp: i as u32,
                heart_rate: Some(150),
                power: Some(p),
                pace: None,
                elevation: None,
                cadence: Some(90),
                speed: None,
                distance: None,
                left_power: Some(p / 2),
                right_power: Some(p / 2),
            }).collect();

            let np = TssCalculator::calculate_normalized_power(&data_points).unwrap();

            prop_assert!(np > 0);

            let avg_power = powers.iter().map(|&p| p as u32).sum::<u32>() / powers.len() as u32;

            // Normalized power should be close to average power for steady efforts
            // but can be higher for variable efforts
            prop_assert!(np >= (avg_power as f32 * 0.8) as u16);
            prop_assert!(np <= (avg_power as f32 * 1.5) as u16);
        }
    }

    // Helper functions for property tests
    fn create_test_athlete_with_ftp(ftp: u16) -> AthleteProfile {
        AthleteProfile {
            id: "test_athlete".to_string(),
            name: "Test Athlete".to_string(),
            date_of_birth: Some(NaiveDate::from_ymd_opt(1990, 1, 1).unwrap()),
            weight: Some(dec!(70.0)),
            height: Some(175),
            ftp: Some(ftp),
            lthr: Some(165),
            threshold_pace: Some(dec!(6.0)),
            max_hr: Some(190),
            resting_hr: Some(50),
            training_zones: TrainingZones::default(),
            preferred_units: Units::Metric,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_athlete_with_lthr(lthr: u16) -> AthleteProfile {
        AthleteProfile {
            id: "test_athlete".to_string(),
            name: "Test Athlete".to_string(),
            date_of_birth: Some(NaiveDate::from_ymd_opt(1990, 1, 1).unwrap()),
            weight: Some(dec!(70.0)),
            height: Some(175),
            ftp: Some(250),
            lthr: Some(lthr),
            threshold_pace: Some(dec!(6.0)),
            max_hr: Some(190),
            resting_hr: Some(50),
            training_zones: TrainingZones::default(),
            preferred_units: Units::Metric,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_athlete_with_pace(threshold_pace: Decimal) -> AthleteProfile {
        AthleteProfile {
            id: "test_athlete".to_string(),
            name: "Test Athlete".to_string(),
            date_of_birth: Some(NaiveDate::from_ymd_opt(1990, 1, 1).unwrap()),
            weight: Some(dec!(70.0)),
            height: Some(175),
            ftp: Some(250),
            lthr: Some(165),
            threshold_pace: Some(threshold_pace),
            max_hr: Some(190),
            resting_hr: Some(50),
            training_zones: TrainingZones::default(),
            preferred_units: Units::Metric,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_power_workout(avg_power: u16, duration: u32) -> Workout {
        // Create realistic power data across the workout duration
        let data_points: Vec<DataPoint> = (0..duration)
            .step_by(1) // 1-second intervals
            .map(|timestamp| {
                // Add some realistic power variation (+/- 10%)
                let power_variation = ((timestamp as f64 * 0.1).sin() * 0.1 + 1.0) * avg_power as f64;
                let power = power_variation as u16;

                DataPoint {
                    timestamp,
                    heart_rate: Some(150),
                    power: Some(power),
                    pace: None,
                    elevation: None,
                    cadence: Some(90),
                    speed: None,
                    distance: None,
                    left_power: Some(power / 2),
                    right_power: Some(power / 2),
                }
            })
            .collect();

        Workout {
            id: "test_power".to_string(),
            athlete_id: Some("test".to_string()),
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            sport: Sport::Cycling,
            workout_type: WorkoutType::Endurance,
            duration_seconds: duration,
            summary: WorkoutSummary {
                avg_power: Some(avg_power),
                normalized_power: None, // Let the calculation determine this
                ..Default::default()
            },
            data_source: DataSource::Power,
            notes: None,
            source: None,
            raw_data: Some(data_points),
        }
    }

    fn create_test_hr_workout(avg_hr: u16, duration: u32) -> Workout {
        Workout {
            id: "test_hr".to_string(),
            athlete_id: Some("test".to_string()),
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            sport: Sport::Cycling,
            workout_type: WorkoutType::Endurance,
            duration_seconds: duration,
            summary: WorkoutSummary {
                avg_heart_rate: Some(avg_hr),
                max_heart_rate: Some(avg_hr + 20),
                ..Default::default()
            },
            data_source: DataSource::HeartRate,
            notes: None,
            source: None,
            raw_data: Some(vec![DataPoint {
                timestamp: 0,
                heart_rate: Some(avg_hr),
                power: None,
                pace: None,
                elevation: None,
                cadence: Some(90),
                speed: None,
                distance: None,
                left_power: None,
                right_power: None,
            }]),
        }
    }

    fn create_test_pace_workout(avg_pace: Decimal, duration: u32) -> Workout {
        Workout {
            id: "test_pace".to_string(),
            athlete_id: Some("test".to_string()),
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            sport: Sport::Running,
            workout_type: WorkoutType::Endurance,
            duration_seconds: duration,
            summary: WorkoutSummary {
                avg_heart_rate: Some(150),
                avg_pace: Some(avg_pace),
                total_distance: Some(dec!(10.0)),
                ..Default::default()
            },
            data_source: DataSource::Pace,
            notes: None,
            source: None,
            raw_data: Some(vec![DataPoint {
                timestamp: 0,
                heart_rate: Some(150),
                power: None,
                pace: Some(avg_pace),
                elevation: None,
                cadence: Some(90),
                speed: Some(dec!(5.0)),
                distance: Some(dec!(0.0)),
                left_power: None,
                right_power: None,
            }]),
        }
    }

    // Regression tests with known TSS values
    #[test]
    fn test_known_tss_values() {
        // Simple known TSS test cases for regression testing
        let test_cases = vec![
            (create_test_power_workout(150, 3600), dec!(36.0)), // Easy endurance: 1hr at 0.6 IF = 36 TSS
            (create_test_power_workout(250, 2400), dec!(66.7)), // Threshold: 40min at 1.0 IF = 66.7 TSS
        ];
        let athlete = create_test_athlete_with_ftp(250);

        for (workout, expected_tss) in test_cases {
            let result = TssCalculator::calculate_tss(&workout, &athlete);

            assert!(result.is_ok(), "TSS calculation failed for workout: {:?}", workout.sport);
            let calculated_tss = result.unwrap().tss;

            // Allow 10% tolerance for TSS calculations
            let tolerance = expected_tss * dec!(0.1);
            let diff = (calculated_tss - expected_tss).abs();

            assert!(
                diff <= tolerance,
                "TSS mismatch for {:?}: expected {}, got {}, diff: {}",
                workout.sport, expected_tss, calculated_tss, diff
            );
        }
    }

    // Edge case tests
    #[test]
    fn test_edge_cases() {
        // Create simple edge case workouts
        let edge_cases = vec![
            ("zero_duration", create_test_power_workout(200, 0)),
            ("very_short_workout", create_test_power_workout(200, 30)),
            ("extreme_high_power", create_test_power_workout(1000, 3600)),
            ("extreme_low_power", create_test_power_workout(50, 3600)),
        ];
        let athlete = create_test_athlete_with_ftp(250);

        for (case_name, workout) in edge_cases {
            let result = TssCalculator::calculate_tss(&workout, &athlete);

            match case_name {
                "zero_duration" => {
                    // Zero duration should either fail gracefully or return zero TSS
                    if let Ok(tss_result) = result {
                        assert_eq!(tss_result.tss, dec!(0));
                    }
                }
                "very_short_workout" => {
                    // Very short workouts should still calculate
                    assert!(result.is_ok());
                    if let Ok(tss_result) = result {
                        assert!(tss_result.tss >= dec!(0));
                        assert!(tss_result.tss <= dec!(5)); // Should be very low
                    }
                }
                "very_long_workout" => {
                    // Very long workouts should still work
                    assert!(result.is_ok());
                    if let Ok(tss_result) = result {
                        assert!(tss_result.tss > dec!(100)); // Should be substantial
                    }
                }
                "extreme_high_power" => {
                    // Extreme values should either work or fail gracefully
                    if let Ok(tss_result) = result {
                        assert!(tss_result.tss > dec!(200)); // Should be very high
                        assert!(tss_result.tss <= dec!(2000)); // But not unreasonable
                    }
                }
                _ => {
                    // Other cases should generally work
                    if result.is_err() {
                        println!("Expected failure for case: {}", case_name);
                    }
                }
            }
        }
    }
}