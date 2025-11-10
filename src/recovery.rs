//! Recovery, HRV, Sleep, Body Battery, Physiological Monitoring, and Unified Metrics
//!
//! This module provides comprehensive data structures and functionality for tracking athlete
//! recovery through Heart Rate Variability (HRV), sleep analysis, Body Battery energy tracking,
//! physiological monitoring, and unified recovery metrics with training readiness assessment.
//!
//! # Sports Science Background
//!
//! HRV measures the variation in time between successive heartbeats. It's a key indicator
//! of autonomic nervous system function and recovery status:
//!
//! - **RMSSD (Root Mean Square of Successive Differences)**: The most common HRV metric,
//!   measured in milliseconds. Typical values range from 20-100ms, with higher values
//!   indicating better recovery.
//!
//! - **HRV Status**: Categorizes current HRV relative to baseline:
//!   - **Balanced**: HRV within normal range, good recovery
//!   - **Unbalanced**: Moderate deviation from baseline, partial recovery
//!   - **Poor**: Significant deviation, inadequate recovery
//!   - **No Reading**: Insufficient data or measurement error
//!
//! - **Baseline**: Personal 7-60 day rolling average used for comparison
//!
//! - **Score**: Normalized 0-100 value for easier interpretation
//!
//! # Measurement Context
//!
//! HRV measurements are most reliable when taken:
//! - First thing in the morning (before getting out of bed)
//! - During sleep (overnight tracking)
//! - Pre-workout (to assess readiness)
//!
//! Consistency in measurement timing improves reliability.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// HRV status categories based on comparison to personal baseline
///
/// # Sports Science Context
///
/// HRV status reflects the balance of the autonomic nervous system:
/// - **Balanced**: Parasympathetic (rest/recovery) dominance
/// - **Unbalanced**: Mixed autonomic state
/// - **Poor**: Sympathetic (stress) dominance, inadequate recovery
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HrvStatus {
    /// HRV well below baseline, indicates poor recovery
    Poor,
    /// HRV moderately below baseline, indicates partial recovery
    Unbalanced,
    /// HRV within normal range of baseline, indicates good recovery
    Balanced,
    /// No valid HRV reading available
    NoReading,
}

impl fmt::Display for HrvStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HrvStatus::Poor => write!(f, "Poor"),
            HrvStatus::Unbalanced => write!(f, "Unbalanced"),
            HrvStatus::Balanced => write!(f, "Balanced"),
            HrvStatus::NoReading => write!(f, "No Reading"),
        }
    }
}

impl HrvStatus {
    /// Determine HRV status from RMSSD value and baseline
    ///
    /// # Algorithm
    ///
    /// Status is determined by deviation from baseline:
    /// - Balanced: Within ±15% of baseline
    /// - Unbalanced: 15-30% below baseline
    /// - Poor: >30% below baseline
    ///
    /// # Arguments
    ///
    /// * `rmssd` - Current RMSSD value in milliseconds
    /// * `baseline` - Personal baseline RMSSD in milliseconds
    ///
    /// # Returns
    ///
    /// HRV status category
    pub fn from_rmssd(rmssd: f64, baseline: f64) -> Self {
        if baseline <= 0.0 {
            return HrvStatus::NoReading;
        }

        let deviation_pct = ((rmssd - baseline) / baseline) * 100.0;

        if deviation_pct >= -15.0 {
            HrvStatus::Balanced
        } else if deviation_pct >= -30.0 {
            HrvStatus::Unbalanced
        } else {
            HrvStatus::Poor
        }
    }
}

/// Comprehensive HRV metrics for a single measurement period
///
/// # Usage
///
/// ```rust
/// use trainrs::recovery::{HrvMetrics, HrvStatus};
/// use chrono::Utc;
///
/// let metrics = HrvMetrics {
///     rmssd: Some(45.5),
///     status: Some(HrvStatus::Balanced),
///     baseline: Some(48.0),
///     score: Some(85),
///     measurement_time: Some(Utc::now()),
///     measurement_context: Some("morning".to_string()),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HrvMetrics {
    /// Root mean square of successive differences in milliseconds
    ///
    /// Typical range: 10-200ms
    /// - Elite athletes: 60-100ms+
    /// - Average adults: 20-50ms
    /// - Low fitness/high stress: <20ms
    pub rmssd: Option<f64>,

    /// HRV status relative to baseline
    pub status: Option<HrvStatus>,

    /// Personal baseline RMSSD in milliseconds
    ///
    /// Typically calculated as 7-60 day rolling average
    pub baseline: Option<f64>,

    /// Normalized HRV score (0-100)
    ///
    /// - 80-100: Excellent recovery
    /// - 60-79: Good recovery
    /// - 40-59: Moderate recovery
    /// - 20-39: Poor recovery
    /// - 0-19: Very poor recovery
    pub score: Option<u8>,

    /// Timestamp of measurement
    pub measurement_time: Option<DateTime<Utc>>,

    /// Context of measurement (e.g., "sleep", "morning", "pre-workout")
    ///
    /// Helps ensure consistency in measurement conditions
    pub measurement_context: Option<String>,
}

impl HrvMetrics {
    /// Create new HRV metrics with validation
    ///
    /// # Arguments
    ///
    /// * `rmssd` - RMSSD value in milliseconds
    /// * `baseline` - Personal baseline in milliseconds
    /// * `measurement_time` - When the measurement was taken
    /// * `measurement_context` - Context of measurement
    ///
    /// # Returns
    ///
    /// Result containing validated HrvMetrics or validation error
    pub fn new(
        rmssd: f64,
        baseline: Option<f64>,
        measurement_time: DateTime<Utc>,
        measurement_context: Option<String>,
    ) -> Result<Self, HrvValidationError> {
        // Validate RMSSD range
        if !Self::is_valid_rmssd(rmssd) {
            return Err(HrvValidationError::InvalidRmssd(rmssd));
        }

        // Validate baseline if provided
        if let Some(b) = baseline {
            if !Self::is_valid_rmssd(b) {
                return Err(HrvValidationError::InvalidBaseline(b));
            }
        }

        // Calculate status if baseline available
        let status = baseline.map(|b| HrvStatus::from_rmssd(rmssd, b));

        // Calculate score
        let score = Self::calculate_score(rmssd, baseline);

        Ok(HrvMetrics {
            rmssd: Some(rmssd),
            status,
            baseline,
            score: Some(score),
            measurement_time: Some(measurement_time),
            measurement_context,
        })
    }

    /// Validate RMSSD value is within physiologically plausible range
    ///
    /// Valid range: 10-200ms
    /// - Below 10ms: Likely measurement error
    /// - Above 200ms: Extremely rare, likely error
    pub fn is_valid_rmssd(rmssd: f64) -> bool {
        rmssd >= 10.0 && rmssd <= 200.0
    }

    /// Calculate normalized HRV score (0-100)
    ///
    /// # Algorithm
    ///
    /// If baseline available:
    /// - Score based on deviation from baseline
    /// - 100 = at or above baseline
    /// - Linear decrease for values below baseline
    ///
    /// If no baseline:
    /// - Score based on absolute RMSSD value
    /// - 100 = 100ms or higher
    /// - Linear scale from 20ms (score 20) to 100ms (score 100)
    fn calculate_score(rmssd: f64, baseline: Option<f64>) -> u8 {
        if let Some(b) = baseline {
            if b <= 0.0 {
                return 0;
            }

            let ratio = rmssd / b;
            let score = (ratio * 100.0).min(100.0).max(0.0);
            score as u8
        } else {
            // Without baseline, use absolute scale
            // 20ms = score 20, 100ms = score 100
            let score = ((rmssd - 20.0) / 80.0 * 80.0 + 20.0)
                .min(100.0)
                .max(20.0);
            score as u8
        }
    }

    /// Update baseline with new RMSSD value using exponential moving average
    ///
    /// # Arguments
    ///
    /// * `alpha` - Smoothing factor (0.0-1.0), typically 0.1-0.3 for HRV
    ///
    /// # Algorithm
    ///
    /// EMA = alpha * new_value + (1 - alpha) * old_baseline
    pub fn update_baseline(&mut self, alpha: f64) {
        if let (Some(rmssd), Some(baseline)) = (self.rmssd, self.baseline) {
            let new_baseline = alpha * rmssd + (1.0 - alpha) * baseline;
            self.baseline = Some(new_baseline);

            // Recalculate status and score with new baseline
            self.status = Some(HrvStatus::from_rmssd(rmssd, new_baseline));
            self.score = Some(Self::calculate_score(rmssd, Some(new_baseline)));
        }
    }
}

/// Individual HRV measurement record
///
/// Represents a single point-in-time HRV measurement, typically from
/// a morning reading or overnight sleep analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HrvMeasurement {
    /// Unique identifier for this measurement
    pub id: Option<i64>,

    /// Athlete identifier
    pub athlete_id: Option<i64>,

    /// When the measurement was taken
    pub timestamp: DateTime<Utc>,

    /// RMSSD value in milliseconds
    pub rmssd: f64,

    /// Personal baseline at time of measurement
    pub baseline: Option<f64>,

    /// HRV status
    pub status: HrvStatus,

    /// Normalized score (0-100)
    pub score: u8,

    /// Measurement context ("sleep", "morning", "pre-workout")
    pub context: Option<String>,

    /// Device or source of measurement
    pub source: Option<String>,

    /// Additional metadata (JSON)
    pub metadata: Option<serde_json::Value>,
}

impl HrvMeasurement {
    /// Create new HRV measurement with validation
    ///
    /// # Arguments
    ///
    /// * `timestamp` - When measurement was taken
    /// * `rmssd` - RMSSD value in milliseconds
    /// * `baseline` - Optional personal baseline
    /// * `context` - Optional measurement context
    ///
    /// # Returns
    ///
    /// Result containing validated measurement or validation error
    pub fn new(
        timestamp: DateTime<Utc>,
        rmssd: f64,
        baseline: Option<f64>,
        context: Option<String>,
    ) -> Result<Self, HrvValidationError> {
        // Validate RMSSD
        if !HrvMetrics::is_valid_rmssd(rmssd) {
            return Err(HrvValidationError::InvalidRmssd(rmssd));
        }

        // Validate baseline if provided
        if let Some(b) = baseline {
            if !HrvMetrics::is_valid_rmssd(b) {
                return Err(HrvValidationError::InvalidBaseline(b));
            }
        }

        let status = baseline
            .map(|b| HrvStatus::from_rmssd(rmssd, b))
            .unwrap_or(HrvStatus::NoReading);

        let score = HrvMetrics::calculate_score(rmssd, baseline);

        Ok(HrvMeasurement {
            id: None,
            athlete_id: None,
            timestamp,
            rmssd,
            baseline,
            status,
            score,
            context,
            source: None,
            metadata: None,
        })
    }
}

/// HRV validation errors
#[derive(Debug, Clone, PartialEq)]
pub enum HrvValidationError {
    /// RMSSD value outside valid range (10-200ms)
    InvalidRmssd(f64),
    /// Baseline value outside valid range
    InvalidBaseline(f64),
    /// Invalid score value (must be 0-100)
    InvalidScore(u8),
}

impl fmt::Display for HrvValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HrvValidationError::InvalidRmssd(v) => {
                write!(f, "Invalid RMSSD value: {}ms (valid range: 10-200ms)", v)
            }
            HrvValidationError::InvalidBaseline(v) => {
                write!(f, "Invalid baseline value: {}ms (valid range: 10-200ms)", v)
            }
            HrvValidationError::InvalidScore(v) => {
                write!(f, "Invalid score value: {} (valid range: 0-100)", v)
            }
        }
    }
}

impl std::error::Error for HrvValidationError {}

//
// ============================================================================
// SLEEP TRACKING
// ============================================================================
//

/// Sleep stages based on polysomnography classifications
///
/// # Sleep Science Background
///
/// Sleep cycles through distinct stages, each with specific physiological characteristics:
///
/// - **Deep Sleep (Slow-Wave Sleep)**: Most restorative stage, growth hormone release,
///   tissue repair, immune system strengthening. Typically 15-25% of sleep.
///
/// - **Light Sleep (NREM 1 & 2)**: Transitional stages, body temperature drops, heart
///   rate slows. Typically 50-60% of sleep.
///
/// - **REM Sleep**: Rapid eye movement, dreaming, memory consolidation, learning.
///   Typically 20-25% of sleep.
///
/// - **Awake**: Brief awakenings during sleep, normal if <5% of time in bed.
///
/// # Stage Distribution
///
/// Healthy adult sleep architecture (% of total sleep time):
/// - Deep: 13-23%
/// - Light: 45-55%
/// - REM: 20-25%
/// - Awake: <5%
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SleepStage {
    /// Deep sleep / Slow-wave sleep (NREM 3)
    Deep,
    /// Light sleep (NREM 1 & 2)
    Light,
    /// REM (Rapid Eye Movement) sleep
    REM,
    /// Awake periods during sleep
    Awake,
}

impl fmt::Display for SleepStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SleepStage::Deep => write!(f, "Deep"),
            SleepStage::Light => write!(f, "Light"),
            SleepStage::REM => write!(f, "REM"),
            SleepStage::Awake => write!(f, "Awake"),
        }
    }
}

/// Individual sleep stage segment with timing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SleepStageSegment {
    /// Sleep stage type
    pub stage: SleepStage,
    /// Start time of this stage segment
    pub start_time: DateTime<Utc>,
    /// End time of this stage segment
    pub end_time: DateTime<Utc>,
    /// Duration in minutes
    pub duration_minutes: u16,
}

impl SleepStageSegment {
    /// Create new sleep stage segment
    pub fn new(
        stage: SleepStage,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Self, SleepValidationError> {
        if end_time <= start_time {
            return Err(SleepValidationError::InvalidTimeRange);
        }

        let duration_minutes = ((end_time - start_time).num_minutes() as u16).min(u16::MAX);

        Ok(SleepStageSegment {
            stage,
            start_time,
            end_time,
            duration_minutes,
        })
    }
}

/// Comprehensive sleep metrics for a sleep session
///
/// # Usage
///
/// ```rust
/// use trainrs::recovery::SleepMetrics;
///
/// let metrics = SleepMetrics {
///     total_sleep: 420,      // 7 hours
///     deep_sleep: 90,        // 1.5 hours (21%)
///     light_sleep: 240,      // 4 hours (57%)
///     rem_sleep: 90,         // 1.5 hours (21%)
///     awake_time: 30,        // 30 minutes
///     sleep_score: Some(85),
///     sleep_efficiency: Some(93.3),
///     sleep_onset: Some(12),
///     interruptions: Some(2),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SleepMetrics {
    /// Total sleep time in minutes (excludes awake time)
    pub total_sleep: u16,

    /// Deep sleep duration in minutes
    pub deep_sleep: u16,

    /// Light sleep duration in minutes
    pub light_sleep: u16,

    /// REM sleep duration in minutes
    pub rem_sleep: u16,

    /// Time spent awake during sleep period in minutes
    pub awake_time: u16,

    /// Sleep quality score (0-100)
    ///
    /// Based on:
    /// - Sleep stage distribution (40%)
    /// - Sleep efficiency (30%)
    /// - Sleep duration vs. need (20%)
    /// - Interruptions (10%)
    pub sleep_score: Option<u8>,

    /// Sleep efficiency percentage
    ///
    /// Formula: (total_sleep / time_in_bed) * 100
    /// - >85%: Excellent
    /// - 75-85%: Good
    /// - 65-75%: Fair
    /// - <65%: Poor
    pub sleep_efficiency: Option<f64>,

    /// Sleep onset latency in minutes (time to fall asleep)
    ///
    /// - <15 min: Good
    /// - 15-30 min: Normal
    /// - >30 min: Delayed onset, possible issue
    pub sleep_onset: Option<u16>,

    /// Number of wake periods during sleep
    ///
    /// - 0-2: Normal
    /// - 3-5: Moderate fragmentation
    /// - >5: High fragmentation
    pub interruptions: Option<u8>,
}

impl SleepMetrics {
    /// Create new sleep metrics with validation and automatic calculations
    pub fn new(
        deep_sleep: u16,
        light_sleep: u16,
        rem_sleep: u16,
        awake_time: u16,
        sleep_onset: Option<u16>,
        interruptions: Option<u8>,
    ) -> Result<Self, SleepValidationError> {
        let total_sleep = deep_sleep + light_sleep + rem_sleep;

        // Validate sleep stage distribution
        if total_sleep == 0 {
            return Err(SleepValidationError::NoSleep);
        }

        // Calculate time in bed
        let time_in_bed = total_sleep + awake_time;

        // Calculate sleep efficiency
        let sleep_efficiency = (total_sleep as f64 / time_in_bed as f64) * 100.0;

        // Calculate sleep score
        let sleep_score = Self::calculate_sleep_score(
            deep_sleep,
            light_sleep,
            rem_sleep,
            total_sleep,
            sleep_efficiency,
            interruptions.unwrap_or(0),
        );

        Ok(SleepMetrics {
            total_sleep,
            deep_sleep,
            light_sleep,
            rem_sleep,
            awake_time,
            sleep_score: Some(sleep_score),
            sleep_efficiency: Some(sleep_efficiency),
            sleep_onset,
            interruptions,
        })
    }

    /// Calculate sleep score (0-100) based on Garmin methodology
    ///
    /// # Algorithm
    ///
    /// Score components:
    /// 1. Stage Distribution (40 points):
    ///    - Deep sleep: 13-23% optimal
    ///    - Light sleep: 45-55% optimal
    ///    - REM sleep: 20-25% optimal
    ///
    /// 2. Sleep Efficiency (30 points):
    ///    - >85% = 30 points
    ///    - Linear scale below 85%
    ///
    /// 3. Sleep Duration (20 points):
    ///    - 7-9 hours = 20 points
    ///    - Penalty for too short or too long
    ///
    /// 4. Interruptions (10 points):
    ///    - 0-2 interruptions = 10 points
    ///    - Penalty for more interruptions
    pub fn calculate_sleep_score(
        deep_sleep: u16,
        light_sleep: u16,
        rem_sleep: u16,
        total_sleep: u16,
        efficiency: f64,
        interruptions: u8,
    ) -> u8 {
        if total_sleep == 0 {
            return 0;
        }

        let mut score = 0.0;

        // 1. Stage Distribution (40 points)
        let deep_pct = (deep_sleep as f64 / total_sleep as f64) * 100.0;
        let light_pct = (light_sleep as f64 / total_sleep as f64) * 100.0;
        let rem_pct = (rem_sleep as f64 / total_sleep as f64) * 100.0;

        // Deep sleep score (15 points) - optimal 13-23%
        let deep_score = if deep_pct >= 13.0 && deep_pct <= 23.0 {
            15.0
        } else if deep_pct < 13.0 {
            (deep_pct / 13.0) * 15.0
        } else {
            15.0 - ((deep_pct - 23.0) / 10.0 * 5.0).min(10.0)
        };

        // Light sleep score (10 points) - optimal 45-55%
        let light_score = if light_pct >= 45.0 && light_pct <= 55.0 {
            10.0
        } else if light_pct < 45.0 {
            (light_pct / 45.0) * 10.0
        } else {
            10.0 - ((light_pct - 55.0) / 10.0 * 3.0).min(7.0)
        };

        // REM sleep score (15 points) - optimal 20-25%
        let rem_score = if rem_pct >= 20.0 && rem_pct <= 25.0 {
            15.0
        } else if rem_pct < 20.0 {
            (rem_pct / 20.0) * 15.0
        } else {
            15.0 - ((rem_pct - 25.0) / 10.0 * 5.0).min(10.0)
        };

        score += deep_score + light_score + rem_score;

        // 2. Sleep Efficiency (30 points)
        let efficiency_score = if efficiency >= 85.0 {
            30.0
        } else {
            (efficiency / 85.0) * 30.0
        };
        score += efficiency_score;

        // 3. Sleep Duration (20 points) - optimal 7-9 hours (420-540 min)
        let duration_hours = total_sleep as f64 / 60.0;
        let duration_score = if duration_hours >= 7.0 && duration_hours <= 9.0 {
            20.0
        } else if duration_hours < 7.0 {
            (duration_hours / 7.0) * 20.0
        } else {
            20.0 - ((duration_hours - 9.0) * 2.0).min(10.0)
        };
        score += duration_score;

        // 4. Interruptions (10 points) - optimal 0-2
        let interruption_score = if interruptions <= 2 {
            10.0
        } else {
            (10.0 - (interruptions as f64 - 2.0) * 2.0).max(0.0)
        };
        score += interruption_score;

        score.min(100.0).max(0.0) as u8
    }

    /// Get sleep stage percentages
    pub fn stage_percentages(&self) -> (f64, f64, f64) {
        if self.total_sleep == 0 {
            return (0.0, 0.0, 0.0);
        }

        let total = self.total_sleep as f64;
        (
            (self.deep_sleep as f64 / total) * 100.0,
            (self.light_sleep as f64 / total) * 100.0,
            (self.rem_sleep as f64 / total) * 100.0,
        )
    }

    /// Validate sleep stage distribution is reasonable
    pub fn validate_distribution(&self) -> bool {
        let (deep_pct, light_pct, rem_pct) = self.stage_percentages();

        // Deep: 0-40% (outliers exist, but >40% is very unusual)
        // Light: 30-70% (broad range for normal variation)
        // REM: 10-35% (can be low with sleep deprivation)
        deep_pct <= 40.0 && light_pct >= 30.0 && light_pct <= 70.0 && rem_pct >= 10.0 && rem_pct <= 35.0
    }
}

/// Complete sleep session with detailed stage tracking
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SleepSession {
    /// Unique identifier
    pub id: Option<i64>,

    /// Athlete identifier
    pub athlete_id: Option<i64>,

    /// Sleep period start time
    pub start_time: DateTime<Utc>,

    /// Sleep period end time
    pub end_time: DateTime<Utc>,

    /// Aggregated sleep metrics
    pub metrics: SleepMetrics,

    /// Detailed sleep stage segments
    pub sleep_stages: Vec<SleepStageSegment>,

    /// Source device or application
    pub source: Option<String>,

    /// Additional metadata (JSON)
    pub metadata: Option<serde_json::Value>,
}

impl SleepSession {
    /// Create new sleep session from stage segments
    pub fn from_stages(
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        sleep_stages: Vec<SleepStageSegment>,
        sleep_onset: Option<u16>,
    ) -> Result<Self, SleepValidationError> {
        if end_time <= start_time {
            return Err(SleepValidationError::InvalidTimeRange);
        }

        if sleep_stages.is_empty() {
            return Err(SleepValidationError::NoStages);
        }

        // Aggregate stage durations
        let mut deep_sleep = 0u16;
        let mut light_sleep = 0u16;
        let mut rem_sleep = 0u16;
        let mut awake_time = 0u16;
        let mut interruption_count = 0u8;
        let mut last_stage = None;

        for segment in &sleep_stages {
            match segment.stage {
                SleepStage::Deep => deep_sleep += segment.duration_minutes,
                SleepStage::Light => light_sleep += segment.duration_minutes,
                SleepStage::REM => rem_sleep += segment.duration_minutes,
                SleepStage::Awake => {
                    awake_time += segment.duration_minutes;
                    // Count transitions to awake as interruptions
                    if let Some(last) = last_stage {
                        if last != SleepStage::Awake {
                            interruption_count = interruption_count.saturating_add(1);
                        }
                    }
                }
            }
            last_stage = Some(segment.stage);
        }

        let metrics = SleepMetrics::new(
            deep_sleep,
            light_sleep,
            rem_sleep,
            awake_time,
            sleep_onset,
            Some(interruption_count),
        )?;

        Ok(SleepSession {
            id: None,
            athlete_id: None,
            start_time,
            end_time,
            metrics,
            sleep_stages,
            source: None,
            metadata: None,
        })
    }

    /// Get total time in bed (minutes)
    pub fn time_in_bed(&self) -> u16 {
        ((self.end_time - self.start_time).num_minutes() as u16).min(u16::MAX)
    }
}

/// Sleep validation errors
#[derive(Debug, Clone, PartialEq)]
pub enum SleepValidationError {
    /// Invalid time range (end before start)
    InvalidTimeRange,
    /// No sleep stages provided
    NoStages,
    /// Total sleep time is zero
    NoSleep,
    /// Invalid sleep duration
    InvalidDuration(u16),
}

impl fmt::Display for SleepValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SleepValidationError::InvalidTimeRange => {
                write!(f, "Invalid time range: end time must be after start time")
            }
            SleepValidationError::NoStages => {
                write!(f, "No sleep stages provided")
            }
            SleepValidationError::NoSleep => {
                write!(f, "Total sleep time cannot be zero")
            }
            SleepValidationError::InvalidDuration(d) => {
                write!(f, "Invalid sleep duration: {} minutes", d)
            }
        }
    }
}

impl std::error::Error for SleepValidationError {}

//
// ============================================================================
// BODY BATTERY & PHYSIOLOGICAL MONITORING
// ============================================================================
//

/// Body Battery energy tracking data
///
/// # Sports Science Background
///
/// Body Battery is Garmin's proprietary energy monitoring system that estimates
/// your body's energy reserves on a scale of 0-100. It combines:
///
/// - **Stress**: Drains battery during physical/mental stress
/// - **Rest**: Charges battery during relaxation and sleep
/// - **Activity**: Drains battery based on intensity and duration
/// - **Sleep Quality**: Primary charging period, quality affects charge rate
///
/// # Energy Dynamics
///
/// - **Drain Rate**: Energy consumed per hour during activity/stress (0-20+ per hour)
/// - **Charge Rate**: Energy restored per hour during rest/sleep (0-30+ per hour)
/// - **Daily Pattern**: Typically drains during day, charges overnight
///
/// # Interpretation
///
/// - 75-100: High energy reserves, ready for intense training
/// - 50-74: Moderate energy, suitable for moderate activity
/// - 25-49: Low energy, consider light activity or rest
/// - 0-24: Very low energy, prioritize recovery
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodyBatteryData {
    /// Battery level at start of period (0-100)
    pub start_level: u8,
    /// Battery level at end of period (0-100)
    pub end_level: u8,
    /// Rate of battery drain per hour during activity
    pub drain_rate: Option<f64>,
    /// Rate of battery charge per hour during rest/sleep
    pub charge_rate: Option<f64>,
    /// Lowest battery level reached in period
    pub lowest_level: Option<u8>,
    /// Highest battery level reached in period
    pub highest_level: Option<u8>,
    /// Timestamp for this measurement
    pub timestamp: DateTime<Utc>,
}

impl BodyBatteryData {
    /// Create new Body Battery data with validation
    ///
    /// # Arguments
    ///
    /// * `start_level` - Starting battery level (0-100)
    /// * `end_level` - Ending battery level (0-100)
    /// * `duration_hours` - Duration of measurement period in hours
    /// * `timestamp` - When the measurement was taken
    ///
    /// # Returns
    ///
    /// Result containing validated Body Battery data or validation error
    pub fn new(
        start_level: u8,
        end_level: u8,
        duration_hours: Option<f64>,
        timestamp: DateTime<Utc>,
    ) -> Result<Self, BodyBatteryValidationError> {
        // Validate battery levels are in valid range (0-100)
        if start_level > 100 {
            return Err(BodyBatteryValidationError::InvalidLevel(start_level));
        }
        if end_level > 100 {
            return Err(BodyBatteryValidationError::InvalidLevel(end_level));
        }

        // Calculate drain or charge rate if duration provided
        let (drain_rate, charge_rate) = if let Some(hours) = duration_hours {
            if hours <= 0.0 {
                return Err(BodyBatteryValidationError::InvalidDuration(hours));
            }

            let change = end_level as f64 - start_level as f64;
            if change < 0.0 {
                // Battery drained
                (Some(-change / hours), None)
            } else if change > 0.0 {
                // Battery charged
                (None, Some(change / hours))
            } else {
                // No change
                (None, None)
            }
        } else {
            (None, None)
        };

        Ok(BodyBatteryData {
            start_level,
            end_level,
            drain_rate,
            charge_rate,
            lowest_level: Some(start_level.min(end_level)),
            highest_level: Some(start_level.max(end_level)),
            timestamp,
        })
    }

    /// Calculate net battery change
    pub fn net_change(&self) -> i16 {
        self.end_level as i16 - self.start_level as i16
    }

    /// Check if battery is draining
    pub fn is_draining(&self) -> bool {
        self.end_level < self.start_level
    }

    /// Check if battery is charging
    pub fn is_charging(&self) -> bool {
        self.end_level > self.start_level
    }

    /// Get energy status interpretation
    pub fn energy_status(&self) -> EnergyStatus {
        match self.end_level {
            75..=100 => EnergyStatus::High,
            50..=74 => EnergyStatus::Moderate,
            25..=49 => EnergyStatus::Low,
            _ => EnergyStatus::VeryLow, // 0-24 and any invalid values
        }
    }
}

/// Energy status categories based on Body Battery level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnergyStatus {
    /// High energy reserves (75-100)
    High,
    /// Moderate energy (50-74)
    Moderate,
    /// Low energy (25-49)
    Low,
    /// Very low energy (0-24)
    VeryLow,
}

impl fmt::Display for EnergyStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnergyStatus::High => write!(f, "High"),
            EnergyStatus::Moderate => write!(f, "Moderate"),
            EnergyStatus::Low => write!(f, "Low"),
            EnergyStatus::VeryLow => write!(f, "Very Low"),
        }
    }
}

/// Physiological monitoring metrics
///
/// # Sports Science Background
///
/// These metrics provide insight into overall health and recovery status:
///
/// - **Resting Heart Rate (RHR)**: Lower RHR typically indicates better cardiovascular fitness.
///   Athletes often have RHR 40-60 bpm. Elevated RHR can indicate fatigue, stress, or illness.
///
/// - **Respiration Rate**: Normal resting respiration is 12-20 breaths/min. Lower rates during
///   sleep indicate better recovery. Elevated rates may signal stress or respiratory issues.
///
/// - **Pulse Oximetry (SpO2)**: Blood oxygen saturation. Normal is 95-100%. Lower values
///   (<90%) may indicate altitude, respiratory issues, or sleep apnea.
///
/// - **Stress Score**: Composite metric (0-100) combining HRV, heart rate, and other factors.
///   Higher scores indicate more stress/sympathetic activity.
///
/// - **Recovery Time**: Estimated hours until full recovery. Based on training load, HRV,
///   and other recovery markers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysiologicalMetrics {
    /// Resting heart rate in beats per minute
    pub resting_hr: Option<u8>,
    /// Respiration rate in breaths per minute
    pub respiration_rate: Option<f64>,
    /// Pulse oximetry (SpO2) percentage
    pub pulse_ox: Option<u8>,
    /// Stress score (0-100, higher = more stress)
    pub stress_score: Option<u8>,
    /// Estimated recovery time in hours
    pub recovery_time: Option<u16>,
    /// Timestamp for this measurement
    pub timestamp: DateTime<Utc>,
}

impl PhysiologicalMetrics {
    /// Create new physiological metrics with validation
    ///
    /// # Arguments
    ///
    /// * `resting_hr` - Resting heart rate in bpm (optional)
    /// * `respiration_rate` - Respiration rate in breaths/min (optional)
    /// * `pulse_ox` - SpO2 percentage (optional)
    /// * `stress_score` - Stress score 0-100 (optional)
    /// * `recovery_time` - Recovery time in hours (optional)
    /// * `timestamp` - When the measurement was taken
    ///
    /// # Returns
    ///
    /// Result containing validated metrics or validation error
    pub fn new(
        resting_hr: Option<u8>,
        respiration_rate: Option<f64>,
        pulse_ox: Option<u8>,
        stress_score: Option<u8>,
        recovery_time: Option<u16>,
        timestamp: DateTime<Utc>,
    ) -> Result<Self, PhysiologicalValidationError> {
        // Validate resting heart rate (30-120 bpm reasonable range)
        if let Some(hr) = resting_hr {
            if !(30..=120).contains(&hr) {
                return Err(PhysiologicalValidationError::InvalidRestingHr(hr));
            }
        }

        // Validate respiration rate (5-40 breaths/min reasonable range)
        if let Some(rr) = respiration_rate {
            if rr < 5.0 || rr > 40.0 {
                return Err(PhysiologicalValidationError::InvalidRespirationRate(rr));
            }
        }

        // Validate pulse oximetry (70-100% reasonable range)
        if let Some(spo2) = pulse_ox {
            if spo2 < 70 || spo2 > 100 {
                return Err(PhysiologicalValidationError::InvalidPulseOx(spo2));
            }
        }

        // Validate stress score (0-100)
        if let Some(stress) = stress_score {
            if stress > 100 {
                return Err(PhysiologicalValidationError::InvalidStressScore(stress));
            }
        }

        Ok(PhysiologicalMetrics {
            resting_hr,
            respiration_rate,
            pulse_ox,
            stress_score,
            recovery_time,
            timestamp,
        })
    }

    /// Get stress level interpretation
    pub fn stress_level(&self) -> Option<StressLevel> {
        self.stress_score.map(|score| match score {
            0..=25 => StressLevel::Low,
            26..=50 => StressLevel::Moderate,
            51..=75 => StressLevel::High,
            _ => StressLevel::VeryHigh, // 76-100 and any invalid values
        })
    }

    /// Check if any metric indicates potential health concern
    pub fn has_health_concerns(&self) -> bool {
        // Elevated resting HR
        if let Some(hr) = self.resting_hr {
            if hr > 100 {
                return true;
            }
        }

        // Low SpO2
        if let Some(spo2) = self.pulse_ox {
            if spo2 < 90 {
                return true;
            }
        }

        // Very high stress
        if let Some(stress) = self.stress_score {
            if stress > 75 {
                return true;
            }
        }

        false
    }
}

/// Stress level categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StressLevel {
    /// Low stress (0-25)
    Low,
    /// Moderate stress (26-50)
    Moderate,
    /// High stress (51-75)
    High,
    /// Very high stress (76-100)
    VeryHigh,
}

impl fmt::Display for StressLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StressLevel::Low => write!(f, "Low"),
            StressLevel::Moderate => write!(f, "Moderate"),
            StressLevel::High => write!(f, "High"),
            StressLevel::VeryHigh => write!(f, "Very High"),
        }
    }
}

/// Body Battery validation errors
#[derive(Debug, Clone, PartialEq)]
pub enum BodyBatteryValidationError {
    /// Battery level outside valid range (0-100)
    InvalidLevel(u8),
    /// Invalid duration value
    InvalidDuration(f64),
}

impl fmt::Display for BodyBatteryValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BodyBatteryValidationError::InvalidLevel(level) => {
                write!(f, "Invalid battery level: {} (must be 0-100)", level)
            }
            BodyBatteryValidationError::InvalidDuration(duration) => {
                write!(f, "Invalid duration: {} (must be > 0)", duration)
            }
        }
    }
}

impl std::error::Error for BodyBatteryValidationError {}

/// Physiological metrics validation errors
#[derive(Debug, Clone, PartialEq)]
pub enum PhysiologicalValidationError {
    /// Resting HR outside valid range (30-120 bpm)
    InvalidRestingHr(u8),
    /// Respiration rate outside valid range (5-40 breaths/min)
    InvalidRespirationRate(f64),
    /// Pulse oximetry outside valid range (70-100%)
    InvalidPulseOx(u8),
    /// Stress score outside valid range (0-100)
    InvalidStressScore(u8),
}

impl fmt::Display for PhysiologicalValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PhysiologicalValidationError::InvalidRestingHr(hr) => {
                write!(f, "Invalid resting HR: {} (must be 30-120 bpm)", hr)
            }
            PhysiologicalValidationError::InvalidRespirationRate(rr) => {
                write!(f, "Invalid respiration rate: {} (must be 5-40 breaths/min)", rr)
            }
            PhysiologicalValidationError::InvalidPulseOx(spo2) => {
                write!(f, "Invalid pulse oximetry: {} (must be 70-100%)", spo2)
            }
            PhysiologicalValidationError::InvalidStressScore(score) => {
                write!(f, "Invalid stress score: {} (must be 0-100)", score)
            }
        }
    }
}

impl std::error::Error for PhysiologicalValidationError {}

//
// ============================================================================
// UNIFIED RECOVERY METRICS
// ============================================================================
//

/// Unified daily recovery metrics aggregating all recovery data
///
/// # Purpose
///
/// RecoveryMetrics provides a comprehensive view of an athlete's recovery status
/// by combining multiple data sources:
///
/// - **HRV Metrics**: Autonomic nervous system balance
/// - **Sleep Data**: Sleep quality and duration
/// - **Body Battery**: Energy reserves
/// - **Physiological Metrics**: Health indicators (HR, respiration, SpO2, stress)
///
/// # Training Readiness
///
/// The training readiness score (0-100) combines multiple recovery indicators:
/// - HRV status and baseline comparison (30%)
/// - Sleep quality and duration (25%)
/// - Body Battery level (25%)
/// - Stress and physiological markers (20%)
///
/// Higher scores indicate better readiness for intense training.
///
/// # Recovery Quality
///
/// Overall recovery quality assessment:
/// - **Excellent** (90-100): Full recovery, ready for hard training
/// - **Good** (70-89): Well recovered, normal training recommended
/// - **Fair** (50-69): Partial recovery, consider lighter training
/// - **Poor** (<50): Inadequate recovery, prioritize rest
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveryMetrics {
    /// Date of these recovery metrics
    pub date: chrono::NaiveDate,
    /// HRV measurements and status
    pub hrv_metrics: Option<HrvMetrics>,
    /// Sleep analysis data
    pub sleep_data: Option<SleepMetrics>,
    /// Body Battery energy tracking
    pub body_battery: Option<BodyBatteryData>,
    /// Physiological monitoring metrics
    pub physiological: Option<PhysiologicalMetrics>,
    /// Composite training readiness score (0-100)
    pub training_readiness: Option<u8>,
    /// Overall recovery quality assessment
    pub recovery_quality: Option<RecoveryQuality>,
}

impl RecoveryMetrics {
    /// Create new recovery metrics for a specific date
    pub fn new(date: chrono::NaiveDate) -> Self {
        RecoveryMetrics {
            date,
            hrv_metrics: None,
            sleep_data: None,
            body_battery: None,
            physiological: None,
            training_readiness: None,
            recovery_quality: None,
        }
    }

    /// Calculate training readiness score from available metrics
    ///
    /// # Algorithm
    ///
    /// Readiness score is calculated as a weighted average of:
    /// - HRV contribution (30%): Based on HRV status and score
    /// - Sleep contribution (25%): Based on sleep score and duration
    /// - Energy contribution (25%): Based on Body Battery level
    /// - Stress contribution (20%): Based on stress and physiological markers
    ///
    /// Missing metrics reduce the total possible score proportionally.
    pub fn calculate_readiness(&mut self) {
        let mut total_score = 0.0;
        let mut total_weight = 0.0;

        // HRV contribution (30% weight)
        if let Some(hrv) = &self.hrv_metrics {
            let hrv_score = match hrv.status {
                Some(HrvStatus::Balanced) => 100.0,
                Some(HrvStatus::Unbalanced) => 65.0,
                Some(HrvStatus::Poor) => 30.0,
                Some(HrvStatus::NoReading) | None => {
                    // Use raw score if available
                    hrv.score.map(|s| s as f64).unwrap_or(50.0)
                }
            };
            total_score += hrv_score * 0.30;
            total_weight += 0.30;
        }

        // Sleep contribution (25% weight)
        if let Some(sleep) = &self.sleep_data {
            if let Some(score) = sleep.sleep_score {
                total_score += score as f64 * 0.25;
                total_weight += 0.25;
            }
        }

        // Energy contribution (25% weight)
        if let Some(battery) = &self.body_battery {
            // Use end level as current energy state
            total_score += battery.end_level as f64 * 0.25;
            total_weight += 0.25;
        }

        // Stress contribution (20% weight) - inverted stress score
        if let Some(phys) = &self.physiological {
            let stress_contribution = if let Some(stress) = phys.stress_score {
                // Invert stress: low stress = high contribution
                (100 - stress) as f64
            } else {
                // Default to neutral if no stress data
                70.0
            };
            total_score += stress_contribution * 0.20;
            total_weight += 0.20;
        }

        // Calculate final readiness score, scaling by available data
        if total_weight > 0.0 {
            // Scale to 0-100 based on available metrics
            let readiness = (total_score / total_weight).min(100.0).max(0.0) as u8;
            self.training_readiness = Some(readiness);
            self.recovery_quality = Some(RecoveryQuality::from_readiness(readiness));
        }
    }

    /// Check if athlete is ready for hard training
    pub fn is_ready_for_hard_training(&self) -> bool {
        self.training_readiness
            .map(|r| r >= 75)
            .unwrap_or(false)
    }

    /// Check if recovery is concerning
    pub fn has_recovery_concerns(&self) -> bool {
        // Check if any metric indicates concern
        let hrv_concern = self.hrv_metrics
            .as_ref()
            .and_then(|h| h.status)
            .map(|s| s == HrvStatus::Poor)
            .unwrap_or(false);

        let sleep_concern = self.sleep_data
            .as_ref()
            .and_then(|s| s.sleep_score)
            .map(|s| s < 50)
            .unwrap_or(false);

        let battery_concern = self.body_battery
            .as_ref()
            .map(|b| b.end_level < 25)
            .unwrap_or(false);

        let phys_concern = self.physiological
            .as_ref()
            .map(|p| p.has_health_concerns())
            .unwrap_or(false);

        hrv_concern || sleep_concern || battery_concern || phys_concern
    }

    /// Get the primary recovery limiting factor
    pub fn limiting_factor(&self) -> Option<String> {
        let mut factors = Vec::new();

        if let Some(hrv) = &self.hrv_metrics {
            if let Some(HrvStatus::Poor) = hrv.status {
                factors.push(("HRV", 10));
            } else if let Some(HrvStatus::Unbalanced) = hrv.status {
                factors.push(("HRV", 5));
            }
        }

        if let Some(sleep) = &self.sleep_data {
            if let Some(score) = sleep.sleep_score {
                if score < 50 {
                    factors.push(("Sleep", 10));
                } else if score < 70 {
                    factors.push(("Sleep", 5));
                }
            }
        }

        if let Some(battery) = &self.body_battery {
            if battery.end_level < 25 {
                factors.push(("Energy", 10));
            } else if battery.end_level < 50 {
                factors.push(("Energy", 5));
            }
        }

        if let Some(phys) = &self.physiological {
            if phys.has_health_concerns() {
                factors.push(("Health", 10));
            }
        }

        // Return highest priority factor
        factors.sort_by_key(|(_, priority)| -priority);
        factors.first().map(|(name, _)| name.to_string())
    }
}

/// Recovery quality assessment categories
///
/// # Interpretation
///
/// - **Excellent**: Full recovery, ready for peak performance training
/// - **Good**: Well recovered, normal training load appropriate
/// - **Fair**: Partial recovery, consider reduced training load
/// - **Poor**: Inadequate recovery, prioritize rest and recovery
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryQuality {
    /// Excellent recovery (90-100)
    Excellent,
    /// Good recovery (70-89)
    Good,
    /// Fair recovery (50-69)
    Fair,
    /// Poor recovery (<50)
    Poor,
}

impl RecoveryQuality {
    /// Determine recovery quality from readiness score
    pub fn from_readiness(readiness: u8) -> Self {
        match readiness {
            90..=100 => RecoveryQuality::Excellent,
            70..=89 => RecoveryQuality::Good,
            50..=69 => RecoveryQuality::Fair,
            _ => RecoveryQuality::Poor,
        }
    }
}

impl fmt::Display for RecoveryQuality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecoveryQuality::Excellent => write!(f, "Excellent"),
            RecoveryQuality::Good => write!(f, "Good"),
            RecoveryQuality::Fair => write!(f, "Fair"),
            RecoveryQuality::Poor => write!(f, "Poor"),
        }
    }
}

/// Multi-day recovery trend analysis
pub struct RecoveryTrend {
    metrics: Vec<RecoveryMetrics>,
}

impl RecoveryTrend {
    /// Create new trend analysis from metrics
    pub fn new(metrics: Vec<RecoveryMetrics>) -> Self {
        RecoveryTrend { metrics }
    }

    /// Calculate average training readiness over period
    pub fn average_readiness(&self) -> Option<f64> {
        let readiness_scores: Vec<u8> = self.metrics
            .iter()
            .filter_map(|m| m.training_readiness)
            .collect();

        if readiness_scores.is_empty() {
            None
        } else {
            let sum: u32 = readiness_scores.iter().map(|&s| s as u32).sum();
            Some(sum as f64 / readiness_scores.len() as f64)
        }
    }

    /// Detect if recovery is trending upward or downward
    pub fn trend_direction(&self) -> Option<TrendDirection> {
        if self.metrics.len() < 3 {
            return None;
        }

        let readiness_scores: Vec<u8> = self.metrics
            .iter()
            .filter_map(|m| m.training_readiness)
            .collect();

        if readiness_scores.len() < 3 {
            return None;
        }

        // Simple linear trend: compare first half to second half
        let mid = readiness_scores.len() / 2;
        let first_half: f64 = readiness_scores[..mid].iter().map(|&s| s as f64).sum::<f64>() / mid as f64;
        let second_half: f64 = readiness_scores[mid..].iter().map(|&s| s as f64).sum::<f64>() / (readiness_scores.len() - mid) as f64;

        let diff = second_half - first_half;

        if diff > 5.0 {
            Some(TrendDirection::Improving)
        } else if diff < -5.0 {
            Some(TrendDirection::Declining)
        } else {
            Some(TrendDirection::Stable)
        }
    }

    /// Check if athlete is showing signs of overtraining
    pub fn overtraining_risk(&self) -> bool {
        // Check for sustained low readiness
        let recent_readiness: Vec<u8> = self.metrics
            .iter()
            .rev()
            .take(5)
            .filter_map(|m| m.training_readiness)
            .collect();

        if recent_readiness.len() < 3 {
            return false;
        }

        // Risk if 3+ days of low readiness
        recent_readiness.iter().filter(|&&r| r < 60).count() >= 3
    }
}

/// Recovery trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    /// Recovery improving over time
    Improving,
    /// Recovery stable
    Stable,
    /// Recovery declining over time
    Declining,
}

impl fmt::Display for TrendDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrendDirection::Improving => write!(f, "Improving"),
            TrendDirection::Stable => write!(f, "Stable"),
            TrendDirection::Declining => write!(f, "Declining"),
        }
    }
}

// ============================================================================
// HRV Baseline Calculation & Trend Analysis Algorithms
// ============================================================================

/// Calculate HRV baseline from a series of measurements using 7-day rolling average
/// with outlier filtering
///
/// # Algorithm
///
/// 1. Sort measurements by timestamp (most recent last)
/// 2. Filter outliers using Modified Z-score (> 3.5)
/// 3. Calculate 7-day rolling average from filtered values
/// 4. Return None if insufficient data (<3 measurements)
///
/// # Arguments
///
/// * `measurements` - Slice of HRV measurements (should be chronologically ordered)
///
/// # Returns
///
/// Optional baseline value in milliseconds, or None if insufficient data
///
/// # Sports Science Context
///
/// A 7-day baseline balances responsiveness to training adaptations with
/// stability against daily fluctuations. Outlier filtering prevents single
/// anomalous measurements from skewing the baseline.
///
/// # Performance
///
/// Time complexity: O(n log n) for sorting + O(n) for filtering and averaging
/// Target: <10ms for 30-day dataset
pub fn calculate_hrv_baseline(measurements: &[HrvMeasurement]) -> Option<f64> {
    if measurements.is_empty() {
        return None;
    }

    // Extract RMSSD values
    let mut values: Vec<(DateTime<Utc>, f64)> = measurements
        .iter()
        .map(|m| (m.timestamp, m.rmssd))
        .collect();

    // Sort by timestamp (oldest first)
    values.sort_by_key(|(timestamp, _)| *timestamp);

    // Take last 7 days of measurements
    let recent_values: Vec<f64> = values
        .iter()
        .rev()
        .take(7)
        .map(|(_, rmssd)| *rmssd)
        .collect();

    if recent_values.len() < 3 {
        return None; // Insufficient data for reliable baseline
    }

    // Filter outliers using Modified Z-score method
    let filtered_values = filter_outliers(&recent_values);

    if filtered_values.len() < 3 {
        return None; // Too many outliers removed
    }

    // Calculate mean of filtered values
    let sum: f64 = filtered_values.iter().sum();
    let baseline = sum / filtered_values.len() as f64;

    Some(baseline)
}

/// Filter outliers from HRV measurements using Modified Z-score
///
/// # Algorithm
///
/// Modified Z-score = 0.6745 * (x - median) / MAD
/// where MAD = median absolute deviation
///
/// Values with |Modified Z-score| > 3.5 are considered outliers
///
/// # Arguments
///
/// * `values` - Slice of RMSSD values
///
/// # Returns
///
/// Vector of filtered values with outliers removed
fn filter_outliers(values: &[f64]) -> Vec<f64> {
    if values.len() < 3 {
        return values.to_vec();
    }

    // Calculate median
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = sorted[sorted.len() / 2];

    // Calculate MAD (Median Absolute Deviation)
    let mut deviations: Vec<f64> = values
        .iter()
        .map(|&v| (v - median).abs())
        .collect();
    deviations.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mad = deviations[deviations.len() / 2];

    // Filter outliers using Modified Z-score > 3.5
    if mad < 0.001 {
        // All values are very similar, no outliers
        return values.to_vec();
    }

    values
        .iter()
        .filter(|&&v| {
            let modified_z = 0.6745 * (v - median).abs() / mad;
            modified_z <= 3.5
        })
        .copied()
        .collect()
}

/// Overreaching alert information
#[derive(Debug, Clone, PartialEq)]
pub struct OverreachingAlert {
    /// Severity level (0-100, higher = more severe)
    pub severity: u8,
    /// Number of consecutive days with declining HRV
    pub consecutive_days: usize,
    /// Average HRV deviation from baseline (negative = below baseline)
    pub avg_deviation_pct: f64,
    /// Recommended action
    pub recommendation: String,
}

/// Detect overreaching based on HRV trend and training load
///
/// # Algorithm
///
/// Overreaching is detected when:
/// 1. HRV shows declining trend for 3+ consecutive days
/// 2. Average HRV is >10% below baseline (sports science threshold)
/// 3. Training load (ATL) is elevated
///
/// Severity calculation:
/// - Base severity from HRV deviation
/// - Increased by consecutive days of decline
/// - Amplified if training load is high
///
/// # Arguments
///
/// * `hrv_trend` - Recent HRV measurements (chronologically ordered)
/// * `training_load` - Recent training stress scores (aligned with HRV dates)
///
/// # Returns
///
/// Optional alert if overreaching is detected
///
/// # Sports Science Context
///
/// Functional overreaching is a normal part of training, but non-functional
/// overreaching (>7 days) can lead to overtraining syndrome. This algorithm
/// provides early warning to adjust training load.
pub fn detect_overreaching(
    hrv_trend: &[f64],
    training_load: &[f64],
) -> Option<OverreachingAlert> {
    if hrv_trend.len() < 3 {
        return None; // Need at least 3 days to detect pattern
    }

    // Detect consecutive declining days
    let mut max_consecutive_days = 0;
    let mut current_consecutive = 0;
    let mut max_decline_start_idx = 0;
    let mut current_decline_start_idx = 0;
    let mut max_declining_end_values = Vec::new();
    let mut current_declining_end_values = Vec::new();

    for (i, window) in hrv_trend.windows(2).enumerate() {
        if window[1] < window[0] {
            if current_consecutive == 0 {
                current_decline_start_idx = i; // Mark where decline started
            }
            current_consecutive += 1;
            current_declining_end_values.push(window[1]);

            // Update maximum if current streak is longer
            if current_consecutive > max_consecutive_days {
                max_consecutive_days = current_consecutive;
                max_decline_start_idx = current_decline_start_idx;
                max_declining_end_values = current_declining_end_values.clone();
            }
        } else {
            // Reset current streak
            current_consecutive = 0;
            current_declining_end_values.clear();
        }
    }

    // Require 3+ consecutive declining days
    if max_consecutive_days < 3 {
        return None;
    }

    // Calculate baseline from the value BEFORE the decline started
    // This represents the athlete's normal state before overreaching
    let baseline = hrv_trend[max_decline_start_idx];

    // Calculate average of the recent low values during decline
    let avg_declining = max_declining_end_values.iter().sum::<f64>() / max_declining_end_values.len() as f64;
    let avg_deviation_pct = ((avg_declining - baseline) / baseline) * 100.0;

    // Only alert if significantly below baseline (>10%)
    // A 10% drop from baseline with 3+ days decline is significant for overreaching
    if avg_deviation_pct > -10.0 {
        return None;
    }

    // Calculate severity (0-100)
    let mut severity = (avg_deviation_pct.abs() * 1.5).min(70.0) as u8;

    // Increase severity based on consecutive days
    severity = severity.saturating_add((max_consecutive_days as u8).saturating_mul(5).min(20));

    // Amplify if training load is high
    if !training_load.is_empty() {
        let recent_load = training_load.iter().rev().take(3).sum::<f64>() / 3.0;
        if recent_load > 150.0 {
            severity = severity.saturating_add(10);
        }
    }

    let recommendation = match severity {
        0..=30 => "Monitor recovery, consider light training day".to_string(),
        31..=60 => "Reduce training intensity, prioritize recovery".to_string(),
        61..=80 => "Rest day recommended, focus on sleep and nutrition".to_string(),
        _ => "Multiple rest days needed, consult coach if symptoms persist".to_string(),
    };

    Some(OverreachingAlert {
        severity: severity.min(100),
        consecutive_days: max_consecutive_days,
        avg_deviation_pct,
        recommendation,
    })
}

/// Calculate training readiness score from HRV deviation, sleep quality, and TSB
///
/// # Algorithm
///
/// Readiness = (HRV_component * 0.4) + (Sleep_component * 0.3) + (TSB_component * 0.3)
///
/// Where:
/// - HRV_component: Based on deviation from baseline (0-100)
/// - Sleep_component: Direct sleep score (0-100)
/// - TSB_component: Derived from Training Stress Balance
///
/// # Arguments
///
/// * `hrv_deviation` - Percentage deviation from baseline (negative = below baseline)
/// * `sleep_score` - Optional sleep quality score (0-100)
/// * `tsb` - Optional Training Stress Balance (fitness - fatigue)
///
/// # Returns
///
/// Training readiness score (0-100)
///
/// # Sports Science Context
///
/// Training readiness integrates multiple recovery markers:
/// - HRV reflects autonomic nervous system recovery
/// - Sleep quality indicates physical and mental restoration
/// - TSB shows fitness vs fatigue balance
///
/// Scores 75+ indicate readiness for hard training
/// Scores <60 suggest light training or rest
pub fn calculate_training_readiness(
    hrv_deviation: f64,
    sleep_score: Option<u8>,
    tsb: Option<i16>,
) -> u8 {
    // HRV component (40% weight)
    // Deviation: 0% = 100 score, -30% = 40 score, -50% = 0 score
    let hrv_component = if hrv_deviation >= 0.0 {
        100.0
    } else {
        ((hrv_deviation + 50.0) / 50.0 * 100.0).max(0.0).min(100.0)
    };

    // Sleep component (30% weight)
    let sleep_component = sleep_score.unwrap_or(70) as f64; // Default to average if missing

    // TSB component (30% weight)
    // Positive TSB (fresh): higher score
    // Negative TSB (fatigued): lower score
    // Scale: TSB of +30 = 100, TSB of 0 = 60, TSB of -30 = 20
    let tsb_component = if let Some(balance) = tsb {
        let tsb_f64 = balance as f64;
        // Linear scale: y = (x + 30) / 60 * 80 + 20
        ((tsb_f64 + 30.0) / 60.0 * 80.0 + 20.0).max(20.0).min(100.0)
    } else {
        60.0 // Default to neutral if missing
    };

    // Weighted average
    let readiness = (hrv_component * 0.4) + (sleep_component * 0.3) + (tsb_component * 0.3);

    readiness.round() as u8
}

// ========================================================================
// RECOVERY & PMC INTEGRATION - Issue #92
// ========================================================================

/// Priority level for recovery actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryPriority {
    /// Critical: Immediate recovery needed
    Critical,
    /// High: Significant recovery needed
    High,
    /// Moderate: Normal recovery status
    Moderate,
    /// Low: Good recovery status
    Low,
    /// Optimal: Excellent recovery status
    Optimal,
}

impl fmt::Display for RecoveryPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecoveryPriority::Critical => write!(f, "Critical"),
            RecoveryPriority::High => write!(f, "High"),
            RecoveryPriority::Moderate => write!(f, "Moderate"),
            RecoveryPriority::Low => write!(f, "Low"),
            RecoveryPriority::Optimal => write!(f, "Optimal"),
        }
    }
}

/// Risk level assessment for overtraining
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Critical risk: Immediate action required
    Critical,
    /// High risk: Close monitoring needed
    High,
    /// Moderate risk: Monitor closely
    Moderate,
    /// Low risk: Normal training
    Low,
}

impl fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RiskLevel::Critical => write!(f, "Critical"),
            RiskLevel::High => write!(f, "High"),
            RiskLevel::Moderate => write!(f, "Moderate"),
            RiskLevel::Low => write!(f, "Low"),
        }
    }
}

/// Enhanced form calculation incorporating recovery metrics with TSB
///
/// Combines Training Stress Balance with HRV, sleep, and physiological metrics
/// for a comprehensive view of athlete readiness.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnhancedForm {
    /// Overall form score (0-100)
    pub score: u8,
    /// Interpretation of the form score
    pub interpretation: String,
    /// Training recommendation based on form
    pub recommendation: String,
    /// Recovery action priority
    pub recovery_priority: RecoveryPriority,
}

impl EnhancedForm {
    /// Calculate enhanced form combining TSB with recovery metrics
    ///
    /// # Arguments
    /// * `tsb` - Training Stress Balance from PMC (-30 to +30 range typical)
    /// * `hrv_deviation` - HRV deviation from baseline (percentage, -100 to +100)
    /// * `sleep_score` - Sleep quality score (0-100)
    /// * `rhr_current` - Current resting heart rate (bpm)
    /// * `rhr_baseline` - Baseline resting heart rate (bpm)
    ///
    /// # Returns
    /// Enhanced form with score and recommendations
    ///
    /// # Algorithm
    /// Weighted composite of five factors:
    /// - TSB (25%): Training load balance
    /// - HRV (35%): Autonomic nervous system status (dominant factor)
    /// - Sleep (25%): Recovery quality
    /// - RHR (15%): Physiological stress indicator
    ///
    /// # Sports Science Background
    /// HRV is given highest weight (35%) because it's the most reliable
    /// indicator of parasympathetic (recovery) status. Sleep is given 25%
    /// weight as the primary recovery mechanism. TSB provides training
    /// context. RHR elevation indicates residual fatigue.
    pub fn calculate(
        tsb: i16,
        hrv_deviation: f64,
        sleep_score: Option<u8>,
        rhr_current: Option<u8>,
        rhr_baseline: Option<u8>,
    ) -> Self {
        // TSB component (25% weight)
        // Scale: TSB +30 = 100, TSB 0 = 60, TSB -30 = 20
        let tsb_component = {
            let tsb_f64 = tsb as f64;
            ((tsb_f64 + 30.0) / 60.0 * 80.0 + 20.0).max(20.0).min(100.0)
        };

        // HRV component (35% weight - dominant factor)
        // Deviation: 0% = 100, -30% = 40, -60% = 0
        let hrv_component = if hrv_deviation >= 0.0 {
            100.0
        } else {
            ((hrv_deviation + 60.0) / 60.0 * 100.0).max(0.0).min(100.0)
        };

        // Sleep component (25% weight)
        let sleep_component = sleep_score.unwrap_or(70) as f64;

        // RHR component (15% weight)
        let rhr_component = if let (Some(current), Some(baseline)) = (rhr_current, rhr_baseline) {
            let rhr_elevation = ((current as f64 - baseline as f64) / baseline as f64) * 100.0;
            // 0% elevation = 100 score, 10% = 70 score, 20%+ = 30 score
            (100.0 - rhr_elevation * 3.0).max(30.0).min(100.0)
        } else {
            75.0 // Default if RHR data missing
        };

        // Weighted composite score
        let score = (tsb_component * 0.25
            + hrv_component * 0.35
            + sleep_component * 0.25
            + rhr_component * 0.15)
            .round() as u8;

        // Interpretation and recommendations
        let (interpretation, recommendation, priority) = Self::interpret_form(score);

        EnhancedForm {
            score,
            interpretation,
            recommendation,
            recovery_priority: priority,
        }
    }

    /// Interpret form score and generate recommendations
    fn interpret_form(score: u8) -> (String, String, RecoveryPriority) {
        match score {
            0..=19 => (
                "Very poor form - critical fatigue".to_string(),
                "Priority: Complete rest required. Avoid hard training. Focus on sleep and nutrition."
                    .to_string(),
                RecoveryPriority::Critical,
            ),
            20..=39 => (
                "Poor form - significant fatigue".to_string(),
                "Reduce intensity. Recovery-focused workouts only (easy pace, < 50% FTP)."
                    .to_string(),
                RecoveryPriority::High,
            ),
            40..=59 => (
                "Fair form - moderate fatigue".to_string(),
                "Normal training acceptable. Consider reducing volume 20-30% for next 2-3 days."
                    .to_string(),
                RecoveryPriority::Moderate,
            ),
            60..=79 => (
                "Good form - ready to train".to_string(),
                "Good for structured training and moderate intensity work.".to_string(),
                RecoveryPriority::Low,
            ),
            _ => (
                "Excellent form - peak readiness".to_string(),
                "Optimal for hard training, races, and high-intensity intervals."
                    .to_string(),
                RecoveryPriority::Optimal,
            ),
        }
    }
}

/// Recommended training intensity based on recovery status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainingRecommendation {
    /// Complete rest day - critical fatigue
    RestDay,
    /// Light recovery work only
    LightRecovery,
    /// Moderate intensity training
    ModerateTraining,
    /// Hard training and intensity work
    HardTraining,
    /// Peak performance - optimal for racing/competition
    PeakPerformance,
}

impl fmt::Display for TrainingRecommendation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrainingRecommendation::RestDay => write!(f, "Rest Day"),
            TrainingRecommendation::LightRecovery => write!(f, "Light Recovery"),
            TrainingRecommendation::ModerateTraining => write!(f, "Moderate Training"),
            TrainingRecommendation::HardTraining => write!(f, "Hard Training"),
            TrainingRecommendation::PeakPerformance => write!(f, "Peak Performance"),
        }
    }
}

/// Training decision with recommended TSS limits and rationale
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingDecision {
    /// Recommended training level
    pub recommendation: TrainingRecommendation,
    /// Maximum recommended TSS for today
    pub max_tss: u16,
    /// Minimum recommended TSS (avoid zero)
    pub min_tss: u16,
    /// Reasoning behind the recommendation
    pub rationale: String,
    /// Overall risk assessment
    pub risk_level: RiskLevel,
}

impl TrainingDecision {
    /// Make training recommendation based on recovery status
    ///
    /// # Arguments
    /// * `readiness_score` - Current training readiness (0-100)
    /// * `planned_tss` - Planned TSS for the workout
    /// * `acwr` - Acute:Chronic Workload Ratio (ATL/CTL)
    ///
    /// # Returns
    /// Training decision with TSS limits and rationale
    pub fn assess(readiness_score: u8, planned_tss: u16, acwr: f64) -> Self {
        let (recommendation, max_tss, min_tss, rationale, risk_level) =
            match readiness_score {
                0..=29 => (
                    TrainingRecommendation::RestDay,
                    0,
                    0,
                    "Critical fatigue detected. Complete rest recommended.".to_string(),
                    RiskLevel::Critical,
                ),
                30..=49 => (
                    TrainingRecommendation::LightRecovery,
                    (planned_tss as f64 * 0.3) as u16,
                    20,
                    "Significant fatigue. Recovery-focused workouts only.".to_string(),
                    RiskLevel::High,
                ),
                50..=69 => {
                    let max = (planned_tss as f64 * 0.8) as u16;
                    (
                        TrainingRecommendation::ModerateTraining,
                        max,
                        40,
                        "Moderate fatigue. Normal training acceptable with reduced volume."
                            .to_string(),
                        if acwr > 1.3 {
                            RiskLevel::Moderate
                        } else {
                            RiskLevel::Low
                        },
                    )
                }
                70..=84 => (
                    TrainingRecommendation::HardTraining,
                    planned_tss,
                    50,
                    "Good recovery. Ready for structured training and intensity work."
                        .to_string(),
                    RiskLevel::Low,
                ),
                _ => (
                    TrainingRecommendation::PeakPerformance,
                    (planned_tss as f64 * 1.1) as u16,
                    50,
                    "Excellent recovery. Optimal for races and high-intensity sessions."
                        .to_string(),
                    RiskLevel::Low,
                ),
            };

        TrainingDecision {
            recommendation,
            max_tss,
            min_tss,
            rationale,
            risk_level,
        }
    }
}

/// Overtraining risk assessment combining multiple physiological indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OvertrainingRisk {
    /// Overall risk level
    pub risk_level: RiskLevel,
    /// Acute:Chronic Workload Ratio (ATL/CTL)
    pub acwr: f64,
    /// HRV trend direction
    pub hrv_trend: TrendDirection,
    /// Accumulated sleep debt in hours
    pub sleep_debt_hours: f64,
    /// Percentage of RHR measurements elevated above baseline
    pub elevated_rhr_percentage: f64,
    /// Recommended actions
    pub action_items: Vec<String>,
    /// Estimated days until recovery (if high risk)
    pub days_to_recovery: Option<u16>,
}

impl OvertrainingRisk {
    /// Assess overtraining risk from multiple indicators
    ///
    /// # Arguments
    /// * `ctl` - Chronic Training Load (fitness)
    /// * `atl` - Acute Training Load (fatigue)
    /// * `recent_hrv` - HRV measurements (last 7 days)
    /// * `sleep_scores` - Sleep quality scores (last 7 days)
    /// * `rhr_elevations` - RHR deviation percentages (last 7 days)
    ///
    /// # Returns
    /// Comprehensive overtraining risk assessment
    ///
    /// # Risk Factors (in order of importance)
    /// 1. ACWR > 1.5: HIGH RISK
    /// 2. HRV declining + poor sleep: HIGH RISK
    /// 3. Multiple risk factors present: ESCALATE
    /// 4. RHR chronically elevated: MODERATE RISK
    pub fn assess(
        ctl: f64,
        atl: f64,
        recent_hrv: &[f64],
        sleep_scores: &[u8],
        rhr_elevations: &[f64],
    ) -> Self {
        // Calculate ACWR (Acute:Chronic Workload Ratio)
        let acwr = if ctl > 0.0 {
            atl / ctl
        } else {
            0.0
        };

        // Determine HRV trend
        let hrv_trend = if recent_hrv.len() >= 3 {
            let early_hrv = recent_hrv[..recent_hrv.len() / 2].iter().sum::<f64>()
                / (recent_hrv.len() / 2) as f64;
            let recent_hrv_avg =
                recent_hrv[recent_hrv.len() / 2..].iter().sum::<f64>()
                    / (recent_hrv.len() - recent_hrv.len() / 2) as f64;

            if (recent_hrv_avg - early_hrv) / early_hrv * 100.0 > 5.0 {
                TrendDirection::Improving
            } else if (recent_hrv_avg - early_hrv) / early_hrv * 100.0 < -5.0 {
                TrendDirection::Declining
            } else {
                TrendDirection::Stable
            }
        } else {
            TrendDirection::Stable
        };

        // Calculate sleep debt (optimal = 7.5 hours/night)
        let avg_sleep = sleep_scores.iter().map(|&s| s as f64).sum::<f64>() / sleep_scores.len() as f64;
        let sleep_debt_hours = ((75.0 - avg_sleep) / 75.0 * 7.5 * sleep_scores.len() as f64).max(0.0);

        // Calculate percentage of elevated RHR
        let elevated_rhr_count = rhr_elevations.iter().filter(|&&e| e > 5.0).count();
        let elevated_rhr_percentage =
            (elevated_rhr_count as f64 / rhr_elevations.len() as f64) * 100.0;

        // Determine risk level and action items
        let (risk_level, action_items, days_to_recovery) = Self::assess_risk_level(
            acwr,
            hrv_trend,
            sleep_debt_hours,
            elevated_rhr_percentage,
        );

        OvertrainingRisk {
            risk_level,
            acwr,
            hrv_trend,
            sleep_debt_hours,
            elevated_rhr_percentage,
            action_items,
            days_to_recovery,
        }
    }

    /// Assess risk level and generate action items
    fn assess_risk_level(
        acwr: f64,
        hrv_trend: TrendDirection,
        sleep_debt: f64,
        elevated_rhr: f64,
    ) -> (RiskLevel, Vec<String>, Option<u16>) {
        let mut risk_factors = 0;
        let mut action_items = Vec::new();

        // ACWR assessment (most important single metric)
        if acwr > 1.5 {
            risk_factors += 2; // Double weight
            action_items.push(
                "ACWR is high (>1.5). Recent load exceeds chronic fitness level significantly."
                    .to_string(),
            );
        } else if acwr > 1.3 {
            risk_factors += 1;
            action_items.push(
                "ACWR is elevated (>1.3). Monitor closely for overtraining signs.".to_string(),
            );
        }

        // HRV trend
        if hrv_trend == TrendDirection::Declining {
            risk_factors += 1;
            action_items.push("HRV is declining. Indicates inadequate recovery.".to_string());
        }

        // Sleep debt
        if sleep_debt > 5.0 {
            risk_factors += 1;
            action_items.push(format!(
                "Sleep debt of {:.1} hours. Prioritize sleep for recovery.",
                sleep_debt
            ));
        }

        // Elevated RHR
        if elevated_rhr > 60.0 {
            risk_factors += 1;
            action_items
                .push("RHR chronically elevated. Sign of residual fatigue.".to_string());
        }

        // Determine overall risk level
        let (risk_level, days_recovery) = match risk_factors {
            0 | 1 => (RiskLevel::Low, None),
            2 => (RiskLevel::Moderate, Some(2)),
            3 => (RiskLevel::High, Some(4)),
            _ => (RiskLevel::Critical, Some(7)),
        };

        if action_items.is_empty() {
            action_items.push("No overtraining risk detected. Continue normal training.".to_string());
        }

        (risk_level, action_items, days_recovery)
    }
}

/// Recovery forecast with daily predictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyRecovery {
    /// Date of prediction
    pub date: NaiveDate,
    /// Predicted readiness score for that day
    pub predicted_readiness: u8,
    /// Confidence in prediction (0-100%)
    pub confidence: u8,
}

/// Multi-day recovery trajectory forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryForecast {
    /// Estimated hours until full recovery
    pub estimated_recovery_hours: f64,
    /// Predicted date of full recovery
    pub full_recovery_date: NaiveDate,
    /// Daily readiness predictions for next 7 days
    pub daily_recovery_trajectory: Vec<DailyRecovery>,
    /// Confidence in forecast (0-100%)
    pub confidence: u8,
    /// Factors affecting recovery
    pub factors: Vec<String>,
}

impl RecoveryForecast {
    /// Predict recovery trajectory from recent data
    ///
    /// # Arguments
    /// * `current_readiness` - Current readiness score (0-100)
    /// * `current_tss` - Today's TSS load
    /// * `atl` - Current Acute Training Load
    /// * `ctl` - Current Chronic Training Load
    /// * `sleep_quality` - Average sleep quality (0-100)
    /// * `hrv_trend` - HRV trend direction
    ///
    /// # Returns
    /// Recovery forecast with daily predictions
    ///
    /// # Algorithm
    /// Base recovery rate: ~1% readiness per hour with good recovery
    /// Adjustments:
    /// - ATL/CTL ratio >1.3: slower recovery (-20%)
    /// - HRV declining: slower recovery (-15%)
    /// - Sleep poor: slower recovery (-25%)
    /// - High TSS (>200): longer recovery (+20%)
    pub fn predict(
        current_readiness: u8,
        current_tss: u16,
        atl: f64,
        ctl: f64,
        sleep_quality: u8,
        hrv_trend: TrendDirection,
        today: NaiveDate,
    ) -> Self {
        // Base recovery rate: 1% per hour = 24% per day
        let mut recovery_rate = 24.0;

        let mut factors = vec!["Base recovery rate: 24% readiness per day".to_string()];

        // Adjust for ATL/CTL ratio
        let acwr = if ctl > 0.0 { atl / ctl } else { 0.0 };
        if acwr > 1.3 {
            recovery_rate *= 0.8; // 20% slower
            factors.push("High ACWR reduces recovery rate".to_string());
        }

        // Adjust for HRV trend
        if hrv_trend == TrendDirection::Declining {
            recovery_rate *= 0.85; // 15% slower
            factors.push("Declining HRV slows recovery".to_string());
        }

        // Adjust for sleep quality
        if sleep_quality < 50 {
            recovery_rate *= 0.75; // 25% slower
            factors.push("Poor sleep impairs recovery".to_string());
        }

        // Adjust for TSS load
        if current_tss > 200 {
            recovery_rate *= 0.8; // 20% slower for high load
            factors.push("High TSS requires longer recovery".to_string());
        }

        // Calculate recovery hours
        let readiness_needed = 100.0 - current_readiness as f64;
        let estimated_recovery_hours = readiness_needed / (recovery_rate / 24.0);

        // Full recovery date
        let recovery_days = (estimated_recovery_hours / 24.0).ceil() as u64;
        let full_recovery_date =
            today + chrono::Duration::days(recovery_days as i64);

        // Generate daily predictions
        let mut daily_trajectory = Vec::new();
        let mut predicted_readiness = current_readiness as f64;
        let daily_improvement = recovery_rate / 24.0;

        for day in 0..7 {
            let date = today + chrono::Duration::days(day as i64);
            predicted_readiness = (predicted_readiness + daily_improvement).min(100.0);

            // Confidence decreases with forecast length
            let confidence = (100 - day * 10).max(30) as u8;

            daily_trajectory.push(DailyRecovery {
                date,
                predicted_readiness: predicted_readiness as u8,
                confidence,
            });
        }

        // Overall forecast confidence (higher for near-term, lower for poor recovery conditions)
        let base_confidence = 75u8;
        let confidence = if acwr > 1.3 {
            base_confidence.saturating_sub(20)
        } else if sleep_quality < 50 {
            base_confidence.saturating_sub(15)
        } else {
            base_confidence
        };

        RecoveryForecast {
            estimated_recovery_hours,
            full_recovery_date,
            daily_recovery_trajectory: daily_trajectory,
            confidence,
            factors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hrv_status_from_rmssd() {
        // Balanced: within ±15% of baseline
        assert_eq!(HrvStatus::from_rmssd(50.0, 50.0), HrvStatus::Balanced);
        assert_eq!(HrvStatus::from_rmssd(48.0, 50.0), HrvStatus::Balanced);
        assert_eq!(HrvStatus::from_rmssd(52.0, 50.0), HrvStatus::Balanced);

        // Unbalanced: 15-30% below baseline
        assert_eq!(HrvStatus::from_rmssd(40.0, 50.0), HrvStatus::Unbalanced);
        assert_eq!(HrvStatus::from_rmssd(37.0, 50.0), HrvStatus::Unbalanced);

        // Poor: >30% below baseline
        assert_eq!(HrvStatus::from_rmssd(30.0, 50.0), HrvStatus::Poor);
        assert_eq!(HrvStatus::from_rmssd(20.0, 50.0), HrvStatus::Poor);

        // No reading for invalid baseline
        assert_eq!(HrvStatus::from_rmssd(50.0, 0.0), HrvStatus::NoReading);
        assert_eq!(HrvStatus::from_rmssd(50.0, -10.0), HrvStatus::NoReading);
    }

    #[test]
    fn test_hrv_status_display() {
        assert_eq!(format!("{}", HrvStatus::Balanced), "Balanced");
        assert_eq!(format!("{}", HrvStatus::Unbalanced), "Unbalanced");
        assert_eq!(format!("{}", HrvStatus::Poor), "Poor");
        assert_eq!(format!("{}", HrvStatus::NoReading), "No Reading");
    }

    #[test]
    fn test_valid_rmssd_range() {
        assert!(!HrvMetrics::is_valid_rmssd(9.9));  // Too low
        assert!(HrvMetrics::is_valid_rmssd(10.0));  // Min valid
        assert!(HrvMetrics::is_valid_rmssd(50.0));  // Normal
        assert!(HrvMetrics::is_valid_rmssd(200.0)); // Max valid
        assert!(!HrvMetrics::is_valid_rmssd(200.1)); // Too high
    }

    #[test]
    fn test_hrv_metrics_creation() {
        let now = Utc::now();
        let metrics = HrvMetrics::new(
            45.5,
            Some(48.0),
            now,
            Some("morning".to_string()),
        ).unwrap();

        assert_eq!(metrics.rmssd, Some(45.5));
        assert_eq!(metrics.baseline, Some(48.0));
        assert_eq!(metrics.status, Some(HrvStatus::Balanced));
        assert!(metrics.score.is_some());
        assert_eq!(metrics.measurement_time, Some(now));
        assert_eq!(metrics.measurement_context, Some("morning".to_string()));
    }

    #[test]
    fn test_hrv_metrics_invalid_rmssd() {
        let now = Utc::now();
        let result = HrvMetrics::new(5.0, None, now, None);
        assert!(result.is_err());

        if let Err(HrvValidationError::InvalidRmssd(v)) = result {
            assert_eq!(v, 5.0);
        }
    }

    #[test]
    fn test_hrv_metrics_invalid_baseline() {
        let now = Utc::now();
        let result = HrvMetrics::new(45.0, Some(250.0), now, None);
        assert!(result.is_err());

        if let Err(HrvValidationError::InvalidBaseline(v)) = result {
            assert_eq!(v, 250.0);
        }
    }

    #[test]
    fn test_hrv_score_calculation() {
        let now = Utc::now();

        // With baseline - at baseline should be 100
        let metrics = HrvMetrics::new(50.0, Some(50.0), now, None).unwrap();
        assert_eq!(metrics.score, Some(100));

        // Below baseline
        let metrics = HrvMetrics::new(40.0, Some(50.0), now, None).unwrap();
        assert_eq!(metrics.score, Some(80));

        // Without baseline - absolute scale
        let metrics = HrvMetrics::new(60.0, None, now, None).unwrap();
        assert!(metrics.score.unwrap() >= 60);
    }

    #[test]
    fn test_baseline_update() {
        let now = Utc::now();
        let mut metrics = HrvMetrics::new(
            45.0,
            Some(50.0),
            now,
            None,
        ).unwrap();

        let old_baseline = metrics.baseline.unwrap();
        metrics.update_baseline(0.2);

        let new_baseline = metrics.baseline.unwrap();
        assert!(new_baseline < old_baseline);
        assert!(new_baseline > 45.0);
    }

    #[test]
    fn test_hrv_measurement_creation() {
        let now = Utc::now();
        let measurement = HrvMeasurement::new(
            now,
            45.5,
            Some(48.0),
            Some("morning".to_string()),
        ).unwrap();

        assert_eq!(measurement.timestamp, now);
        assert_eq!(measurement.rmssd, 45.5);
        assert_eq!(measurement.baseline, Some(48.0));
        assert_eq!(measurement.status, HrvStatus::Balanced);
        assert!(measurement.score > 0 && measurement.score <= 100);
        assert_eq!(measurement.context, Some("morning".to_string()));
    }

    #[test]
    fn test_hrv_measurement_serialization() {
        let now = Utc::now();
        let measurement = HrvMeasurement::new(
            now,
            45.5,
            Some(48.0),
            Some("sleep".to_string()),
        ).unwrap();

        // Test JSON serialization
        let json = serde_json::to_string(&measurement).unwrap();
        let deserialized: HrvMeasurement = serde_json::from_str(&json).unwrap();

        assert_eq!(measurement, deserialized);
    }

    #[test]
    fn test_hrv_status_transitions() {
        let baseline = 50.0;

        // Test all status transitions
        let values_and_expected = vec![
            (50.0, HrvStatus::Balanced),    // At baseline
            (42.5, HrvStatus::Balanced),    // -15% (boundary)
            (42.4, HrvStatus::Unbalanced),  // Just below -15%
            (35.0, HrvStatus::Unbalanced),  // -30% (boundary)
            (34.9, HrvStatus::Poor),        // Just below -30%
            (20.0, HrvStatus::Poor),        // Well below baseline
        ];

        for (rmssd, expected_status) in values_and_expected {
            assert_eq!(
                HrvStatus::from_rmssd(rmssd, baseline),
                expected_status,
                "RMSSD {} should be {:?}",
                rmssd,
                expected_status
            );
        }
    }

    // ========================================================================
    // SLEEP TESTS
    // ========================================================================

    #[test]
    fn test_sleep_stage_display() {
        assert_eq!(format!("{}", SleepStage::Deep), "Deep");
        assert_eq!(format!("{}", SleepStage::Light), "Light");
        assert_eq!(format!("{}", SleepStage::REM), "REM");
        assert_eq!(format!("{}", SleepStage::Awake), "Awake");
    }

    #[test]
    fn test_sleep_metrics_creation() {
        // 7 hours total sleep with good distribution
        let metrics = SleepMetrics::new(
            90,   // 1.5 hours deep (21%)
            240,  // 4 hours light (57%)
            90,   // 1.5 hours REM (21%)
            30,   // 30 min awake
            Some(12), // 12 min to fall asleep
            Some(2),  // 2 interruptions
        ).unwrap();

        assert_eq!(metrics.total_sleep, 420);  // 7 hours
        assert_eq!(metrics.deep_sleep, 90);
        assert_eq!(metrics.light_sleep, 240);
        assert_eq!(metrics.rem_sleep, 90);
        assert_eq!(metrics.awake_time, 30);

        // Sleep efficiency should be high
        let efficiency = metrics.sleep_efficiency.unwrap();
        assert!(efficiency > 90.0 && efficiency < 95.0);

        // Score should be good (>80)
        let score = metrics.sleep_score.unwrap();
        assert!(score >= 80, "Score {} should be >= 80", score);
    }

    #[test]
    fn test_sleep_metrics_no_sleep_error() {
        let result = SleepMetrics::new(0, 0, 0, 30, None, None);
        assert!(result.is_err());

        if let Err(SleepValidationError::NoSleep) = result {
            // Expected
        } else {
            panic!("Expected NoSleep error");
        }
    }

    #[test]
    fn test_sleep_score_perfect() {
        // Perfect sleep: optimal distribution, duration, and efficiency
        let score = SleepMetrics::calculate_sleep_score(
            90,   // 18% deep (within 13-23%)
            250,  // 50% light (within 45-55%)
            110,  // 22% REM (within 20-25%)
            480,  // 8 hours total (within 7-9)
            95.0, // Excellent efficiency (>85%)
            1,    // 1 interruption (0-2)
        );

        assert!(score >= 95, "Perfect sleep score {} should be >= 95", score);
    }

    #[test]
    fn test_sleep_score_poor() {
        // Poor sleep: low deep sleep, short duration, low efficiency
        let score = SleepMetrics::calculate_sleep_score(
            30,   // 10% deep (below 13%)
            180,  // 60% light (above 55%)
            90,   // 30% REM (above 25%)
            300,  // 5 hours (below 7)
            70.0, // Fair efficiency (below 85%)
            6,    // 6 interruptions (above 5)
        );

        // Expected: deep ~11.5 + light ~8.5 + rem ~12.5 + eff ~24.7 + dur ~14.3 + int 2 = ~73
        assert!(score >= 70 && score <= 75, "Poor sleep score {} should be 70-75", score);
    }

    #[test]
    fn test_sleep_efficiency_calculation() {
        let metrics = SleepMetrics::new(
            60,   // 1 hour deep
            180,  // 3 hours light
            60,   // 1 hour REM
            60,   // 1 hour awake
            None,
            None,
        ).unwrap();

        // Total sleep: 300 min, time in bed: 360 min
        // Efficiency: 300/360 = 83.33%
        let efficiency = metrics.sleep_efficiency.unwrap();
        assert!((efficiency - 83.33).abs() < 0.1);
    }

    #[test]
    fn test_sleep_stage_percentages() {
        let metrics = SleepMetrics::new(
            100,  // deep
            200,  // light
            100,  // REM
            0,    // no awake time
            None,
            None,
        ).unwrap();

        let (deep_pct, light_pct, rem_pct) = metrics.stage_percentages();

        assert!((deep_pct - 25.0).abs() < 0.1);   // 100/400
        assert!((light_pct - 50.0).abs() < 0.1);  // 200/400
        assert!((rem_pct - 25.0).abs() < 0.1);    // 100/400
    }

    #[test]
    fn test_sleep_distribution_validation() {
        // Good distribution
        let good_metrics = SleepMetrics::new(
            80,   // 20% deep
            200,  // 50% light
            120,  // 30% REM
            0,
            None,
            None,
        ).unwrap();
        assert!(good_metrics.validate_distribution());

        // Bad distribution - too much deep sleep
        let bad_metrics = SleepMetrics::new(
            200,  // 50% deep (>40%)
            150,  // 37.5% light
            50,   // 12.5% REM
            0,
            None,
            None,
        ).unwrap();
        assert!(!bad_metrics.validate_distribution());
    }

    #[test]
    fn test_sleep_stage_segment() {
        use chrono::Duration;

        let start = Utc::now();
        let end = start + Duration::minutes(90);

        let segment = SleepStageSegment::new(
            SleepStage::Deep,
            start,
            end,
        ).unwrap();

        assert_eq!(segment.stage, SleepStage::Deep);
        assert_eq!(segment.duration_minutes, 90);
        assert_eq!(segment.start_time, start);
        assert_eq!(segment.end_time, end);
    }

    #[test]
    fn test_sleep_stage_segment_invalid_time() {
        let start = Utc::now();
        let end = start; // Same time, should error

        let result = SleepStageSegment::new(
            SleepStage::Light,
            start,
            end,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_sleep_session_from_stages() {
        use chrono::Duration;

        let start = Utc::now();
        let mut segments = Vec::new();
        let mut current_time = start;

        // Create realistic sleep pattern
        // Light sleep 30 min
        let seg1_end = current_time + Duration::minutes(30);
        segments.push(SleepStageSegment::new(
            SleepStage::Light,
            current_time,
            seg1_end,
        ).unwrap());
        current_time = seg1_end;

        // Deep sleep 90 min
        let seg2_end = current_time + Duration::minutes(90);
        segments.push(SleepStageSegment::new(
            SleepStage::Deep,
            current_time,
            seg2_end,
        ).unwrap());
        current_time = seg2_end;

        // Light sleep 60 min
        let seg3_end = current_time + Duration::minutes(60);
        segments.push(SleepStageSegment::new(
            SleepStage::Light,
            current_time,
            seg3_end,
        ).unwrap());
        current_time = seg3_end;

        // REM sleep 90 min
        let seg4_end = current_time + Duration::minutes(90);
        segments.push(SleepStageSegment::new(
            SleepStage::REM,
            current_time,
            seg4_end,
        ).unwrap());
        current_time = seg4_end;

        // Awake 15 min (interruption)
        let seg5_end = current_time + Duration::minutes(15);
        segments.push(SleepStageSegment::new(
            SleepStage::Awake,
            current_time,
            seg5_end,
        ).unwrap());
        current_time = seg5_end;

        // Light sleep 90 min
        let end = current_time + Duration::minutes(90);
        segments.push(SleepStageSegment::new(
            SleepStage::Light,
            current_time,
            end,
        ).unwrap());

        let session = SleepSession::from_stages(
            start,
            end,
            segments,
            Some(10), // 10 min to fall asleep
        ).unwrap();

        assert_eq!(session.start_time, start);
        assert_eq!(session.end_time, end);
        assert_eq!(session.metrics.deep_sleep, 90);
        assert_eq!(session.metrics.light_sleep, 30 + 60 + 90); // 180
        assert_eq!(session.metrics.rem_sleep, 90);
        assert_eq!(session.metrics.awake_time, 15);
        assert_eq!(session.metrics.interruptions, Some(1)); // 1 transition to awake
        assert_eq!(session.sleep_stages.len(), 6);
    }

    #[test]
    fn test_sleep_session_no_stages_error() {
        let start = Utc::now();
        let end = start + chrono::Duration::hours(8);

        let result = SleepSession::from_stages(
            start,
            end,
            Vec::new(), // No stages
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_sleep_session_time_in_bed() {
        use chrono::Duration;

        let start = Utc::now();
        let end = start + Duration::hours(8);

        let segments = vec![
            SleepStageSegment::new(
                SleepStage::Light,
                start,
                end,
            ).unwrap(),
        ];

        let session = SleepSession::from_stages(
            start,
            end,
            segments,
            None,
        ).unwrap();

        assert_eq!(session.time_in_bed(), 480); // 8 hours = 480 minutes
    }

    #[test]
    fn test_sleep_metrics_serialization() {
        let metrics = SleepMetrics::new(
            90,
            240,
            90,
            30,
            Some(15),
            Some(2),
        ).unwrap();

        // Test JSON serialization
        let json = serde_json::to_string(&metrics).unwrap();
        let deserialized: SleepMetrics = serde_json::from_str(&json).unwrap();

        // Compare fields individually due to floating point precision
        assert_eq!(metrics.total_sleep, deserialized.total_sleep);
        assert_eq!(metrics.deep_sleep, deserialized.deep_sleep);
        assert_eq!(metrics.light_sleep, deserialized.light_sleep);
        assert_eq!(metrics.rem_sleep, deserialized.rem_sleep);
        assert_eq!(metrics.awake_time, deserialized.awake_time);
        assert_eq!(metrics.sleep_score, deserialized.sleep_score);
        assert_eq!(metrics.sleep_onset, deserialized.sleep_onset);
        assert_eq!(metrics.interruptions, deserialized.interruptions);

        // Compare float with tolerance
        if let (Some(e1), Some(e2)) = (metrics.sleep_efficiency, deserialized.sleep_efficiency) {
            assert!((e1 - e2).abs() < 0.001, "Efficiency mismatch: {} vs {}", e1, e2);
        }
    }

    #[test]
    fn test_sleep_session_serialization() {
        use chrono::Duration;

        let start = Utc::now();
        let end = start + Duration::hours(7);

        let segments = vec![
            SleepStageSegment::new(
                SleepStage::Deep,
                start,
                start + Duration::minutes(90),
            ).unwrap(),
            SleepStageSegment::new(
                SleepStage::Light,
                start + Duration::minutes(90),
                end,
            ).unwrap(),
        ];

        let session = SleepSession::from_stages(
            start,
            end,
            segments,
            Some(10),
        ).unwrap();

        // Test JSON serialization
        let json = serde_json::to_string(&session).unwrap();
        let deserialized: SleepSession = serde_json::from_str(&json).unwrap();

        assert_eq!(session, deserialized);
    }

    #[test]
    fn test_nap_scenario() {
        // Short nap: 90 minutes, mostly light sleep
        let metrics = SleepMetrics::new(
            10,   // minimal deep
            60,   // mostly light
            20,   // some REM
            0,    // no interruptions
            Some(5),
            Some(0),
        ).unwrap();

        assert_eq!(metrics.total_sleep, 90);
        assert!(metrics.sleep_efficiency.unwrap() == 100.0); // No awake time

        // Score will be lower due to short duration
        let score = metrics.sleep_score.unwrap();
        assert!(score < 80, "Nap score {} should be < 80", score);
    }

    #[test]
    fn test_disrupted_sleep() {
        // Disrupted sleep with multiple awakenings
        let metrics = SleepMetrics::new(
            60,   // 1 hour deep
            180,  // 3 hours light
            60,   // 1 hour REM
            60,   // 1 hour awake
            Some(30), // 30 min to fall asleep
            Some(8),  // 8 interruptions
        ).unwrap();

        // Efficiency should be lower (300/(300+60) = 83.3%)
        let efficiency = metrics.sleep_efficiency.unwrap();
        assert!(efficiency < 85.0);

        // Score reflects disrupted sleep but good stage distribution
        // Expected: deep 15 + light ~8.5 + rem 15 + eff ~29.4 + dur ~14.3 + int 0 = ~82
        let score = metrics.sleep_score.unwrap();
        assert!(score >= 80 && score <= 85, "Disrupted sleep score {} should be 80-85", score);
    }

    // Body Battery tests
    #[test]
    fn test_body_battery_creation() {
        let now = Utc::now();

        // Battery draining during activity
        let battery = BodyBatteryData::new(80, 50, Some(3.0), now).unwrap();
        assert_eq!(battery.start_level, 80);
        assert_eq!(battery.end_level, 50);
        assert_eq!(battery.net_change(), -30);
        assert!(battery.is_draining());
        assert!(!battery.is_charging());
        assert_eq!(battery.drain_rate, Some(10.0)); // 30 points in 3 hours = 10/hour
        assert_eq!(battery.charge_rate, None);
    }

    #[test]
    fn test_body_battery_charging() {
        let now = Utc::now();

        // Battery charging during sleep
        let battery = BodyBatteryData::new(30, 85, Some(8.0), now).unwrap();
        assert_eq!(battery.start_level, 30);
        assert_eq!(battery.end_level, 85);
        assert_eq!(battery.net_change(), 55);
        assert!(!battery.is_draining());
        assert!(battery.is_charging());
        assert_eq!(battery.drain_rate, None);
        assert_eq!(battery.charge_rate, Some(6.875)); // 55 points in 8 hours
    }

    #[test]
    fn test_body_battery_invalid_level() {
        let now = Utc::now();

        // Invalid start level
        let result = BodyBatteryData::new(150, 50, Some(1.0), now);
        assert!(result.is_err());

        if let Err(BodyBatteryValidationError::InvalidLevel(level)) = result {
            assert_eq!(level, 150);
        }
    }

    #[test]
    fn test_body_battery_invalid_duration() {
        let now = Utc::now();

        // Invalid duration (negative)
        let result = BodyBatteryData::new(80, 50, Some(-1.0), now);
        assert!(result.is_err());

        if let Err(BodyBatteryValidationError::InvalidDuration(d)) = result {
            assert_eq!(d, -1.0);
        }
    }

    #[test]
    fn test_body_battery_energy_status() {
        let now = Utc::now();

        // High energy
        let battery = BodyBatteryData::new(90, 90, None, now).unwrap();
        assert_eq!(battery.energy_status(), EnergyStatus::High);

        // Moderate energy
        let battery = BodyBatteryData::new(60, 60, None, now).unwrap();
        assert_eq!(battery.energy_status(), EnergyStatus::Moderate);

        // Low energy
        let battery = BodyBatteryData::new(40, 40, None, now).unwrap();
        assert_eq!(battery.energy_status(), EnergyStatus::Low);

        // Very low energy
        let battery = BodyBatteryData::new(15, 15, None, now).unwrap();
        assert_eq!(battery.energy_status(), EnergyStatus::VeryLow);
    }

    #[test]
    fn test_energy_status_display() {
        assert_eq!(EnergyStatus::High.to_string(), "High");
        assert_eq!(EnergyStatus::Moderate.to_string(), "Moderate");
        assert_eq!(EnergyStatus::Low.to_string(), "Low");
        assert_eq!(EnergyStatus::VeryLow.to_string(), "Very Low");
    }

    // Physiological Metrics tests
    #[test]
    fn test_physiological_metrics_creation() {
        let now = Utc::now();

        let metrics = PhysiologicalMetrics::new(
            Some(55),    // resting HR
            Some(14.5),  // respiration rate
            Some(97),    // pulse ox
            Some(35),    // stress score
            Some(12),    // recovery time
            now,
        ).unwrap();

        assert_eq!(metrics.resting_hr, Some(55));
        assert_eq!(metrics.respiration_rate, Some(14.5));
        assert_eq!(metrics.pulse_ox, Some(97));
        assert_eq!(metrics.stress_score, Some(35));
        assert_eq!(metrics.recovery_time, Some(12));
    }

    #[test]
    fn test_physiological_metrics_invalid_resting_hr() {
        let now = Utc::now();

        // HR too low
        let result = PhysiologicalMetrics::new(Some(25), None, None, None, None, now);
        assert!(result.is_err());

        if let Err(PhysiologicalValidationError::InvalidRestingHr(hr)) = result {
            assert_eq!(hr, 25);
        }

        // HR too high
        let result = PhysiologicalMetrics::new(Some(130), None, None, None, None, now);
        assert!(result.is_err());
    }

    #[test]
    fn test_physiological_metrics_invalid_respiration() {
        let now = Utc::now();

        // Rate too low
        let result = PhysiologicalMetrics::new(None, Some(3.0), None, None, None, now);
        assert!(result.is_err());

        if let Err(PhysiologicalValidationError::InvalidRespirationRate(rr)) = result {
            assert_eq!(rr, 3.0);
        }

        // Rate too high
        let result = PhysiologicalMetrics::new(None, Some(50.0), None, None, None, now);
        assert!(result.is_err());
    }

    #[test]
    fn test_physiological_metrics_invalid_pulse_ox() {
        let now = Utc::now();

        // SpO2 too low
        let result = PhysiologicalMetrics::new(None, None, Some(60), None, None, now);
        assert!(result.is_err());

        if let Err(PhysiologicalValidationError::InvalidPulseOx(spo2)) = result {
            assert_eq!(spo2, 60);
        }
    }

    #[test]
    fn test_physiological_metrics_invalid_stress_score() {
        let now = Utc::now();

        // Stress score too high
        let result = PhysiologicalMetrics::new(None, None, None, Some(150), None, now);
        assert!(result.is_err());

        if let Err(PhysiologicalValidationError::InvalidStressScore(score)) = result {
            assert_eq!(score, 150);
        }
    }

    #[test]
    fn test_stress_level_interpretation() {
        let now = Utc::now();

        // Low stress
        let metrics = PhysiologicalMetrics::new(None, None, None, Some(15), None, now).unwrap();
        assert_eq!(metrics.stress_level(), Some(StressLevel::Low));

        // Moderate stress
        let metrics = PhysiologicalMetrics::new(None, None, None, Some(40), None, now).unwrap();
        assert_eq!(metrics.stress_level(), Some(StressLevel::Moderate));

        // High stress
        let metrics = PhysiologicalMetrics::new(None, None, None, Some(65), None, now).unwrap();
        assert_eq!(metrics.stress_level(), Some(StressLevel::High));

        // Very high stress
        let metrics = PhysiologicalMetrics::new(None, None, None, Some(85), None, now).unwrap();
        assert_eq!(metrics.stress_level(), Some(StressLevel::VeryHigh));
    }

    #[test]
    fn test_stress_level_display() {
        assert_eq!(StressLevel::Low.to_string(), "Low");
        assert_eq!(StressLevel::Moderate.to_string(), "Moderate");
        assert_eq!(StressLevel::High.to_string(), "High");
        assert_eq!(StressLevel::VeryHigh.to_string(), "Very High");
    }

    #[test]
    fn test_health_concerns_detection() {
        let now = Utc::now();

        // Elevated resting HR
        let metrics = PhysiologicalMetrics::new(Some(105), None, None, None, None, now).unwrap();
        assert!(metrics.has_health_concerns());

        // Low SpO2
        let metrics = PhysiologicalMetrics::new(None, None, Some(88), None, None, now).unwrap();
        assert!(metrics.has_health_concerns());

        // Very high stress
        let metrics = PhysiologicalMetrics::new(None, None, None, Some(80), None, now).unwrap();
        assert!(metrics.has_health_concerns());

        // Normal metrics
        let metrics = PhysiologicalMetrics::new(Some(60), Some(15.0), Some(98), Some(30), None, now).unwrap();
        assert!(!metrics.has_health_concerns());
    }

    #[test]
    fn test_body_battery_no_duration() {
        let now = Utc::now();

        // No duration provided - rates should be None
        let battery = BodyBatteryData::new(70, 40, None, now).unwrap();
        assert_eq!(battery.drain_rate, None);
        assert_eq!(battery.charge_rate, None);
        assert_eq!(battery.lowest_level, Some(40));
        assert_eq!(battery.highest_level, Some(70));
    }

    #[test]
    fn test_body_battery_serialization() {
        let now = Utc::now();
        let battery = BodyBatteryData::new(80, 50, Some(3.0), now).unwrap();

        // Test JSON serialization
        let json = serde_json::to_string(&battery).unwrap();
        let deserialized: BodyBatteryData = serde_json::from_str(&json).unwrap();

        assert_eq!(battery.start_level, deserialized.start_level);
        assert_eq!(battery.end_level, deserialized.end_level);
        assert_eq!(battery.drain_rate, deserialized.drain_rate);
    }

    #[test]
    fn test_physiological_metrics_serialization() {
        let now = Utc::now();
        let metrics = PhysiologicalMetrics::new(
            Some(55),
            Some(14.5),
            Some(97),
            Some(35),
            Some(12),
            now,
        ).unwrap();

        // Test JSON serialization
        let json = serde_json::to_string(&metrics).unwrap();
        let deserialized: PhysiologicalMetrics = serde_json::from_str(&json).unwrap();

        assert_eq!(metrics.resting_hr, deserialized.resting_hr);
        assert_eq!(metrics.respiration_rate, deserialized.respiration_rate);
        assert_eq!(metrics.pulse_ox, deserialized.pulse_ox);
        assert_eq!(metrics.stress_score, deserialized.stress_score);
        assert_eq!(metrics.recovery_time, deserialized.recovery_time);
    }

    // Unified Recovery Metrics tests
    #[test]
    fn test_recovery_metrics_creation() {
        use chrono::NaiveDate;

        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let metrics = RecoveryMetrics::new(date);

        assert_eq!(metrics.date, date);
        assert!(metrics.hrv_metrics.is_none());
        assert!(metrics.sleep_data.is_none());
        assert!(metrics.body_battery.is_none());
        assert!(metrics.physiological.is_none());
        assert!(metrics.training_readiness.is_none());
        assert!(metrics.recovery_quality.is_none());
    }

    #[test]
    fn test_readiness_calculation_full_data() {
        use chrono::NaiveDate;

        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let now = Utc::now();

        let mut metrics = RecoveryMetrics::new(date);

        // Excellent HRV
        metrics.hrv_metrics = Some(HrvMetrics {
            rmssd: Some(60.0),
            status: Some(HrvStatus::Balanced),
            baseline: Some(55.0),
            score: Some(100),
            measurement_time: Some(now),
            measurement_context: None,
        });

        // Good sleep
        metrics.sleep_data = Some(SleepMetrics {
            total_sleep: 450,
            deep_sleep: 90,
            light_sleep: 240,
            rem_sleep: 120,
            awake_time: 30,
            sleep_score: Some(85),
            sleep_efficiency: Some(93.75),
            sleep_onset: Some(10),
            interruptions: Some(2),
        });

        // High energy
        metrics.body_battery = Some(BodyBatteryData::new(90, 90, None, now).unwrap());

        // Low stress
        metrics.physiological = Some(PhysiologicalMetrics::new(
            Some(55),
            Some(14.0),
            Some(98),
            Some(20),
            Some(6),
            now,
        ).unwrap());

        metrics.calculate_readiness();

        // Expected: HRV(100*0.3) + Sleep(85*0.25) + Battery(90*0.25) + Stress((100-20)*0.2)
        // = 30 + 21.25 + 22.5 + 16 = 89.75 ≈ 89-90
        assert!(metrics.training_readiness.is_some());
        let readiness = metrics.training_readiness.unwrap();
        assert!(readiness >= 88 && readiness <= 91, "Readiness {} should be 88-91", readiness);
        assert_eq!(metrics.recovery_quality, Some(RecoveryQuality::Good));
    }

    #[test]
    fn test_readiness_calculation_partial_data() {
        use chrono::NaiveDate;

        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let now = Utc::now();

        let mut metrics = RecoveryMetrics::new(date);

        // Only HRV and sleep data
        metrics.hrv_metrics = Some(HrvMetrics {
            rmssd: Some(40.0),
            status: Some(HrvStatus::Unbalanced),
            baseline: Some(50.0),
            score: Some(65),
            measurement_time: Some(now),
            measurement_context: None,
        });

        metrics.sleep_data = Some(SleepMetrics {
            total_sleep: 360,
            deep_sleep: 50,
            light_sleep: 200,
            rem_sleep: 110,
            awake_time: 40,
            sleep_score: Some(70),
            sleep_efficiency: Some(85.0),
            sleep_onset: Some(20),
            interruptions: Some(4),
        });

        metrics.calculate_readiness();

        // Expected: HRV(65*0.3) + Sleep(70*0.25) scaled to full weight
        // = (19.5 + 17.5) / 0.55 ≈ 67
        assert!(metrics.training_readiness.is_some());
        let readiness = metrics.training_readiness.unwrap();
        assert!(readiness >= 65 && readiness <= 69, "Readiness {} should be 65-69", readiness);
        assert_eq!(metrics.recovery_quality, Some(RecoveryQuality::Fair));
    }

    #[test]
    fn test_readiness_poor_recovery() {
        use chrono::NaiveDate;

        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let now = Utc::now();

        let mut metrics = RecoveryMetrics::new(date);

        // Poor HRV
        metrics.hrv_metrics = Some(HrvMetrics {
            rmssd: Some(25.0),
            status: Some(HrvStatus::Poor),
            baseline: Some(50.0),
            score: Some(30),
            measurement_time: Some(now),
            measurement_context: None,
        });

        // Poor sleep
        metrics.sleep_data = Some(SleepMetrics {
            total_sleep: 300,
            deep_sleep: 30,
            light_sleep: 180,
            rem_sleep: 90,
            awake_time: 60,
            sleep_score: Some(45),
            sleep_efficiency: Some(75.0),
            sleep_onset: Some(30),
            interruptions: Some(8),
        });

        // Low energy
        metrics.body_battery = Some(BodyBatteryData::new(20, 20, None, now).unwrap());

        metrics.calculate_readiness();

        // Expected: Poor across all metrics
        assert!(metrics.training_readiness.is_some());
        let readiness = metrics.training_readiness.unwrap();
        assert!(readiness < 50, "Poor recovery readiness {} should be < 50", readiness);
        assert_eq!(metrics.recovery_quality, Some(RecoveryQuality::Poor));
    }

    #[test]
    fn test_is_ready_for_hard_training() {
        use chrono::NaiveDate;

        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let mut metrics = RecoveryMetrics::new(date);

        // No readiness calculated
        assert!(!metrics.is_ready_for_hard_training());

        // Good readiness
        metrics.training_readiness = Some(80);
        assert!(metrics.is_ready_for_hard_training());

        // Border case
        metrics.training_readiness = Some(75);
        assert!(metrics.is_ready_for_hard_training());

        // Not ready
        metrics.training_readiness = Some(74);
        assert!(!metrics.is_ready_for_hard_training());
    }

    #[test]
    fn test_recovery_concerns() {
        use chrono::NaiveDate;

        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let now = Utc::now();

        let mut metrics = RecoveryMetrics::new(date);

        // No concerns initially
        assert!(!metrics.has_recovery_concerns());

        // Add poor HRV
        metrics.hrv_metrics = Some(HrvMetrics {
            rmssd: Some(25.0),
            status: Some(HrvStatus::Poor),
            baseline: Some(50.0),
            score: Some(30),
            measurement_time: Some(now),
            measurement_context: None,
        });
        assert!(metrics.has_recovery_concerns());

        // Reset and try with poor sleep
        metrics = RecoveryMetrics::new(date);
        metrics.sleep_data = Some(SleepMetrics {
            total_sleep: 300,
            deep_sleep: 30,
            light_sleep: 180,
            rem_sleep: 90,
            awake_time: 60,
            sleep_score: Some(40),
            sleep_efficiency: Some(75.0),
            sleep_onset: None,
            interruptions: None,
        });
        assert!(metrics.has_recovery_concerns());
    }

    #[test]
    fn test_limiting_factor() {
        use chrono::NaiveDate;

        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let now = Utc::now();

        let mut metrics = RecoveryMetrics::new(date);

        // No limiting factor
        assert!(metrics.limiting_factor().is_none());

        // Poor HRV is limiting
        metrics.hrv_metrics = Some(HrvMetrics {
            rmssd: Some(25.0),
            status: Some(HrvStatus::Poor),
            baseline: Some(50.0),
            score: Some(30),
            measurement_time: Some(now),
            measurement_context: None,
        });
        assert_eq!(metrics.limiting_factor(), Some("HRV".to_string()));

        // Poor sleep overrides if both present
        metrics.sleep_data = Some(SleepMetrics {
            total_sleep: 300,
            deep_sleep: 30,
            light_sleep: 180,
            rem_sleep: 90,
            awake_time: 60,
            sleep_score: Some(40),
            sleep_efficiency: Some(75.0),
            sleep_onset: None,
            interruptions: None,
        });
        // Both HRV and Sleep are priority 10, should return first one (HRV)
        assert_eq!(metrics.limiting_factor(), Some("HRV".to_string()));
    }

    #[test]
    fn test_recovery_quality_display() {
        assert_eq!(RecoveryQuality::Excellent.to_string(), "Excellent");
        assert_eq!(RecoveryQuality::Good.to_string(), "Good");
        assert_eq!(RecoveryQuality::Fair.to_string(), "Fair");
        assert_eq!(RecoveryQuality::Poor.to_string(), "Poor");
    }

    #[test]
    fn test_recovery_quality_from_readiness() {
        assert_eq!(RecoveryQuality::from_readiness(95), RecoveryQuality::Excellent);
        assert_eq!(RecoveryQuality::from_readiness(90), RecoveryQuality::Excellent);
        assert_eq!(RecoveryQuality::from_readiness(89), RecoveryQuality::Good);
        assert_eq!(RecoveryQuality::from_readiness(70), RecoveryQuality::Good);
        assert_eq!(RecoveryQuality::from_readiness(69), RecoveryQuality::Fair);
        assert_eq!(RecoveryQuality::from_readiness(50), RecoveryQuality::Fair);
        assert_eq!(RecoveryQuality::from_readiness(49), RecoveryQuality::Poor);
        assert_eq!(RecoveryQuality::from_readiness(0), RecoveryQuality::Poor);
    }

    #[test]
    fn test_recovery_trend_average() {
        use chrono::NaiveDate;

        let mut metrics_vec = Vec::new();

        for i in 0..5 {
            let date = NaiveDate::from_ymd_opt(2024, 1, 10 + i).unwrap();
            let mut metrics = RecoveryMetrics::new(date);
            metrics.training_readiness = Some(70 + i as u8 * 5); // 70, 75, 80, 85, 90
            metrics_vec.push(metrics);
        }

        let trend = RecoveryTrend::new(metrics_vec);
        let avg = trend.average_readiness().unwrap();

        // Average of 70, 75, 80, 85, 90 = 80
        assert!((avg - 80.0).abs() < 0.1);
    }

    #[test]
    fn test_recovery_trend_improving() {
        use chrono::NaiveDate;

        let mut metrics_vec = Vec::new();

        // Improving trend: 50, 55, 60, 75, 85
        let scores = vec![50, 55, 60, 75, 85];
        for (i, score) in scores.iter().enumerate() {
            let date = NaiveDate::from_ymd_opt(2024, 1, 10 + i as u32).unwrap();
            let mut metrics = RecoveryMetrics::new(date);
            metrics.training_readiness = Some(*score);
            metrics_vec.push(metrics);
        }

        let trend = RecoveryTrend::new(metrics_vec);
        assert_eq!(trend.trend_direction(), Some(TrendDirection::Improving));
    }

    #[test]
    fn test_recovery_trend_declining() {
        use chrono::NaiveDate;

        let mut metrics_vec = Vec::new();

        // Declining trend: 85, 80, 75, 60, 50
        let scores = vec![85, 80, 75, 60, 50];
        for (i, score) in scores.iter().enumerate() {
            let date = NaiveDate::from_ymd_opt(2024, 1, 10 + i as u32).unwrap();
            let mut metrics = RecoveryMetrics::new(date);
            metrics.training_readiness = Some(*score);
            metrics_vec.push(metrics);
        }

        let trend = RecoveryTrend::new(metrics_vec);
        assert_eq!(trend.trend_direction(), Some(TrendDirection::Declining));
    }

    #[test]
    fn test_recovery_trend_stable() {
        use chrono::NaiveDate;

        let mut metrics_vec = Vec::new();

        // Stable trend: 75, 78, 74, 77, 76
        let scores = vec![75, 78, 74, 77, 76];
        for (i, score) in scores.iter().enumerate() {
            let date = NaiveDate::from_ymd_opt(2024, 1, 10 + i as u32).unwrap();
            let mut metrics = RecoveryMetrics::new(date);
            metrics.training_readiness = Some(*score);
            metrics_vec.push(metrics);
        }

        let trend = RecoveryTrend::new(metrics_vec);
        assert_eq!(trend.trend_direction(), Some(TrendDirection::Stable));
    }

    #[test]
    fn test_overtraining_risk_detection() {
        use chrono::NaiveDate;

        let mut metrics_vec = Vec::new();

        // 5 days with low readiness (< 60)
        for i in 0..5 {
            let date = NaiveDate::from_ymd_opt(2024, 1, 10 + i).unwrap();
            let mut metrics = RecoveryMetrics::new(date);
            metrics.training_readiness = Some(55);
            metrics_vec.push(metrics);
        }

        let trend = RecoveryTrend::new(metrics_vec);
        assert!(trend.overtraining_risk());

        // Now test with mixed scores (not overtraining)
        let mut metrics_vec = Vec::new();
        let scores = vec![80, 75, 85, 78, 82];
        for (i, score) in scores.iter().enumerate() {
            let date = NaiveDate::from_ymd_opt(2024, 1, 10 + i as u32).unwrap();
            let mut metrics = RecoveryMetrics::new(date);
            metrics.training_readiness = Some(*score);
            metrics_vec.push(metrics);
        }

        let trend = RecoveryTrend::new(metrics_vec);
        assert!(!trend.overtraining_risk());
    }

    #[test]
    fn test_trend_direction_display() {
        assert_eq!(TrendDirection::Improving.to_string(), "Improving");
        assert_eq!(TrendDirection::Stable.to_string(), "Stable");
        assert_eq!(TrendDirection::Declining.to_string(), "Declining");
    }

    #[test]
    fn test_recovery_metrics_serialization() {
        use chrono::NaiveDate;

        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let now = Utc::now();

        let mut metrics = RecoveryMetrics::new(date);
        metrics.hrv_metrics = Some(HrvMetrics {
            rmssd: Some(55.0),
            status: Some(HrvStatus::Balanced),
            baseline: Some(50.0),
            score: Some(95),
            measurement_time: Some(now),
            measurement_context: None,
        });
        metrics.training_readiness = Some(85);
        metrics.recovery_quality = Some(RecoveryQuality::Good);

        // Test JSON serialization
        let json = serde_json::to_string(&metrics).unwrap();
        let deserialized: RecoveryMetrics = serde_json::from_str(&json).unwrap();

        assert_eq!(metrics.date, deserialized.date);
        assert_eq!(metrics.training_readiness, deserialized.training_readiness);
        assert_eq!(metrics.recovery_quality, deserialized.recovery_quality);
    }

    // ========================================================================
    // HRV Baseline Calculation Tests
    // ========================================================================

    #[test]
    fn test_calculate_hrv_baseline_normal() {
        use chrono::Duration;

        let base_time = Utc::now();
        let measurements: Vec<HrvMeasurement> = vec![
            HrvMeasurement::new(base_time - Duration::days(6), 50.0, None, None).unwrap(),
            HrvMeasurement::new(base_time - Duration::days(5), 52.0, None, None).unwrap(),
            HrvMeasurement::new(base_time - Duration::days(4), 48.0, None, None).unwrap(),
            HrvMeasurement::new(base_time - Duration::days(3), 51.0, None, None).unwrap(),
            HrvMeasurement::new(base_time - Duration::days(2), 49.0, None, None).unwrap(),
            HrvMeasurement::new(base_time - Duration::days(1), 50.0, None, None).unwrap(),
            HrvMeasurement::new(base_time, 52.0, None, None).unwrap(),
        ];

        let baseline = calculate_hrv_baseline(&measurements);
        assert!(baseline.is_some());

        let baseline_value = baseline.unwrap();
        assert!(baseline_value >= 48.0 && baseline_value <= 52.0);
    }

    #[test]
    fn test_calculate_hrv_baseline_with_outlier() {
        use chrono::Duration;

        let base_time = Utc::now();
        let measurements: Vec<HrvMeasurement> = vec![
            HrvMeasurement::new(base_time - Duration::days(6), 50.0, None, None).unwrap(),
            HrvMeasurement::new(base_time - Duration::days(5), 52.0, None, None).unwrap(),
            HrvMeasurement::new(base_time - Duration::days(4), 48.0, None, None).unwrap(),
            HrvMeasurement::new(base_time - Duration::days(3), 120.0, None, None).unwrap(), // Outlier!
            HrvMeasurement::new(base_time - Duration::days(2), 49.0, None, None).unwrap(),
            HrvMeasurement::new(base_time - Duration::days(1), 50.0, None, None).unwrap(),
            HrvMeasurement::new(base_time, 51.0, None, None).unwrap(),
        ];

        let baseline = calculate_hrv_baseline(&measurements);
        assert!(baseline.is_some());

        let baseline_value = baseline.unwrap();
        // Outlier should be filtered out, baseline should be around 50
        assert!(baseline_value >= 48.0 && baseline_value <= 52.0);
    }

    #[test]
    fn test_calculate_hrv_baseline_insufficient_data() {
        use chrono::Duration;

        let base_time = Utc::now();

        // Test with no measurements
        let empty_measurements: Vec<HrvMeasurement> = vec![];
        assert!(calculate_hrv_baseline(&empty_measurements).is_none());

        // Test with too few measurements
        let few_measurements = vec![
            HrvMeasurement::new(base_time, 50.0, None, None).unwrap(),
            HrvMeasurement::new(base_time - Duration::days(1), 52.0, None, None).unwrap(),
        ];
        assert!(calculate_hrv_baseline(&few_measurements).is_none());
    }

    #[test]
    fn test_filter_outliers_normal_distribution() {
        let values = vec![48.0, 49.0, 50.0, 51.0, 52.0];
        let filtered = filter_outliers(&values);
        assert_eq!(filtered.len(), 5); // No outliers
    }

    #[test]
    fn test_filter_outliers_with_extreme_values() {
        let values = vec![50.0, 51.0, 52.0, 150.0]; // 150 is extreme outlier
        let filtered = filter_outliers(&values);
        assert!(filtered.len() < values.len());
        assert!(!filtered.contains(&150.0)); // Outlier should be removed
    }

    #[test]
    fn test_filter_outliers_all_similar() {
        let values = vec![50.0, 50.0, 50.0, 50.0];
        let filtered = filter_outliers(&values);
        assert_eq!(filtered.len(), 4); // All values retained
    }

    // ========================================================================
    // Overreaching Detection Tests
    // ========================================================================

    #[test]
    fn test_detect_overreaching_normal_variation() {
        let hrv_trend = vec![50.0, 51.0, 49.0, 52.0, 50.0];
        let training_load = vec![100.0, 95.0, 105.0, 100.0, 98.0];

        let alert = detect_overreaching(&hrv_trend, &training_load);
        assert!(alert.is_none()); // Normal variation, no alert
    }

    #[test]
    fn test_detect_overreaching_declining_trend() {
        let hrv_trend = vec![50.0, 45.0, 40.0, 38.0]; // Consecutive decline
        let training_load = vec![100.0, 110.0, 120.0, 130.0];

        let alert = detect_overreaching(&hrv_trend, &training_load);
        assert!(alert.is_some());

        let alert_data = alert.unwrap();
        assert!(alert_data.consecutive_days >= 3);
        assert!(alert_data.avg_deviation_pct < -10.0);
        assert!(alert_data.severity > 0);
        assert!(!alert_data.recommendation.is_empty());
    }

    #[test]
    fn test_detect_overreaching_minor_decline() {
        let hrv_trend = vec![50.0, 49.0, 48.0, 47.0]; // Declining but not severe
        let training_load = vec![100.0, 105.0, 110.0, 115.0];

        let alert = detect_overreaching(&hrv_trend, &training_load);
        // Should not alert for minor decline (<10% below baseline)
        assert!(alert.is_none());
    }

    #[test]
    fn test_detect_overreaching_insufficient_data() {
        let hrv_trend = vec![50.0, 45.0]; // Too few data points
        let training_load = vec![100.0, 110.0];

        let alert = detect_overreaching(&hrv_trend, &training_load);
        assert!(alert.is_none());
    }

    #[test]
    fn test_detect_overreaching_severity_levels() {
        // Moderate overreaching
        let moderate_trend = vec![50.0, 42.0, 40.0, 38.0]; // ~20% below baseline
        let moderate_load = vec![100.0, 120.0, 130.0, 140.0];
        let moderate_alert = detect_overreaching(&moderate_trend, &moderate_load);

        assert!(moderate_alert.is_some());
        let moderate = moderate_alert.unwrap();
        assert!(moderate.severity >= 30 && moderate.severity <= 60);

        // Severe overreaching
        let severe_trend = vec![50.0, 35.0, 32.0, 30.0, 28.0]; // ~40% below baseline
        let severe_load = vec![100.0, 150.0, 160.0, 170.0, 180.0];
        let severe_alert = detect_overreaching(&severe_trend, &severe_load);

        assert!(severe_alert.is_some());
        let severe = severe_alert.unwrap();
        assert!(severe.severity > 60);
    }

    #[test]
    fn test_detect_overreaching_high_training_load() {
        let hrv_trend = vec![50.0, 42.0, 40.0, 38.0];
        let high_load = vec![180.0, 190.0, 200.0, 210.0]; // Very high load
        let low_load = vec![80.0, 85.0, 90.0, 95.0]; // Low load

        let high_load_alert = detect_overreaching(&hrv_trend, &high_load);
        let low_load_alert = detect_overreaching(&hrv_trend, &low_load);

        assert!(high_load_alert.is_some());
        assert!(low_load_alert.is_some());

        // High training load should increase severity
        assert!(high_load_alert.unwrap().severity > low_load_alert.unwrap().severity);
    }

    // ========================================================================
    // Training Readiness Tests
    // ========================================================================

    #[test]
    fn test_training_readiness_optimal() {
        // Optimal recovery: HRV at baseline, good sleep, positive TSB
        let readiness = calculate_training_readiness(0.0, Some(90), Some(15));
        assert!(readiness >= 88 && readiness <= 95);
    }

    #[test]
    fn test_training_readiness_poor_hrv() {
        // Poor HRV: 30% below baseline
        let readiness = calculate_training_readiness(-30.0, Some(80), Some(10));
        assert!(readiness < 70); // Should indicate reduced readiness
    }

    #[test]
    fn test_training_readiness_poor_sleep() {
        // Good HRV but poor sleep
        let readiness = calculate_training_readiness(0.0, Some(40), Some(10));
        assert!(readiness < 80); // Sleep impacts readiness
    }

    #[test]
    fn test_training_readiness_negative_tsb() {
        // Negative TSB (fatigued)
        let readiness = calculate_training_readiness(0.0, Some(80), Some(-20));
        assert!(readiness < 75); // Fatigue reduces readiness
    }

    #[test]
    fn test_training_readiness_missing_data() {
        // Test with missing sleep score
        let readiness_no_sleep = calculate_training_readiness(0.0, None, Some(10));
        assert!(readiness_no_sleep > 0 && readiness_no_sleep <= 100);

        // Test with missing TSB
        let readiness_no_tsb = calculate_training_readiness(0.0, Some(80), None);
        assert!(readiness_no_tsb > 0 && readiness_no_tsb <= 100);

        // Test with all missing
        let readiness_minimal = calculate_training_readiness(0.0, None, None);
        assert!(readiness_minimal > 0 && readiness_minimal <= 100);
    }

    #[test]
    fn test_training_readiness_component_weights() {
        // Verify HRV has 40% weight (dominant factor)
        let good_hrv = calculate_training_readiness(0.0, Some(50), Some(0));
        let poor_hrv = calculate_training_readiness(-40.0, Some(50), Some(0));

        let hrv_impact = good_hrv as i16 - poor_hrv as i16;
        assert!(hrv_impact > 15); // Significant impact from HRV

        // Verify sleep has 30% weight
        let good_sleep = calculate_training_readiness(0.0, Some(90), Some(0));
        let poor_sleep = calculate_training_readiness(0.0, Some(40), Some(0));

        let sleep_impact = good_sleep as i16 - poor_sleep as i16;
        assert!(sleep_impact > 10); // Moderate impact from sleep
    }

    #[test]
    fn test_training_readiness_extreme_values() {
        // Test extreme HRV deviation
        let extreme_low = calculate_training_readiness(-60.0, Some(80), Some(10));
        assert!(extreme_low < 60);

        let extreme_high = calculate_training_readiness(20.0, Some(80), Some(10));
        assert!(extreme_high >= 85);

        // Test extreme TSB
        let very_fresh = calculate_training_readiness(0.0, Some(80), Some(50));
        let very_fatigued = calculate_training_readiness(0.0, Some(80), Some(-50));
        assert!(very_fresh > very_fatigued);
    }

    #[test]
    fn test_training_readiness_boundary_values() {
        // Test boundary conditions
        let min_readiness = calculate_training_readiness(-100.0, Some(0), Some(-100));
        assert!(min_readiness >= 0);
        assert!(min_readiness < 30);

        let max_readiness = calculate_training_readiness(50.0, Some(100), Some(50));
        assert!(max_readiness <= 100);
        assert!(max_readiness > 90);
    }

    // ========================================================================
    // PMC INTEGRATION TESTS - Issue #92
    // ========================================================================

    #[test]
    fn test_enhanced_form_optimal() {
        // Optimal conditions: good TSB, HRV, sleep, and RHR
        let form = EnhancedForm::calculate(
            10,     // Positive TSB
            5.0,    // Above baseline
            Some(85),    // Good sleep
            Some(60),     // Resting HR
            Some(58),     // Baseline RHR
        );

        assert!(form.score >= 80);
        assert_eq!(form.recovery_priority, RecoveryPriority::Optimal);
        assert!(form.recommendation.contains("hard training"));
    }

    #[test]
    fn test_enhanced_form_poor() {
        // Poor conditions: negative TSB, low HRV, poor sleep, elevated RHR
        let form = EnhancedForm::calculate(
            -25,     // Negative TSB (fatigued)
            -40.0,   // Well below baseline
            Some(30),     // Poor sleep
            Some(75),      // Elevated Resting HR
            Some(58),      // Baseline RHR
        );

        assert!(form.score < 40);
        assert_eq!(form.recovery_priority, RecoveryPriority::High);
        assert!(form.recommendation.contains("Recovery"));
    }

    #[test]
    fn test_enhanced_form_missing_data() {
        // Test with missing RHR data
        let form = EnhancedForm::calculate(
            5,
            -10.0,
            Some(70),
            None,  // No RHR
            None,  // No baseline
        );

        // Should still produce valid score
        assert!(form.score > 0 && form.score <= 100);
    }

    #[test]
    fn test_training_decision_rest_day() {
        let decision = TrainingDecision::assess(20, 150, 1.2);

        assert_eq!(decision.recommendation, TrainingRecommendation::RestDay);
        assert_eq!(decision.risk_level, RiskLevel::Critical);
        assert_eq!(decision.max_tss, 0);
    }

    #[test]
    fn test_training_decision_hard_training() {
        let decision = TrainingDecision::assess(75, 150, 1.0);

        assert_eq!(decision.recommendation, TrainingRecommendation::HardTraining);
        assert_eq!(decision.risk_level, RiskLevel::Low);
        assert_eq!(decision.max_tss, 150);
    }

    #[test]
    fn test_training_decision_with_high_acwr() {
        let decision = TrainingDecision::assess(60, 150, 1.5);

        assert_eq!(decision.recommendation, TrainingRecommendation::ModerateTraining);
        assert_eq!(decision.risk_level, RiskLevel::Moderate);
        assert!(decision.max_tss <= 150);
    }

    #[test]
    fn test_overtraining_risk_low() {
        let hrv_data = vec![50.0, 51.0, 52.0, 51.0, 50.0, 49.0, 50.0];
        let sleep_scores = vec![80, 82, 78, 81, 79, 80, 81];
        let rhr_elevations = vec![2.0, 1.5, 2.5, 1.0, 2.0, 1.5, 2.0];

        let risk = OvertrainingRisk::assess(100.0, 70.0, &hrv_data, &sleep_scores, &rhr_elevations);

        assert_eq!(risk.risk_level, RiskLevel::Low);
        assert!(risk.acwr < 1.3);
    }

    #[test]
    fn test_overtraining_risk_high() {
        let hrv_data = vec![50.0, 48.0, 45.0, 42.0, 40.0, 38.0, 35.0]; // Declining
        let sleep_scores = vec![50, 45, 40, 40, 45, 50, 45]; // Poor
        let rhr_elevations = vec![8.0, 10.0, 12.0, 11.0, 13.0, 14.0, 15.0]; // Elevated

        let risk = OvertrainingRisk::assess(50.0, 110.0, &hrv_data, &sleep_scores, &rhr_elevations);

        assert!(risk.risk_level == RiskLevel::High || risk.risk_level == RiskLevel::Critical);
        assert!(risk.acwr > 1.3);
        assert_eq!(risk.hrv_trend, TrendDirection::Declining);
    }

    #[test]
    fn test_overtraining_risk_acwr_high() {
        // High ACWR with poor recovery signals
        let hrv_data = vec![50.0, 48.0, 46.0, 45.0, 44.0, 43.0, 42.0]; // Declining HRV
        let sleep_scores = vec![50, 50, 50, 50, 50, 50, 50]; // Poor sleep
        let rhr_elevations = vec![8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0]; // Elevated RHR

        let risk = OvertrainingRisk::assess(30.0, 100.0, &hrv_data, &sleep_scores, &rhr_elevations);

        // ACWR = 100/30 = 3.33, very high + multiple other risk factors
        // With declining HRV, poor sleep, and elevated RHR, should be high or critical
        assert!(
            risk.risk_level == RiskLevel::Critical || risk.risk_level == RiskLevel::High,
            "High ACWR with multiple risk factors should indicate high or critical risk"
        );
        assert!(risk.acwr > 1.5);
    }

    #[test]
    fn test_recovery_forecast_optimal_conditions() {
        let today = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let forecast = RecoveryForecast::predict(
            75,  // Good readiness (needs 25 points to recover)
            100, // Moderate TSS
            40.0, // ATL
            60.0, // CTL (good ratio)
            80,  // Good sleep
            TrendDirection::Improving, // Positive trend
            today,
        );

        // Should recover to 100 readiness in reasonable time
        assert!(forecast.estimated_recovery_hours > 0.0);
        assert!(forecast.confidence >= 70);
        assert_eq!(forecast.daily_recovery_trajectory.len(), 7);

        // Readiness should improve or stay same each day
        for i in 1..forecast.daily_recovery_trajectory.len() {
            assert!(
                forecast.daily_recovery_trajectory[i].predicted_readiness
                    >= forecast.daily_recovery_trajectory[i - 1].predicted_readiness
            );
        }
    }

    #[test]
    fn test_recovery_forecast_poor_conditions() {
        let today = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let forecast = RecoveryForecast::predict(
            20,  // Poor readiness
            250, // High TSS
            110.0, // ATL
            50.0, // CTL (poor ratio, ACWR=2.2)
            40,  // Poor sleep
            TrendDirection::Declining, // Negative trend
            today,
        );

        // Should take longer to recover
        assert!(forecast.estimated_recovery_hours > 48.0);
        assert!(forecast.confidence < 70);
        assert!(forecast.factors.iter().any(|f| f.contains("ACWR")));
        assert!(forecast.factors.iter().any(|f| f.contains("sleep")));
    }

    #[test]
    fn test_recovery_priority_display() {
        assert_eq!(format!("{}", RecoveryPriority::Critical), "Critical");
        assert_eq!(format!("{}", RecoveryPriority::Optimal), "Optimal");
    }

    #[test]
    fn test_risk_level_display() {
        assert_eq!(format!("{}", RiskLevel::Critical), "Critical");
        assert_eq!(format!("{}", RiskLevel::Low), "Low");
    }

    #[test]
    fn test_training_recommendation_display() {
        assert_eq!(format!("{}", TrainingRecommendation::RestDay), "Rest Day");
        assert_eq!(
            format!("{}", TrainingRecommendation::PeakPerformance),
            "Peak Performance"
        );
    }

    #[test]
    fn test_enhanced_form_weight_validation() {
        // Verify that improvements in recovery metrics affect score
        let base_form = EnhancedForm::calculate(0, -20.0, Some(70), Some(60), Some(58));
        let better_hrv = EnhancedForm::calculate(0, 10.0, Some(70), Some(60), Some(58));
        let better_sleep = EnhancedForm::calculate(0, -20.0, Some(85), Some(60), Some(58));

        // HRV improvement from -20% to +10% should increase score
        let hrv_improvement = better_hrv.score > base_form.score;

        // Better sleep should improve score
        let sleep_improvement = better_sleep.score > base_form.score;

        assert!(hrv_improvement, "HRV improvement should increase score");
        assert!(sleep_improvement, "Better sleep should improve score");
    }

    #[test]
    fn test_training_decision_light_recovery() {
        let decision = TrainingDecision::assess(40, 150, 1.0);

        assert_eq!(decision.recommendation, TrainingRecommendation::LightRecovery);
        assert_eq!(decision.risk_level, RiskLevel::High);
        assert!(decision.max_tss <= (150.0 * 0.3) as u16 + 1); // Allow 30% of planned
        assert!(decision.min_tss >= 20);
    }

    #[test]
    fn test_overtraining_risk_sleep_debt_calculation() {
        let hrv_data = vec![50.0; 7];
        let sleep_scores = vec![40, 40, 40, 40, 40, 40, 40]; // Consistently poor
        let rhr_elevations = vec![2.0; 7];

        let risk = OvertrainingRisk::assess(100.0, 80.0, &hrv_data, &sleep_scores, &rhr_elevations);

        assert!(risk.sleep_debt_hours > 0.0);
        assert!(risk.action_items.iter().any(|a| a.contains("Sleep debt")));
    }

    #[test]
    fn test_recovery_forecast_accuracy_check() {
        let today = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let forecast = RecoveryForecast::predict(
            75, 100, 50.0, 60.0, 75, TrendDirection::Stable, today,
        );

        // First day should already have better readiness
        assert!(forecast.daily_recovery_trajectory[0].predicted_readiness >= 75);

        // Confidence should decrease over time
        for i in 1..forecast.daily_recovery_trajectory.len() {
            assert!(
                forecast.daily_recovery_trajectory[i].confidence
                    <= forecast.daily_recovery_trajectory[i - 1].confidence
            );
        }
    }
}
