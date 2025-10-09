use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput, black_box};
use chrono::NaiveDate;
use rust_decimal_macros::dec;
use std::path::Path;
use trainrs::{models, tss, pmc, zones, power, running, multisport, database, import};

/// Performance benchmarks for training analysis system
///
/// These benchmarks test the performance of core calculations
/// with varying dataset sizes to ensure scalability.

fn bench_tss_calculation(c: &mut Criterion) {
    let athlete = create_benchmark_athlete();

    let mut group = c.benchmark_group("TSS Calculation");

    // Test different workout sizes
    for &size in &[1, 10, 100, 1000] {
        let workouts = create_workout_dataset(size);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("calculate_tss", size),
            &workouts,
            |b, workouts| {
                b.iter(|| {
                    for workout in workouts {
                        let _ = tss::TssCalculator::calculate_tss(workout, &athlete);
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_pmc_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("PMC Calculation");

    let pmc_calculator = pmc::PmcCalculator::new();
    let pmc_config = pmc::PmcConfig::default();

    // Test different dataset sizes
    for &days in &[7, 30, 90, 365] {
        let workouts = create_workout_series(days);

        group.throughput(Throughput::Elements(days as u64));
        group.bench_with_input(
            BenchmarkId::new("calculate_pmc_series", days),
            &workouts,
            |b, workouts| {
                b.iter(|| {
                    let _ = pmc_calculator.calculate_pmc_series(workouts, &pmc_config);
                });
            },
        );
    }

    group.finish();
}

fn bench_power_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("Power Analysis");

    // Test power curve calculation with different data sizes
    for &duration in &[1800, 3600, 7200, 14400] { // 30min to 4 hours
        let workout = create_power_workout(duration);

        group.throughput(Throughput::Elements(duration as u64));
        group.bench_with_input(
            BenchmarkId::new("power_curve", duration),
            &workout,
            |b, workout| {
                b.iter(|| {
                    let _ = power::PowerAnalyzer::calculate_power_curve(workout);
                });
            },
        );
    }

    // Test normalized power calculation
    for &data_points in &[360, 3600, 36000] { // 6min to 10 hours worth of data
        let power_data: Vec<rust_decimal::Decimal> = (0..data_points)
            .map(|i| dec!(200) + rust_decimal::Decimal::from(i % 100))
            .collect();

        group.throughput(Throughput::Elements(data_points as u64));
        group.bench_with_input(
            BenchmarkId::new("normalized_power", data_points),
            &power_data,
            |b, data| {
                b.iter(|| {
                    let _ = tss::TssCalculator::calculate_normalized_power(data);
                });
            },
        );
    }

    group.finish();
}

fn bench_zone_analysis(c: &mut Criterion) {
    let athlete = create_benchmark_athlete();
    let mut group = c.benchmark_group("Zone Analysis");

    // Test zone calculation with different workout types
    let workouts = vec![
        ("cycling_power", create_power_workout(3600)),
        ("cycling_hr", create_hr_workout(3600)),
        ("running_pace", create_running_workout(2700)),
    ];

    for (workout_type, workout) in workouts {
        group.bench_with_input(
            BenchmarkId::new("zone_analysis", workout_type),
            &workout,
            |b, workout| {
                b.iter(|| {
                    match workout.sport {
                        models::Sport::Cycling => {
                            if workout.summary.avg_power.is_some() {
                                let _ = zones::calculate_power_zones(workout, &athlete);
                            }
                            if workout.summary.avg_hr.is_some() {
                                let _ = zones::calculate_heart_rate_zones(workout, &athlete);
                            }
                        },
                        models::Sport::Running => {
                            if workout.summary.avg_speed.is_some() {
                                let _ = zones::calculate_pace_zones(workout, &athlete);
                            }
                            if workout.summary.avg_hr.is_some() {
                                let _ = zones::calculate_heart_rate_zones(workout, &athlete);
                            }
                        },
                        _ => {}
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_running_analysis(c: &mut Criterion) {
    let athlete = create_benchmark_athlete();
    let mut group = c.benchmark_group("Running Analysis");

    for &duration in &[1800, 3600, 5400, 7200] { // 30min to 2 hours
        let workout = create_running_workout_with_elevation(duration);

        group.throughput(Throughput::Elements(duration as u64));
        group.bench_with_input(
            BenchmarkId::new("pace_analysis", duration),
            &workout,
            |b, workout| {
                b.iter(|| {
                    let _ = running::RunningAnalyzer::analyze_pace(workout, &athlete);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("elevation_analysis", duration),
            &workout,
            |b, workout| {
                b.iter(|| {
                    let _ = running::RunningAnalyzer::analyze_elevation(workout);
                });
            },
        );
    }

    group.finish();
}

fn bench_multisport_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("Multisport Analysis");

    // Test with different numbers of workouts across multiple sports
    for &num_workouts in &[10, 50, 100, 500] {
        let workouts = create_multisport_dataset(num_workouts);

        group.throughput(Throughput::Elements(num_workouts as u64));
        group.bench_with_input(
            BenchmarkId::new("combined_load", num_workouts),
            &workouts,
            |b, workouts| {
                b.iter(|| {
                    let _ = multisport::calculate_combined_load(workouts);
                });
            },
        );
    }

    group.finish();
}

fn bench_data_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("Data Serialization");

    // Test JSON export/import performance
    for &num_workouts in &[10, 100, 1000] {
        let workouts = create_workout_dataset(num_workouts);

        group.throughput(Throughput::Elements(num_workouts as u64));
        group.bench_with_input(
            BenchmarkId::new("json_serialize", num_workouts),
            &workouts,
            |b, workouts| {
                b.iter(|| {
                    let _ = serde_json::to_string(workouts);
                });
            },
        );

        // Test deserialization
        let json_data = serde_json::to_string(&workouts).unwrap();
        group.bench_with_input(
            BenchmarkId::new("json_deserialize", num_workouts),
            &json_data,
            |b, json| {
                b.iter(|| {
                    let _: Result<Vec<models::Workout>, _> = serde_json::from_str(json);
                });
            },
        );
    }

    group.finish();
}

fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("Memory Usage");

    // Test memory efficiency with large datasets
    for &size in &[1000, 5000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("large_dataset_creation", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    let workouts = create_workout_dataset(size);
                    std::hint::black_box(workouts);
                });
            },
        );
    }

    group.finish();
}

fn bench_fit_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("FIT File Parsing");

    // Create test FIT file data (simulated)
    // In real scenario, would use actual FIT files from benches/fixtures/
    let test_data_sizes = vec![
        ("small_1mb", 1_000_000),
        ("medium_10mb", 10_000_000),
        ("large_50mb", 50_000_000),
    ];

    for (name, size) in test_data_sizes {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("parse_throughput", name),
            &size,
            |b, &_size| {
                b.iter(|| {
                    // Simulate FIT parsing by creating workout data
                    let workout = create_power_workout(3600);
                    black_box(workout);
                });
            },
        );
    }

    group.finish();
}

fn bench_database_operations(c: &mut Criterion) {
    use tempfile::TempDir;

    let mut group = c.benchmark_group("Database Operations");

    // Test database insert performance
    for &batch_size in &[10, 100, 1000, 10000] {
        let workouts = create_workout_dataset(batch_size);

        group.throughput(Throughput::Elements(batch_size as u64));
        group.bench_with_input(
            BenchmarkId::new("insert_workouts", batch_size),
            &workouts,
            |b, workouts| {
                b.iter_batched(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        let db_path = temp_dir.path().join("bench.db");
                        let db = database::Database::new(&db_path).unwrap();
                        (db, temp_dir)
                    },
                    |(db, _temp_dir)| {
                        for workout in workouts {
                            let _ = db.save_workout(workout);
                        }
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    // Test database query performance
    for &workout_count in &[100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("query_date_range", workout_count),
            &workout_count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        let db_path = temp_dir.path().join("bench.db");
                        let db = database::Database::new(&db_path).unwrap();

                        // Populate database
                        let workouts = create_workout_dataset(count);
                        for workout in &workouts {
                            let _ = db.save_workout(workout);
                        }

                        (db, temp_dir)
                    },
                    |(db, _temp_dir)| {
                        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
                        let end = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
                        black_box(db.get_workouts_by_date_range(&start, &end, None));
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

fn bench_batch_import(c: &mut Criterion) {
    let mut group = c.benchmark_group("Batch Import");

    // Test batch import throughput
    for &file_count in &[10, 50, 100] {
        let workouts: Vec<Vec<models::Workout>> = (0..file_count)
            .map(|i| vec![create_simple_workout(models::Sport::Cycling, 3600, i)])
            .collect();

        group.throughput(Throughput::Elements(file_count as u64));
        group.bench_with_input(
            BenchmarkId::new("import_multiple_files", file_count),
            &workouts,
            |b, workouts| {
                b.iter(|| {
                    for workout_batch in workouts {
                        black_box(workout_batch);
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_time_series_queries(c: &mut Criterion) {
    use tempfile::TempDir;

    let mut group = c.benchmark_group("Time Series Queries");

    // Test time-series query performance with different data densities
    for &days in &[30, 90, 365] {
        group.bench_with_input(
            BenchmarkId::new("aggregate_by_week", days),
            &days,
            |b, &days| {
                b.iter_batched(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        let db_path = temp_dir.path().join("bench.db");
                        let db = database::Database::new(&db_path).unwrap();

                        // Create daily workouts
                        let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
                        for day in 0..days {
                            let mut workout = create_simple_workout(
                                models::Sport::Cycling,
                                3600,
                                day
                            );
                            workout.date = start_date + chrono::Duration::days(day as i64);
                            let _ = db.save_workout(&workout);
                        }

                        (db, temp_dir)
                    },
                    |(db, _temp_dir)| {
                        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
                        let end = start + chrono::Duration::days(days as i64);
                        black_box(db.get_workouts_by_date_range(&start, &end, None));
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

// Helper functions for benchmarks

fn create_benchmark_athlete() -> models::AthleteProfile {
    use chrono::Utc;

    models::AthleteProfile {
        id: "benchmark_athlete".to_string(),
        name: "Benchmark Athlete".to_string(),
        date_of_birth: Some(NaiveDate::from_ymd_opt(1990, 1, 1).unwrap()),
        weight: Some(dec!(70.0)),
        height: Some(175),
        ftp: Some(250),
        lthr: Some(165),
        max_hr: Some(190),
        resting_hr: Some(50),
        threshold_pace: Some(dec!(4.0)), // 4:00 min/km
        training_zones: models::TrainingZones::default(),
        preferred_units: models::Units::Metric,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn create_workout_dataset(size: usize) -> Vec<models::Workout> {
    (0..size)
        .map(|i| {
            let sport = match i % 3 {
                0 => models::Sport::Cycling,
                1 => models::Sport::Running,
                _ => models::Sport::Swimming,
            };

            create_simple_workout(sport, 3600 + (i % 3600) as u32, i)
        })
        .collect()
}

fn create_workout_series(days: usize) -> Vec<models::Workout> {
    let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

    (0..days)
        .filter_map(|day| {
            // Skip some days for rest
            if day % 7 == 6 {
                return None;
            }

            let date = start_date + chrono::Duration::days(day as i64);
            let tss_variation = (day as f64 * 0.1).sin() * 0.3;
            let base_tss = 100.0 + tss_variation * 50.0;

            Some(create_workout_with_tss(
                models::Sport::Cycling,
                3600,
                rust_decimal::Decimal::from_f64(base_tss).unwrap_or(dec!(100)),
                day,
            ))
        })
        .collect()
}

fn create_simple_workout(sport: models::Sport, duration: u32, seed: usize) -> models::Workout {
    use chrono::Utc;

    let variation = (seed as f64 * 0.1).sin();

    models::Workout {
        id: format!("bench_workout_{}", seed),
        athlete_id: Some("benchmark_athlete".to_string()),
        date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap() + chrono::Duration::days((seed % 365) as i64),
        sport,
        workout_type: models::WorkoutType::Endurance,
        duration_seconds: duration,
        summary: models::WorkoutSummary {
            total_distance: Some(dec!(30.0) + rust_decimal::Decimal::from_f64(variation * 10.0).unwrap_or_default()),
            avg_heart_rate: Some(150 + (variation * 20.0) as u16),
            max_heart_rate: Some(180),
            avg_power: Some(200 + (variation * 50.0) as u16),
            normalized_power: Some(220),
            avg_pace: Some(dec!(4.0)),
            intensity_factor: Some(dec!(0.8)),
            tss: Some(dec!(100) + rust_decimal::Decimal::from_f64(variation * 50.0).unwrap_or_default()),
            elevation_gain: Some(100),
            avg_cadence: Some(90),
            calories: Some((duration / 60 * 12) as u16),
        },
        data_source: models::DataSource::Power,
        notes: Some(format!("Benchmark Workout {}", seed)),
        source: None,
        raw_data: None,
    }
}

fn create_workout_with_tss(sport: models::Sport, duration: u32, tss: rust_decimal::Decimal, seed: usize) -> models::Workout {
    let mut workout = create_simple_workout(sport, duration, seed);
    workout.summary.tss = Some(tss);
    workout
}

fn create_power_workout(duration: u32) -> models::Workout {
    let mut workout = create_simple_workout(models::Sport::Cycling, duration, 0);
    workout.summary.avg_power = Some(250);
    workout.summary.normalized_power = Some(270);
    workout
}

fn create_hr_workout(duration: u32) -> models::Workout {
    let mut workout = create_simple_workout(models::Sport::Cycling, duration, 0);
    workout.summary.avg_heart_rate = Some(160);
    workout.summary.max_heart_rate = Some(185);
    workout
}

fn create_running_workout(duration: u32) -> models::Workout {
    let mut workout = create_simple_workout(models::Sport::Running, duration, 0);
    workout.summary.avg_pace = Some(dec!(5.0)); // 5:00 min/km pace
    workout.summary.total_distance = Some(dec!(10.0));
    workout
}

fn create_running_workout_with_elevation(duration: u32) -> models::Workout {
    let mut workout = create_running_workout(duration);
    workout.summary.elevation_gain = Some(300);
    workout
}

fn create_multisport_dataset(size: usize) -> Vec<models::Workout> {
    (0..size)
        .map(|i| {
            let sport = match i % 3 {
                0 => models::Sport::Cycling,
                1 => models::Sport::Running,
                _ => models::Sport::Swimming,
            };

            create_simple_workout(sport, 3600, i)
        })
        .collect()
}

// Define benchmark groups
criterion_group!(
    benches,
    bench_tss_calculation,
    bench_pmc_calculation,
    bench_power_analysis,
    bench_zone_analysis,
    bench_running_analysis,
    bench_multisport_analysis,
    bench_data_serialization,
    bench_memory_usage,
    bench_fit_parsing,
    bench_database_operations,
    bench_batch_import,
    bench_time_series_queries
);

criterion_main!(benches);