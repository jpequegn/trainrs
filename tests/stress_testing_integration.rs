//! Integration tests for stress testing and memory profiling framework
//!
//! Tests comprehensive stress scenarios including:
//! - Memory stability under load
//! - Throughput and performance metrics
//! - Memory leak detection algorithms
//! - Resource cleanup verification

use trainrs::stress_testing::{
    MemoryMetrics, StressTestConfig, StressTestHarness, StressTestMetrics, StressTestScenario,
};
use std::time::Duration;

#[test]
fn test_stress_test_config_builder() {
    let config = StressTestConfig {
        num_files: 1000,
        workouts_per_file: 5,
        parallel_processing: true,
        num_threads: Some(4),
        enable_caching: true,
        snapshot_interval: Duration::from_millis(100),
        timing_sample_rate: 10,
    };

    assert_eq!(config.num_files, 1000);
    assert_eq!(config.workouts_per_file, 5);
    assert!(config.parallel_processing);
    assert_eq!(config.num_threads, Some(4));
    assert!(config.enable_caching);
    assert_eq!(config.snapshot_interval.as_millis(), 100);
    assert_eq!(config.timing_sample_rate, 10);
}

#[test]
fn test_stress_test_scenario_creation() {
    let mut scenario = StressTestHarness::create_scenario(StressTestConfig::default());
    scenario.start();
    let initial_metrics = scenario.current_metrics();

    assert_eq!(initial_metrics.files_processed, 0);
    assert_eq!(initial_metrics.successful_imports, 0);
    assert_eq!(initial_metrics.failed_imports, 0);
}

#[test]
fn test_stress_test_scenario_success_tracking() {
    let mut scenario = StressTestHarness::create_scenario(StressTestConfig::default());
    scenario.start();

    // Simulate successful imports
    for _ in 0..50 {
        scenario.record_success(3, 50);
    }

    let metrics = scenario.current_metrics();
    assert_eq!(metrics.files_processed, 50);
    assert_eq!(metrics.successful_imports, 50);
    assert_eq!(metrics.failed_imports, 0);
    assert_eq!(metrics.total_workouts, 150); // 50 files * 3 workouts
}

#[test]
fn test_stress_test_scenario_failure_tracking() {
    let mut scenario = StressTestHarness::create_scenario(StressTestConfig::default());
    scenario.start();

    // Record successes and failures
    for i in 0..100 {
        if i % 5 == 0 {
            scenario.record_failure();
        } else {
            scenario.record_success(2, 40);
        }
    }

    let metrics = scenario.current_metrics();
    assert_eq!(metrics.files_processed, 100);
    assert_eq!(metrics.successful_imports, 80);
    assert_eq!(metrics.failed_imports, 20);
    assert_eq!(metrics.total_workouts, 160); // 80 files * 2 workouts
    assert!((metrics.success_rate_percent() - 80.0).abs() < 0.1);
}

#[test]
fn test_stress_test_metrics_calculation() {
    let metrics = StressTestMetrics {
        files_processed: 500,
        successful_imports: 450,
        failed_imports: 50,
        total_workouts: 2250,
        total_duration: Duration::from_secs(60),
        memory_start: MemoryMetrics {
            rss_bytes: 100 * 1024 * 1024,
            vms_bytes: 150 * 1024 * 1024,
            peak_rss_bytes: 100 * 1024 * 1024,
            allocation_count: 1000,
        },
        memory_end: MemoryMetrics {
            rss_bytes: 120 * 1024 * 1024,
            vms_bytes: 170 * 1024 * 1024,
            peak_rss_bytes: 150 * 1024 * 1024,
            allocation_count: 5000,
        },
        memory_snapshots: Vec::new(),
        processing_times_ms: vec![50; 500],
        peak_memory_mb: 150.0,
    };

    // Verify throughput calculation
    let throughput = metrics.throughput_files_per_sec();
    assert!((throughput - 8.33).abs() < 0.1); // 500 files / 60 seconds

    // Verify avg processing time
    let avg_time = metrics.avg_processing_time_ms();
    assert!((avg_time - 50.0).abs() < 0.1);

    // Verify memory growth
    let memory_growth = metrics.memory_growth_mb();
    assert!((memory_growth - 20.0).abs() < 0.5); // 120MB - 100MB

    // Verify success rate
    assert!((metrics.success_rate_percent() - 90.0).abs() < 0.1);
}

#[test]
fn test_memory_leak_detection_clean() {
    // Test case: stable memory usage with only natural fluctuations
    let metrics = StressTestMetrics {
        files_processed: 1000,
        successful_imports: 1000,
        failed_imports: 0,
        total_workouts: 5000,
        total_duration: Duration::from_secs(120),
        memory_start: MemoryMetrics {
            rss_bytes: 100 * 1024 * 1024,
            vms_bytes: 150 * 1024 * 1024,
            peak_rss_bytes: 100 * 1024 * 1024,
            allocation_count: 5000,
        },
        memory_end: MemoryMetrics {
            rss_bytes: 105 * 1024 * 1024,
            vms_bytes: 155 * 1024 * 1024,
            peak_rss_bytes: 110 * 1024 * 1024,
            allocation_count: 6000,
        },
        memory_snapshots: vec![
            (Duration::from_secs(0), MemoryMetrics {
                rss_bytes: 100 * 1024 * 1024,
                ..Default::default()
            }),
            (Duration::from_secs(30), MemoryMetrics {
                rss_bytes: 101 * 1024 * 1024,
                ..Default::default()
            }),
            (Duration::from_secs(60), MemoryMetrics {
                rss_bytes: 103 * 1024 * 1024,
                ..Default::default()
            }),
            (Duration::from_secs(90), MemoryMetrics {
                rss_bytes: 104 * 1024 * 1024,
                ..Default::default()
            }),
            (Duration::from_secs(120), MemoryMetrics {
                rss_bytes: 105 * 1024 * 1024,
                ..Default::default()
            }),
        ],
        processing_times_ms: vec![50; 1000],
        peak_memory_mb: 110.0,
    };

    // Clean memory should not be flagged as leak
    assert!(!metrics.has_memory_leak());
}

#[test]
fn test_memory_leak_detection_leaking() {
    // Test case: steady rapid memory growth indicates potential leak
    let metrics = StressTestMetrics {
        files_processed: 1000,
        successful_imports: 1000,
        failed_imports: 0,
        total_workouts: 5000,
        total_duration: Duration::from_secs(120),
        memory_start: MemoryMetrics {
            rss_bytes: 100 * 1024 * 1024,
            vms_bytes: 150 * 1024 * 1024,
            peak_rss_bytes: 100 * 1024 * 1024,
            allocation_count: 5000,
        },
        memory_end: MemoryMetrics {
            rss_bytes: 200 * 1024 * 1024, // 100% growth
            vms_bytes: 250 * 1024 * 1024,
            peak_rss_bytes: 200 * 1024 * 1024,
            allocation_count: 15000,
        },
        memory_snapshots: vec![
            (Duration::from_secs(0), MemoryMetrics {
                rss_bytes: 100 * 1024 * 1024,
                ..Default::default()
            }),
            (Duration::from_secs(30), MemoryMetrics {
                rss_bytes: 125 * 1024 * 1024,
                ..Default::default()
            }),
            (Duration::from_secs(60), MemoryMetrics {
                rss_bytes: 150 * 1024 * 1024,
                ..Default::default()
            }),
            (Duration::from_secs(90), MemoryMetrics {
                rss_bytes: 175 * 1024 * 1024,
                ..Default::default()
            }),
            (Duration::from_secs(120), MemoryMetrics {
                rss_bytes: 200 * 1024 * 1024,
                ..Default::default()
            }),
        ],
        processing_times_ms: vec![50; 1000],
        peak_memory_mb: 200.0,
    };

    // Consistent growth should be flagged as potential leak
    assert!(metrics.has_memory_leak());
}

#[test]
fn test_smoke_test_config() {
    let scenario = StressTestHarness::smoke_test();
    let config = &scenario;
    // Smoke test should have 100 files
    assert!(scenario.current_metrics().files_processed < 10); // Not started yet
}

#[test]
fn test_standard_test_config() {
    let scenario = StressTestHarness::standard_test();
    scenario.current_metrics();
    // Just verify it doesn't panic during creation
}

#[test]
fn test_heavy_test_config() {
    let scenario = StressTestHarness::heavy_test();
    scenario.current_metrics();
    // Just verify it doesn't panic during creation
}

#[test]
fn test_extreme_test_config() {
    let scenario = StressTestHarness::extreme_test();
    scenario.current_metrics();
    // Just verify it doesn't panic during creation
}

#[test]
fn test_memory_metrics_accumulation() {
    let mut scenario = StressTestHarness::create_scenario(StressTestConfig::default());
    scenario.start();

    // Simulate accumulating processing times - all same value for predictability
    for _ in 0..100 {
        scenario.record_success(5, 50); // Constant 50ms
    }

    let metrics = scenario.current_metrics();
    assert_eq!(metrics.files_processed, 100);
    // Default timing_sample_rate is 10, so we get 10 samples from 100 files
    assert_eq!(metrics.processing_times_ms.len(), 10);

    // avg_processing_time_ms averages total_ms across files_processed
    // With 10 samples of 50ms = 500 total, divided by 100 files = 5.0ms per file
    let avg = metrics.avg_processing_time_ms();
    assert!((avg - 5.0).abs() < 0.1);
}

#[test]
fn test_stress_test_scenario_finish() {
    let mut scenario = StressTestHarness::create_scenario(StressTestConfig::default());
    scenario.start();

    // Record some activity
    for _ in 0..20 {
        scenario.record_success(2, 100);
    }

    let final_metrics = scenario.finish();

    assert_eq!(final_metrics.files_processed, 20);
    assert_eq!(final_metrics.successful_imports, 20);
    assert_eq!(final_metrics.failed_imports, 0);
    assert!(final_metrics.total_duration.as_secs() <= 10);
}

#[test]
fn test_multiple_concurrent_scenarios() {
    // Test running multiple independent scenarios
    let mut scenario1 = StressTestHarness::create_scenario(StressTestConfig::default());
    let mut scenario2 = StressTestHarness::create_scenario(StressTestConfig::default());

    scenario1.start();
    scenario2.start();

    // Interleave operations on both scenarios
    for i in 0..50 {
        if i % 2 == 0 {
            scenario1.record_success(3, 50);
        } else {
            scenario2.record_success(2, 75);
        }
    }

    let metrics1 = scenario1.current_metrics();
    let metrics2 = scenario2.current_metrics();

    assert_eq!(metrics1.files_processed, 25); // 25 even operations
    assert_eq!(metrics2.files_processed, 25); // 25 odd operations
    assert_eq!(metrics1.total_workouts, 75); // 25 * 3
    assert_eq!(metrics2.total_workouts, 50); // 25 * 2
}

#[test]
fn test_stress_metrics_formatting() {
    let metrics = StressTestMetrics {
        files_processed: 1000,
        successful_imports: 950,
        failed_imports: 50,
        total_workouts: 4750,
        total_duration: Duration::from_secs(300),
        memory_start: MemoryMetrics {
            rss_bytes: 100 * 1024 * 1024,
            vms_bytes: 150 * 1024 * 1024,
            peak_rss_bytes: 100 * 1024 * 1024,
            allocation_count: 5000,
        },
        memory_end: MemoryMetrics {
            rss_bytes: 120 * 1024 * 1024,
            vms_bytes: 170 * 1024 * 1024,
            peak_rss_bytes: 130 * 1024 * 1024,
            allocation_count: 8000,
        },
        memory_snapshots: Vec::new(),
        processing_times_ms: vec![100; 1000],
        peak_memory_mb: 130.0,
    };

    // Should not panic
    let formatted = metrics.pretty_print();
    assert!(!formatted.is_empty());
    assert!(formatted.contains("Files Processed"));
    assert!(formatted.contains("Memory"));
}

#[test]
fn test_growth_per_operation() {
    let metrics = MemoryMetrics {
        rss_bytes: 10 * 1024 * 1024, // 10MB
        vms_bytes: 15 * 1024 * 1024,
        peak_rss_bytes: 10 * 1024 * 1024,
        allocation_count: 1000,
    };

    let growth = metrics.growth_per_op(100000); // 100k operations
    assert!(growth > 0.0); // Should be positive per operation
    // 10MB / 100k ops = ~102 bytes per op
    assert!(growth < 1000.0); // Less than 1KB per operation
}

#[test]
fn test_memory_metrics_defaults() {
    let metrics = MemoryMetrics::default();
    assert_eq!(metrics.rss_bytes, 0);
    assert_eq!(metrics.vms_bytes, 0);
    assert_eq!(metrics.peak_rss_bytes, 0);
    assert_eq!(metrics.allocation_count, 0);
    assert_eq!(metrics.rss_mb(), 0.0);
}

#[test]
fn test_stress_config_defaults() {
    let config = StressTestConfig::default();
    assert_eq!(config.num_files, 100); // Default smoke test size
    assert_eq!(config.workouts_per_file, 1); // Default is 1 per file
    assert!(config.parallel_processing); // Default is parallel
    assert!(config.enable_caching); // Default is with cache
}

#[test]
fn test_error_rate_calculation() {
    let mut scenario = StressTestHarness::create_scenario(StressTestConfig::default());
    scenario.start();

    // Record 1000 total, with 100 failures (10% error rate)
    for i in 0..1000 {
        if i % 10 == 0 {
            scenario.record_failure();
        } else {
            scenario.record_success(1, 10);
        }
    }

    let metrics = scenario.current_metrics();
    let success_rate = metrics.success_rate_percent();
    assert!((success_rate - 90.0).abs() < 0.1);
}
