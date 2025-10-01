//! Recovery and HRV (Heart Rate Variability) Metrics
//!
//! This module provides data structures and functionality for tracking athlete recovery
//! through Heart Rate Variability (HRV) measurements.
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
}
