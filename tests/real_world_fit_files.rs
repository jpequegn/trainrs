/// Real-world FIT file validation test suite
///
/// This module tests FIT file parsing with real-world files from diverse sources:
/// - Multiple device manufacturers (Garmin, Wahoo, Stages, etc.)
/// - Various sports (cycling, running, swimming, triathlon)
/// - Different durations (short to ultra-endurance)
/// - Edge cases and corrupted files
///
/// Test files should be anonymized (PII removed, GPS stripped) before committing.

use std::path::PathBuf;
use trainrs::import::{fit::FitImporter, ImportFormat};
use trainrs::models::{Sport, Workout};

// Test helper functions

/// Get all FIT files in a directory recursively
fn get_fit_files(dir: &str) -> Vec<PathBuf> {
    let pattern = format!("tests/fixtures/{}/**/*.fit", dir);
    glob::glob(&pattern)
        .expect("Failed to read glob pattern")
        .filter_map(Result::ok)
        .collect()
}

/// Count FIT files in a directory
fn count_fit_files(dir: &str) -> usize {
    get_fit_files(dir).len()
}

/// Validate basic workout properties
fn validate_workout(workout: &Workout, context: &str) {
    // Basic validations
    assert!(!workout.id.is_empty(), "{}: Workout ID should not be empty", context);
    assert!(workout.duration_seconds > 0, "{}: Duration should be positive", context);

    // Sport should be valid
    match workout.sport {
        Sport::Cycling | Sport::Running | Sport::Swimming |
        Sport::Triathlon | Sport::Rowing | Sport::CrossTraining => {},
    }

    // If we have power data, it should be reasonable
    if let Some(avg_power) = workout.summary.avg_power {
        assert!(avg_power > 0 && avg_power < 2000,
                "{}: Average power {} seems unreasonable", context, avg_power);
    }

    // If we have heart rate, it should be reasonable
    if let Some(avg_hr) = workout.summary.avg_heart_rate {
        assert!(avg_hr > 30 && avg_hr < 220,
                "{}: Average HR {} seems unreasonable", context, avg_hr);
    }

    // If we have distance, it should be positive
    if let Some(distance) = workout.summary.total_distance {
        assert!(distance > rust_decimal::Decimal::ZERO,
                "{}: Distance should be positive", context);
    }
}

// Garmin device tests

#[test]
fn test_garmin_edge_520_files() {
    let files = get_fit_files("garmin/edge_520");
    let importer = FitImporter::new();

    for file_path in files {
        let result = importer.import_file(&file_path);
        assert!(result.is_ok(),
                "Failed to import Garmin Edge 520 file: {:?}\nError: {:?}",
                file_path, result.err());

        if let Ok(workouts) = result {
            for workout in &workouts {
                validate_workout(workout, &format!("Edge 520: {:?}", file_path));
                // Edge 520 is primarily a cycling computer
                assert_eq!(workout.sport, Sport::Cycling,
                          "Edge 520 file should be cycling: {:?}", file_path);
            }
        }
    }
}

#[test]
fn test_garmin_edge_1030_files() {
    let files = get_fit_files("garmin/edge_1030");
    let importer = FitImporter::new();

    for file_path in files {
        let result = importer.import_file(&file_path);
        assert!(result.is_ok(),
                "Failed to import Garmin Edge 1030 file: {:?}\nError: {:?}",
                file_path, result.err());

        if let Ok(workouts) = result {
            for workout in &workouts {
                validate_workout(workout, &format!("Edge 1030: {:?}", file_path));
            }
        }
    }
}

#[test]
fn test_garmin_forerunner_945_files() {
    let files = get_fit_files("garmin/forerunner_945");
    let importer = FitImporter::new();

    for file_path in files {
        let result = importer.import_file(&file_path);
        assert!(result.is_ok(),
                "Failed to import Garmin Forerunner 945 file: {:?}\nError: {:?}",
                file_path, result.err());

        if let Ok(workouts) = result {
            for workout in &workouts {
                validate_workout(workout, &format!("Forerunner 945: {:?}", file_path));
                // Forerunner is primarily for running
                assert!(matches!(workout.sport, Sport::Running | Sport::Triathlon),
                       "Forerunner 945 should be running/triathlon: {:?}", file_path);
            }
        }
    }
}

#[test]
fn test_garmin_fenix_6_files() {
    let files = get_fit_files("garmin/fenix_6");
    let importer = FitImporter::new();

    for file_path in files {
        let result = importer.import_file(&file_path);
        assert!(result.is_ok(),
                "Failed to import Garmin Fenix 6 file: {:?}\nError: {:?}",
                file_path, result.err());

        if let Ok(workouts) = result {
            for workout in &workouts {
                validate_workout(workout, &format!("Fenix 6: {:?}", file_path));
                // Fenix supports multiple sports
            }
        }
    }
}

// Wahoo device tests

#[test]
fn test_wahoo_elemnt_bolt_files() {
    let files = get_fit_files("wahoo/elemnt_bolt");
    let importer = FitImporter::new();

    for file_path in files {
        let result = importer.import_file(&file_path);
        assert!(result.is_ok(),
                "Failed to import Wahoo ELEMNT BOLT file: {:?}\nError: {:?}",
                file_path, result.err());

        if let Ok(workouts) = result {
            for workout in &workouts {
                validate_workout(workout, &format!("ELEMNT BOLT: {:?}", file_path));
                assert_eq!(workout.sport, Sport::Cycling,
                          "ELEMNT BOLT should be cycling: {:?}", file_path);

                // Check for power spike quirk application
                if let Some(notes) = &workout.notes {
                    if notes.contains("Device Quirks") {
                        println!("✓ Quirks applied to ELEMNT BOLT file: {:?}", file_path);
                    }
                }
            }
        }
    }
}

#[test]
fn test_wahoo_elemnt_roam_files() {
    let files = get_fit_files("wahoo/elemnt_roam");
    let importer = FitImporter::new();

    for file_path in files {
        let result = importer.import_file(&file_path);
        assert!(result.is_ok(),
                "Failed to import Wahoo ELEMNT ROAM file: {:?}\nError: {:?}",
                file_path, result.err());

        if let Ok(workouts) = result {
            for workout in &workouts {
                validate_workout(workout, &format!("ELEMNT ROAM: {:?}", file_path));
            }
        }
    }
}

// Third-party app tests

#[test]
fn test_zwift_files() {
    let files = get_fit_files("zwift");
    let importer = FitImporter::new();

    for file_path in files {
        let result = importer.import_file(&file_path);
        assert!(result.is_ok(),
                "Failed to import Zwift file: {:?}\nError: {:?}",
                file_path, result.err());

        if let Ok(workouts) = result {
            for workout in &workouts {
                validate_workout(workout, &format!("Zwift: {:?}", file_path));
                assert_eq!(workout.sport, Sport::Cycling,
                          "Zwift file should be cycling: {:?}", file_path);

                // Zwift files should have power data
                assert!(workout.summary.avg_power.is_some(),
                       "Zwift file should have power data: {:?}", file_path);
            }
        }
    }
}

// Developer fields tests

#[test]
fn test_developer_field_files() {
    let files = get_fit_files("developer_fields");
    let importer = FitImporter::new();

    for file_path in files {
        let result = importer.import_file(&file_path);
        assert!(result.is_ok(),
                "Failed to import file with developer fields: {:?}\nError: {:?}",
                file_path, result.err());

        if let Ok(workouts) = result {
            for workout in &workouts {
                validate_workout(workout, &format!("Developer fields: {:?}", file_path));

                // Check if developer fields were extracted
                if let Some(ref raw_data) = workout.raw_data {
                    // At least some data points should be present
                    assert!(!raw_data.is_empty(),
                           "File with developer fields should have data points: {:?}", file_path);
                }
            }
        }
    }
}

// Edge case and error handling tests

#[test]
fn test_corrupted_files() {
    let files = get_fit_files("corrupted");
    let importer = FitImporter::new();

    for file_path in files {
        let result = importer.import_file(&file_path);

        // Corrupted files may fail, but should not panic
        match result {
            Ok(workouts) => {
                // If it somehow parsed successfully, that's ok
                println!("✓ Corrupted file recovered: {:?}", file_path);
                for workout in &workouts {
                    // Still validate what we got
                    validate_workout(workout, &format!("Corrupted (recovered): {:?}", file_path));
                }
            }
            Err(e) => {
                // Should have a descriptive error message
                let error_msg = e.to_string();
                assert!(!error_msg.is_empty(),
                       "Error message should not be empty for: {:?}", file_path);
                assert!(error_msg.len() > 10,
                       "Error message should be descriptive for: {:?}", file_path);
                println!("✓ Corrupted file handled gracefully: {:?} - {}", file_path, error_msg);
            }
        }
    }
}

#[test]
fn test_edge_case_files() {
    let files = get_fit_files("edge_cases");
    let importer = FitImporter::new();

    for file_path in files {
        let result = importer.import_file(&file_path);

        match result {
            Ok(workouts) => {
                for workout in &workouts {
                    validate_workout(workout, &format!("Edge case: {:?}", file_path));
                }
            }
            Err(e) => {
                // Edge cases might fail, but should have good error messages
                println!("Edge case file failed (expected): {:?} - {}", file_path, e);
            }
        }
    }
}

// Comprehensive test suites

#[test]
fn test_all_fit_files_parse() {
    let categories = vec![
        "garmin/edge_520",
        "garmin/edge_1030",
        "garmin/forerunner_945",
        "garmin/fenix_6",
        "wahoo/elemnt_bolt",
        "wahoo/elemnt_roam",
        "zwift",
        "developer_fields",
    ];

    let importer = FitImporter::new();
    let mut total_files = 0;
    let mut successful_parses = 0;
    let mut failed_parses = 0;

    for category in categories {
        let files = get_fit_files(category);
        total_files += files.len();

        for file_path in files {
            match importer.import_file(&file_path) {
                Ok(_) => successful_parses += 1,
                Err(e) => {
                    failed_parses += 1;
                    eprintln!("Failed to parse {:?}: {}", file_path, e);
                }
            }
        }
    }

    println!("\n========== FIT File Test Summary ==========");
    println!("Total files tested: {}", total_files);
    println!("Successful parses: {}", successful_parses);
    println!("Failed parses: {}", failed_parses);

    if total_files > 0 {
        let success_rate = (successful_parses as f64 / total_files as f64) * 100.0;
        println!("Success rate: {:.1}%", success_rate);

        // We want at least 95% success rate for non-corrupted files
        assert!(success_rate >= 95.0,
               "Success rate should be at least 95%, got {:.1}%", success_rate);
    }
}

#[test]
fn test_file_coverage() {
    let garmin_count = count_fit_files("garmin");
    let wahoo_count = count_fit_files("wahoo");
    let zwift_count = count_fit_files("zwift");
    let dev_fields_count = count_fit_files("developer_fields");
    let corrupted_count = count_fit_files("corrupted");
    let edge_cases_count = count_fit_files("edge_cases");

    let total = garmin_count + wahoo_count + zwift_count + dev_fields_count + corrupted_count + edge_cases_count;

    println!("\n========== Test File Coverage ==========");
    println!("Garmin files: {}", garmin_count);
    println!("Wahoo files: {}", wahoo_count);
    println!("Zwift files: {}", zwift_count);
    println!("Developer fields: {}", dev_fields_count);
    println!("Corrupted files: {}", corrupted_count);
    println!("Edge cases: {}", edge_cases_count);
    println!("Total files: {}", total);

    // Note: Initially this will be 0 until real files are added
    // The test passes either way to allow gradual file collection
    if total > 0 {
        println!("✓ Test suite contains {} real-world FIT files", total);
    } else {
        println!("⚠ No test files present yet - add real FIT files to tests/fixtures/");
    }
}

// Sport-specific validation tests

#[test]
fn test_cycling_files() {
    let categories = vec!["garmin/edge_520", "garmin/edge_1030", "wahoo/elemnt_bolt", "wahoo/elemnt_roam", "zwift"];
    let importer = FitImporter::new();

    for category in categories {
        let files = get_fit_files(category);
        for file_path in files {
            if let Ok(workouts) = importer.import_file(&file_path) {
                for workout in &workouts {
                    if workout.sport == Sport::Cycling {
                        // Cycling-specific validations
                        if let Some(avg_power) = workout.summary.avg_power {
                            assert!(avg_power >= 50 && avg_power <= 500,
                                   "Cycling avg power seems unreasonable: {} in {:?}", avg_power, file_path);
                        }

                        if let Some(avg_cadence) = workout.summary.avg_cadence {
                            assert!(avg_cadence >= 40 && avg_cadence <= 130,
                                   "Cycling avg cadence seems unreasonable: {} in {:?}", avg_cadence, file_path);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn test_running_files() {
    let categories = vec!["garmin/forerunner_945"];
    let importer = FitImporter::new();

    for category in categories {
        let files = get_fit_files(category);
        for file_path in files {
            if let Ok(workouts) = importer.import_file(&file_path) {
                for workout in &workouts {
                    if workout.sport == Sport::Running {
                        // Running-specific validations
                        if let Some(avg_pace) = workout.summary.avg_pace {
                            // Pace in min/km, should be between 2:30 and 10:00 min/km for most runs
                            use rust_decimal::prelude::ToPrimitive;
                            let pace_f64 = avg_pace.to_f64().unwrap_or(0.0);
                            assert!(pace_f64 >= 2.5 && pace_f64 <= 10.0,
                                   "Running avg pace seems unreasonable: {} min/km in {:?}", pace_f64, file_path);
                        }

                        if let Some(avg_cadence) = workout.summary.avg_cadence {
                            // Running cadence in spm, typically 150-200
                            assert!(avg_cadence >= 120 && avg_cadence <= 220,
                                   "Running avg cadence seems unreasonable: {} spm in {:?}", avg_cadence, file_path);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod duration_tests {
    use super::*;

    #[test]
    fn test_short_duration_files() {
        // Files under 30 minutes
        let all_files = get_fit_files("");
        let importer = FitImporter::new();

        for file_path in all_files {
            if let Ok(workouts) = importer.import_file(&file_path) {
                for workout in &workouts {
                    if workout.duration_seconds < 1800 {
                        println!("Short workout: {}s in {:?}", workout.duration_seconds, file_path);
                        validate_workout(workout, &format!("Short duration: {:?}", file_path));
                    }
                }
            }
        }
    }

    #[test]
    fn test_long_duration_files() {
        // Files over 3 hours
        let all_files = get_fit_files("");
        let importer = FitImporter::new();

        for file_path in all_files {
            if let Ok(workouts) = importer.import_file(&file_path) {
                for workout in &workouts {
                    if workout.duration_seconds > 10800 {
                        println!("Long workout: {}s ({:.1}h) in {:?}",
                                workout.duration_seconds,
                                workout.duration_seconds as f64 / 3600.0,
                                file_path);
                        validate_workout(workout, &format!("Long duration: {:?}", file_path));
                    }
                }
            }
        }
    }
}
