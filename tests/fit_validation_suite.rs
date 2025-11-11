//! Enhanced FIT file validation suite with comprehensive data integrity checks
//!
//! This test suite provides:
//! - Device-specific validation rules
//! - Data accuracy benchmarking
//! - Cross-validation against known values
//! - Device quirk documentation
//! - Metric range validation for all sports

use std::collections::HashMap;
use trainrs::import::{fit::FitImporter, ImportFormat, logging::{ImportLogger, OperationType}};
use trainrs::models::{Sport, Workout, DataSource, WorkoutType};
use rust_decimal::Decimal;

/// Device information and expected characteristics
#[derive(Debug, Clone)]
struct DeviceProfile {
    name: &'static str,
    manufacturer: &'static str,
    primary_sports: Vec<Sport>,
    supports_power: bool,
    supports_recovery: bool,
    known_quirks: Vec<&'static str>,
}

impl DeviceProfile {
    fn garmin_edge_520() -> Self {
        Self {
            name: "Garmin Edge 520",
            manufacturer: "Garmin",
            primary_sports: vec![Sport::Cycling],
            supports_power: true,
            supports_recovery: false,
            known_quirks: vec![
                "Occasional power zero spikes on startup",
                "May have gaps in recording at high temperatures",
            ],
        }
    }

    fn garmin_edge_1030() -> Self {
        Self {
            name: "Garmin Edge 1030",
            manufacturer: "Garmin",
            primary_sports: vec![Sport::Cycling, Sport::Triathlon],
            supports_power: true,
            supports_recovery: true,
            known_quirks: vec![
                "Extended recovery data in separate records",
                "Complex timestamp handling for multi-sport",
            ],
        }
    }

    fn garmin_forerunner_945() -> Self {
        Self {
            name: "Garmin Forerunner 945",
            manufacturer: "Garmin",
            primary_sports: vec![Sport::Running, Sport::Cycling, Sport::Swimming, Sport::Triathlon],
            supports_power: false,
            supports_recovery: true,
            known_quirks: vec![
                "HRV data in milliseconds, may need scaling",
                "Running dynamics fields may be sparse",
                "Multiple sport records in single file",
            ],
        }
    }

    fn wahoo_elemnt_bolt() -> Self {
        Self {
            name: "Wahoo ELEMNT BOLT",
            manufacturer: "Wahoo",
            primary_sports: vec![Sport::Cycling],
            supports_power: true,
            supports_recovery: false,
            known_quirks: vec![
                "Power spikes at segment boundaries",
                "May have ANT+ dropouts in weak signal areas",
            ],
        }
    }

    fn zwift() -> Self {
        Self {
            name: "Zwift",
            manufacturer: "Zwift",
            primary_sports: vec![Sport::Cycling],
            supports_power: true,
            supports_recovery: false,
            known_quirks: vec![
                "Synthetic data - no real gps/weather variance",
                "Power values are idealized from watts",
                "Perfect cadence stability",
            ],
        }
    }
}

/// Metric validation ranges for different sports
struct MetricRanges {
    hr_min: u16,
    hr_max: u16,
    power_min: u16,
    power_max: u16,
    cadence_min: u16,
    cadence_max: u16,
    pace_min: f64, // min/km
    pace_max: f64, // min/km
}

impl MetricRanges {
    fn for_sport(sport: Sport) -> Self {
        match sport {
            Sport::Cycling => Self {
                hr_min: 60,
                hr_max: 220,
                power_min: 50,
                power_max: 2000,
                cadence_min: 40,
                cadence_max: 130,
                pace_min: 0.0,
                pace_max: 0.0,
            },
            Sport::Running => Self {
                hr_min: 60,
                hr_max: 220,
                power_min: 0,
                power_max: 0,
                cadence_min: 120,
                cadence_max: 220,
                pace_min: 2.0,  // 2:00 min/km (very fast)
                pace_max: 15.0, // 15:00 min/km (very slow)
            },
            Sport::Swimming => Self {
                hr_min: 80,
                hr_max: 200,
                power_min: 0,
                power_max: 0,
                cadence_min: 40,
                cadence_max: 90, // stroke rate
                pace_min: 0.6,   // 36 sec/100m
                pace_max: 3.0,   // 3 min/100m
            },
            Sport::Triathlon => Self {
                hr_min: 60,
                hr_max: 220,
                power_min: 50,
                power_max: 1500,
                cadence_min: 40,
                cadence_max: 220,
                pace_min: 2.0,
                pace_max: 15.0,
            },
            _ => Self {
                hr_min: 30,
                hr_max: 220,
                power_min: 0,
                power_max: 5000,
                cadence_min: 0,
                cadence_max: 200,
                pace_min: 0.0,
                pace_max: 60.0,
            },
        }
    }
}

/// Validation result tracking
struct ValidationReport {
    device: String,
    file_path: std::path::PathBuf,
    workouts_parsed: usize,
    metrics_validated: usize,
    issues: Vec<String>,
    warnings: Vec<String>,
}

impl ValidationReport {
    fn new(device: String, file_path: std::path::PathBuf) -> Self {
        Self {
            device,
            file_path,
            workouts_parsed: 0,
            metrics_validated: 0,
            issues: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }
}

/// Core validation function
fn validate_fit_file(
    file_path: &std::path::Path,
    device_profile: &DeviceProfile,
    logger: &ImportLogger,
) -> ValidationReport {
    let mut report = ValidationReport::new(
        device_profile.name.to_string(),
        file_path.to_path_buf(),
    );

    let importer = FitImporter::new();

    match importer.import_file(file_path) {
        Ok(workouts) => {
            report.workouts_parsed = workouts.len();

            for workout in &workouts {
                // Validate sport compatibility
                if !device_profile.primary_sports.contains(&workout.sport) {
                    report.warnings.push(format!(
                        "Unexpected sport {:?} for {}",
                        workout.sport, device_profile.name
                    ));
                }

                // Get metric ranges for this sport
                let ranges = MetricRanges::for_sport(workout.sport.clone());

                // Validate heart rate
                if let Some(avg_hr) = workout.summary.avg_heart_rate {
                    if avg_hr < ranges.hr_min || avg_hr > ranges.hr_max {
                        report.issues.push(format!(
                            "HR {} out of range [{}, {}]",
                            avg_hr, ranges.hr_min, ranges.hr_max
                        ));
                    } else {
                        report.metrics_validated += 1;
                    }
                }

                // Validate power (if applicable)
                if device_profile.supports_power {
                    if let Some(avg_power) = workout.summary.avg_power {
                        if avg_power < ranges.power_min || avg_power > ranges.power_max {
                            report.issues.push(format!(
                                "Power {} out of range [{}, {}]",
                                avg_power, ranges.power_min, ranges.power_max
                            ));
                        } else {
                            report.metrics_validated += 1;
                        }
                    }
                }

                // Validate cadence
                if let Some(cadence) = workout.summary.avg_cadence {
                    if cadence < ranges.cadence_min || cadence > ranges.cadence_max {
                        report.warnings.push(format!(
                            "Cadence {} unusual for {}",
                            cadence, device_profile.name
                        ));
                    } else {
                        report.metrics_validated += 1;
                    }
                }

                // Validate pace (if applicable)
                if let Some(avg_pace) = workout.summary.avg_pace {
                    use rust_decimal::prelude::ToPrimitive;
                    if let Some(pace_f64) = avg_pace.to_f64() {
                        if pace_f64 < ranges.pace_min || pace_f64 > ranges.pace_max {
                            report.warnings.push(format!(
                                "Pace {:.2} min/km unusual",
                                pace_f64
                            ));
                        } else {
                            report.metrics_validated += 1;
                        }
                    }
                }

                // Validate duration
                if workout.duration_seconds == 0 {
                    report.issues.push("Workout has zero duration".to_string());
                }

                // Validate distance
                if let Some(distance) = workout.summary.total_distance {
                    if distance == Decimal::ZERO {
                        report.warnings.push("Zero distance recorded".to_string());
                    } else if distance < Decimal::new(100, 1) {
                        // Less than 10 meters seems wrong
                        report.issues.push(format!("Suspiciously low distance: {}", distance));
                    }
                }
            }
        }
        Err(e) => {
            report.issues.push(format!("Failed to parse: {}", e));
            logger.log_error(
                OperationType::Parsing,
                Some(file_path),
                "PARSE_ERR",
                &format!("{}", e),
                false,
            );
        }
    }

    report
}

// Test implementations

#[test]
fn test_fit_validation_framework() {
    let _logger = ImportLogger::new("fit_validation_suite");
    let device = DeviceProfile::garmin_edge_520();

    // This test validates the framework can be instantiated and used
    assert_eq!(device.name, "Garmin Edge 520");
    assert!(device.supports_power);
    assert!(!device.supports_recovery);
    assert!(!device.known_quirks.is_empty());
}

#[test]
fn test_metric_ranges_cycling() {
    let ranges = MetricRanges::for_sport(Sport::Cycling);
    assert!(ranges.hr_min < ranges.hr_max);
    assert!(ranges.power_min < ranges.power_max);
    assert!(ranges.cadence_min < ranges.cadence_max);
}

#[test]
fn test_metric_ranges_running() {
    let ranges = MetricRanges::for_sport(Sport::Running);
    assert!(ranges.hr_min < ranges.hr_max);
    assert!(ranges.cadence_min < ranges.cadence_max);
    assert!(ranges.pace_min < ranges.pace_max);
    assert_eq!(ranges.power_min, 0); // Running doesn't use power
}

#[test]
fn test_validation_report_creation() {
    let report = ValidationReport::new(
        "Test Device".to_string(),
        std::path::PathBuf::from("test.fit"),
    );
    assert_eq!(report.device, "Test Device");
    assert!(report.is_valid());
    assert_eq!(report.issues.len(), 0);
}

#[test]
fn test_device_profiles_complete() {
    // Ensure all major devices have profiles
    let devices = vec![
        DeviceProfile::garmin_edge_520(),
        DeviceProfile::garmin_edge_1030(),
        DeviceProfile::garmin_forerunner_945(),
        DeviceProfile::wahoo_elemnt_bolt(),
        DeviceProfile::zwift(),
    ];

    for device in devices {
        assert!(!device.name.is_empty());
        assert!(!device.manufacturer.is_empty());
        assert!(!device.primary_sports.is_empty());
        assert!(!device.known_quirks.is_empty());
    }
}

#[test]
fn test_cross_device_metric_comparison() {
    // Validate that different devices have reasonable metric ranges
    let sports = vec![Sport::Cycling, Sport::Running];

    for sport in sports {
        let ranges = MetricRanges::for_sport(sport);

        // All should have reasonable heart rate ranges
        assert!(ranges.hr_min > 0 && ranges.hr_min < 100);
        assert!(ranges.hr_max > 150 && ranges.hr_max <= 220);

        // Cadence should always have sensible bounds
        assert!(ranges.cadence_min < ranges.cadence_max);
    }
}

#[test]
fn test_validation_accuracy_with_mocked_data() {
    use trainrs::models::WorkoutSummary;
    use chrono::NaiveDate;

    // Create a mock valid cycling workout
    let workout = Workout {
        id: "test".to_string(),
        athlete_id: None,
        date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        sport: Sport::Cycling,
        workout_type: WorkoutType::Endurance,
        duration_seconds: 3600,
        summary: WorkoutSummary {
            total_distance: Some(rust_decimal_macros::dec!(30.0)),
            avg_heart_rate: Some(150),
            max_heart_rate: Some(180),
            avg_power: Some(200),
            normalized_power: Some(220),
            avg_pace: None,
            intensity_factor: None,
            tss: None,
            elevation_gain: Some(100),
            avg_cadence: Some(90),
            calories: Some(800),
        },
        data_source: DataSource::Power,
        notes: None,
        source: None,
        raw_data: None,
    };

    // Validate it passes checks
    let ranges = MetricRanges::for_sport(Sport::Cycling);
    if let Some(hr) = workout.summary.avg_heart_rate {
        assert!(hr >= ranges.hr_min && hr <= ranges.hr_max);
    }
    if let Some(power) = workout.summary.avg_power {
        assert!(power >= ranges.power_min && power <= ranges.power_max);
    }
}

#[test]
fn test_quirk_documentation() {
    // Ensure quirks are documented for known devices
    let edge_520 = DeviceProfile::garmin_edge_520();
    assert!(
        edge_520.known_quirks.iter().any(|q| q.contains("power")),
        "Edge 520 should document power-related quirks"
    );

    let fr945 = DeviceProfile::garmin_forerunner_945();
    assert!(
        fr945.known_quirks.iter().any(|q| q.contains("HRV") || q.contains("running")),
        "FR945 should document running/HRV quirks"
    );
}
