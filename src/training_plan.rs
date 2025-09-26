use anyhow::Result;
use chrono::{Duration, NaiveDate};
use clap::Subcommand;
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::{Sport, WorkoutType, AthleteProfile};
use crate::pmc::PmcMetrics;

#[derive(Debug, Clone, Subcommand)]
pub enum TrainingPlanCommands {
    /// Generate a new training plan
    Generate {
        /// Goal type (marathon, century, triathlon, maintenance, base-building, return)
        #[arg(long)]
        goal: String,
        /// Target event date (YYYY-MM-DD)
        #[arg(long)]
        target_date: Option<NaiveDate>,
        /// Training weeks
        #[arg(long, default_value_t = 12)]
        weeks: u32,
        /// Periodization model (traditional, block, reverse)
        #[arg(long, default_value = "traditional")]
        model: String,
        /// Recovery pattern (3:1, 4:1, 2:1)
        #[arg(long, default_value = "3:1")]
        recovery: String,
    },
    /// Monitor and analyze plan progress
    Monitor {
        /// Plan ID or name
        #[arg(long)]
        plan: Option<String>,
        /// Show adjustments needed
        #[arg(long)]
        adjustments: bool,
    },
    /// Adjust existing plan
    Adjust {
        /// Plan ID or name
        #[arg(long)]
        plan: String,
        /// Adjustment type (increase, decrease, recovery)
        #[arg(long)]
        adjustment: String,
        /// Adjustment percentage
        #[arg(long, default_value_t = 10)]
        percentage: u32,
    },
}

/// Periodization model for training plans
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeriodizationModel {
    /// Traditional: Base → Build → Peak → Taper
    Traditional,
    /// Block: Focused blocks of specific adaptations
    Block,
    /// Reverse: Intensity first, then volume
    Reverse,
}

impl PeriodizationModel {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "traditional" => Ok(Self::Traditional),
            "block" => Ok(Self::Block),
            "reverse" => Ok(Self::Reverse),
            _ => anyhow::bail!("Unknown periodization model: {}", s),
        }
    }
}

/// Training goal types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainingGoal {
    Marathon,
    HalfMarathon,
    FiveK,
    TenK,
    Century,      // 100 mile bike ride
    Metric,       // 100 km bike ride
    Triathlon,
    IronmanFull,
    Ironman70_3,
    Maintenance,
    BaseBuilding,
    ReturnFromBreak,
}

impl TrainingGoal {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "marathon" => Ok(Self::Marathon),
            "half" | "halfmarathon" | "half-marathon" => Ok(Self::HalfMarathon),
            "5k" => Ok(Self::FiveK),
            "10k" => Ok(Self::TenK),
            "century" => Ok(Self::Century),
            "metric" | "metric-century" => Ok(Self::Metric),
            "triathlon" | "tri" => Ok(Self::Triathlon),
            "ironman" | "ironman-full" => Ok(Self::IronmanFull),
            "ironman70.3" | "70.3" | "half-ironman" => Ok(Self::Ironman70_3),
            "maintenance" => Ok(Self::Maintenance),
            "base" | "base-building" => Ok(Self::BaseBuilding),
            "return" | "return-from-break" => Ok(Self::ReturnFromBreak),
            _ => anyhow::bail!("Unknown training goal: {}", s),
        }
    }

    /// Get primary sport for this goal
    pub fn primary_sport(&self) -> Sport {
        match self {
            Self::Marathon | Self::HalfMarathon | Self::FiveK | Self::TenK => Sport::Running,
            Self::Century | Self::Metric => Sport::Cycling,
            Self::Triathlon | Self::IronmanFull | Self::Ironman70_3 => Sport::Triathlon,
            _ => Sport::CrossTraining,
        }
    }

    /// Get typical training duration in weeks
    pub fn typical_duration_weeks(&self) -> u32 {
        match self {
            Self::Marathon => 16,
            Self::HalfMarathon => 12,
            Self::FiveK | Self::TenK => 8,
            Self::Century => 12,
            Self::Metric => 10,
            Self::IronmanFull => 24,
            Self::Ironman70_3 => 16,
            Self::Triathlon => 12,
            Self::Maintenance => 4,
            Self::BaseBuilding => 8,
            Self::ReturnFromBreak => 6,
        }
    }
}

/// Recovery pattern for training cycles
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryPattern {
    TwoToOne,   // 2 build weeks, 1 recovery week
    ThreeToOne, // 3 build weeks, 1 recovery week
    FourToOne,  // 4 build weeks, 1 recovery week
}

impl RecoveryPattern {
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "2:1" => Ok(Self::TwoToOne),
            "3:1" => Ok(Self::ThreeToOne),
            "4:1" => Ok(Self::FourToOne),
            _ => anyhow::bail!("Unknown recovery pattern: {}", s),
        }
    }

    pub fn cycle_length(&self) -> u32 {
        match self {
            Self::TwoToOne => 3,
            Self::ThreeToOne => 4,
            Self::FourToOne => 5,
        }
    }

    pub fn build_weeks(&self) -> u32 {
        match self {
            Self::TwoToOne => 2,
            Self::ThreeToOne => 3,
            Self::FourToOne => 4,
        }
    }
}

/// Individual planned workout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedWorkout {
    pub date: NaiveDate,
    pub sport: Sport,
    pub workout_type: WorkoutType,
    pub planned_duration_minutes: u32,
    pub planned_tss: Decimal,
    pub description: String,
    pub intensity_factor: Decimal,
    pub notes: Option<String>,
}

/// Weekly training structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingWeek {
    pub week_number: u32,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub phase: String,
    pub focus: String,
    pub planned_tss: Decimal,
    pub planned_hours: Decimal,
    pub is_recovery_week: bool,
    pub workouts: Vec<PlannedWorkout>,
}

/// Overall training plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingPlan {
    pub id: String,
    pub name: String,
    pub goal: TrainingGoal,
    pub periodization_model: PeriodizationModel,
    pub recovery_pattern: RecoveryPattern,
    pub start_date: NaiveDate,
    pub target_date: Option<NaiveDate>,
    pub total_weeks: u32,
    pub weeks: Vec<TrainingWeek>,
    pub total_planned_tss: Decimal,
    pub total_planned_hours: Decimal,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Plan monitoring data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanMonitoring {
    pub plan_id: String,
    pub current_week: u32,
    pub actual_vs_planned_tss: HashMap<NaiveDate, (Decimal, Decimal)>, // (actual, planned)
    pub completion_rate: Decimal,
    pub adjustments_needed: Vec<String>,
    pub recovery_recommendations: Vec<String>,
}

/// Training plan generator
pub struct TrainingPlanGenerator;

impl TrainingPlanGenerator {
    /// Generate a new training plan
    pub fn generate_plan(
        goal: TrainingGoal,
        model: PeriodizationModel,
        recovery: RecoveryPattern,
        weeks: u32,
        target_date: Option<NaiveDate>,
        athlete: &AthleteProfile,
        current_metrics: Option<&PmcMetrics>,
    ) -> Result<TrainingPlan> {
        let plan_id = format!("plan_{}", chrono::Utc::now().timestamp());
        let name = format!("{:?} Training Plan", goal);

        let start_date = target_date
            .map(|td| td - Duration::weeks(weeks as i64))
            .unwrap_or_else(|| chrono::Local::now().date_naive());

        let mut plan = TrainingPlan {
            id: plan_id,
            name,
            goal: goal.clone(),
            periodization_model: model.clone(),
            recovery_pattern: recovery.clone(),
            start_date,
            target_date,
            total_weeks: weeks,
            weeks: Vec::new(),
            total_planned_tss: dec!(0),
            total_planned_hours: dec!(0),
            created_at: chrono::Utc::now(),
        };

        // Generate weeks based on periodization model
        match model {
            PeriodizationModel::Traditional => {
                plan.weeks = Self::generate_traditional_weeks(
                    &goal,
                    &recovery,
                    weeks,
                    start_date,
                    athlete,
                    current_metrics,
                )?;
            }
            PeriodizationModel::Block => {
                plan.weeks = Self::generate_block_weeks(
                    &goal,
                    &recovery,
                    weeks,
                    start_date,
                    athlete,
                    current_metrics,
                )?;
            }
            PeriodizationModel::Reverse => {
                plan.weeks = Self::generate_reverse_weeks(
                    &goal,
                    &recovery,
                    weeks,
                    start_date,
                    athlete,
                    current_metrics,
                )?;
            }
        }

        // Calculate totals
        for week in &plan.weeks {
            plan.total_planned_tss += week.planned_tss;
            plan.total_planned_hours += week.planned_hours;
        }

        Ok(plan)
    }

    /// Generate traditional periodization weeks (Base → Build → Peak → Taper)
    fn generate_traditional_weeks(
        goal: &TrainingGoal,
        recovery: &RecoveryPattern,
        total_weeks: u32,
        start_date: NaiveDate,
        athlete: &AthleteProfile,
        current_metrics: Option<&PmcMetrics>,
    ) -> Result<Vec<TrainingWeek>> {
        let mut weeks = Vec::new();

        // Calculate phase durations
        let base_weeks = (total_weeks as f32 * 0.4) as u32; // 40% base
        let build_weeks = (total_weeks as f32 * 0.3) as u32; // 30% build
        let peak_weeks = (total_weeks as f32 * 0.2) as u32; // 20% peak
        let taper_weeks = total_weeks - base_weeks - build_weeks - peak_weeks; // remaining for taper

        // Starting TSS (based on current fitness or conservative estimate)
        let starting_weekly_tss = current_metrics
            .map(|m| m.ctl * dec!(7))
            .unwrap_or(dec!(200));

        let mut current_date = start_date;
        let mut week_number = 1;

        // Base Phase
        for i in 0..base_weeks {
            let is_recovery = (i + 1) % recovery.cycle_length() == 0;
            let week = Self::create_week(
                week_number,
                current_date,
                "Base",
                "Aerobic Development",
                starting_weekly_tss * Self::get_base_multiplier(i, base_weeks),
                is_recovery,
                goal,
                athlete,
            )?;
            weeks.push(week);
            current_date = current_date + Duration::weeks(1);
            week_number += 1;
        }

        // Build Phase
        for i in 0..build_weeks {
            let is_recovery = (base_weeks + i + 1) % recovery.cycle_length() == 0;
            let week = Self::create_week(
                week_number,
                current_date,
                "Build",
                "Threshold Development",
                starting_weekly_tss * Self::get_build_multiplier(i, build_weeks),
                is_recovery,
                goal,
                athlete,
            )?;
            weeks.push(week);
            current_date = current_date + Duration::weeks(1);
            week_number += 1;
        }

        // Peak Phase
        for i in 0..peak_weeks {
            let is_recovery = (base_weeks + build_weeks + i + 1) % recovery.cycle_length() == 0;
            let week = Self::create_week(
                week_number,
                current_date,
                "Peak",
                "Race Specific",
                starting_weekly_tss * Self::get_peak_multiplier(i, peak_weeks),
                is_recovery,
                goal,
                athlete,
            )?;
            weeks.push(week);
            current_date = current_date + Duration::weeks(1);
            week_number += 1;
        }

        // Taper Phase
        for i in 0..taper_weeks {
            let week = Self::create_week(
                week_number,
                current_date,
                "Taper",
                "Race Preparation",
                starting_weekly_tss * Self::get_taper_multiplier(i, taper_weeks),
                false, // No recovery weeks during taper
                goal,
                athlete,
            )?;
            weeks.push(week);
            current_date = current_date + Duration::weeks(1);
            week_number += 1;
        }

        Ok(weeks)
    }

    /// Generate block periodization weeks
    fn generate_block_weeks(
        goal: &TrainingGoal,
        recovery: &RecoveryPattern,
        total_weeks: u32,
        start_date: NaiveDate,
        athlete: &AthleteProfile,
        current_metrics: Option<&PmcMetrics>,
    ) -> Result<Vec<TrainingWeek>> {
        let mut weeks = Vec::new();

        // Block periodization focuses on one quality at a time
        // Typical blocks: Accumulation → Intensification → Realization
        let block_length = 4; // 4-week blocks
        let num_blocks = total_weeks / block_length;

        let starting_weekly_tss = current_metrics
            .map(|m| m.ctl * dec!(7))
            .unwrap_or(dec!(200));

        let mut current_date = start_date;
        let mut week_number = 1;

        for block_num in 0..num_blocks {
            let block_type = match block_num % 3 {
                0 => ("Accumulation", "Volume Focus"),
                1 => ("Intensification", "Intensity Focus"),
                2 => ("Realization", "Race Pace Focus"),
                _ => ("Accumulation", "Volume Focus"),
            };

            for week_in_block in 0..block_length {
                let is_recovery = week_in_block == block_length - 1; // Last week of block is recovery
                let tss_multiplier = match block_type.0 {
                    "Accumulation" => dec!(1.2) + (dec!(0.1) * Decimal::from(block_num)),
                    "Intensification" => dec!(1.0) + (dec!(0.05) * Decimal::from(block_num)),
                    "Realization" => dec!(0.9) + (dec!(0.02) * Decimal::from(block_num)),
                    _ => dec!(1.0),
                };

                let week = Self::create_week(
                    week_number,
                    current_date,
                    block_type.0,
                    block_type.1,
                    starting_weekly_tss * tss_multiplier,
                    is_recovery,
                    goal,
                    athlete,
                )?;
                weeks.push(week);
                current_date = current_date + Duration::weeks(1);
                week_number += 1;
            }
        }

        // Handle remaining weeks
        for _ in weeks.len()..total_weeks as usize {
            let week = Self::create_week(
                week_number,
                current_date,
                "Taper",
                "Recovery & Sharpening",
                starting_weekly_tss * dec!(0.7),
                false,
                goal,
                athlete,
            )?;
            weeks.push(week);
            current_date = current_date + Duration::weeks(1);
            week_number += 1;
        }

        Ok(weeks)
    }

    /// Generate reverse periodization weeks (Intensity → Volume)
    fn generate_reverse_weeks(
        goal: &TrainingGoal,
        recovery: &RecoveryPattern,
        total_weeks: u32,
        start_date: NaiveDate,
        athlete: &AthleteProfile,
        current_metrics: Option<&PmcMetrics>,
    ) -> Result<Vec<TrainingWeek>> {
        let mut weeks = Vec::new();

        // Reverse periodization: Start with intensity, build volume
        let intensity_weeks = (total_weeks as f32 * 0.3) as u32;
        let volume_weeks = (total_weeks as f32 * 0.5) as u32;
        let race_prep_weeks = total_weeks - intensity_weeks - volume_weeks;

        let starting_weekly_tss = current_metrics
            .map(|m| m.ctl * dec!(7))
            .unwrap_or(dec!(200));

        let mut current_date = start_date;
        let mut week_number = 1;

        // Intensity Phase
        for i in 0..intensity_weeks {
            let is_recovery = (i + 1) % recovery.cycle_length() == 0;
            let week = Self::create_week(
                week_number,
                current_date,
                "Intensity",
                "Speed & Power Development",
                starting_weekly_tss * dec!(0.8), // Lower volume, high intensity
                is_recovery,
                goal,
                athlete,
            )?;
            weeks.push(week);
            current_date = current_date + Duration::weeks(1);
            week_number += 1;
        }

        // Volume Phase
        for i in 0..volume_weeks {
            let is_recovery = (intensity_weeks + i + 1) % recovery.cycle_length() == 0;
            let volume_multiplier = dec!(1.0) + (dec!(0.1) * Decimal::from(i) / Decimal::from(volume_weeks.max(1)));
            let week = Self::create_week(
                week_number,
                current_date,
                "Volume",
                "Endurance Building",
                starting_weekly_tss * volume_multiplier * dec!(1.3),
                is_recovery,
                goal,
                athlete,
            )?;
            weeks.push(week);
            current_date = current_date + Duration::weeks(1);
            week_number += 1;
        }

        // Race Preparation Phase
        for i in 0..race_prep_weeks {
            let week = Self::create_week(
                week_number,
                current_date,
                "Race Prep",
                "Sharpening & Taper",
                starting_weekly_tss * (dec!(1.0) - (dec!(0.2) * Decimal::from(i) / Decimal::from(race_prep_weeks.max(1)))),
                false,
                goal,
                athlete,
            )?;
            weeks.push(week);
            current_date = current_date + Duration::weeks(1);
            week_number += 1;
        }

        Ok(weeks)
    }

    /// Create a training week with planned workouts
    fn create_week(
        week_number: u32,
        start_date: NaiveDate,
        phase: &str,
        focus: &str,
        planned_tss: Decimal,
        is_recovery: bool,
        goal: &TrainingGoal,
        athlete: &AthleteProfile,
    ) -> Result<TrainingWeek> {
        let end_date = start_date + Duration::days(6);
        let tss = if is_recovery {
            planned_tss * dec!(0.6) // 40% reduction for recovery weeks
        } else {
            planned_tss
        };

        // Calculate hours from TSS (rough estimate: 100 TSS ≈ 1.5 hours)
        let planned_hours = tss / dec!(100) * dec!(1.5);

        let mut week = TrainingWeek {
            week_number,
            start_date,
            end_date,
            phase: phase.to_string(),
            focus: focus.to_string(),
            planned_tss: tss,
            planned_hours,
            is_recovery_week: is_recovery,
            workouts: Vec::new(),
        };

        // Generate workouts for the week
        week.workouts = Self::generate_week_workouts(
            &week,
            goal,
            athlete,
        )?;

        Ok(week)
    }

    /// Generate individual workouts for a week
    fn generate_week_workouts(
        week: &TrainingWeek,
        goal: &TrainingGoal,
        athlete: &AthleteProfile,
    ) -> Result<Vec<PlannedWorkout>> {
        let mut workouts = Vec::new();
        let sport = goal.primary_sport();

        // Distribute TSS across the week based on phase and recovery status
        let daily_distribution = if week.is_recovery_week {
            vec![0.0, 0.15, 0.0, 0.20, 0.0, 0.25, 0.0] // Light week
        } else {
            match week.phase.as_str() {
                "Base" | "Accumulation" | "Volume" => {
                    vec![0.0, 0.15, 0.10, 0.20, 0.10, 0.30, 0.15] // Volume focus
                }
                "Build" | "Intensification" | "Intensity" => {
                    vec![0.0, 0.20, 0.15, 0.25, 0.0, 0.25, 0.15] // Intensity focus
                }
                "Peak" | "Realization" | "Race Prep" => {
                    vec![0.0, 0.25, 0.10, 0.30, 0.0, 0.20, 0.15] // Race specific
                }
                "Taper" => {
                    vec![0.0, 0.15, 0.10, 0.15, 0.0, 0.10, 0.0] // Reduced load
                }
                _ => vec![0.0, 0.15, 0.15, 0.20, 0.10, 0.25, 0.15] // Default
            }
        };

        for (day, distribution) in daily_distribution.iter().enumerate() {
            if *distribution > 0.0 {
                let workout_date = week.start_date + Duration::days(day as i64);
                let workout_tss = week.planned_tss * Decimal::from_f64(*distribution).unwrap_or(dec!(0));

                let (workout_type, description, intensity_factor) = Self::determine_workout_type(
                    day,
                    &week.phase,
                    week.is_recovery_week,
                    goal,
                );

                let duration_minutes = (workout_tss / intensity_factor / dec!(100) * dec!(60)).to_u32().unwrap_or(60);

                workouts.push(PlannedWorkout {
                    date: workout_date,
                    sport: sport.clone(),
                    workout_type,
                    planned_duration_minutes: duration_minutes,
                    planned_tss: workout_tss,
                    description,
                    intensity_factor,
                    notes: None,
                });
            }
        }

        Ok(workouts)
    }

    /// Determine workout type based on day and phase
    fn determine_workout_type(
        day: usize,
        phase: &str,
        is_recovery: bool,
        goal: &TrainingGoal,
    ) -> (WorkoutType, String, Decimal) {
        if is_recovery {
            return (
                WorkoutType::Recovery,
                "Easy recovery session".to_string(),
                dec!(0.65),
            );
        }

        match day {
            1 | 3 => {
                // Tuesday/Thursday - Quality days
                match phase {
                    "Base" | "Accumulation" => (
                        WorkoutType::Tempo,
                        "Steady state tempo work".to_string(),
                        dec!(0.85),
                    ),
                    "Build" | "Intensification" => (
                        WorkoutType::Threshold,
                        "Threshold intervals".to_string(),
                        dec!(0.90),
                    ),
                    "Peak" | "Realization" => (
                        WorkoutType::Race,
                        "Race pace intervals".to_string(),
                        dec!(0.95),
                    ),
                    _ => (
                        WorkoutType::Endurance,
                        "Moderate effort".to_string(),
                        dec!(0.75),
                    ),
                }
            }
            5 => {
                // Saturday - Long workout
                (
                    WorkoutType::Endurance,
                    "Long endurance session".to_string(),
                    dec!(0.70),
                )
            }
            6 => {
                // Sunday - Recovery or easy
                (
                    WorkoutType::Recovery,
                    "Recovery or easy session".to_string(),
                    dec!(0.65),
                )
            }
            _ => {
                // Other days
                (
                    WorkoutType::Endurance,
                    "Base endurance work".to_string(),
                    dec!(0.75),
                )
            }
        }
    }

    // Helper methods for TSS multipliers
    fn get_base_multiplier(week: u32, total: u32) -> Decimal {
        dec!(1.0) + (dec!(0.3) * Decimal::from(week) / Decimal::from(total.max(1)))
    }

    fn get_build_multiplier(week: u32, total: u32) -> Decimal {
        dec!(1.3) + (dec!(0.2) * Decimal::from(week) / Decimal::from(total.max(1)))
    }

    fn get_peak_multiplier(week: u32, total: u32) -> Decimal {
        dec!(1.5) - (dec!(0.1) * Decimal::from(week) / Decimal::from(total.max(1)))
    }

    fn get_taper_multiplier(week: u32, total: u32) -> Decimal {
        dec!(1.0) - (dec!(0.5) * Decimal::from(week) / Decimal::from(total.max(1)))
    }
}

/// Plan monitor for tracking and adjustments
pub struct PlanMonitor;

impl PlanMonitor {
    /// Monitor plan progress
    pub fn monitor_progress(
        plan: &TrainingPlan,
        actual_workouts: &[crate::models::Workout],
    ) -> Result<PlanMonitoring> {
        let mut monitoring = PlanMonitoring {
            plan_id: plan.id.clone(),
            current_week: Self::calculate_current_week(plan),
            actual_vs_planned_tss: HashMap::new(),
            completion_rate: dec!(0),
            adjustments_needed: Vec::new(),
            recovery_recommendations: Vec::new(),
        };

        // Calculate actual vs planned TSS
        for week in &plan.weeks {
            for workout in &week.workouts {
                let actual_tss = actual_workouts
                    .iter()
                    .filter(|w| w.date == workout.date)
                    .map(|w| w.summary.tss.unwrap_or(dec!(0)))
                    .sum::<Decimal>();

                monitoring.actual_vs_planned_tss.insert(
                    workout.date,
                    (actual_tss, workout.planned_tss),
                );
            }
        }

        // Calculate completion rate
        let total_planned = monitoring.actual_vs_planned_tss
            .values()
            .map(|(_, p)| p)
            .sum::<Decimal>();
        let total_actual = monitoring.actual_vs_planned_tss
            .values()
            .map(|(a, _)| a)
            .sum::<Decimal>();

        monitoring.completion_rate = if total_planned > dec!(0) {
            (total_actual / total_planned * dec!(100)).round()
        } else {
            dec!(0)
        };

        // Generate adjustments and recommendations
        monitoring.adjustments_needed = Self::generate_adjustments(&monitoring);
        monitoring.recovery_recommendations = Self::generate_recovery_recommendations(&monitoring);

        Ok(monitoring)
    }

    fn calculate_current_week(plan: &TrainingPlan) -> u32 {
        let today = chrono::Local::now().date_naive();
        let days_since_start = (today - plan.start_date).num_days();
        ((days_since_start / 7) + 1).max(1) as u32
    }

    fn generate_adjustments(monitoring: &PlanMonitoring) -> Vec<String> {
        let mut adjustments = Vec::new();

        if monitoring.completion_rate < dec!(80) {
            adjustments.push("Consider reducing planned TSS by 10-15% for upcoming weeks".to_string());
        }

        if monitoring.completion_rate > dec!(110) {
            adjustments.push("You're exceeding planned load - monitor fatigue carefully".to_string());
        }

        // Check for consistent under/over performance
        let recent_performance: Vec<_> = monitoring.actual_vs_planned_tss
            .values()
            .take(7) // Last week
            .collect();

        if recent_performance.iter().all(|(a, p)| a < &(p * dec!(0.9))) {
            adjustments.push("Consistently under target - consider recovery or reduced volume".to_string());
        }

        if recent_performance.iter().all(|(a, p)| a > &(p * dec!(1.1))) {
            adjustments.push("Consistently over target - great progress but watch for overtraining".to_string());
        }

        adjustments
    }

    fn generate_recovery_recommendations(monitoring: &PlanMonitoring) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Check recent load
        let recent_actual: Decimal = monitoring.actual_vs_planned_tss
            .values()
            .take(7)
            .map(|(a, _)| a)
            .sum();

        if recent_actual > dec!(600) {
            recommendations.push("High recent load - ensure adequate rest and nutrition".to_string());
        }

        if monitoring.current_week % 4 == 0 {
            recommendations.push("Recovery week scheduled - reduce intensity and volume".to_string());
        }

        recommendations
    }

    /// Adjust existing plan based on progress
    pub fn adjust_plan(
        plan: &mut TrainingPlan,
        adjustment_type: &str,
        percentage: u32,
    ) -> Result<()> {
        let adjustment_factor = match adjustment_type {
            "increase" => dec!(1) + (Decimal::from(percentage) / dec!(100)),
            "decrease" => dec!(1) - (Decimal::from(percentage) / dec!(100)),
            "recovery" => dec!(0.6), // Standard recovery week reduction
            _ => anyhow::bail!("Unknown adjustment type: {}", adjustment_type),
        };

        // Apply adjustment to future weeks
        let today = chrono::Local::now().date_naive();
        for week in &mut plan.weeks {
            if week.start_date >= today {
                week.planned_tss *= adjustment_factor;
                week.planned_hours *= adjustment_factor;

                // Adjust individual workouts
                for workout in &mut week.workouts {
                    workout.planned_tss *= adjustment_factor;
                    workout.planned_duration_minutes =
                        (Decimal::from(workout.planned_duration_minutes) * adjustment_factor)
                            .to_u32()
                            .unwrap_or(30);
                }
            }
        }

        // Recalculate totals
        plan.total_planned_tss = plan.weeks.iter().map(|w| w.planned_tss).sum();
        plan.total_planned_hours = plan.weeks.iter().map(|w| w.planned_hours).sum();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_periodization_model_parsing() {
        assert_eq!(
            PeriodizationModel::from_str("traditional").unwrap(),
            PeriodizationModel::Traditional
        );
        assert_eq!(
            PeriodizationModel::from_str("block").unwrap(),
            PeriodizationModel::Block
        );
        assert_eq!(
            PeriodizationModel::from_str("reverse").unwrap(),
            PeriodizationModel::Reverse
        );
    }

    #[test]
    fn test_training_goal_parsing() {
        assert_eq!(TrainingGoal::from_str("marathon").unwrap(), TrainingGoal::Marathon);
        assert_eq!(TrainingGoal::from_str("5k").unwrap(), TrainingGoal::FiveK);
        assert_eq!(TrainingGoal::from_str("century").unwrap(), TrainingGoal::Century);
        assert_eq!(TrainingGoal::from_str("base").unwrap(), TrainingGoal::BaseBuilding);
    }

    #[test]
    fn test_recovery_pattern_parsing() {
        assert_eq!(RecoveryPattern::from_str("3:1").unwrap(), RecoveryPattern::ThreeToOne);
        assert_eq!(RecoveryPattern::from_str("3:1").unwrap().cycle_length(), 4);
        assert_eq!(RecoveryPattern::from_str("3:1").unwrap().build_weeks(), 3);
    }

    #[test]
    fn test_plan_generation() {
        let athlete = crate::models::AthleteProfile {
            id: "test".to_string(),
            name: "Test Athlete".to_string(),
            date_of_birth: None,
            weight: Some(dec!(70)),
            height: Some(180),
            ftp: Some(250),
            lthr: Some(165),
            threshold_pace: Some(dec!(4.5)),
            max_hr: Some(185),
            resting_hr: Some(50),
            training_zones: Default::default(),
            preferred_units: crate::models::Units::Metric,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let plan = TrainingPlanGenerator::generate_plan(
            TrainingGoal::Marathon,
            PeriodizationModel::Traditional,
            RecoveryPattern::ThreeToOne,
            12,
            None,
            &athlete,
            None,
        ).unwrap();

        assert_eq!(plan.total_weeks, 12);
        assert_eq!(plan.weeks.len(), 12);
        assert!(plan.total_planned_tss > dec!(0));
    }
}