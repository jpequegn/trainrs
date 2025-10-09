/// Configurable sport-specific data validation rules
///
/// This module provides a flexible validation framework for workout data
/// with sport-specific rules, configurable severity levels, and multiple
/// validation actions.

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::models::{DataPoint, Sport, Workout};

/// Validation severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Invalid data, reject import
    Error,
    /// Suspicious data, flag for review
    Warning,
    /// Unusual but acceptable
    Info,
}

/// Action to take when validation fails
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationAction {
    /// Reject entire workout
    Reject,
    /// Clip value to valid range
    Clip,
    /// Import with warning flag
    Flag,
    /// Replace with interpolated value
    Interpolate,
}

/// Field type for validation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Power,
    HeartRate,
    Cadence,
    Speed,
    Pace,
    Elevation,
    PowerBalance,
    StrideLength,
    VerticalOscillation,
    GroundContactTime,
    StrokeRate,
    StrokeCount,
}

impl FieldType {
    /// Get field value from data point
    pub fn get_value(&self, point: &DataPoint) -> Option<f64> {
        match self {
            FieldType::Power => point.power.map(|p| p as f64),
            FieldType::HeartRate => point.heart_rate.map(|h| h as f64),
            FieldType::Cadence => point.cadence.map(|c| c as f64),
            FieldType::Speed => point.speed.and_then(|s| s.to_f64()),
            FieldType::Pace => point.pace.and_then(|p| p.to_f64()),
            FieldType::Elevation => point.elevation.map(|e| e as f64),
            FieldType::PowerBalance => {
                if let (Some(left), Some(right)) = (point.left_power, point.right_power) {
                    let total = left + right;
                    if total > 0 {
                        Some((left as f64 / total as f64) * 100.0)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            FieldType::StrideLength => point.stride_length.and_then(|s| s.to_f64()),
            FieldType::VerticalOscillation => point.vertical_oscillation.map(|v| v as f64 / 10.0), // Convert mm to cm
            FieldType::GroundContactTime => point.ground_contact_time.map(|g| g as f64),
            FieldType::StrokeRate => None, // Not directly stored
            FieldType::StrokeCount => point.stroke_count.map(|s| s as f64),
        }
    }

    /// Set field value on data point
    pub fn set_value(&self, point: &mut DataPoint, value: Option<f64>) {
        match self {
            FieldType::Power => point.power = value.map(|v| v as u16),
            FieldType::HeartRate => point.heart_rate = value.map(|v| v as u16),
            FieldType::Cadence => point.cadence = value.map(|v| v as u16),
            FieldType::Speed => point.speed = value.and_then(|v| Decimal::try_from(v).ok()),
            FieldType::Pace => point.pace = value.and_then(|v| Decimal::try_from(v).ok()),
            FieldType::Elevation => point.elevation = value.map(|v| v as i16),
            FieldType::PowerBalance => {}, // Read-only calculated field
            FieldType::StrideLength => point.stride_length = value.and_then(|v| Decimal::try_from(v).ok()),
            FieldType::VerticalOscillation => point.vertical_oscillation = value.map(|v| (v * 10.0) as u16), // Convert cm to mm
            FieldType::GroundContactTime => point.ground_contact_time = value.map(|v| v as u16),
            FieldType::StrokeRate => {}, // Not directly stored
            FieldType::StrokeCount => point.stroke_count = value.map(|v| v as u16),
        }
    }
}

/// A single validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    /// Rule name/description
    pub name: String,

    /// Sport this rule applies to
    pub sport: Sport,

    /// Field to validate
    pub field: FieldType,

    /// Minimum acceptable value (inclusive)
    pub min: Option<f64>,

    /// Maximum acceptable value (inclusive)
    pub max: Option<f64>,

    /// Severity level
    pub severity: Severity,

    /// Action to take on violation
    pub action: ValidationAction,
}

impl ValidationRule {
    /// Create a new validation rule
    pub fn new(
        name: String,
        sport: Sport,
        field: FieldType,
        min: Option<f64>,
        max: Option<f64>,
        severity: Severity,
        action: ValidationAction,
    ) -> Self {
        Self {
            name,
            sport,
            field,
            min,
            max,
            severity,
            action,
        }
    }

    /// Check if value is valid according to this rule
    pub fn is_valid(&self, value: f64) -> bool {
        if let Some(min) = self.min {
            if value < min {
                return false;
            }
        }
        if let Some(max) = self.max {
            if value > max {
                return false;
            }
        }
        true
    }

    /// Clip value to valid range
    pub fn clip_value(&self, value: f64) -> f64 {
        let mut clipped = value;
        if let Some(min) = self.min {
            if clipped < min {
                clipped = min;
            }
        }
        if let Some(max) = self.max {
            if clipped > max {
                clipped = max;
            }
        }
        clipped
    }
}

/// Validation issue found during validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Rule name that was violated
    pub rule_name: String,

    /// Severity of the issue
    pub severity: Severity,

    /// Field that had the issue
    pub field: String,

    /// Timestamp in the workout
    pub timestamp: u32,

    /// Actual value found
    pub value: f64,

    /// Expected range
    pub expected_range: (Option<f64>, Option<f64>),

    /// Action that was taken
    pub action_taken: ValidationAction,
}

impl ValidationIssue {
    /// Generate a human-readable message for this issue
    pub fn message(&self) -> String {
        let range_str = match self.expected_range {
            (Some(min), Some(max)) => format!("{} - {}", min, max),
            (Some(min), None) => format!(">= {}", min),
            (None, Some(max)) => format!("<= {}", max),
            (None, None) => "unspecified".to_string(),
        };

        format!(
            "{} at t={}s: {} = {} (expected: {}), action: {:?}",
            self.rule_name,
            self.timestamp,
            self.field,
            self.value,
            range_str,
            self.action_taken
        )
    }
}

/// Validation report for a workout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Whether validation passed overall
    pub passed: bool,

    /// Total data points validated
    pub total_points: usize,

    /// Data points that passed validation
    pub passed_points: usize,

    /// Error-level issues
    pub errors: Vec<ValidationIssue>,

    /// Warning-level issues
    pub warnings: Vec<ValidationIssue>,

    /// Info-level issues
    pub info: Vec<ValidationIssue>,
}

impl ValidationReport {
    /// Create a new empty validation report
    pub fn new() -> Self {
        Self {
            passed: true,
            total_points: 0,
            passed_points: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
            info: Vec::new(),
        }
    }

    /// Add an issue to the report
    pub fn add_issue(&mut self, issue: ValidationIssue) {
        match issue.severity {
            Severity::Error => {
                self.errors.push(issue);
                self.passed = false;
            }
            Severity::Warning => self.warnings.push(issue),
            Severity::Info => self.info.push(issue),
        }
    }

    /// Get total issue count
    pub fn total_issues(&self) -> usize {
        self.errors.len() + self.warnings.len() + self.info.len()
    }

    /// Check if report has any issues
    pub fn has_issues(&self) -> bool {
        self.total_issues() > 0
    }

    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_points == 0 {
            return 100.0;
        }
        (self.passed_points as f64 / self.total_points as f64) * 100.0
    }
}

impl Default for ValidationReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Data validator with configurable rules
pub struct DataValidator {
    /// Validation rules by sport
    rules: HashMap<Sport, Vec<ValidationRule>>,
}

impl DataValidator {
    /// Create a new validator with no rules
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }

    /// Create a validator with default sport-specific rules
    pub fn with_defaults() -> Self {
        let mut validator = Self::new();
        validator.add_default_rules();
        validator
    }

    /// Add a validation rule
    pub fn add_rule(&mut self, rule: ValidationRule) {
        self.rules
            .entry(rule.sport.clone())
            .or_insert_with(Vec::new)
            .push(rule);
    }

    /// Add default validation rules for all sports
    fn add_default_rules(&mut self) {
        // Cycling rules
        self.add_cycling_rules();

        // Running rules
        self.add_running_rules();

        // Swimming rules
        self.add_swimming_rules();
    }

    /// Add default cycling validation rules
    fn add_cycling_rules(&mut self) {
        // Power validation
        self.add_rule(ValidationRule::new(
            "Cycling power range".to_string(),
            Sport::Cycling,
            FieldType::Power,
            Some(0.0),
            Some(2000.0),
            Severity::Warning,
            ValidationAction::Clip,
        ));

        // Heart rate validation
        self.add_rule(ValidationRule::new(
            "Cycling heart rate range".to_string(),
            Sport::Cycling,
            FieldType::HeartRate,
            Some(30.0),
            Some(220.0),
            Severity::Error,
            ValidationAction::Flag,
        ));

        // Cadence validation
        self.add_rule(ValidationRule::new(
            "Cycling cadence range".to_string(),
            Sport::Cycling,
            FieldType::Cadence,
            Some(0.0),
            Some(220.0),
            Severity::Warning,
            ValidationAction::Flag,
        ));

        // Speed validation
        self.add_rule(ValidationRule::new(
            "Cycling speed range".to_string(),
            Sport::Cycling,
            FieldType::Speed,
            Some(0.0),
            Some(33.3), // 120 km/h = 33.3 m/s
            Severity::Warning,
            ValidationAction::Clip,
        ));

        // Power balance validation
        self.add_rule(ValidationRule::new(
            "Cycling power balance".to_string(),
            Sport::Cycling,
            FieldType::PowerBalance,
            Some(30.0),
            Some(70.0),
            Severity::Info,
            ValidationAction::Flag,
        ));
    }

    /// Add default running validation rules
    fn add_running_rules(&mut self) {
        // Heart rate validation
        self.add_rule(ValidationRule::new(
            "Running heart rate range".to_string(),
            Sport::Running,
            FieldType::HeartRate,
            Some(30.0),
            Some(220.0),
            Severity::Error,
            ValidationAction::Flag,
        ));

        // Cadence validation
        self.add_rule(ValidationRule::new(
            "Running cadence range".to_string(),
            Sport::Running,
            FieldType::Cadence,
            Some(120.0),
            Some(220.0),
            Severity::Warning,
            ValidationAction::Flag,
        ));

        // Speed validation
        self.add_rule(ValidationRule::new(
            "Running speed range".to_string(),
            Sport::Running,
            FieldType::Speed,
            Some(0.0),
            Some(6.94), // 25 km/h = 6.94 m/s
            Severity::Warning,
            ValidationAction::Clip,
        ));

        // Stride length validation
        self.add_rule(ValidationRule::new(
            "Running stride length".to_string(),
            Sport::Running,
            FieldType::StrideLength,
            Some(0.5),
            Some(2.5),
            Severity::Warning,
            ValidationAction::Flag,
        ));

        // Vertical oscillation validation
        self.add_rule(ValidationRule::new(
            "Running vertical oscillation".to_string(),
            Sport::Running,
            FieldType::VerticalOscillation,
            Some(5.0),
            Some(15.0),
            Severity::Info,
            ValidationAction::Flag,
        ));

        // Ground contact time validation
        self.add_rule(ValidationRule::new(
            "Running ground contact time".to_string(),
            Sport::Running,
            FieldType::GroundContactTime,
            Some(150.0),
            Some(350.0),
            Severity::Info,
            ValidationAction::Flag,
        ));
    }

    /// Add default swimming validation rules
    fn add_swimming_rules(&mut self) {
        // Heart rate validation (lower max due to underwater effects)
        self.add_rule(ValidationRule::new(
            "Swimming heart rate range".to_string(),
            Sport::Swimming,
            FieldType::HeartRate,
            Some(30.0),
            Some(190.0),
            Severity::Error,
            ValidationAction::Flag,
        ));

        // Speed validation
        self.add_rule(ValidationRule::new(
            "Swimming speed range".to_string(),
            Sport::Swimming,
            FieldType::Speed,
            Some(0.0),
            Some(3.0), // 3 m/s elite swimmer
            Severity::Warning,
            ValidationAction::Clip,
        ));

        // Stroke count validation
        self.add_rule(ValidationRule::new(
            "Swimming stroke count".to_string(),
            Sport::Swimming,
            FieldType::StrokeCount,
            Some(5.0),
            Some(50.0),
            Severity::Info,
            ValidationAction::Flag,
        ));
    }

    /// Get reference to all validation rules
    pub fn rules(&self) -> &HashMap<Sport, Vec<ValidationRule>> {
        &self.rules
    }

    /// Validate a complete workout
    pub fn validate_workout(&self, workout: &Workout) -> ValidationReport {
        let mut report = ValidationReport::new();

        if let Some(ref raw_data) = workout.raw_data {
            report.total_points = raw_data.len();

            // Get rules for this sport
            let rules = self.rules.get(&workout.sport).cloned().unwrap_or_default();

            for point in raw_data {
                let mut point_passed = true;

                for rule in &rules {
                    if let Some(value) = rule.field.get_value(point) {
                        if !rule.is_valid(value) {
                            point_passed = false;

                            let issue = ValidationIssue {
                                rule_name: rule.name.clone(),
                                severity: rule.severity,
                                field: format!("{:?}", rule.field),
                                timestamp: point.timestamp,
                                value,
                                expected_range: (rule.min, rule.max),
                                action_taken: rule.action,
                            };

                            report.add_issue(issue);
                        }
                    }
                }

                if point_passed {
                    report.passed_points += 1;
                }
            }
        }

        report
    }

    /// Apply validation rules to a workout, modifying it according to actions
    pub fn apply_rules(&self, workout: &mut Workout) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        if let Some(ref mut raw_data) = workout.raw_data {
            // Get rules for this sport
            let rules = self.rules.get(&workout.sport).cloned().unwrap_or_default();

            for point in raw_data.iter_mut() {
                for rule in &rules {
                    if let Some(value) = rule.field.get_value(point) {
                        if !rule.is_valid(value) {
                            let issue = ValidationIssue {
                                rule_name: rule.name.clone(),
                                severity: rule.severity,
                                field: format!("{:?}", rule.field),
                                timestamp: point.timestamp,
                                value,
                                expected_range: (rule.min, rule.max),
                                action_taken: rule.action,
                            };

                            // Apply action
                            match rule.action {
                                ValidationAction::Clip => {
                                    let clipped = rule.clip_value(value);
                                    rule.field.set_value(point, Some(clipped));
                                }
                                ValidationAction::Interpolate => {
                                    // Remove the value (interpolation done in post-processing)
                                    rule.field.set_value(point, None);
                                }
                                ValidationAction::Flag => {
                                    // Just flag, don't modify
                                }
                                ValidationAction::Reject => {
                                    // Rejection handled at report level
                                }
                            }

                            issues.push(issue);
                        }
                    }
                }
            }
        }

        issues
    }

    /// Load validation rules from TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read validation rules file: {}", path.as_ref().display()))?;

        let config: ValidationConfig = toml::from_str(&content)
            .with_context(|| "Failed to parse TOML validation rules")?;

        let mut validator = Self::new();
        for rule in config.rules {
            validator.add_rule(rule);
        }

        Ok(validator)
    }

    /// Save validation rules to TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut all_rules = Vec::new();
        for rules in self.rules.values() {
            all_rules.extend(rules.clone());
        }

        let config = ValidationConfig { rules: all_rules };

        let toml_content = toml::to_string_pretty(&config)
            .with_context(|| "Failed to serialize validation rules to TOML")?;

        fs::write(&path, toml_content)
            .with_context(|| format!("Failed to write validation rules file: {}", path.as_ref().display()))?;

        Ok(())
    }

    /// Get rules for a specific sport
    pub fn get_rules_for_sport(&self, sport: &Sport) -> Vec<&ValidationRule> {
        self.rules.get(sport).map(|r| r.iter().collect()).unwrap_or_default()
    }
}

impl Default for DataValidator {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Validation configuration for TOML serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ValidationConfig {
    #[serde(rename = "rule")]
    rules: Vec<ValidationRule>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn create_test_data_point(power: Option<u16>, heart_rate: Option<u16>, cadence: Option<u16>) -> DataPoint {
        DataPoint {
            timestamp: 0,
            power,
            heart_rate,
            cadence,
            pace: None,
            speed: Some(dec!(10.0)),
            elevation: Some(100),
            distance: Some(dec!(1000.0)),
            left_power: power.map(|p| p / 2),
            right_power: power.map(|p| p / 2),
            ground_contact_time: None,
            vertical_oscillation: None,
            stride_length: None,
            stroke_count: None,
            stroke_type: None,
            lap_number: Some(1),
            sport_transition: None,
        }
    }

    #[test]
    fn test_validation_rule_is_valid() {
        let rule = ValidationRule::new(
            "Test power".to_string(),
            Sport::Cycling,
            FieldType::Power,
            Some(0.0),
            Some(2000.0),
            Severity::Warning,
            ValidationAction::Clip,
        );

        assert!(rule.is_valid(100.0));
        assert!(rule.is_valid(0.0));
        assert!(rule.is_valid(2000.0));
        assert!(!rule.is_valid(-1.0));
        assert!(!rule.is_valid(2001.0));
    }

    #[test]
    fn test_validation_rule_clip() {
        let rule = ValidationRule::new(
            "Test power".to_string(),
            Sport::Cycling,
            FieldType::Power,
            Some(0.0),
            Some(2000.0),
            Severity::Warning,
            ValidationAction::Clip,
        );

        assert_eq!(rule.clip_value(100.0), 100.0);
        assert_eq!(rule.clip_value(-50.0), 0.0);
        assert_eq!(rule.clip_value(2500.0), 2000.0);
    }

    #[test]
    fn test_field_type_get_value() {
        let point = create_test_data_point(Some(250), Some(150), Some(90));

        assert_eq!(FieldType::Power.get_value(&point), Some(250.0));
        assert_eq!(FieldType::HeartRate.get_value(&point), Some(150.0));
        assert_eq!(FieldType::Cadence.get_value(&point), Some(90.0));
    }

    #[test]
    fn test_validator_with_defaults() {
        let validator = DataValidator::with_defaults();

        // Check that rules exist for main sports
        assert!(!validator.get_rules_for_sport(&Sport::Cycling).is_empty());
        assert!(!validator.get_rules_for_sport(&Sport::Running).is_empty());
        assert!(!validator.get_rules_for_sport(&Sport::Swimming).is_empty());
    }

    #[test]
    fn test_validation_report() {
        let mut report = ValidationReport::new();

        assert!(report.passed);
        assert_eq!(report.total_issues(), 0);

        report.add_issue(ValidationIssue {
            rule_name: "Test".to_string(),
            severity: Severity::Warning,
            field: "Power".to_string(),
            timestamp: 0,
            value: 2500.0,
            expected_range: (Some(0.0), Some(2000.0)),
            action_taken: ValidationAction::Clip,
        });

        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.total_issues(), 1);
        assert!(report.passed); // Warnings don't fail validation

        report.add_issue(ValidationIssue {
            rule_name: "Test".to_string(),
            severity: Severity::Error,
            field: "HeartRate".to_string(),
            timestamp: 0,
            value: 250.0,
            expected_range: (Some(30.0), Some(220.0)),
            action_taken: ValidationAction::Reject,
        });

        assert_eq!(report.errors.len(), 1);
        assert!(!report.passed); // Errors fail validation
    }
}
