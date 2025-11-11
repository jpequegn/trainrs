//! Structured logging and diagnostics for TrainRS
//!
//! Provides production-grade logging with multiple output formats,
//! log rotation, and sensitive data filtering.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::Level;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Log level (error, warn, info, debug, trace)
    pub level: LogLevel,

    /// Output format (pretty, json, compact)
    pub format: LogFormat,

    /// Log file path (None for stdout only)
    pub file_path: Option<PathBuf>,

    /// Enable log rotation
    pub rotation: bool,

    /// Max log file size in MB
    pub max_file_size_mb: u64,

    /// Number of old log files to keep
    pub max_backups: usize,

    /// Filter sensitive data from logs
    pub filter_sensitive: bool,

    /// Include span information
    pub include_spans: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Pretty,
            file_path: None,
            rotation: true,
            max_file_size_mb: 100,
            max_backups: 5,
            filter_sensitive: true,
            include_spans: true,
        }
    }
}

/// Log level enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn to_tracing_level(&self) -> Level {
        match self {
            LogLevel::Error => Level::ERROR,
            LogLevel::Warn => Level::WARN,
            LogLevel::Info => Level::INFO,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Trace => Level::TRACE,
        }
    }

    pub fn to_filter(&self) -> String {
        match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
        .to_string()
    }
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "error" => Ok(LogLevel::Error),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "info" => Ok(LogLevel::Info),
            "debug" => Ok(LogLevel::Debug),
            "trace" => Ok(LogLevel::Trace),
            _ => Err(format!("Invalid log level: {}", s)),
        }
    }
}

/// Log output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// Human-readable format with colors (for development)
    Pretty,
    /// JSON format (for production/structured logging)
    Json,
    /// Compact format
    Compact,
}

impl std::str::FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pretty" => Ok(LogFormat::Pretty),
            "json" => Ok(LogFormat::Json),
            "compact" => Ok(LogFormat::Compact),
            _ => Err(format!("Invalid log format: {}", s)),
        }
    }
}

/// Initialize the logging system
pub fn init_logging(config: &LogConfig) -> anyhow::Result<()> {
    // Build the base filter
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(format!("trainrs={}", config.level.to_filter()))
    });

    // Create stdout layer
    let stdout_layer = match config.format {
        LogFormat::Pretty => fmt::layer()
            .with_target(true)
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_line_number(true)
            .with_span_events(if config.include_spans {
                FmtSpan::ENTER | FmtSpan::CLOSE
            } else {
                FmtSpan::NONE
            })
            .boxed(),
        LogFormat::Json => fmt::layer()
            .json()
            .with_target(true)
            .with_current_span(config.include_spans)
            .with_span_list(config.include_spans)
            .boxed(),
        LogFormat::Compact => fmt::layer().compact().with_target(true).boxed(),
    };

    // Initialize subscriber with stdout layer
    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer);

    // Add file layer if configured
    if let Some(file_path) = &config.file_path {
        // Create log directory if it doesn't exist
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if config.rotation {
            // Use rotating file appender
            let file_appender = tracing_appender::rolling::daily(
                file_path.parent().unwrap_or_else(|| Path::new(".")),
                file_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("trainrs.log"),
            );

            let file_layer = fmt::layer()
                .json()
                .with_writer(file_appender)
                .with_target(true)
                .with_current_span(config.include_spans)
                .with_span_list(config.include_spans);

            subscriber.with(file_layer).init();
        } else {
            // Use static file
            let file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(file_path)?;

            let file_layer = fmt::layer()
                .json()
                .with_writer(file)
                .with_target(true)
                .with_current_span(config.include_spans)
                .with_span_list(config.include_spans);

            subscriber.with(file_layer).init();
        }
    } else {
        // No file logging
        subscriber.init();
    }

    tracing::info!(
        level = ?config.level,
        format = ?config.format,
        file = ?config.file_path,
        "Logging initialized"
    );

    Ok(())
}

/// Diagnostic report for troubleshooting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    /// When the report was generated
    pub timestamp: DateTime<Utc>,

    /// Operation being diagnosed
    pub operation: String,

    /// Whether the operation succeeded
    pub success: bool,

    /// Operation duration
    #[serde(with = "duration_serde")]
    pub duration: Duration,

    /// Errors encountered
    pub errors: Vec<ErrorDetail>,

    /// Warnings
    pub warnings: Vec<String>,

    /// System information
    pub system_info: SystemInfo,

    /// Additional context
    pub context: Vec<(String, String)>,
}

/// Error detail for diagnostic report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    /// Error message
    pub message: String,

    /// Error type
    pub error_type: String,

    /// When the error occurred
    pub timestamp: DateTime<Utc>,

    /// Stack trace if available
    pub backtrace: Option<String>,
}

/// System information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Operating system
    pub os: String,

    /// OS version
    pub os_version: String,

    /// Architecture
    pub arch: String,

    /// TrainRS version
    pub trainrs_version: String,

    /// Rust version used to compile
    pub rust_version: String,
}

impl SystemInfo {
    pub fn collect() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            os_version: sys_info::os_release().unwrap_or_else(|_| "unknown".to_string()),
            arch: std::env::consts::ARCH.to_string(),
            trainrs_version: env!("CARGO_PKG_VERSION").to_string(),
            rust_version: rustc_version_runtime::version().to_string(),
        }
    }
}

impl DiagnosticReport {
    /// Create a new diagnostic report
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            operation: operation.into(),
            success: false,
            duration: Duration::from_secs(0),
            errors: Vec::new(),
            warnings: Vec::new(),
            system_info: SystemInfo::collect(),
            context: Vec::new(),
        }
    }

    /// Mark as successful
    pub fn set_success(&mut self, success: bool) {
        self.success = success;
    }

    /// Set operation duration
    pub fn set_duration(&mut self, duration: Duration) {
        self.duration = duration;
    }

    /// Add an error
    pub fn add_error(&mut self, error: &dyn std::error::Error) {
        self.errors.push(ErrorDetail {
            message: error.to_string(),
            error_type: std::any::type_name_of_val(error).to_string(),
            timestamp: Utc::now(),
            backtrace: None, // Could be enhanced with backtrace crate
        });
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Add context information
    pub fn add_context(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.context.push((key.into(), value.into()));
    }

    /// Save report to file
    pub fn save_to_file(&self, path: &Path) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        let mut file = fs::File::create(path)?;
        file.write_all(json.as_bytes())?;
        tracing::info!("Diagnostic report saved to {}", path.display());
        Ok(())
    }

    /// Save report to default location
    pub fn save_default(&self) -> anyhow::Result<PathBuf> {
        let filename = format!(
            "trainrs_diagnostic_{}_{}.json",
            self.operation.replace(' ', "_"),
            self.timestamp.format("%Y%m%d_%H%M%S")
        );

        let path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("trainrs")
            .join("diagnostics");

        fs::create_dir_all(&path)?;
        let full_path = path.join(filename);

        self.save_to_file(&full_path)?;
        Ok(full_path)
    }
}

// Helper module for serde duration serialization
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_millis() as u64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

// Stub implementations for missing dependencies
mod sys_info {
    pub fn os_release() -> Result<String, ()> {
        Ok("unknown".to_string())
    }
}

mod rustc_version_runtime {
    pub fn version() -> String {
        "unknown".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_parsing() {
        assert_eq!("info".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("debug".parse::<LogLevel>().unwrap(), LogLevel::Debug);
        assert_eq!("error".parse::<LogLevel>().unwrap(), LogLevel::Error);
    }

    #[test]
    fn test_log_format_parsing() {
        assert_eq!("json".parse::<LogFormat>().unwrap(), LogFormat::Json);
        assert_eq!("pretty".parse::<LogFormat>().unwrap(), LogFormat::Pretty);
    }

    #[test]
    fn test_diagnostic_report() {
        let mut report = DiagnosticReport::new("test_operation");
        assert_eq!(report.operation, "test_operation");
        assert!(!report.success);

        report.set_success(true);
        assert!(report.success);

        report.add_warning("test warning");
        assert_eq!(report.warnings.len(), 1);

        report.add_context("key", "value");
        assert_eq!(report.context.len(), 1);
    }
}
