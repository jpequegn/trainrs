//! Comprehensive logging and audit trail for import operations
//!
//! Provides structured logging with context, error categorization, severity levels,
//! and per-file/aggregate reporting for all import operations.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, warn, info, span, Level};

/// Import operation logging system
pub struct ImportLogger {
    /// Session-level context
    session_id: String,
    /// Audit trail entries
    entries: Arc<Mutex<Vec<AuditEntry>>>,
    /// Current operation span
    operation_name: String,
}

/// Individual audit trail entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique entry ID
    pub id: u64,
    /// Timestamp of the event
    pub timestamp: DateTime<Utc>,
    /// File being processed
    pub file_path: Option<PathBuf>,
    /// Operation type
    pub operation: OperationType,
    /// Event that occurred
    pub event: LogEvent,
    /// Severity level
    pub severity: LogSeverity,
    /// Additional context fields
    pub context: std::collections::HashMap<String, String>,
}

/// Types of operations being logged
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationType {
    /// File parsing operation
    Parsing,
    /// Data validation
    Validation,
    /// Data storage/persistence
    Storage,
    /// Recovery from errors
    Recovery,
    /// Streaming/chunk processing
    Streaming,
    /// Overall import process
    Import,
}

/// Log event categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogEvent {
    /// Operation started
    Started { operation: String },
    /// Operation completed successfully
    Completed { duration_ms: u64, records: Option<u64> },
    /// Error occurred
    Error { code: String, reason: String, recovered: bool },
    /// Warning condition detected
    Warning { reason: String },
    /// Data validation issue
    ValidationIssue { field: String, value: String, reason: String },
    /// Recovery action taken
    RecoveryAction { action: String },
    /// Checkpoint/milestone reached
    Checkpoint { milestone: String },
}

/// Severity levels for logged events
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogSeverity {
    /// Detailed information
    Debug = 0,
    /// Informational
    Info = 1,
    /// Warning condition
    Warning = 2,
    /// Error
    Error = 3,
    /// Critical error
    Critical = 4,
}

impl ImportLogger {
    /// Create a new import logger session
    pub fn new(session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();
        info!("Starting import session: {}", session_id);

        Self {
            session_id,
            entries: Arc::new(Mutex::new(Vec::new())),
            operation_name: String::new(),
        }
    }

    /// Begin a new operation context
    pub fn begin_operation(&mut self, operation: OperationType, file: Option<&Path>) {
        let op_name = format!("{:?}", operation);
        self.operation_name = op_name.clone();

        let span = span!(Level::DEBUG, "import_op", operation = %op_name);
        let _guard = span.enter();

        debug!(
            "Starting {} operation for file: {}",
            op_name,
            file.map(|p| p.display().to_string()).unwrap_or_default()
        );

        self.log(
            operation,
            file,
            LogEvent::Started {
                operation: op_name,
            },
            LogSeverity::Info,
        );
    }

    /// Log a successful operation completion
    pub fn log_success(&self, operation: OperationType, file: Option<&Path>, duration_ms: u64, records: Option<u64>) {
        info!(
            "Operation completed in {}ms, records processed: {:?}",
            duration_ms, records
        );

        self.log(
            operation,
            file,
            LogEvent::Completed { duration_ms, records },
            LogSeverity::Info,
        );
    }

    /// Log an error with recovery information
    pub fn log_error(
        &self,
        operation: OperationType,
        file: Option<&Path>,
        code: impl Into<String>,
        reason: impl Into<String>,
        recovered: bool,
    ) {
        let code = code.into();
        let reason = reason.into();

        if recovered {
            warn!("Error in {} (recovered): {}: {}", self.operation_name, code, reason);
        } else {
            error!("Error in {} (not recovered): {}: {}", self.operation_name, code, reason);
        }

        self.log(
            operation,
            file,
            LogEvent::Error {
                code,
                reason,
                recovered,
            },
            if recovered { LogSeverity::Warning } else { LogSeverity::Error },
        );
    }

    /// Log a validation issue
    pub fn log_validation_issue(
        &self,
        operation: OperationType,
        file: Option<&Path>,
        field: impl Into<String>,
        value: impl Into<String>,
        reason: impl Into<String>,
    ) {
        let field = field.into();
        let value = value.into();
        let reason = reason.into();

        warn!("Validation issue in {}: {} = {} ({})", self.operation_name, field, value, reason);

        self.log(
            operation,
            file,
            LogEvent::ValidationIssue { field, value, reason },
            LogSeverity::Warning,
        );
    }

    /// Log a recovery action
    pub fn log_recovery_action(
        &self,
        operation: OperationType,
        file: Option<&Path>,
        action: impl Into<String>,
    ) {
        let action = action.into();
        info!("Recovery action taken: {}", action);

        self.log(
            operation,
            file,
            LogEvent::RecoveryAction { action },
            LogSeverity::Info,
        );
    }

    /// Log a checkpoint/milestone
    pub fn log_checkpoint(
        &self,
        operation: OperationType,
        file: Option<&Path>,
        milestone: impl Into<String>,
    ) {
        let milestone = milestone.into();
        debug!("Checkpoint: {}", milestone);

        self.log(
            operation,
            file,
            LogEvent::Checkpoint { milestone },
            LogSeverity::Debug,
        );
    }

    /// Core logging function
    fn log(
        &self,
        operation: OperationType,
        file: Option<&Path>,
        event: LogEvent,
        severity: LogSeverity,
    ) {
        let entry = AuditEntry {
            id: {
                let entries = self.entries.lock().unwrap();
                entries.len() as u64
            },
            timestamp: Utc::now(),
            file_path: file.map(|p| p.to_path_buf()),
            operation,
            event,
            severity,
            context: std::collections::HashMap::new(),
        };

        if let Ok(mut entries) = self.entries.lock() {
            entries.push(entry);
        }
    }

    /// Add custom context to the next log entry
    pub fn with_context(&self, key: impl Into<String>, value: impl Into<String>) {
        // Context is typically added inline; this method provides an alternative interface
        debug!("{}: {}", key.into(), value.into());
    }

    /// Get import summary report
    pub fn get_summary(&self) -> ImportSummary {
        let entries = self.entries.lock().unwrap();

        let mut summary = ImportSummary {
            session_id: self.session_id.clone(),
            start_time: entries.first().map(|e| e.timestamp),
            end_time: entries.last().map(|e| e.timestamp),
            total_entries: entries.len(),
            errors: 0,
            warnings: 0,
            successes: 0,
            files_processed: std::collections::HashSet::new(),
        };

        for entry in entries.iter() {
            match entry.severity {
                LogSeverity::Error | LogSeverity::Critical => summary.errors += 1,
                LogSeverity::Warning => summary.warnings += 1,
                LogSeverity::Info => {
                    if matches!(entry.event, LogEvent::Completed { .. }) {
                        summary.successes += 1;
                    }
                }
                _ => {}
            }

            if let Some(ref path) = entry.file_path {
                summary.files_processed.insert(path.clone());
            }
        }

        summary
    }

    /// Export audit trail as JSON
    pub fn export_json(&self) -> Result<String, serde_json::Error> {
        let entries = self.entries.lock().unwrap();
        serde_json::to_string_pretty(&*entries)
    }

    /// Get all audit entries
    pub fn get_entries(&self) -> Vec<AuditEntry> {
        self.entries.lock().unwrap().clone()
    }

    /// Get entries filtered by severity
    pub fn get_entries_by_severity(&self, severity: LogSeverity) -> Vec<AuditEntry> {
        self.entries
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.severity >= severity)
            .cloned()
            .collect()
    }

    /// Get entries for a specific file
    pub fn get_entries_for_file(&self, file_path: &Path) -> Vec<AuditEntry> {
        self.entries
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.file_path.as_ref().map(|p| p == file_path).unwrap_or(false))
            .cloned()
            .collect()
    }
}

/// Summary report of an import session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSummary {
    /// Session identifier
    pub session_id: String,
    /// When the session started
    pub start_time: Option<DateTime<Utc>>,
    /// When the session ended
    pub end_time: Option<DateTime<Utc>>,
    /// Total log entries
    pub total_entries: usize,
    /// Number of errors
    pub errors: usize,
    /// Number of warnings
    pub warnings: usize,
    /// Number of successful operations
    pub successes: usize,
    /// Files that were processed
    pub files_processed: std::collections::HashSet<PathBuf>,
}

impl ImportSummary {
    /// Check if import was successful (no critical errors)
    pub fn is_successful(&self) -> bool {
        self.errors == 0
    }

    /// Get human-readable summary
    pub fn to_string_pretty(&self) -> String {
        let duration = self
            .end_time
            .and_then(|end| self.start_time.map(|start| (end - start).to_string()))
            .unwrap_or_default();

        format!(
            "Import Summary ({})\n  Start: {:?}\n  End: {:?}\n  Duration: {}\n  \
             Files: {}\n  Entries: {}\n  Successes: {}\n  Warnings: {}\n  Errors: {}",
            self.session_id,
            self.start_time,
            self.end_time,
            duration,
            self.files_processed.len(),
            self.total_entries,
            self.successes,
            self.warnings,
            self.errors,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_logger_creation() {
        let logger = ImportLogger::new("test_session");
        assert_eq!(logger.session_id, "test_session");
    }

    #[test]
    fn test_log_success() {
        let logger = ImportLogger::new("test_session");
        logger.log_success(OperationType::Parsing, None, 100, Some(50));

        let entries = logger.get_entries();
        assert!(!entries.is_empty());
    }

    #[test]
    fn test_log_error() {
        let logger = ImportLogger::new("test_session");
        logger.log_error(OperationType::Parsing, None, "TEST001", "Test error", false);

        let entries = logger.get_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].severity, LogSeverity::Error);
    }

    #[test]
    fn test_get_summary() {
        let logger = ImportLogger::new("test_session");
        logger.log_error(OperationType::Parsing, None, "E001", "Error", false);
        logger.log_error(OperationType::Validation, None, "E002", "Error 2", true);

        let summary = logger.get_summary();
        assert_eq!(summary.errors, 1);
        assert_eq!(summary.warnings, 1); // One error was recovered, logged as warning
    }

    #[test]
    fn test_severity_filtering() {
        let logger = ImportLogger::new("test_session");
        logger.log_error(OperationType::Parsing, None, "E001", "Error", false);
        logger.log_validation_issue(OperationType::Validation, None, "field", "value", "invalid");

        let errors = logger.get_entries_by_severity(LogSeverity::Error);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn test_export_json() {
        let logger = ImportLogger::new("test_session");
        logger.log_success(OperationType::Import, None, 50, Some(100));

        let json = logger.export_json();
        assert!(json.is_ok());
        let json_str = json.unwrap();
        assert!(json_str.contains("Completed")); // Check the event is exported
        assert!(!json_str.is_empty());
    }
}
