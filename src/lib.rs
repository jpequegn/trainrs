// Library interface for TrainRS modules
// This allows integration tests to access the core functionality

pub mod config;
pub mod database;
pub mod data_management;
pub mod device_quirks;
pub mod error;
pub mod export;
pub mod formulas;
pub mod import;
pub mod logging;
pub mod models;
pub mod multisport;
pub mod performance;
pub mod pmc;
pub mod power;
pub mod recovery;
pub mod running;
pub mod stress_testing;
pub mod swimming;
pub mod training_effect;
pub mod training_plan;
pub mod tss;
pub mod vo2max;
pub mod zones;

// Test utilities have been integrated into individual test files

// Re-export commonly used types for convenience
pub use models::*;
pub use tss::TssCalculator;
pub use pmc::PmcCalculator;
pub use training_effect::TrainingEffectAnalyzer;
pub use zones::ZoneCalculator;
pub use formulas::{
    CalculationConfig, CustomFormula, FormulaEngine, FormulaError, FtpMethod, NormalizedPowerConfig,
    SmoothingAlgorithm, TssFormula,
};
pub use export::ml::{MlCsvExporter, SplitConfig, SplitType};
pub use error::{TrainRsError, Result};
pub use logging::{LogConfig, LogLevel, LogFormat, DiagnosticReport};
