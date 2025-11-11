//! Configurable formula engine for metric calculations
//!
//! Provides a flexible system for customizing sports science calculations
//! with support for multiple TSS methods, normalized power windows, and custom formulas.
//!
//! # Architecture
//!
//! The formula system consists of:
//! - **CalculationConfig**: Configuration container for all calculation parameters
//! - **TssFormula**: Enum of TSS calculation methods (Classic, BikeScore, Custom)
//! - **FtpMethod**: FTP detection/estimation methods
//! - **NormalizedPowerConfig**: Configurable normalized power parameters
//! - **CustomFormula**: User-defined metric calculations
//! - **FormulaEngine**: Expression evaluation and validation
//!
//! # Example
//!
//! ```rust,ignore
//! use trainrs::formulas::{CalculationConfig, TssFormula, FtpMethod};
//!
//! let config = CalculationConfig {
//!     tss_formula: TssFormula::Classic,
//!     np_config: Default::default(),
//!     ftp_method: FtpMethod::TwentyMinute { factor: 0.95 },
//!     custom_formulas: Default::default(),
//! };
//!
//! // Configuration is applied during metric calculations
//! ```

use std::collections::HashMap;
use thiserror::Error;

/// Formula engine errors
#[derive(Error, Debug)]
pub enum FormulaError {
    #[error("Invalid formula syntax: {0}")]
    InvalidSyntax(String),

    #[error("Unknown variable: {0}")]
    UnknownVariable(String),

    #[error("Undefined function: {0}")]
    UndefinedFunction(String),

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("Evaluation error: {0}")]
    EvaluationError(String),

    #[error("Division by zero")]
    DivisionByZero,

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub type FormulaResult<T> = Result<T, FormulaError>;

/// TSS calculation formula variants
#[derive(Debug, Clone, PartialEq)]
pub enum TssFormula {
    /// Classic TSS: (duration × IF²) × 100
    Classic,
    /// BikeScore™ TSS variant with different weighting
    BikeScore,
    /// Custom formula expression
    Custom(String),
}

impl TssFormula {
    /// Get the formula expression as a string
    pub fn expression(&self) -> String {
        match self {
            TssFormula::Classic => "(duration * IF^2) * 100".to_string(),
            TssFormula::BikeScore => {
                // BikeScore applies intensity weighting
                "(duration * (IF^1.5)) * 100".to_string()
            }
            TssFormula::Custom(expr) => expr.clone(),
        }
    }

    /// Check if this is a custom formula
    pub fn is_custom(&self) -> bool {
        matches!(self, TssFormula::Custom(_))
    }
}

impl Default for TssFormula {
    fn default() -> Self {
        TssFormula::Classic
    }
}

/// FTP estimation/detection methods
#[derive(Debug, Clone, PartialEq)]
pub enum FtpMethod {
    /// 95% of 20-minute power (standard)
    TwentyMinute { factor: f64 },
    /// 95% of 8-minute power
    EightMinute { factor: f64 },
    /// Critical Power model
    CriticalPower,
    /// Custom formula
    Custom(String),
}

impl FtpMethod {
    /// Get description of this FTP method
    pub fn description(&self) -> &str {
        match self {
            FtpMethod::TwentyMinute { .. } => "95% of 20-minute power",
            FtpMethod::EightMinute { .. } => "95% of 8-minute power",
            FtpMethod::CriticalPower => "Critical Power model",
            FtpMethod::Custom(_) => "Custom formula",
        }
    }

    /// Get default factor for this method
    pub fn default_factor(&self) -> Option<f64> {
        match self {
            FtpMethod::TwentyMinute { factor } => Some(*factor),
            FtpMethod::EightMinute { factor } => Some(*factor),
            _ => None,
        }
    }
}

impl Default for FtpMethod {
    fn default() -> Self {
        FtpMethod::TwentyMinute { factor: 0.95 }
    }
}

/// Normalized Power calculation configuration
#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedPowerConfig {
    /// Window size for rolling average (seconds)
    pub window_seconds: u32,
    /// Smoothing algorithm to use
    pub smoothing: SmoothingAlgorithm,
}

impl NormalizedPowerConfig {
    /// Create with default settings (30-second window, rolling average)
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom window size
    pub fn with_window(window_seconds: u32) -> Self {
        Self {
            window_seconds,
            smoothing: SmoothingAlgorithm::RollingAverage,
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> FormulaResult<()> {
        if self.window_seconds == 0 {
            return Err(FormulaError::ValidationFailed(
                "Window size must be > 0 seconds".to_string(),
            ));
        }
        if self.window_seconds > 3600 {
            return Err(FormulaError::ValidationFailed(
                "Window size must be <= 3600 seconds".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for NormalizedPowerConfig {
    fn default() -> Self {
        Self {
            window_seconds: 30,
            smoothing: SmoothingAlgorithm::RollingAverage,
        }
    }
}

/// Smoothing algorithms for normalized power
#[derive(Debug, Clone, PartialEq)]
pub enum SmoothingAlgorithm {
    /// Simple rolling average
    RollingAverage,
    /// Exponential moving average
    ExponentialMovingAverage { alpha: f64 },
    /// Weighted moving average
    WeightedMovingAverage { weights: Vec<f64> },
}

impl Default for SmoothingAlgorithm {
    fn default() -> Self {
        SmoothingAlgorithm::RollingAverage
    }
}

/// Custom formula definition
#[derive(Debug, Clone, PartialEq)]
pub struct CustomFormula {
    /// Name of the formula (e.g., "my_custom_tss")
    pub name: String,
    /// Formula expression (e.g., "(NP / FTP) * duration * 100")
    pub expression: String,
    /// Available variables in formula
    pub variables: Vec<String>,
    /// Optional description
    pub description: Option<String>,
}

impl CustomFormula {
    /// Create a new custom formula
    pub fn new(name: impl Into<String>, expression: impl Into<String>) -> Self {
        let expression_str = expression.into();
        let variables = Self::extract_variables(&expression_str);
        Self {
            name: name.into(),
            expression: expression_str,
            variables,
            description: None,
        }
    }

    /// Add description to formula
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Extract variable names from formula expression
    fn extract_variables(expression: &str) -> Vec<String> {
        let mut variables = Vec::new();
        let mut current = String::new();

        for ch in expression.chars() {
            if ch.is_alphanumeric() || ch == '_' {
                current.push(ch);
            } else {
                if !current.is_empty() && !current.chars().all(|c| c.is_numeric()) {
                    if !variables.contains(&current) {
                        variables.push(current.clone());
                    }
                    current.clear();
                }
            }
        }

        if !current.is_empty() && !current.chars().all(|c| c.is_numeric()) {
            if !variables.contains(&current) {
                variables.push(current);
            }
        }

        variables
    }

    /// Validate formula structure
    pub fn validate(&self) -> FormulaResult<()> {
        if self.name.is_empty() {
            return Err(FormulaError::ValidationFailed(
                "Formula name cannot be empty".to_string(),
            ));
        }

        if self.expression.is_empty() {
            return Err(FormulaError::ValidationFailed(
                "Formula expression cannot be empty".to_string(),
            ));
        }

        // Check for balanced parentheses
        let mut paren_count = 0;
        for ch in self.expression.chars() {
            match ch {
                '(' => paren_count += 1,
                ')' => paren_count -= 1,
                _ => {}
            }
            if paren_count < 0 {
                return Err(FormulaError::InvalidSyntax(
                    "Unbalanced parentheses".to_string(),
                ));
            }
        }

        if paren_count != 0 {
            return Err(FormulaError::InvalidSyntax(
                "Unbalanced parentheses".to_string(),
            ));
        }

        Ok(())
    }
}

/// Main calculation configuration container
#[derive(Debug, Clone)]
pub struct CalculationConfig {
    /// TSS calculation formula
    pub tss_formula: TssFormula,
    /// Normalized Power configuration
    pub np_config: NormalizedPowerConfig,
    /// FTP estimation method
    pub ftp_method: FtpMethod,
    /// Custom formula definitions
    pub custom_formulas: HashMap<String, CustomFormula>,
}

impl CalculationConfig {
    /// Create with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom TSS formula
    pub fn with_tss_formula(mut self, formula: TssFormula) -> Self {
        self.tss_formula = formula;
        self
    }

    /// Add a custom formula
    pub fn add_custom_formula(mut self, formula: CustomFormula) -> FormulaResult<Self> {
        formula.validate()?;
        self.custom_formulas.insert(formula.name.clone(), formula);
        Ok(self)
    }

    /// Get a custom formula by name
    pub fn get_custom_formula(&self, name: &str) -> Option<&CustomFormula> {
        self.custom_formulas.get(name)
    }

    /// Remove a custom formula
    pub fn remove_custom_formula(&mut self, name: &str) -> Option<CustomFormula> {
        self.custom_formulas.remove(name)
    }

    /// List all custom formulas
    pub fn list_custom_formulas(&self) -> Vec<&CustomFormula> {
        self.custom_formulas.values().collect()
    }

    /// Validate entire configuration
    pub fn validate(&self) -> FormulaResult<()> {
        self.np_config.validate()?;

        for formula in self.custom_formulas.values() {
            formula.validate()?;
        }

        Ok(())
    }

    /// Get configuration summary
    pub fn summary(&self) -> String {
        format!(
            "CalculationConfig {{\n  \
             tss_formula: {:?}\n  \
             np_window_seconds: {}\n  \
             ftp_method: {}\n  \
             custom_formulas: {}\n\
             }}",
            self.tss_formula,
            self.np_config.window_seconds,
            self.ftp_method.description(),
            self.custom_formulas.len()
        )
    }
}

impl Default for CalculationConfig {
    fn default() -> Self {
        Self {
            tss_formula: TssFormula::default(),
            np_config: NormalizedPowerConfig::default(),
            ftp_method: FtpMethod::default(),
            custom_formulas: HashMap::new(),
        }
    }
}

/// Formula validation and evaluation engine
///
/// This is a placeholder for the actual expression evaluation engine.
/// The real implementation would use a math expression parser library
/// like `mun` or `rhai` for safe runtime evaluation.
pub struct FormulaEngine;

impl FormulaEngine {
    /// Validate a formula expression
    pub fn validate_formula(formula: &str) -> FormulaResult<()> {
        if formula.is_empty() {
            return Err(FormulaError::InvalidSyntax(
                "Formula cannot be empty".to_string(),
            ));
        }

        // Check for balanced parentheses
        let mut paren_count = 0;
        for ch in formula.chars() {
            match ch {
                '(' => paren_count += 1,
                ')' => paren_count -= 1,
                _ => {}
            }
            if paren_count < 0 {
                return Err(FormulaError::InvalidSyntax(
                    "Unbalanced parentheses".to_string(),
                ));
            }
        }

        if paren_count != 0 {
            return Err(FormulaError::InvalidSyntax(
                "Unbalanced parentheses in formula".to_string(),
            ));
        }

        // Check for common operators and functions
        let allowed_chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_()^*+/-. ";
        for ch in formula.chars() {
            if !allowed_chars.contains(ch) {
                return Err(FormulaError::InvalidSyntax(format!(
                    "Invalid character in formula: '{}'",
                    ch
                )));
            }
        }

        Ok(())
    }

    /// Extract variables from a formula
    pub fn extract_variables(formula: &str) -> Vec<String> {
        CustomFormula::extract_variables(formula)
    }

}

// NOTE: Actual evaluation requires a math expression library
// This is a stub that documents the expected interface
//
// For runtime formula evaluation, integrate with a library like:
// - `mun`: Statically-typed scripting language
// - `rhai`: Embedded scripting language for Rust
// - `evalexpr`: Mathematical expression parser
// - `fastexpr`: Fast expression evaluation
//
// Example with evalexpr:
// ```ignore
// pub fn evaluate(
//     formula: &str,
//     variables: &HashMap<String, f64>,
// ) -> FormulaResult<f64> {
//     let mut context = evalexpr::HashMapContext::new();
//     for (name, value) in variables {
//         context.set_value(name.clone(), evalexpr::Value::Float(*value))?;
//     }
//     let expr = evalexpr::build_operator_tree(formula)?;
//     match expr.eval_with_context(&context)? {
//         evalexpr::Value::Float(f) => Ok(f),
//         _ => Err(FormulaError::TypeMismatch {
//             expected: "number".to_string(),
//             actual: "other".to_string(),
//         }),
//     }
// }
// ```

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tss_formula_classic() {
        let formula = TssFormula::Classic;
        assert_eq!(formula.expression(), "(duration * IF^2) * 100");
        assert!(!formula.is_custom());
    }

    #[test]
    fn test_tss_formula_bikescore() {
        let formula = TssFormula::BikeScore;
        assert_eq!(formula.expression(), "(duration * (IF^1.5)) * 100");
    }

    #[test]
    fn test_ftp_method_default() {
        let method = FtpMethod::default();
        match method {
            FtpMethod::TwentyMinute { factor } => assert_eq!(factor, 0.95),
            _ => panic!("Expected TwentyMinute"),
        }
    }

    #[test]
    fn test_normalized_power_config_validate() {
        let config = NormalizedPowerConfig::new();
        assert!(config.validate().is_ok());

        let invalid = NormalizedPowerConfig {
            window_seconds: 0,
            smoothing: SmoothingAlgorithm::RollingAverage,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_custom_formula_creation() {
        let formula = CustomFormula::new("my_score", "(NP / FTP) * duration * 100")
            .with_description("Custom scoring formula");

        assert_eq!(formula.name, "my_score");
        assert!(formula.description.is_some());
        assert!(formula.variables.contains(&"NP".to_string()));
        assert!(formula.variables.contains(&"FTP".to_string()));
        assert!(formula.variables.contains(&"duration".to_string()));
    }

    #[test]
    fn test_custom_formula_validation() {
        let formula = CustomFormula::new("test", "(NP / FTP)");
        assert!(formula.validate().is_ok());

        let invalid_parens = CustomFormula::new("test", "(NP / FTP))");
        assert!(invalid_parens.validate().is_err());

        let empty_name = CustomFormula::new("", "(NP / FTP)");
        assert!(empty_name.validate().is_err());
    }

    #[test]
    fn test_calculation_config_default() {
        let config = CalculationConfig::new();
        assert_eq!(config.tss_formula, TssFormula::Classic);
        assert_eq!(config.np_config.window_seconds, 30);
        assert!(config.custom_formulas.is_empty());
    }

    #[test]
    fn test_calculation_config_add_formula() {
        let config = CalculationConfig::new();
        let formula = CustomFormula::new("test", "(A + B) * C");

        let config = config.add_custom_formula(formula.clone()).unwrap();
        assert_eq!(config.custom_formulas.len(), 1);
        assert!(config.get_custom_formula("test").is_some());
    }

    #[test]
    fn test_formula_engine_validate() {
        assert!(FormulaEngine::validate_formula("(NP / FTP) * 100").is_ok());
        assert!(FormulaEngine::validate_formula("((A + B) * C)").is_ok());
        assert!(FormulaEngine::validate_formula("(A + B))").is_err());
        assert!(FormulaEngine::validate_formula("").is_err());
    }

    #[test]
    fn test_formula_engine_extract_variables() {
        let variables = FormulaEngine::extract_variables("(NP / FTP) * duration + IF");
        assert!(variables.contains(&"NP".to_string()));
        assert!(variables.contains(&"FTP".to_string()));
        assert!(variables.contains(&"duration".to_string()));
        assert!(variables.contains(&"IF".to_string()));
    }
}
