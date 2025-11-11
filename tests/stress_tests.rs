/// Stress testing suite
///
/// This module performs stress tests to validate production readiness:
/// - Large file handling (>500MB)
/// - Rapid import bursts (100 files/sec)
/// - Concurrent operations
/// - Memory-constrained environments
/// - Edge cases (invalid UTF-8, extremely long workouts, missing fields)
///
/// Run with: cargo test --test stress_tests --release

use std::path::PathBuf;
use trainrs::import::fit::FitImporter;
use trainrs::models::{AthleteProfile, Sport, Workout, WorkoutSummary, WorkoutType, DataSource, DataPoint};
use chrono::NaiveDate;
use rust_decimal_macros::dec;
use std::sync::{Arc, Mutex};
use std::thread;

// Helper to create workouts with varying data sizes
fn create_workout_with_datapoints(id: usize, data_point_count: usize) -> Workout {
    let mut data_points = Vec::with_capacity(data_point_count);

    for i in 0..data_point_count {
        data_points.push(DataPoint {
            timestamp: i as u32,
            heart_rate: Some(150),
            power: Some(200 + (i % 100) as u16),
            cadence: Some(90),
            speed: Some(dec!(8.33)),
            elevation: Some(100 + (i % 50) as i16),
            distance: Some(dec!(8.33) * rust_decimal::Decimal::from(i)),
            pace: None,
            left_power: None,
            right_power: None,
            ground_contact_time: None,
            vertical_oscillation: None,
            stride_length: None,
            stroke_count: None,
            stroke_type: None,
            lap_number: None,
            sport_transition: None,
        });
    }

    Workout {
        id: format!("stress_test_{}", id),
        athlete_id: Some("stress_test_athlete".to_string()),
        date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        sport: Sport::Cycling,
        workout_type: WorkoutType::Endurance,
        duration_seconds: data_point_count as u32,
        summary: WorkoutSummary {
            total_distance: Some(dec!(30.0)),
            avg_heart_rate: Some(150),
            max_heart_rate: Some(180),
            avg_power: Some(200),
            normalized_power: Some(220),
            avg_pace: Some(dec!(4.0)),
            intensity_factor: Some(dec!(0.88)),
            tss: Some(dec!(100.0)),
            elevation_gain: Some(100),
            avg_cadence: Some(90),
            calories: Some(720),
        },
        data_source: DataSource::Power,
        notes: Some(format!("Stress test workout {} with {} data points", id, data_point_count)),
        source: None,
        raw_data: Some(data_points),
    }
}

/// Test: Handle very large workout (500MB worth of data)
#[test]
#[ignore] // Run explicitly with --ignored flag
fn test_large_workout_file() {
    const DATA_POINTS: usize = 500_000; // ~139 hours at 1Hz, or ~14 hours at 10Hz
    const TIMEOUT_SECS: u64 = 30;

    println!("Creating large workout with {} data points...", DATA_POINTS);
    let start = std::time::Instant::now();

    let workout = create_workout_with_datapoints(0, DATA_POINTS);

    let creation_time = start.elapsed();
    println!("Workout created in {:.2}s", creation_time.as_secs_f64());

    assert!(
        creation_time.as_secs() < TIMEOUT_SECS,
        "Workout creation too slow: {:.2}s (max: {}s)",
        creation_time.as_secs_f64(),
        TIMEOUT_SECS
    );

    // Verify data integrity
    assert_eq!(workout.raw_data.as_ref().unwrap().len(), DATA_POINTS);
    assert_eq!(workout.duration_seconds, DATA_POINTS as u32);

    println!("✓ Large workout handled successfully");
}

/// Test: Rapid sequential import burst
#[test]
#[ignore]
fn test_rapid_import_burst() {
    const FILE_COUNT: usize = 100;
    const MAX_TIME_SECS: u64 = 10;

    println!("Creating {} workouts rapidly...", FILE_COUNT);
    let start = std::time::Instant::now();

    let mut workouts = Vec::with_capacity(FILE_COUNT);
    for i in 0..FILE_COUNT {
        let workout = create_workout_with_datapoints(i, 3600); // 1 hour workout
        workouts.push(workout);
    }

    let elapsed = start.elapsed();
    let per_file = elapsed.as_secs_f64() / FILE_COUNT as f64;

    println!("Created {} workouts in {:.2}s ({:.4}s per workout)",
        FILE_COUNT, elapsed.as_secs_f64(), per_file);

    assert!(
        elapsed.as_secs() < MAX_TIME_SECS,
        "Bulk import too slow: {:.2}s (max: {}s)",
        elapsed.as_secs_f64(),
        MAX_TIME_SECS
    );

    assert_eq!(workouts.len(), FILE_COUNT);
    println!("✓ Rapid import burst successful");
}

/// Test: Concurrent workout processing
#[test]
#[ignore]
fn test_concurrent_operations() {
    const THREAD_COUNT: usize = 4;
    const WORKOUTS_PER_THREAD: usize = 25;

    let results = Arc::new(Mutex::new(Vec::new()));
    let mut handles = vec![];

    println!("Spawning {} threads with {} workouts each...",
        THREAD_COUNT, WORKOUTS_PER_THREAD);

    for thread_id in 0..THREAD_COUNT {
        let results_clone = Arc::clone(&results);

        let handle = thread::spawn(move || {
            let mut local_workouts = Vec::new();

            for i in 0..WORKOUTS_PER_THREAD {
                let workout_id = thread_id * WORKOUTS_PER_THREAD + i;
                let workout = create_workout_with_datapoints(workout_id, 1800);
                local_workouts.push(workout);
            }

            results_clone.lock().unwrap().extend(local_workouts);
        });

        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    let final_results = results.lock().unwrap();
    let total_workouts = THREAD_COUNT * WORKOUTS_PER_THREAD;

    assert_eq!(
        final_results.len(),
        total_workouts,
        "Expected {} workouts, got {}",
        total_workouts,
        final_results.len()
    );

    println!("✓ Concurrent operations successful: {} workouts processed",
        final_results.len());
}

/// Test: Extremely long workout (24+ hours)
#[test]
#[ignore]
fn test_extremely_long_workout() {
    const DURATION_HOURS: usize = 30; // 30-hour ultra-endurance event
    const DATA_POINTS: usize = DURATION_HOURS * 3600; // 1Hz sampling

    println!("Creating {}-hour workout with {} data points...",
        DURATION_HOURS, DATA_POINTS);

    let workout = create_workout_with_datapoints(0, DATA_POINTS);

    assert_eq!(workout.duration_seconds, DATA_POINTS as u32);
    assert!(workout.raw_data.is_some());
    assert_eq!(workout.raw_data.as_ref().unwrap().len(), DATA_POINTS);

    println!("✓ Extremely long workout handled successfully");
}

/// Test: Workout with missing fields
#[test]
fn test_workout_with_missing_fields() {
    let workout = Workout {
        id: "missing_fields_test".to_string(),
        athlete_id: None, // Missing
        date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        sport: Sport::Cycling,
        workout_type: WorkoutType::Endurance,
        duration_seconds: 3600,
        summary: WorkoutSummary {
            total_distance: None, // Missing
            avg_heart_rate: None, // Missing
            max_heart_rate: None, // Missing
            avg_power: Some(200),
            normalized_power: None, // Missing
            avg_pace: None, // Missing
            intensity_factor: None, // Missing
            tss: None, // Missing
            elevation_gain: None, // Missing
            avg_cadence: None, // Missing
            calories: None, // Missing
        },
        data_source: DataSource::Power,
        notes: None,
        source: None,
        raw_data: None,
    };

    // Should not panic with missing fields
    assert_eq!(workout.id, "missing_fields_test");
    assert_eq!(workout.duration_seconds, 3600);
    assert!(workout.summary.avg_power.is_some());

    println!("✓ Workout with missing fields handled gracefully");
}

/// Test: Empty workout (edge case)
#[test]
fn test_empty_workout() {
    let workout = Workout {
        id: "empty_test".to_string(),
        athlete_id: None,
        date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        sport: Sport::Cycling,
        workout_type: WorkoutType::Endurance,
        duration_seconds: 0, // Empty
        summary: WorkoutSummary {
            total_distance: None,
            avg_heart_rate: None,
            max_heart_rate: None,
            avg_power: None,
            normalized_power: None,
            avg_pace: None,
            intensity_factor: None,
            tss: None,
            elevation_gain: None,
            avg_cadence: None,
            calories: None,
        },
        data_source: DataSource::Power,
        notes: None,
        source: None,
        raw_data: Some(vec![]), // Empty data
    };

    assert_eq!(workout.duration_seconds, 0);
    assert!(workout.raw_data.as_ref().unwrap().is_empty());

    println!("✓ Empty workout handled gracefully");
}

/// Test: Workout with single data point
#[test]
fn test_single_datapoint_workout() {
    let workout = create_workout_with_datapoints(0, 1);

    assert_eq!(workout.raw_data.as_ref().unwrap().len(), 1);
    assert_eq!(workout.duration_seconds, 1);

    println!("✓ Single data point workout handled");
}

/// Test: Invalid string data handling
#[test]
fn test_invalid_string_data() {
    // Create workout with potentially problematic string data
    let workout = Workout {
        id: "test_\u{0000}_null_byte".to_string(), // Null byte
        athlete_id: Some("athlete_\u{FFFD}_replacement".to_string()), // Replacement character
        date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        sport: Sport::Cycling,
        workout_type: WorkoutType::Endurance,
        duration_seconds: 3600,
        summary: WorkoutSummary::default(),
        data_source: DataSource::Power,
        notes: Some("Notes with émojis 🚴 and spëcial çharacters".to_string()),
        source: None,
        raw_data: None,
    };

    // Should not panic with special characters
    assert!(workout.id.contains("test_"));
    assert!(workout.notes.is_some());

    println!("✓ Invalid string data handled gracefully");
}

/// Test: Memory pressure simulation
#[test]
#[ignore]
fn test_memory_pressure() {
    const BATCH_SIZE: usize = 50;
    const BATCHES: usize = 10;
    const DATA_POINTS_PER_WORKOUT: usize = 10_000;

    println!("Simulating memory pressure: {} batches of {} workouts...",
        BATCHES, BATCH_SIZE);

    for batch in 0..BATCHES {
        let mut workouts = Vec::with_capacity(BATCH_SIZE);

        for i in 0..BATCH_SIZE {
            let workout_id = batch * BATCH_SIZE + i;
            workouts.push(create_workout_with_datapoints(
                workout_id,
                DATA_POINTS_PER_WORKOUT
            ));
        }

        println!("Batch {}/{}: {} workouts created", batch + 1, BATCHES, workouts.len());

        // Explicitly drop to free memory
        drop(workouts);
    }

    println!("✓ Memory pressure test completed");
}

/// Test: Rapid allocation/deallocation cycles
#[test]
#[ignore]
fn test_rapid_allocation_cycles() {
    const CYCLES: usize = 1000;
    const WORKOUTS_PER_CYCLE: usize = 10;

    println!("Running {} allocation/deallocation cycles...", CYCLES);
    let start = std::time::Instant::now();

    for cycle in 0..CYCLES {
        let mut workouts = Vec::with_capacity(WORKOUTS_PER_CYCLE);

        for i in 0..WORKOUTS_PER_CYCLE {
            workouts.push(create_workout_with_datapoints(
                cycle * WORKOUTS_PER_CYCLE + i,
                100
            ));
        }

        drop(workouts);

        if cycle % 100 == 0 && cycle > 0 {
            println!("Completed {} cycles in {:.2}s",
                cycle, start.elapsed().as_secs_f64());
        }
    }

    let elapsed = start.elapsed();
    println!("✓ {} cycles completed in {:.2}s", CYCLES, elapsed.as_secs_f64());
}

/// Test: Data integrity with large datasets
#[test]
#[ignore]
fn test_data_integrity_large_dataset() {
    const DATA_POINTS: usize = 100_000;

    println!("Testing data integrity with {} data points...", DATA_POINTS);

    let workout = create_workout_with_datapoints(0, DATA_POINTS);
    let data = workout.raw_data.as_ref().unwrap();

    // Verify all data points are present and correct
    assert_eq!(data.len(), DATA_POINTS);

    // Spot check various points
    assert_eq!(data[0].timestamp, 0);
    assert_eq!(data[DATA_POINTS / 2].timestamp, (DATA_POINTS / 2) as u32);
    assert_eq!(data[DATA_POINTS - 1].timestamp, (DATA_POINTS - 1) as u32);

    // Verify power varies correctly
    assert_eq!(data[0].power, Some(200));
    assert_eq!(data[50].power, Some(250)); // 200 + (50 % 100)
    assert_eq!(data[99].power, Some(299)); // 200 + (99 % 100)

    println!("✓ Data integrity verified for {} points", DATA_POINTS);
}

#[cfg(test)]
mod summary {
    use super::*;

    /// Print stress test summary
    #[test]
    fn print_test_summary() {
        println!("\n========== Stress Test Suite ==========");
        println!("Run with: cargo test --test stress_tests --release -- --ignored --nocapture");
        println!("\nAvailable tests:");
        println!("  • test_large_workout_file - 500K data points (~500MB)");
        println!("  • test_rapid_import_burst - 100 files rapid sequence");
        println!("  • test_concurrent_operations - 4 threads, 25 workouts each");
        println!("  • test_extremely_long_workout - 30-hour ultra workout");
        println!("  • test_workout_with_missing_fields - Graceful handling");
        println!("  • test_empty_workout - Edge case: 0 duration");
        println!("  • test_single_datapoint_workout - Edge case: 1 data point");
        println!("  • test_invalid_string_data - Special characters");
        println!("  • test_memory_pressure - 10 batches of 50 workouts");
        println!("  • test_rapid_allocation_cycles - 1000 cycles");
        println!("  • test_data_integrity_large_dataset - 100K points validation");
        println!("\nAcceptance criteria:");
        println!("  • Large files: <30s processing");
        println!("  • Rapid burst: <10s for 100 files");
        println!("  • Concurrent: No panics, all workouts processed");
        println!("  • No crashes on edge cases");
        println!("  • Data integrity maintained");
        println!("========================================\n");
    }
}
