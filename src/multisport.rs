#![allow(dead_code)]

use anyhow::Result;
use chrono::{Duration, NaiveDate};
use clap::Subcommand;
use rust_decimal::{Decimal, prelude::ToPrimitive};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::{Sport, Workout, AthleteProfile};
use crate::tss::TssCalculator;
use crate::pmc::{PmcCalculator, PmcMetrics};

#[derive(Debug, Clone, Subcommand)]
pub enum MultiSportCommands {
    /// Display combined training load across all sports
    Load {
        /// Start date (YYYY-MM-DD)
        #[arg(long)]
        from: Option<NaiveDate>,
        /// End date (YYYY-MM-DD)
        #[arg(long)]
        to: Option<NaiveDate>,
        /// Show sport-specific breakdown
        #[arg(long)]
        breakdown: bool,
    },
    /// Analyze training distribution by sport
    Distribution {
        /// Time period in days (default: 28)
        #[arg(long, default_value_t = 28)]
        period: u32,
        /// Show weekly breakdown
        #[arg(long)]
        weekly: bool,
    },
    /// Calculate sport equivalency and conversion factors
    Equivalency {
        /// Source sport for conversion
        #[arg(long)]
        from_sport: String,
        /// Target sport for conversion
        #[arg(long)]
        to_sport: String,
        /// TSS value to convert
        #[arg(long)]
        tss: f64,
    },
    /// Triathlon-specific analysis
    Triathlon {
        /// Show Critical Swim Speed analysis
        #[arg(long)]
        css: bool,
        /// Show brick workout analysis
        #[arg(long)]
        brick: bool,
        /// Show transition training summary
        #[arg(long)]
        transitions: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiSportLoad {
    pub date: NaiveDate,
    pub total_tss: Decimal,
    pub sport_breakdown: HashMap<Sport, Decimal>,
    pub duration_breakdown: HashMap<Sport, u32>, // duration in seconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SportDistribution {
    pub period_days: u32,
    pub total_time: u32, // total training time in seconds
    pub total_tss: Decimal,
    pub sport_time_distribution: HashMap<Sport, f64>, // percentage
    pub sport_tss_distribution: HashMap<Sport, f64>, // percentage
    pub weekly_breakdown: Option<Vec<WeeklyDistribution>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyDistribution {
    pub week_start: NaiveDate,
    pub sport_time: HashMap<Sport, u32>,
    pub sport_tss: HashMap<Sport, Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SportEquivalency {
    pub from_sport: Sport,
    pub to_sport: Sport,
    pub conversion_factor: Decimal,
    pub original_tss: Decimal,
    pub equivalent_tss: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriathlonMetrics {
    pub css_metrics: Option<CssMetrics>,
    pub brick_analysis: Option<BrickAnalysis>,
    pub transition_summary: Option<TransitionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CssMetrics {
    pub css_pace: Decimal, // Critical Swim Speed in min/100m
    pub css_threshold_power: Option<Decimal>,
    pub recent_performances: Vec<SwimPerformance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwimPerformance {
    pub date: NaiveDate,
    pub distance: Decimal, // distance in meters
    pub time: u32, // time in seconds
    pub pace: Decimal, // pace in min/100m
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrickAnalysis {
    pub total_brick_sessions: u32,
    pub avg_bike_duration: u32,
    pub avg_run_duration: u32,
    pub transition_efficiency: Decimal, // percentage improvement over standalone runs
    pub recent_sessions: Vec<BrickSession>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrickSession {
    pub date: NaiveDate,
    pub bike_duration: u32,
    pub bike_tss: Decimal,
    pub run_duration: u32,
    pub run_tss: Decimal,
    pub transition_time: Option<u32>, // transition time in seconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionSummary {
    pub t1_sessions: u32, // swim-to-bike transitions
    pub t2_sessions: u32, // bike-to-run transitions
    pub avg_t1_time: Option<u32>,
    pub avg_t2_time: Option<u32>,
    pub improvement_trend: Decimal, // percentage improvement over time
}

/// Calculate combined training load across all sports
pub fn calculate_combined_load(
    workouts: &[Workout],
    athlete: &AthleteProfile,
    from_date: Option<NaiveDate>,
    to_date: Option<NaiveDate>,
) -> Result<Vec<MultiSportLoad>> {
    let filtered_workouts: Vec<&Workout> = workouts
        .iter()
        .filter(|workout| {
            if let Some(from) = from_date {
                if workout.date < from {
                    return false;
                }
            }
            if let Some(to) = to_date {
                if workout.date > to {
                    return false;
                }
            }
            true
        })
        .collect();

    // Group workouts by date
    let mut daily_loads: HashMap<NaiveDate, MultiSportLoad> = HashMap::new();

    for workout in filtered_workouts {
        let tss_result = TssCalculator::calculate_tss(workout, athlete)?;
        let tss_value = tss_result.tss;

        let load = daily_loads.entry(workout.date).or_insert(MultiSportLoad {
            date: workout.date,
            total_tss: dec!(0),
            sport_breakdown: HashMap::new(),
            duration_breakdown: HashMap::new(),
        });

        load.total_tss += tss_value;
        *load.sport_breakdown.entry(workout.sport.clone()).or_insert(dec!(0)) += tss_value;
        *load.duration_breakdown.entry(workout.sport.clone()).or_insert(0) += workout.duration_seconds;
    }

    let mut result: Vec<MultiSportLoad> = daily_loads.into_values().collect();
    result.sort_by(|a, b| a.date.cmp(&b.date));

    Ok(result)
}

/// Calculate sport-specific CTL/ATL tracking
pub fn calculate_sport_specific_pmc(
    workouts: &[Workout],
    _athlete: &AthleteProfile,
    sport: Sport,
    from_date: NaiveDate,
    to_date: NaiveDate,
) -> Result<Vec<PmcMetrics>> {
    let sport_workouts: Vec<Workout> = workouts
        .iter()
        .filter(|w| w.sport == sport && w.date >= from_date && w.date <= to_date)
        .cloned()
        .collect();

    let pmc_calculator = PmcCalculator::new();
    let daily_tss = pmc_calculator.aggregate_daily_tss(&sport_workouts);
    Ok(pmc_calculator.calculate_pmc_series(&daily_tss, from_date, to_date)?)
}

/// Calculate training distribution by sport
pub fn calculate_sport_distribution(
    workouts: &[Workout],
    athlete: &AthleteProfile,
    period_days: u32,
    include_weekly: bool,
) -> Result<SportDistribution> {
    let end_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(); // Use current date in real implementation
    let start_date = end_date - Duration::days(period_days as i64);

    let filtered_workouts: Vec<&Workout> = workouts
        .iter()
        .filter(|w| w.date >= start_date && w.date <= end_date)
        .collect();

    let mut sport_time: HashMap<Sport, u32> = HashMap::new();
    let mut sport_tss: HashMap<Sport, Decimal> = HashMap::new();
    let mut total_time = 0u32;
    let mut total_tss = dec!(0);

    for workout in &filtered_workouts {
        let tss_result = TssCalculator::calculate_tss(workout, athlete)?;
        let tss_value = tss_result.tss;

        *sport_time.entry(workout.sport.clone()).or_insert(0) += workout.duration_seconds;
        *sport_tss.entry(workout.sport.clone()).or_insert(dec!(0)) += tss_value;

        total_time += workout.duration_seconds;
        total_tss += tss_value;
    }

    // Calculate percentages
    let sport_time_distribution: HashMap<Sport, f64> = sport_time
        .iter()
        .map(|(sport, time)| {
            let percentage = if total_time > 0 {
                (*time as f64 / total_time as f64) * 100.0
            } else {
                0.0
            };
            (sport.clone(), percentage)
        })
        .collect();

    let sport_tss_distribution: HashMap<Sport, f64> = sport_tss
        .iter()
        .map(|(sport, tss)| {
            let percentage = if total_tss > dec!(0) {
                (tss.to_f64().unwrap_or(0.0) / total_tss.to_f64().unwrap_or(1.0)) * 100.0
            } else {
                0.0
            };
            (sport.clone(), percentage)
        })
        .collect();

    let weekly_breakdown = if include_weekly {
        Some(calculate_weekly_breakdown(&filtered_workouts, athlete, start_date, end_date)?)
    } else {
        None
    };

    Ok(SportDistribution {
        period_days,
        total_time,
        total_tss,
        sport_time_distribution,
        sport_tss_distribution,
        weekly_breakdown,
    })
}

/// Calculate weekly breakdown for distribution analysis
fn calculate_weekly_breakdown(
    workouts: &[&Workout],
    athlete: &AthleteProfile,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<WeeklyDistribution>> {
    let mut weekly_data: Vec<WeeklyDistribution> = Vec::new();
    let mut current_week_start = start_date;

    while current_week_start <= end_date {
        let week_end = current_week_start + Duration::days(6);

        let week_workouts: Vec<&Workout> = workouts
            .iter()
            .filter(|w| w.date >= current_week_start && w.date <= week_end)
            .cloned()
            .collect();

        let mut sport_time: HashMap<Sport, u32> = HashMap::new();
        let mut sport_tss: HashMap<Sport, Decimal> = HashMap::new();

        for workout in week_workouts {
            let tss_result = TssCalculator::calculate_tss(workout, athlete)?;
            let tss_value = tss_result.tss;

            *sport_time.entry(workout.sport.clone()).or_insert(0) += workout.duration_seconds;
            *sport_tss.entry(workout.sport.clone()).or_insert(dec!(0)) += tss_value;
        }

        weekly_data.push(WeeklyDistribution {
            week_start: current_week_start,
            sport_time,
            sport_tss,
        });

        current_week_start = current_week_start + Duration::days(7);
    }

    Ok(weekly_data)
}

/// Calculate sport equivalency conversion factors
pub fn calculate_sport_equivalency(
    from_sport: Sport,
    to_sport: Sport,
    tss_value: Decimal,
) -> SportEquivalency {
    // Base conversion factors (these would be configurable in a real implementation)
    let conversion_matrix = get_sport_conversion_matrix();

    let conversion_factor = conversion_matrix
        .get(&(from_sport.clone(), to_sport.clone()))
        .unwrap_or(&dec!(1.0))
        .clone();

    let equivalent_tss = tss_value * conversion_factor;

    SportEquivalency {
        from_sport,
        to_sport,
        conversion_factor,
        original_tss: tss_value,
        equivalent_tss,
    }
}

/// Get sport conversion matrix (placeholder implementation)
fn get_sport_conversion_matrix() -> HashMap<(Sport, Sport), Decimal> {
    let mut matrix = HashMap::new();

    // Running to other sports
    matrix.insert((Sport::Running, Sport::Cycling), dec!(0.75));
    matrix.insert((Sport::Running, Sport::Swimming), dec!(1.2));
    matrix.insert((Sport::Running, Sport::Rowing), dec!(0.9));

    // Cycling to other sports
    matrix.insert((Sport::Cycling, Sport::Running), dec!(1.33));
    matrix.insert((Sport::Cycling, Sport::Swimming), dec!(1.6));
    matrix.insert((Sport::Cycling, Sport::Rowing), dec!(1.2));

    // Swimming to other sports
    matrix.insert((Sport::Swimming, Sport::Running), dec!(0.83));
    matrix.insert((Sport::Swimming, Sport::Cycling), dec!(0.625));
    matrix.insert((Sport::Swimming, Sport::Rowing), dec!(0.75));

    // Rowing to other sports
    matrix.insert((Sport::Rowing, Sport::Running), dec!(1.11));
    matrix.insert((Sport::Rowing, Sport::Cycling), dec!(0.83));
    matrix.insert((Sport::Rowing, Sport::Swimming), dec!(1.33));

    // Same sport conversions
    matrix.insert((Sport::Running, Sport::Running), dec!(1.0));
    matrix.insert((Sport::Cycling, Sport::Cycling), dec!(1.0));
    matrix.insert((Sport::Swimming, Sport::Swimming), dec!(1.0));
    matrix.insert((Sport::Rowing, Sport::Rowing), dec!(1.0));
    matrix.insert((Sport::Triathlon, Sport::Triathlon), dec!(1.0));
    matrix.insert((Sport::CrossTraining, Sport::CrossTraining), dec!(1.0));

    matrix
}

/// Calculate Critical Swim Speed (CSS)
pub fn calculate_css(swim_workouts: &[Workout]) -> Option<CssMetrics> {
    let swim_performances: Vec<SwimPerformance> = swim_workouts
        .iter()
        .filter(|w| w.sport == Sport::Swimming && w.duration_seconds > 0)
        .filter_map(|w| {
            if let Some(distance) = w.summary.total_distance {
                Some(SwimPerformance {
                    date: w.date,
                    distance,
                    time: w.duration_seconds,
                    pace: calculate_swim_pace(distance, w.duration_seconds),
                })
            } else {
                None
            }
        })
        .collect();

    if swim_performances.len() < 2 {
        return None;
    }

    // Simple CSS calculation using best performances
    // In a real implementation, this would use a more sophisticated algorithm
    let mut performances = swim_performances.clone();
    performances.sort_by(|a, b| a.pace.cmp(&b.pace));

    let css_pace = if performances.len() >= 3 {
        // Average of top 3 performances
        let top_3_avg = (performances[0].pace + performances[1].pace + performances[2].pace) / dec!(3);
        top_3_avg
    } else {
        // Average of available performances
        performances[0].pace
    };

    Some(CssMetrics {
        css_pace,
        css_threshold_power: None, // Would be calculated from power data
        recent_performances: performances.into_iter().take(10).collect(),
    })
}

/// Calculate swim pace in min/100m
fn calculate_swim_pace(distance_m: Decimal, time_seconds: u32) -> Decimal {
    if distance_m <= dec!(0) {
        return dec!(0);
    }

    let time_minutes = Decimal::from(time_seconds) / dec!(60);
    let pace_per_100m = (time_minutes * dec!(100)) / distance_m;
    pace_per_100m
}