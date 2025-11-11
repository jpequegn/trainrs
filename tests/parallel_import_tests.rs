//! Integration tests for parallel FIT file import
//!
//! Tests parallel processing capabilities including:
//! - Concurrent file handling
//! - Error recovery in parallel context
//! - Performance benchmarking
//! - Throughput measurement

use trainrs::import::parallel::{ParallelImporter, ParallelImportConfig};
use std::path::PathBuf;

#[test]
fn test_parallel_importer_creation() {
    let importer = ParallelImporter::new();
    assert!(importer.config.show_progress);
    assert!(importer.config.continue_on_error);
}

#[test]
fn test_parallel_config_custom() {
    let config = ParallelImportConfig {
        num_threads: Some(4),
        show_progress: false,
        batch_size: Some(10),
        continue_on_error: true,
    };

    assert_eq!(config.num_threads, Some(4));
    assert!(!config.show_progress);
    assert_eq!(config.batch_size, Some(10));
}

#[test]
fn test_parallel_importer_with_config() {
    let config = ParallelImportConfig {
        num_threads: Some(2),
        show_progress: false,
        batch_size: None,
        continue_on_error: true,
    };

    let importer = ParallelImporter::with_config(config);
    assert_eq!(importer.config.num_threads, Some(2));
}

#[test]
fn test_parallel_import_empty_directory() {
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let importer = ParallelImporter::new();

    let result = importer.import_directory(temp_dir.path());
    assert!(result.is_ok());

    let (workouts, summary) = result.unwrap();
    assert_eq!(workouts.len(), 0);
    assert_eq!(summary.total_files, 0);
    assert!(summary.is_fully_successful());
}

#[test]
fn test_parallel_summary_calculations() {
    let summary = trainrs::import::parallel::ParallelImportSummary {
        total_files: 20,
        successful_files: 20,
        failed_files: 0,
        total_workouts: 100,
        total_duration_ms: 2000,
        results: Vec::new(),
        errors: Vec::new(),
    };

    // Throughput: 20 files / 2 sec = 10 files/sec
    assert_eq!(summary.throughput_files_per_sec(), 10.0);

    // Avg time: 2000ms / 20 = 100ms per file
    assert_eq!(summary.avg_time_per_file_ms(), 100.0);

    assert!(summary.is_fully_successful());
}

#[test]
fn test_parallel_summary_with_failures() {
    let summary = trainrs::import::parallel::ParallelImportSummary {
        total_files: 20,
        successful_files: 18,
        failed_files: 2,
        total_workouts: 90,
        total_duration_ms: 2000,
        results: Vec::new(),
        errors: vec![(
            PathBuf::from("file1.fit"),
            "Parse error".to_string(),
        )],
    };

    assert!(!summary.is_fully_successful());
    assert_eq!(summary.failed_files, 2);
    assert_eq!(summary.errors.len(), 1);
}

#[test]
fn test_parallel_summary_pretty_print() {
    let summary = trainrs::import::parallel::ParallelImportSummary {
        total_files: 10,
        successful_files: 10,
        failed_files: 0,
        total_workouts: 50,
        total_duration_ms: 1000,
        results: Vec::new(),
        errors: Vec::new(),
    };

    let output = summary.to_string_pretty();

    assert!(output.contains("Parallel Import Summary"));
    assert!(output.contains("Total Files: 10"));
    assert!(output.contains("Successful: 10"));
    assert!(output.contains("Failed: 0"));
    assert!(output.contains("Total Workouts: 50"));
    assert!(output.contains("Throughput"));
}

#[test]
fn test_parallel_summary_throughput_zero_duration() {
    let summary = trainrs::import::parallel::ParallelImportSummary {
        total_files: 10,
        successful_files: 10,
        failed_files: 0,
        total_workouts: 50,
        total_duration_ms: 0,
        results: Vec::new(),
        errors: Vec::new(),
    };

    // Should handle division by zero gracefully
    assert_eq!(summary.throughput_files_per_sec(), 0.0);
}

#[test]
fn test_parallel_summary_avg_time_zero_successful() {
    let summary = trainrs::import::parallel::ParallelImportSummary {
        total_files: 10,
        successful_files: 0,
        failed_files: 10,
        total_workouts: 0,
        total_duration_ms: 1000,
        results: Vec::new(),
        errors: Vec::new(),
    };

    // Should handle division by zero gracefully
    assert_eq!(summary.avg_time_per_file_ms(), 0.0);
}

#[test]
fn test_file_import_result_success() {
    let result = trainrs::import::parallel::FileImportResult {
        file_path: PathBuf::from("test.fit"),
        workouts: vec![],
        duration_ms: 100,
        success: true,
        error: None,
    };

    assert!(result.success);
    assert!(result.error.is_none());
    assert_eq!(result.duration_ms, 100);
}

#[test]
fn test_file_import_result_failure() {
    let result = trainrs::import::parallel::FileImportResult {
        file_path: PathBuf::from("corrupted.fit"),
        workouts: vec![],
        duration_ms: 50,
        success: false,
        error: Some("Invalid file format".to_string()),
    };

    assert!(!result.success);
    assert!(result.error.is_some());
    assert_eq!(result.error.unwrap(), "Invalid file format");
}

#[test]
fn test_parallel_import_with_default_config() {
    let importer = ParallelImporter::default();
    assert!(importer.config.continue_on_error);
}

#[test]
fn test_parallel_summary_performance_metrics() {
    // Test case: 50 files, 2500 workouts, 5 seconds
    let summary = trainrs::import::parallel::ParallelImportSummary {
        total_files: 50,
        successful_files: 50,
        failed_files: 0,
        total_workouts: 2500,
        total_duration_ms: 5000,
        results: Vec::new(),
        errors: Vec::new(),
    };

    // Throughput: 50 files / 5 sec = 10 files/sec
    assert_eq!(summary.throughput_files_per_sec(), 10.0);

    // Avg time: 5000ms / 50 = 100ms per file
    assert_eq!(summary.avg_time_per_file_ms(), 100.0);

    // Verify workouts per file
    assert_eq!(summary.total_workouts / summary.total_files, 50);
}

#[test]
fn test_parallel_import_non_existent_directory() {
    let importer = ParallelImporter::new();
    let non_existent = PathBuf::from("/non/existent/path");

    let result = importer.import_directory(&non_existent);
    assert!(result.is_err());
}

#[test]
fn test_parallel_import_file_list_empty() {
    let importer = ParallelImporter::new();
    let files = vec![];

    let result = importer.import_files(&files);
    assert!(result.is_ok());

    let (workouts, summary) = result.unwrap();
    assert_eq!(workouts.len(), 0);
    assert_eq!(summary.total_files, 0);
}
