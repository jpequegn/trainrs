//! Integration tests for comprehensive error reporting and logging
//!
//! Tests error scenarios and logging output for FIT import operations

use trainrs::import::logging::{ImportLogger, OperationType, LogSeverity};
use std::path::Path;

#[test]
fn test_file_parsing_error_logging() {
    let mut logger = ImportLogger::new("file_parse_session");
    let file_path = Path::new("corrupted.fit");

    logger.begin_operation(OperationType::Parsing, Some(file_path));
    logger.log_error(
        OperationType::Parsing,
        Some(file_path),
        "PARSE_001",
        "Invalid FIT file header: unexpected magic number",
        false,
    );

    let entries = logger.get_entries();
    assert_eq!(entries.len(), 2); // Started + Error

    let error_entry = &entries[1];
    assert_eq!(error_entry.severity, LogSeverity::Error);
    assert_eq!(error_entry.file_path, Some(file_path.to_path_buf()));
}

#[test]
fn test_validation_error_recovery() {
    let mut logger = ImportLogger::new("validation_recovery_session");
    let file_path = Path::new("incomplete.fit");

    logger.begin_operation(OperationType::Validation, Some(file_path));

    // Log missing field validation issue
    logger.log_validation_issue(
        OperationType::Validation,
        Some(file_path),
        "avg_heart_rate",
        "null",
        "Required field is missing",
    );

    // Attempt recovery
    logger.log_recovery_action(
        OperationType::Validation,
        Some(file_path),
        "Using estimated heart rate from max heart rate",
    );

    logger.log_success(OperationType::Validation, Some(file_path), 150, Some(3600));

    let entries = logger.get_entries();
    assert!(entries.len() >= 3);

    let summary = logger.get_summary();
    assert_eq!(summary.warnings, 1); // Validation issue
    assert_eq!(summary.successes, 1); // Completion
}

#[test]
fn test_multi_file_error_tracking() {
    let mut logger = ImportLogger::new("batch_import_session");

    // File 1 - Success
    let file1 = Path::new("workout1.fit");
    logger.begin_operation(OperationType::Parsing, Some(file1));
    logger.log_checkpoint(OperationType::Parsing, Some(file1), "Header validation passed");
    logger.log_checkpoint(OperationType::Parsing, Some(file1), "Data parsing complete");
    logger.log_success(OperationType::Parsing, Some(file1), 100, Some(3600));

    // File 2 - Error
    let file2 = Path::new("workout2.fit");
    logger.begin_operation(OperationType::Parsing, Some(file2));
    logger.log_error(
        OperationType::Parsing,
        Some(file2),
        "PARSE_002",
        "Corrupted data section at byte 1024",
        false,
    );

    // File 3 - Recovery
    let file3 = Path::new("workout3.fit");
    logger.begin_operation(OperationType::Parsing, Some(file3));
    logger.log_error(
        OperationType::Parsing,
        Some(file3),
        "PARSE_003",
        "Checksum mismatch",
        true,
    );
    logger.log_recovery_action(
        OperationType::Parsing,
        Some(file3),
        "Skipped checksum verification for device quirk",
    );
    logger.log_success(OperationType::Parsing, Some(file3), 120, Some(3600));

    let summary = logger.get_summary();
    assert_eq!(summary.total_entries, 10);
    assert_eq!(summary.files_processed.len(), 3);
    assert_eq!(summary.errors, 1);
    assert_eq!(summary.successes, 2);

    // Check file-specific entries
    let file2_entries = logger.get_entries_for_file(file2);
    assert!(file2_entries.iter().any(|e| e.severity == LogSeverity::Error));
}

#[test]
fn test_error_severity_escalation() {
    let mut logger = ImportLogger::new("severity_escalation_session");
    let file_path = Path::new("problematic.fit");

    // Start with warning
    logger.log_validation_issue(
        OperationType::Validation,
        Some(file_path),
        "power",
        "0",
        "Zero power recorded at 12:34:56",
    );

    // Escalate to error
    logger.log_error(
        OperationType::Parsing,
        Some(file_path),
        "PARSE_004",
        "Multiple validation failures exceed threshold",
        false,
    );

    let entries = logger.get_entries_by_severity(LogSeverity::Error);
    assert_eq!(entries.len(), 1);
    assert!(entries[0].severity >= LogSeverity::Error);
}

#[test]
fn test_summary_report_generation() {
    let mut logger = ImportLogger::new("report_session");
    let file1 = Path::new("file1.fit");
    let file2 = Path::new("file2.fit");

    // Simulate processing
    logger.log_success(OperationType::Import, Some(file1), 200, Some(1000));
    logger.log_error(OperationType::Import, Some(file2), "E001", "Parse failed", false);

    let summary = logger.get_summary();
    let report = summary.to_string_pretty();

    assert!(report.contains("Import Summary"));
    assert!(report.contains("Files: 2"));
    assert!(report.contains("Errors: 1"));
    assert!(report.contains("Successes: 1"));
}

#[test]
fn test_json_export_completeness() {
    let mut logger = ImportLogger::new("export_session");
    let file_path = Path::new("test.fit");

    logger.begin_operation(OperationType::Parsing, Some(file_path));
    logger.log_validation_issue(
        OperationType::Validation,
        Some(file_path),
        "cadence",
        "150",
        "Unusually high cadence",
    );
    logger.log_recovery_action(
        OperationType::Recovery,
        Some(file_path),
        "Capped cadence at device maximum",
    );

    let json = logger.export_json().expect("JSON export failed");

    // Verify JSON contains expected keys
    assert!(json.contains("timestamp"));
    assert!(json.contains("operation"));
    assert!(json.contains("severity"));
    assert!(json.contains("event"));
    assert!(json.contains("file_path"));
}

#[test]
fn test_performance_impact() {
    use std::time::Instant;

    let mut logger = ImportLogger::new("perf_test_session");
    let file_path = Path::new("perf_test.fit");

    let start = Instant::now();

    // Log 100 events
    for i in 0..100 {
        if i % 10 == 0 {
            logger.log_success(OperationType::Parsing, Some(file_path), 50, Some(100));
        } else {
            logger.log_checkpoint(
                OperationType::Parsing,
                Some(file_path),
                &format!("Processed chunk {}", i),
            );
        }
    }

    let duration = start.elapsed();

    // Logging 100 events should be very fast (<100ms on modern hardware)
    assert!(duration.as_millis() < 100, "Logging took too long: {:?}", duration);

    // Verify all events were logged
    let entries = logger.get_entries();
    assert_eq!(entries.len(), 100);
}

#[test]
fn test_concurrent_file_tracking() {
    let mut logger = ImportLogger::new("concurrent_session");

    // Simulate processing multiple files in sequence
    let files = vec!["file1.fit", "file2.fit", "file3.fit", "file4.fit"];

    for (idx, filename) in files.iter().enumerate() {
        let file = Path::new(filename);
        logger.begin_operation(OperationType::Parsing, Some(file));

        if idx % 2 == 0 {
            logger.log_success(OperationType::Parsing, Some(file), 100 + (idx as u64 * 10), Some(3600));
        } else {
            logger.log_error(
                OperationType::Parsing,
                Some(file),
                "E001",
                "Test error",
                true,
            );
        }
    }

    let summary = logger.get_summary();
    assert_eq!(summary.files_processed.len(), 4);
    assert_eq!(summary.errors, 0); // Recovered errors show as warnings
    assert_eq!(summary.warnings, 2); // Recovered errors
    assert_eq!(summary.successes, 2);
}
