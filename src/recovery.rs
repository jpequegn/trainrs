//! Recovery, HRV (Heart Rate Variability), and Sleep Tracking
//!
//! This module provides data structures and functionality for tracking athlete recovery
//! through Heart Rate Variability (HRV) measurements and sleep analysis.
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

use chrono::{DateTime, Utc};
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
}
