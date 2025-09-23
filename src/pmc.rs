use crate::models::Workout;
use chrono::NaiveDate;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

/// PMC calculation errors
#[derive(Error, Debug)]
pub enum PmcError {
    #[error("Insufficient data: {0}")]
    InsufficientData(String),
    #[error("Invalid date range: {0}")]
    InvalidDateRange(String),
    #[error("Calculation error: {0}")]
    CalculationError(String),
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

/// Daily TSS record with optional workout data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DailyTss {
    /// Date of the training day
    pub date: NaiveDate,

    /// Total TSS for the day (sum of all workouts)
    pub total_tss: Decimal,

    /// Number of workouts completed on this day
    pub workout_count: u16,

    /// True if there were actual workouts, false if this is a rest day
    pub has_workouts: bool,

    /// Individual workout TSS values for detailed analysis
    pub workout_tss_values: Vec<Decimal>,
}

/// Performance Management Chart (PMC) metrics for a specific date
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PmcMetrics {
    /// Date these metrics are calculated for
    pub date: NaiveDate,

    /// Chronic Training Load (42-day exponentially weighted average)
    pub ctl: Decimal,

    /// Acute Training Load (7-day exponentially weighted average)
    pub atl: Decimal,

    /// Training Stress Balance (CTL - ATL)
    pub tsb: Decimal,

    /// Daily TSS value used in calculations
    pub daily_tss: Decimal,

    /// Ramp rate (CTL change per week)
    pub ctl_ramp_rate: Option<Decimal>,

    /// ATL spike indicator (unusually high recent load)
    pub atl_spike: bool,
}

/// PMC configuration with customizable time constants
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PmcConfig {
    /// CTL time constant in days (default: 42)
    pub ctl_time_constant: u16,

    /// ATL time constant in days (default: 7)
    pub atl_time_constant: u16,

    /// Minimum days required for reliable PMC calculations
    pub min_data_days: u16,

    /// ATL spike threshold (percentage above recent average)
    pub atl_spike_threshold: Decimal,

    /// Ramp rate calculation period in days
    pub ramp_rate_days: u16,
}

impl Default for PmcConfig {
    fn default() -> Self {
        PmcConfig {
            ctl_time_constant: 42,
            atl_time_constant: 7,
            min_data_days: 14,
            atl_spike_threshold: Decimal::from_f32(1.5).unwrap(), // 50% above average
            ramp_rate_days: 7,
        }
    }
}

/// Training Stress Balance interpretation ranges
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TsbInterpretation {
    VeryFresh,    // +25 and above
    Fresh,        // +5 to +25
    Neutral,      // -10 to +5
    Fatigued,     // -30 to -10
    VeryFatigued, // Below -30
}

impl TsbInterpretation {
    /// Get TSB interpretation from numeric value
    pub fn from_tsb(tsb: Decimal) -> Self {
        if tsb >= Decimal::from(25) {
            TsbInterpretation::VeryFresh
        } else if tsb >= Decimal::from(5) {
            TsbInterpretation::Fresh
        } else if tsb >= Decimal::from(-10) {
            TsbInterpretation::Neutral
        } else if tsb >= Decimal::from(-30) {
            TsbInterpretation::Fatigued
        } else {
            TsbInterpretation::VeryFatigued
        }
    }

    /// Get interpretation description
    pub fn description(&self) -> &'static str {
        match self {
            TsbInterpretation::VeryFresh => "Very fresh (may be losing fitness)",
            TsbInterpretation::Fresh => "Fresh and ready for hard training/racing",
            TsbInterpretation::Neutral => "Neutral (normal training)",
            TsbInterpretation::Fatigued => "Fatigued (monitor closely)",
            TsbInterpretation::VeryFatigued => "Very fatigued (rest needed)",
        }
    }

    /// Get training recommendation
    pub fn recommendation(&self) -> &'static str {
        match self {
            TsbInterpretation::VeryFresh => {
                "Consider increasing training load or plan peak performance"
            }
            TsbInterpretation::Fresh => "Good time for high-intensity sessions or racing",
            TsbInterpretation::Neutral => "Continue normal training progression",
            TsbInterpretation::Fatigued => "Reduce intensity, focus on recovery sessions",
            TsbInterpretation::VeryFatigued => {
                "Prioritize rest and recovery before resuming training"
            }
        }
    }
}

/// PMC trend analysis results
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PmcTrends {
    /// CTL trend over the analysis period
    pub ctl_trend: TrendDirection,

    /// ATL trend over the analysis period
    pub atl_trend: TrendDirection,

    /// TSB trend over the analysis period
    pub tsb_trend: TrendDirection,

    /// Average CTL ramp rate (TSS/week)
    pub avg_ctl_ramp_rate: Decimal,

    /// Number of ATL spikes detected
    pub atl_spike_count: u16,

    /// Days since last ATL spike
    pub days_since_last_spike: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    Increasing,
    Stable,
    Decreasing,
}

/// Core PMC calculation engine
pub struct PmcCalculator {
    config: PmcConfig,
}

impl PmcCalculator {
    /// Create new PMC calculator with default configuration
    pub fn new() -> Self {
        PmcCalculator {
            config: PmcConfig::default(),
        }
    }

    /// Create new PMC calculator with custom configuration
    pub fn with_config(config: PmcConfig) -> Self {
        PmcCalculator { config }
    }

    /// Aggregate daily TSS from a collection of workouts
    pub fn aggregate_daily_tss(&self, workouts: &[Workout]) -> BTreeMap<NaiveDate, DailyTss> {
        let mut daily_tss: BTreeMap<NaiveDate, DailyTss> = BTreeMap::new();

        for workout in workouts {
            let tss = workout.summary.tss.unwrap_or(Decimal::ZERO);

            daily_tss
                .entry(workout.date)
                .and_modify(|day| {
                    day.total_tss += tss;
                    day.workout_count += 1;
                    day.workout_tss_values.push(tss);
                    day.has_workouts = true;
                })
                .or_insert(DailyTss {
                    date: workout.date,
                    total_tss: tss,
                    workout_count: 1,
                    has_workouts: true,
                    workout_tss_values: vec![tss],
                });
        }

        daily_tss
    }

    /// Calculate PMC metrics for a date range
    pub fn calculate_pmc_series(
        &self,
        daily_tss: &BTreeMap<NaiveDate, DailyTss>,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<PmcMetrics>, PmcError> {
        if start_date > end_date {
            return Err(PmcError::InvalidDateRange(
                "Start date must be before end date".to_string(),
            ));
        }

        let mut pmc_series = Vec::new();
        let mut current_ctl = Decimal::ZERO;
        let mut current_atl = Decimal::ZERO;

        // Calculate the date range ensuring we have enough data
        let calculation_start = start_date
            .checked_sub_days(chrono::Days::new(self.config.ctl_time_constant as u64))
            .unwrap_or(start_date);

        let mut current_date = calculation_start;
        let mut ctl_history = Vec::new();

        while current_date <= end_date {
            let daily_tss_value = daily_tss
                .get(&current_date)
                .map(|d| d.total_tss)
                .unwrap_or(Decimal::ZERO);

            // Calculate CTL: CTL_today = CTL_yesterday + (TSS_today - CTL_yesterday) × (1/time_constant)
            let ctl_factor = Decimal::ONE / Decimal::from(self.config.ctl_time_constant);
            current_ctl = current_ctl + (daily_tss_value - current_ctl) * ctl_factor;

            // Calculate ATL: ATL_today = ATL_yesterday + (TSS_today - ATL_yesterday) × (1/time_constant)
            let atl_factor = Decimal::ONE / Decimal::from(self.config.atl_time_constant);
            current_atl = current_atl + (daily_tss_value - current_atl) * atl_factor;

            // Calculate TSB: TSB = CTL_yesterday - ATL_today
            // Note: Using CTL from previous day as is standard in PMC calculations
            let previous_ctl = if ctl_history.is_empty() {
                current_ctl
            } else {
                ctl_history[ctl_history.len() - 1]
            };
            let tsb = previous_ctl - current_atl;

            ctl_history.push(current_ctl);

            // Only include metrics for the requested date range
            if current_date >= start_date {
                let ramp_rate =
                    self.calculate_ctl_ramp_rate(&ctl_history, self.config.ramp_rate_days);
                let atl_spike = self.detect_atl_spike(current_atl, &pmc_series);

                pmc_series.push(PmcMetrics {
                    date: current_date,
                    ctl: current_ctl,
                    atl: current_atl,
                    tsb,
                    daily_tss: daily_tss_value,
                    ctl_ramp_rate: ramp_rate,
                    atl_spike,
                });
            }

            current_date = current_date.succ_opt().unwrap();
        }

        Ok(pmc_series)
    }

    /// Calculate CTL ramp rate (change per week)
    fn calculate_ctl_ramp_rate(&self, ctl_history: &[Decimal], days: u16) -> Option<Decimal> {
        if ctl_history.len() < days as usize {
            return None;
        }

        let recent_ctl = ctl_history[ctl_history.len() - 1];
        let past_ctl = ctl_history[ctl_history.len() - days as usize];

        // Calculate ramp rate as CTL change per week
        let change = recent_ctl - past_ctl;
        let weeks = Decimal::from(days) / Decimal::from(7);
        Some(change / weeks)
    }

    /// Detect ATL spike (unusually high recent load)
    fn detect_atl_spike(&self, current_atl: Decimal, pmc_history: &[PmcMetrics]) -> bool {
        if pmc_history.len() < 7 {
            return false;
        }

        // Calculate average ATL over the past week
        let recent_atl_avg: Decimal = pmc_history
            .iter()
            .rev()
            .take(7)
            .map(|m| m.atl)
            .sum::<Decimal>()
            / Decimal::from(7);

        current_atl > recent_atl_avg * self.config.atl_spike_threshold
    }

    /// Analyze PMC trends over a period
    pub fn analyze_trends(&self, pmc_series: &[PmcMetrics]) -> Result<PmcTrends, PmcError> {
        if pmc_series.len() < self.config.min_data_days as usize {
            return Err(PmcError::InsufficientData(format!(
                "Need at least {} days of data for trend analysis",
                self.config.min_data_days
            )));
        }

        let first = &pmc_series[0];
        let last = &pmc_series[pmc_series.len() - 1];

        // Determine trends
        let ctl_trend = Self::determine_trend(first.ctl, last.ctl);
        let atl_trend = Self::determine_trend(first.atl, last.atl);
        let tsb_trend = Self::determine_trend(first.tsb, last.tsb);

        // Calculate average CTL ramp rate
        let ramp_rates: Vec<Decimal> = pmc_series.iter().filter_map(|m| m.ctl_ramp_rate).collect();

        let avg_ctl_ramp_rate = if ramp_rates.is_empty() {
            Decimal::ZERO
        } else {
            ramp_rates.iter().sum::<Decimal>() / Decimal::from(ramp_rates.len())
        };

        // Count ATL spikes
        let atl_spike_count = pmc_series.iter().filter(|m| m.atl_spike).count() as u16;

        // Find days since last spike
        let days_since_last_spike = pmc_series
            .iter()
            .rev()
            .position(|m| m.atl_spike)
            .map(|pos| pos as u16);

        Ok(PmcTrends {
            ctl_trend,
            atl_trend,
            tsb_trend,
            avg_ctl_ramp_rate,
            atl_spike_count,
            days_since_last_spike,
        })
    }

    /// Determine trend direction between two values
    fn determine_trend(start: Decimal, end: Decimal) -> TrendDirection {
        let change_threshold = Decimal::from_f32(0.05).unwrap(); // 5% threshold
        let percent_change = (end - start) / start.abs().max(Decimal::from(1));

        if percent_change > change_threshold {
            TrendDirection::Increasing
        } else if percent_change < -change_threshold {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        }
    }

    /// Get latest PMC metrics for an athlete
    pub fn get_latest_metrics(
        &self,
        workouts: &[Workout],
        as_of_date: Option<NaiveDate>,
    ) -> Result<PmcMetrics, PmcError> {
        let end_date = as_of_date.unwrap_or_else(|| chrono::Utc::now().date_naive());
        let start_date = end_date
            .checked_sub_days(chrono::Days::new(self.config.ctl_time_constant as u64 * 2))
            .unwrap_or(end_date);

        let daily_tss = self.aggregate_daily_tss(workouts);
        let pmc_series = self.calculate_pmc_series(&daily_tss, start_date, end_date)?;

        pmc_series
            .into_iter()
            .last()
            .ok_or_else(|| PmcError::InsufficientData("No PMC data available".to_string()))
    }

    /// Generate training recommendations based on PMC metrics
    pub fn generate_recommendations(&self, metrics: &PmcMetrics) -> Vec<String> {
        let mut recommendations = Vec::new();

        let tsb_interpretation = TsbInterpretation::from_tsb(metrics.tsb);
        recommendations.push(tsb_interpretation.recommendation().to_string());

        // CTL ramp rate recommendations
        if let Some(ramp_rate) = metrics.ctl_ramp_rate {
            if ramp_rate > Decimal::from(8) {
                recommendations
                    .push("CTL ramp rate is aggressive - monitor for overreaching".to_string());
            } else if ramp_rate < Decimal::from(-5) {
                recommendations.push(
                    "CTL is declining rapidly - consider increasing training load".to_string(),
                );
            }
        }

        // ATL spike warnings
        if metrics.atl_spike {
            recommendations.push("ATL spike detected - plan recovery in coming days".to_string());
        }

        // TSB-specific recommendations
        match tsb_interpretation {
            TsbInterpretation::VeryFresh => {
                recommendations.push("Consider a training block or planned event".to_string());
            }
            TsbInterpretation::Fresh => {
                recommendations.push("Good opportunity for high-quality training".to_string());
            }
            TsbInterpretation::VeryFatigued => {
                recommendations
                    .push("Prioritize sleep, nutrition, and active recovery".to_string());
            }
            _ => {}
        }

        recommendations
    }
}

impl Default for PmcCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DataSource, Sport, WorkoutSummary, WorkoutType};
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn create_test_workout(date: NaiveDate, tss: Decimal) -> Workout {
        Workout {
            id: format!("workout_{}", date.format("%Y%m%d")),
            date,
            sport: Sport::Cycling,
            duration_seconds: 3600,
            workout_type: WorkoutType::Endurance,
            data_source: DataSource::Power,
            raw_data: None,
            summary: WorkoutSummary {
                tss: Some(tss),
                ..WorkoutSummary::default()
            },
            notes: None,
            athlete_id: Some("test_athlete".to_string()),
            source: None,
        }
    }

    #[test]
    fn test_daily_tss_aggregation() {
        let calculator = PmcCalculator::new();
        let date = NaiveDate::from_ymd_opt(2024, 9, 23).unwrap();

        let workouts = vec![
            create_test_workout(date, dec!(50)),
            create_test_workout(date, dec!(30)),
        ];

        let daily_tss = calculator.aggregate_daily_tss(&workouts);

        assert_eq!(daily_tss.len(), 1);
        let day = daily_tss.get(&date).unwrap();
        assert_eq!(day.total_tss, dec!(80));
        assert_eq!(day.workout_count, 2);
        assert!(day.has_workouts);
        assert_eq!(day.workout_tss_values, vec![dec!(50), dec!(30)]);
    }

    #[test]
    fn test_ctl_calculation() {
        let calculator = PmcCalculator::new();
        let start_date = NaiveDate::from_ymd_opt(2024, 9, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2024, 9, 7).unwrap();

        // Create week of 100 TSS per day
        let mut workouts = Vec::new();
        let mut current_date = start_date;
        while current_date <= end_date {
            workouts.push(create_test_workout(current_date, dec!(100)));
            current_date = current_date.succ_opt().unwrap();
        }

        let daily_tss = calculator.aggregate_daily_tss(&workouts);
        let pmc_series = calculator
            .calculate_pmc_series(&daily_tss, start_date, end_date)
            .unwrap();

        // CTL should increase each day with consistent training
        assert!(pmc_series.len() > 0);
        assert!(pmc_series[pmc_series.len() - 1].ctl > pmc_series[0].ctl);
    }

    #[test]
    fn test_atl_calculation() {
        let calculator = PmcCalculator::new();
        let start_date = NaiveDate::from_ymd_opt(2024, 9, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2024, 9, 7).unwrap();

        let mut workouts = Vec::new();
        let mut current_date = start_date;
        while current_date <= end_date {
            // Higher TSS on last day to test ATL response
            let tss = if current_date == end_date {
                dec!(200)
            } else {
                dec!(50)
            };
            workouts.push(create_test_workout(current_date, tss));
            current_date = current_date.succ_opt().unwrap();
        }

        let daily_tss = calculator.aggregate_daily_tss(&workouts);
        let pmc_series = calculator
            .calculate_pmc_series(&daily_tss, start_date, end_date)
            .unwrap();

        // ATL should respond quickly to the high TSS day
        let last_metrics = &pmc_series[pmc_series.len() - 1];
        let second_last_metrics = &pmc_series[pmc_series.len() - 2];
        assert!(last_metrics.atl > second_last_metrics.atl);
    }

    #[test]
    fn test_tsb_calculation() {
        let calculator = PmcCalculator::new();
        let date = NaiveDate::from_ymd_opt(2024, 9, 23).unwrap();

        let workouts = vec![create_test_workout(date, dec!(100))];
        let daily_tss = calculator.aggregate_daily_tss(&workouts);
        let pmc_series = calculator
            .calculate_pmc_series(&daily_tss, date, date)
            .unwrap();

        let metrics = &pmc_series[0];
        // TSB = CTL_yesterday - ATL_today
        // For single day, this should be negative (fatigue > fitness)
        assert!(metrics.tsb <= Decimal::ZERO);
    }

    #[test]
    fn test_tsb_interpretation() {
        assert_eq!(
            TsbInterpretation::from_tsb(dec!(30)),
            TsbInterpretation::VeryFresh
        );
        assert_eq!(
            TsbInterpretation::from_tsb(dec!(10)),
            TsbInterpretation::Fresh
        );
        assert_eq!(
            TsbInterpretation::from_tsb(dec!(0)),
            TsbInterpretation::Neutral
        );
        assert_eq!(
            TsbInterpretation::from_tsb(dec!(-20)),
            TsbInterpretation::Fatigued
        );
        assert_eq!(
            TsbInterpretation::from_tsb(dec!(-40)),
            TsbInterpretation::VeryFatigued
        );
    }

    #[test]
    fn test_atl_spike_detection() {
        let calculator = PmcCalculator::new();
        let start_date = NaiveDate::from_ymd_opt(2024, 9, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2024, 9, 14).unwrap();

        let mut workouts = Vec::new();
        let mut current_date = start_date;
        while current_date <= end_date {
            // Create spike on day 10
            let days_from_start = (current_date - start_date).num_days();
            let tss = if days_from_start == 10 {
                dec!(300)
            } else {
                dec!(50)
            };
            workouts.push(create_test_workout(current_date, tss));
            current_date = current_date.succ_opt().unwrap();
        }

        let daily_tss = calculator.aggregate_daily_tss(&workouts);
        let pmc_series = calculator
            .calculate_pmc_series(&daily_tss, start_date, end_date)
            .unwrap();

        // Should detect spike on high TSS day
        let spike_day = &pmc_series[10];
        assert!(spike_day.atl_spike);
    }

    #[test]
    fn test_ctl_ramp_rate() {
        let calculator = PmcCalculator::new();
        let start_date = NaiveDate::from_ymd_opt(2024, 9, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2024, 9, 21).unwrap();

        // Progressive overload: increasing TSS each week
        let mut workouts = Vec::new();
        let mut current_date = start_date;
        while current_date <= end_date {
            let week = (current_date - start_date).num_days() / 7;
            let base_tss = dec!(50) + Decimal::from(week * 20);
            workouts.push(create_test_workout(current_date, base_tss));
            current_date = current_date.succ_opt().unwrap();
        }

        let daily_tss = calculator.aggregate_daily_tss(&workouts);
        let pmc_series = calculator
            .calculate_pmc_series(&daily_tss, start_date, end_date)
            .unwrap();

        // Should show positive ramp rate with progressive overload
        let final_metrics = &pmc_series[pmc_series.len() - 1];
        assert!(final_metrics.ctl_ramp_rate.unwrap_or(Decimal::ZERO) > Decimal::ZERO);
    }

    #[test]
    fn test_trend_analysis() {
        let calculator = PmcCalculator::new();
        let start_date = NaiveDate::from_ymd_opt(2024, 9, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2024, 9, 21).unwrap();

        // Create progressive training load
        let mut workouts = Vec::new();
        let mut current_date = start_date;
        while current_date <= end_date {
            let days = (current_date - start_date).num_days();
            let tss = dec!(30) + Decimal::from(days) * dec!(2); // Gradual increase
            workouts.push(create_test_workout(current_date, tss));
            current_date = current_date.succ_opt().unwrap();
        }

        let daily_tss = calculator.aggregate_daily_tss(&workouts);
        let pmc_series = calculator
            .calculate_pmc_series(&daily_tss, start_date, end_date)
            .unwrap();
        let trends = calculator.analyze_trends(&pmc_series).unwrap();

        assert_eq!(trends.ctl_trend, TrendDirection::Increasing);
        assert!(trends.avg_ctl_ramp_rate > Decimal::ZERO);
    }

    #[test]
    fn test_training_recommendations() {
        let calculator = PmcCalculator::new();

        // Fresh athlete (positive TSB)
        let fresh_metrics = PmcMetrics {
            date: NaiveDate::from_ymd_opt(2024, 9, 23).unwrap(),
            ctl: dec!(50),
            atl: dec!(30),
            tsb: dec!(15), // Fresh
            daily_tss: dec!(60),
            ctl_ramp_rate: Some(dec!(5)),
            atl_spike: false,
        };

        let recommendations = calculator.generate_recommendations(&fresh_metrics);
        assert!(!recommendations.is_empty());

        // Fatigued athlete (negative TSB)
        let fatigued_metrics = PmcMetrics {
            date: NaiveDate::from_ymd_opt(2024, 9, 23).unwrap(),
            ctl: dec!(40),
            atl: dec!(60),
            tsb: dec!(-25), // Fatigued
            daily_tss: dec!(80),
            ctl_ramp_rate: Some(dec!(2)),
            atl_spike: true,
        };

        let fatigued_recommendations = calculator.generate_recommendations(&fatigued_metrics);
        assert!(fatigued_recommendations.len() >= 2); // Should have multiple recommendations for fatigue
    }

    #[test]
    fn test_custom_config() {
        let custom_config = PmcConfig {
            ctl_time_constant: 28, // Shorter CTL period
            atl_time_constant: 5,  // Shorter ATL period
            ..PmcConfig::default()
        };

        let calculator = PmcCalculator::with_config(custom_config);
        let date = NaiveDate::from_ymd_opt(2024, 9, 23).unwrap();

        let workouts = vec![create_test_workout(date, dec!(100))];
        let daily_tss = calculator.aggregate_daily_tss(&workouts);
        let pmc_series = calculator
            .calculate_pmc_series(&daily_tss, date, date)
            .unwrap();

        // With shorter time constants, metrics should respond more quickly
        assert!(!pmc_series.is_empty());
        let metrics = &pmc_series[0];
        assert!(metrics.ctl > Decimal::ZERO);
        assert!(metrics.atl > Decimal::ZERO);
    }

    #[test]
    fn test_missing_tss_handling() {
        let calculator = PmcCalculator::new();
        let date = NaiveDate::from_ymd_opt(2024, 9, 23).unwrap();

        // Create workout without TSS
        let workout = Workout {
            id: "test_workout".to_string(),
            date,
            sport: Sport::Running,
            duration_seconds: 3600,
            workout_type: WorkoutType::Endurance,
            data_source: DataSource::HeartRate,
            raw_data: None,
            summary: WorkoutSummary {
                tss: None, // Missing TSS
                ..WorkoutSummary::default()
            },
            notes: None,
            athlete_id: Some("test_athlete".to_string()),
            source: None,
        };

        let daily_tss = calculator.aggregate_daily_tss(&[workout]);

        // Should handle missing TSS gracefully with zero
        let day = daily_tss.get(&date).unwrap();
        assert_eq!(day.total_tss, Decimal::ZERO);
        assert!(day.has_workouts);
    }
}
