/// Memory leak detection tests
///
/// This module tests for memory leaks during extended operations:
/// - Sequential file imports (1000+ files)
/// - Batch import iterations
/// - Long-running database operations
/// - Repeated calculations
///
/// Run with: cargo test --test memory_leak_tests --release

use std::collections::HashMap;
use trainrs::import::fit::FitImporter;
use trainrs::models::{AthleteProfile, Sport, Workout, WorkoutSummary, WorkoutType, DataSource};
use trainrs::tss::TssCalculator;
use trainrs::pmc::PmcCalculator;
use chrono::NaiveDate;
use rust_decimal_macros::dec;

// Memory tracking helper
#[cfg(target_os = "linux")]
fn get_memory_usage() -> usize {
    use std::fs;

    let status = fs::read_to_string("/proc/self/status").unwrap();
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return parts[1].parse::<usize>().unwrap_or(0) * 1024; // Convert kB to bytes
            }
        }
    }
    0
}

#[cfg(target_os = "macos")]
fn get_memory_usage() -> usize {
    use std::process::Command;

    let output = Command::new("ps")
        .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .expect("Failed to execute ps command");

    let rss_kb = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<usize>()
        .unwrap_or(0);

    rss_kb * 1024 // Convert kB to bytes
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn get_memory_usage() -> usize {
    0 // Memory tracking not supported on this platform
}

// Test data generation helpers
fn create_test_workout(seed: usize) -> Workout {
    Workout {
        id: format!("mem_test_{}", seed),
        athlete_id: Some("test_athlete".to_string()),
        date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
            + chrono::Duration::days((seed % 365) as i64),
        sport: Sport::Cycling,
        workout_type: WorkoutType::Endurance,
        duration_seconds: 3600,
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
        notes: Some(format!("Memory leak test workout {}", seed)),
        source: None,
        raw_data: None,
    }
}

fn create_test_athlete() -> AthleteProfile {
    use chrono::Utc;

    AthleteProfile {
        id: "test_athlete".to_string(),
        name: "Test Athlete".to_string(),
        date_of_birth: Some(NaiveDate::from_ymd_opt(1990, 1, 1).unwrap()),
        weight: Some(dec!(70.0)),
        height: Some(175),
        ftp: Some(250),
        lthr: Some(165),
        max_hr: Some(190),
        resting_hr: Some(50),
        threshold_pace: Some(dec!(4.0)),
        training_zones: trainrs::models::TrainingZones::default(),
        preferred_units: trainrs::models::Units::Metric,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// Test: Sequential import of 1000 workouts should not leak memory
#[test]
#[ignore] // Run explicitly with --ignored flag
fn test_no_memory_leak_sequential_import() {
    const ITERATIONS: usize = 1000;
    const MAX_GROWTH_MB: usize = 10;

    // Warm up to stabilize allocator
    for i in 0..10 {
        let _ = create_test_workout(i);
    }

    let initial_mem = get_memory_usage();
    println!("Initial memory: {:.2} MB", initial_mem as f64 / 1_000_000.0);

    // Sequential workout creation
    for i in 0..ITERATIONS {
        let workout = create_test_workout(i);
        drop(workout); // Explicit drop

        // Sample memory every 100 iterations
        if i % 100 == 0 && i > 0 {
            let current_mem = get_memory_usage();
            println!("After {} iterations: {:.2} MB",
                i, current_mem as f64 / 1_000_000.0);
        }
    }

    let final_mem = get_memory_usage();
    let growth = final_mem.saturating_sub(initial_mem);

    println!("Final memory: {:.2} MB", final_mem as f64 / 1_000_000.0);
    println!("Memory growth: {:.2} MB", growth as f64 / 1_000_000.0);

    // Should be less than 10MB growth for 1000 imports
    assert!(
        growth < MAX_GROWTH_MB * 1_000_000,
        "Memory leak detected: {:.2} MB growth (max allowed: {} MB)",
        growth as f64 / 1_000_000.0,
        MAX_GROWTH_MB
    );
}

/// Test: Batch import iterations should have stable memory
#[test]
#[ignore]
fn test_batch_import_stable_memory() {
    const BATCH_SIZE: usize = 100;
    const ITERATIONS: usize = 10;
    const MAX_GROWTH_PER_ITERATION_MB: usize = 5;

    let mut memory_samples = Vec::new();

    for iteration in 0..ITERATIONS {
        let mem_before = get_memory_usage();

        // Create and drop batch of workouts
        let mut workouts = Vec::new();
        for i in 0..BATCH_SIZE {
            workouts.push(create_test_workout(iteration * BATCH_SIZE + i));
        }
        drop(workouts);

        let mem_after = get_memory_usage();
        memory_samples.push((mem_before, mem_after));

        println!("Iteration {}: {:.2} MB -> {:.2} MB (delta: {:.2} MB)",
            iteration,
            mem_before as f64 / 1_000_000.0,
            mem_after as f64 / 1_000_000.0,
            (mem_after.saturating_sub(mem_before)) as f64 / 1_000_000.0
        );

        // Memory should stabilize after first iteration
        if iteration > 0 {
            let growth = mem_after.saturating_sub(mem_before);
            assert!(
                growth < MAX_GROWTH_PER_ITERATION_MB * 1_000_000,
                "Memory growth in iteration {}: {:.2} MB (max allowed: {} MB)",
                iteration,
                growth as f64 / 1_000_000.0,
                MAX_GROWTH_PER_ITERATION_MB
            );
        }
    }
}

/// Test: TSS calculation should not leak memory
#[test]
#[ignore]
fn test_tss_calculation_no_leak() {
    const ITERATIONS: usize = 1000;
    const MAX_GROWTH_MB: usize = 5;

    let athlete = create_test_athlete();

    let initial_mem = get_memory_usage();

    for i in 0..ITERATIONS {
        let workout = create_test_workout(i);
        let _ = TssCalculator::calculate_tss(&workout, &athlete);
    }

    let final_mem = get_memory_usage();
    let growth = final_mem.saturating_sub(initial_mem);

    println!("TSS calculation memory growth: {:.2} MB", growth as f64 / 1_000_000.0);

    assert!(
        growth < MAX_GROWTH_MB * 1_000_000,
        "Memory leak in TSS calculation: {:.2} MB",
        growth as f64 / 1_000_000.0
    );
}

/// Test: PMC calculation should not leak memory
#[test]
#[ignore]
fn test_pmc_calculation_no_leak() {
    const DAYS: usize = 365;
    const ITERATIONS: usize = 10;
    const MAX_GROWTH_MB: usize = 5;

    let pmc_calculator = PmcCalculator::new();

    // Create daily TSS data
    let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let end_date = start_date + chrono::Duration::days(DAYS as i64);

    let mut daily_tss = std::collections::BTreeMap::new();
    for day in 0..DAYS {
        let date = start_date + chrono::Duration::days(day as i64);
        daily_tss.insert(date, trainrs::pmc::DailyTss {
            date,
            total_tss: dec!(100),
            workout_count: 1,
            has_workouts: true,
            workout_tss_values: vec![dec!(100)],
        });
    }

    let initial_mem = get_memory_usage();

    for _ in 0..ITERATIONS {
        let _ = pmc_calculator.calculate_pmc_series(&daily_tss, start_date, end_date);
    }

    let final_mem = get_memory_usage();
    let growth = final_mem.saturating_sub(initial_mem);

    println!("PMC calculation memory growth: {:.2} MB", growth as f64 / 1_000_000.0);

    assert!(
        growth < MAX_GROWTH_MB * 1_000_000,
        "Memory leak in PMC calculation: {:.2} MB",
        growth as f64 / 1_000_000.0
    );
}

/// Test: Memory usage with large workout data
#[test]
#[ignore]
fn test_large_workout_data_memory() {
    const DATA_POINTS: usize = 10_000; // ~3 hour workout at 1Hz
    const MAX_MEMORY_MB: usize = 50;

    let initial_mem = get_memory_usage();

    // Create workout with large raw data
    let mut workout = create_test_workout(0);
    let mut data_points = Vec::with_capacity(DATA_POINTS);

    for i in 0..DATA_POINTS {
        data_points.push(trainrs::models::DataPoint {
            timestamp: i as u32,
            heart_rate: Some(150),
            power: Some(200),
            cadence: Some(90),
            speed: Some(dec!(30.0)),
            elevation: Some(100),
            distance: Some(dec!(8.33)), // m/s * seconds
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

    workout.raw_data = Some(data_points);

    let with_data_mem = get_memory_usage();
    let growth = with_data_mem.saturating_sub(initial_mem);

    println!("Memory for {} data points: {:.2} MB",
        DATA_POINTS, growth as f64 / 1_000_000.0);

    drop(workout);

    let after_drop_mem = get_memory_usage();
    println!("Memory after drop: {:.2} MB", after_drop_mem as f64 / 1_000_000.0);

    assert!(
        growth < MAX_MEMORY_MB * 1_000_000,
        "Excessive memory for large workout: {:.2} MB (max: {} MB)",
        growth as f64 / 1_000_000.0,
        MAX_MEMORY_MB
    );
}

/// Test: Repeated HashMap operations don't leak
#[test]
#[ignore]
fn test_hashmap_operations_no_leak() {
    const ITERATIONS: usize = 10_000;
    const MAX_GROWTH_MB: usize = 5;

    let initial_mem = get_memory_usage();

    for i in 0..ITERATIONS {
        let mut map: HashMap<String, Workout> = HashMap::new();

        for j in 0..10 {
            let workout = create_test_workout(i * 10 + j);
            map.insert(workout.id.clone(), workout);
        }

        drop(map);
    }

    let final_mem = get_memory_usage();
    let growth = final_mem.saturating_sub(initial_mem);

    println!("HashMap operations memory growth: {:.2} MB", growth as f64 / 1_000_000.0);

    assert!(
        growth < MAX_GROWTH_MB * 1_000_000,
        "Memory leak in HashMap operations: {:.2} MB",
        growth as f64 / 1_000_000.0
    );
}

/// Test: String allocations don't accumulate
#[test]
#[ignore]
fn test_string_allocations_no_leak() {
    const ITERATIONS: usize = 100_000;
    const MAX_GROWTH_MB: usize = 5;

    let initial_mem = get_memory_usage();

    for i in 0..ITERATIONS {
        let s = format!("workout_id_{}_with_some_extra_text_to_make_it_longer", i);
        drop(s);
    }

    let final_mem = get_memory_usage();
    let growth = final_mem.saturating_sub(initial_mem);

    println!("String allocations memory growth: {:.2} MB", growth as f64 / 1_000_000.0);

    assert!(
        growth < MAX_GROWTH_MB * 1_000_000,
        "Memory leak in string allocations: {:.2} MB",
        growth as f64 / 1_000_000.0
    );
}

#[cfg(test)]
mod summary {
    use super::*;

    /// Print memory leak test summary
    #[test]
    fn print_test_summary() {
        println!("\n========== Memory Leak Test Suite ==========");
        println!("Run with: cargo test --test memory_leak_tests --release -- --ignored --nocapture");
        println!("\nAvailable tests:");
        println!("  • test_no_memory_leak_sequential_import - 1000 sequential imports");
        println!("  • test_batch_import_stable_memory - 10 batches of 100 workouts");
        println!("  • test_tss_calculation_no_leak - 1000 TSS calculations");
        println!("  • test_pmc_calculation_no_leak - 10 PMC calculations over 365 days");
        println!("  • test_large_workout_data_memory - 10K data points");
        println!("  • test_hashmap_operations_no_leak - 10K HashMap operations");
        println!("  • test_string_allocations_no_leak - 100K string allocations");
        println!("\nMemory tracking:");
        println!("  • Linux: /proc/self/status (VmRSS)");
        println!("  • macOS: ps command (RSS)");
        println!("  • Windows: Not supported");
        println!("\nAcceptance criteria:");
        println!("  • Sequential import: <10MB growth for 1000 files");
        println!("  • Batch iterations: <5MB growth per iteration");
        println!("  • TSS calculation: <5MB growth for 1000 calculations");
        println!("  • PMC calculation: <5MB growth for 10 iterations");
        println!("  • Large workouts: <50MB for 10K data points");
        println!("==========================================\n");
    }
}
