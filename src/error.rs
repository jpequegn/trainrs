//! Unified error hierarchy for TrainRS
//!
//! Provides a comprehensive error type system with structured error information,
//! context preservation, and integration with the tracing system.

use std::path::PathBuf;
use thiserror::Error;

/// Top-level error type for all TrainRS operations
#[derive(Debug, Error)]
pub enum TrainRsError {
    /// FIT file parsing errors
    #[error("FIT parsing error: {0}")]
    FitParsing(#[from] FitError),

    /// Database operation errors
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),

    /// Data validation errors
    #[error("Validation error: {0}")]
    Validation(String),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Import/export errors
    #[error("Import/Export error: {0}")]
    ImportExport(#[from] ImportExportError),

    /// Calculation errors
    #[error("Calculation error: {0}")]
    Calculation(#[from] CalculationError),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Authentication/authorization errors
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Generic internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

/// FIT file parsing specific errors
#[derive(Debug, Error)]
pub enum FitError {
    /// File not found at specified path
    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },

    /// File is corrupted or invalid
    #[error("Corrupted file: {reason}")]
    Corrupted { reason: String },

    /// Unsupported FIT version
    #[error("Unsupported FIT version: {version}")]
    UnsupportedVersion { version: u16 },

    /// Missing required FIT message
    #[error("Missing required message: {message_type}")]
    MissingMessage { message_type: String },

    /// Invalid field value
    #[error("Invalid field value in {field}: {reason}")]
    InvalidField { field: String, reason: String },

    /// Checksum mismatch
    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: u16, actual: u16 },

    /// Unknown developer field
    #[error("Unknown developer field: uuid={uuid}, field_num={field_num}")]
    UnknownDeveloperField { uuid: String, field_num: u8 },
}

/// Database operation errors
#[derive(Debug, Error)]
pub enum DatabaseError {
    /// Connection failed
    #[error("Database connection failed: {reason}")]
    ConnectionFailed { reason: String },

    /// Query execution failed
    #[error("Query failed: {query}")]
    QueryFailed { query: String },

    /// Transaction error
    #[error("Transaction error: {reason}")]
    TransactionError { reason: String },

    /// Migration error
    #[error("Migration error: {version}")]
    MigrationError { version: u32 },

    /// Constraint violation
    #[error("Constraint violation: {constraint}")]
    ConstraintViolation { constraint: String },

    /// Record not found
    #[error("Record not found: {table}.{id}")]
    NotFound { table: String, id: String },

    /// Duplicate entry
    #[error("Duplicate entry: {table}.{key}")]
    Duplicate { table: String, key: String },
}

/// Import and export errors
#[derive(Debug, Error)]
pub enum ImportExportError {
    /// Unsupported format
    #[error("Unsupported format: {format}")]
    UnsupportedFormat { format: String },

    /// Format-specific parsing error
    #[error("Parse error in {format}: {reason}")]
    ParseError { format: String, reason: String },

    /// Missing required data
    #[error("Missing required data: {field}")]
    MissingData { field: String },

    /// Invalid data structure
    #[error("Invalid data structure: {reason}")]
    InvalidStructure { reason: String },

    /// Export failed
    #[error("Export failed to {path}: {reason}")]
    ExportFailed { path: PathBuf, reason: String },
}

/// Calculation errors
#[derive(Debug, Error)]
pub enum CalculationError {
    /// Insufficient data for calculation
    #[error("Insufficient data for {calculation}: {reason}")]
    InsufficientData { calculation: String, reason: String },

    /// Invalid parameter
    #[error("Invalid parameter for {calculation}: {parameter}={value}")]
    InvalidParameter {
        calculation: String,
        parameter: String,
        value: String,
    },

    /// Numerical overflow
    #[error("Numerical overflow in {calculation}")]
    Overflow { calculation: String },

    /// Division by zero
    #[error("Division by zero in {calculation}")]
    DivisionByZero { calculation: String },

    /// Missing athlete profile data
    #[error("Missing athlete profile: {field}")]
    MissingProfile { field: String },
}

/// Result type alias for TrainRS operations
pub type Result<T> = std::result::Result<T, TrainRsError>;

impl TrainRsError {
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            TrainRsError::Database(DatabaseError::ConnectionFailed { .. })
                | TrainRsError::Io(_)
        )
    }

    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            TrainRsError::FitParsing(FitError::FileNotFound { .. }) => ErrorSeverity::Warning,
            TrainRsError::Database(DatabaseError::NotFound { .. }) => ErrorSeverity::Warning,
            TrainRsError::Validation(_) => ErrorSeverity::Warning,
            TrainRsError::Database(DatabaseError::ConnectionFailed { .. }) => ErrorSeverity::Error,
            TrainRsError::Database(_) => ErrorSeverity::Error,
            TrainRsError::Auth(_) => ErrorSeverity::Error,
            TrainRsError::Internal(_) => ErrorSeverity::Critical,
            _ => ErrorSeverity::Error,
        }
    }

    /// Get user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            TrainRsError::FitParsing(FitError::FileNotFound { path }) => {
                format!("Could not find workout file: {}", path.display())
            }
            TrainRsError::FitParsing(FitError::Corrupted { reason }) => {
                format!("Workout file is corrupted: {}", reason)
            }
            TrainRsError::Database(DatabaseError::ConnectionFailed { .. }) => {
                "Unable to connect to database. Please check your configuration.".to_string()
            }
            TrainRsError::Calculation(CalculationError::InsufficientData {
                calculation,
                ..
            }) => {
                format!(
                    "Not enough data to calculate {}. Please ensure your workout has complete data.",
                    calculation
                )
            }
            _ => self.to_string(),
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Critical system error requiring immediate attention
    Critical,
    /// Error that prevents operation but system can continue
    Error,
    /// Warning that doesn't prevent operation
    Warning,
    /// Informational message
    Info,
}

impl ErrorSeverity {
    /// Convert to tracing level
    pub fn to_tracing_level(&self) -> tracing::Level {
        match self {
            ErrorSeverity::Critical => tracing::Level::ERROR,
            ErrorSeverity::Error => tracing::Level::ERROR,
            ErrorSeverity::Warning => tracing::Level::WARN,
            ErrorSeverity::Info => tracing::Level::INFO,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_severity() {
        let err = TrainRsError::FitParsing(FitError::FileNotFound {
            path: PathBuf::from("/test/file.fit"),
        });
        assert_eq!(err.severity(), ErrorSeverity::Warning);

        let err = TrainRsError::Internal("test".to_string());
        assert_eq!(err.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn test_error_retryable() {
        let err = TrainRsError::Database(DatabaseError::ConnectionFailed {
            reason: "timeout".to_string(),
        });
        assert!(err.is_retryable());

        let err = TrainRsError::Validation("test".to_string());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_user_messages() {
        let err = TrainRsError::FitParsing(FitError::FileNotFound {
            path: PathBuf::from("workout.fit"),
        });
        assert!(err.user_message().contains("Could not find"));
    }
}
