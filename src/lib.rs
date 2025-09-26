// Library interface for TrainRS modules
// This allows integration tests to access the core functionality

pub mod config;
pub mod database;
pub mod data_management;
pub mod export;
pub mod import;
pub mod models;
pub mod multisport;
pub mod performance;
pub mod pmc;
pub mod power;
pub mod running;
pub mod training_plan;
pub mod tss;
pub mod zones;

// Test utilities have been integrated into individual test files

// Re-export commonly used types for convenience
pub use models::*;
pub use tss::TssCalculator;
pub use pmc::PmcCalculator;
pub use zones::ZoneCalculator;