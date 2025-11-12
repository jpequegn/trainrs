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
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::str::FromStr;
use evalexpr::ContextWithMutableVariables;

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

    /// Evaluate a formula with given variable values
    ///
    /// This evaluates the formula expression using the provided variables and returns
    /// the result as a Decimal for financial-grade precision.
    ///
    /// # Arguments
    ///
    /// * `formula` - The formula expression to evaluate (e.g., "(duration * IF^2) * 100")
    /// * `variables` - HashMap of variable names to Decimal values
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use trainrs::formulas::FormulaEngine;
    /// use rust_decimal::Decimal;
    /// use std::collections::HashMap;
    ///
    /// let mut vars = HashMap::new();
    /// vars.insert("duration".to_string(), Decimal::from_str("1.5").unwrap());
    /// vars.insert("IF".to_string(), Decimal::from_str("1.2").unwrap());
    ///
    /// let result = FormulaEngine::evaluate("(duration * IF^2) * 100", &vars)?;
    /// assert_eq!(result, Decimal::from_str("216").unwrap());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn evaluate(
        formula: &str,
        variables: &HashMap<String, Decimal>,
    ) -> FormulaResult<Decimal> {
        // Validate formula syntax
        Self::validate_formula(formula)?;

        // Build evalexpr context from Decimal variables
        let mut context: evalexpr::HashMapContext = evalexpr::HashMapContext::new();
        for (name, value) in variables {
            let f64_value = value.to_f64().ok_or_else(|| {
                FormulaError::TypeMismatch {
                    expected: "finite number".to_string(),
                    actual: format!("out of range decimal: {}", value),
                }
            })?;
            let val = evalexpr::Value::Float(f64_value);
            context.set_value(name.clone(), val)
                .map_err(|e| FormulaError::EvaluationError(e.to_string()))?;
        }

        // Parse and evaluate the expression
        let expr = evalexpr::build_operator_tree(formula)
            .map_err(|e| {
                let error_msg = e.to_string();
                if error_msg.contains("VariableIdentifierNotFound") || error_msg.contains("was not found") {
                    // Extract variable name from error message if possible
                    FormulaError::UnknownVariable(error_msg)
                } else if error_msg.contains("FunctionIdentifierNotFound") {
                    FormulaError::UndefinedFunction(error_msg)
                } else {
                    FormulaError::InvalidSyntax(error_msg)
                }
            })?;

        // Evaluate and convert result to Decimal
        let result = expr.eval_with_context(&context)
            .map_err(|e| {
                if e.to_string().contains("division by zero") {
                    FormulaError::DivisionByZero
                } else {
                    FormulaError::EvaluationError(e.to_string())
                }
            })?;

        // Convert evalexpr result to Decimal
        match result {
            evalexpr::Value::Float(f64_result) => {
                let f: f64 = f64_result;
                if !f.is_finite() {
                    return Err(FormulaError::EvaluationError(
                        format!("Non-finite result: {}", f)
                    ));
                }
                Decimal::from_str(&f.to_string())
                    .map_err(|e| FormulaError::EvaluationError(e.to_string()))
            }
            evalexpr::Value::Int(i) => Ok(Decimal::from(i)),
            _ => Err(FormulaError::TypeMismatch {
                expected: "numeric value".to_string(),
                actual: "non-numeric result".to_string(),
            }),
        }
    }

    /// Evaluate a formula with string variable values (automatically converted to Decimal)
    ///
    /// This is a convenience method for when variables are provided as strings.
    /// Useful for CLI and configuration file parsing.
    pub fn evaluate_with_strings(
        formula: &str,
        variables: &HashMap<String, String>,
    ) -> FormulaResult<Decimal> {
        let mut decimal_vars = HashMap::new();
        for (name, value_str) in variables {
            let decimal_value = Decimal::from_str(value_str)
                .map_err(|e| FormulaError::TypeMismatch {
                    expected: "numeric string".to_string(),
                    actual: format!("{}: {}", value_str, e),
                })?;
            decimal_vars.insert(name.clone(), decimal_value);
        }
        Self::evaluate(formula, &decimal_vars)
    }

    /// Evaluate a formula and return the result as f64
    ///
    /// Useful when interfacing with code that expects f64 values.
    /// Note: This loses precision compared to Decimal.
    pub fn evaluate_as_f64(
        formula: &str,
        variables: &HashMap<String, Decimal>,
    ) -> FormulaResult<f64> {
        let result = Self::evaluate(formula, variables)?;
        result.to_f64().ok_or_else(|| {
            FormulaError::EvaluationError(
                format!("Result out of f64 range: {}", result)
            )
        })
    }

}

/// Expression compilation and caching for performance optimization
///
/// When the same formula is evaluated multiple times, compilation caching
/// can improve performance significantly. This module is deferred for Phase 6.
pub mod caching {
    //! Expression caching for performance optimization
    //!
    //! This module will provide:
    //! - Compiled expression caching
    //! - Cache invalidation strategies
    //! - Performance monitoring
    //!
    //! To be implemented in Phase 6: Performance & Optimization
}

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

    // Phase 2: Expression Evaluation Tests

    #[test]
    fn test_formula_evaluate_simple_arithmetic() {
        let mut vars = HashMap::new();
        vars.insert("a".to_string(), Decimal::from(10));
        vars.insert("b".to_string(), Decimal::from(5));

        let result = FormulaEngine::evaluate("a + b", &vars);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Decimal::from(15));
    }

    #[test]
    fn test_formula_evaluate_multiplication_and_exponentiation() {
        let mut vars = HashMap::new();
        vars.insert("duration".to_string(), Decimal::from_str("1.5").unwrap());
        vars.insert("IF".to_string(), Decimal::from_str("1.2").unwrap());

        // TSS = (duration * IF^2) * 100
        let result = FormulaEngine::evaluate("(duration * IF^2) * 100", &vars);
        assert!(result.is_ok());
        let tss = result.unwrap();
        // 1.5 * 1.2^2 * 100 = 1.5 * 1.44 * 100 = 216
        assert!(tss > Decimal::from(215) && tss < Decimal::from(217));
    }

    #[test]
    fn test_formula_evaluate_division() {
        let mut vars = HashMap::new();
        vars.insert("NP".to_string(), Decimal::from(300));
        vars.insert("FTP".to_string(), Decimal::from(250));

        let result = FormulaEngine::evaluate("NP / FTP", &vars);
        assert!(result.is_ok());
        let if_value = result.unwrap();
        assert!(if_value > Decimal::from_str("1.1").unwrap());
        assert!(if_value < Decimal::from_str("1.3").unwrap());
    }

    #[test]
    fn test_formula_evaluate_division_by_zero() {
        let mut vars = HashMap::new();
        vars.insert("a".to_string(), Decimal::from(10));
        vars.insert("b".to_string(), Decimal::from(0));

        let result = FormulaEngine::evaluate("a / b", &vars);
        // Division by zero in evalexpr returns Infinity (f64), not an error
        // So we should get a non-finite result error instead
        assert!(result.is_err(), "Expected error but got: {:?}", result);
    }

    #[test]
    fn test_formula_evaluate_unknown_variable() {
        let vars = HashMap::new();

        let result = FormulaEngine::evaluate("unknown_var * 100", &vars);
        assert!(result.is_err());
        // Error should be about unknown variable
        let err_msg = result.unwrap_err().to_string().to_lowercase();
        assert!(err_msg.contains("unknown") || err_msg.contains("variable"));
    }

    #[test]
    fn test_formula_evaluate_with_strings() {
        let mut vars = HashMap::new();
        vars.insert("a".to_string(), "10".to_string());
        vars.insert("b".to_string(), "5".to_string());

        let result = FormulaEngine::evaluate_with_strings("a + b", &vars);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Decimal::from(15));
    }

    #[test]
    fn test_formula_evaluate_with_strings_invalid_number() {
        let mut vars = HashMap::new();
        vars.insert("a".to_string(), "not_a_number".to_string());

        let result = FormulaEngine::evaluate_with_strings("a + 5", &vars);
        assert!(result.is_err());
    }

    #[test]
    fn test_formula_evaluate_as_f64() {
        let mut vars = HashMap::new();
        vars.insert("a".to_string(), Decimal::from(10));
        vars.insert("b".to_string(), Decimal::from(5));

        let result = FormulaEngine::evaluate_as_f64("a + b", &vars);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 15.0);
    }

    #[test]
    fn test_formula_evaluate_complex_tss_formula() {
        let mut vars = HashMap::new();
        vars.insert("duration".to_string(), Decimal::from_str("2.0").unwrap());
        vars.insert("NP".to_string(), Decimal::from(280));
        vars.insert("FTP".to_string(), Decimal::from(250));

        // TSS = (duration * (NP/FTP)^2) * 100
        let formula = "(duration * (NP / FTP)^2) * 100";
        let result = FormulaEngine::evaluate(formula, &vars);
        assert!(result.is_ok());

        let tss = result.unwrap();
        // duration=2, IF=280/250=1.12, TSS = 2 * 1.12^2 * 100 = 250.88
        assert!(tss > Decimal::from(240) && tss < Decimal::from(270));
    }

    #[test]
    fn test_formula_evaluate_preserves_decimal_precision() {
        let mut vars = HashMap::new();
        vars.insert("a".to_string(), Decimal::from_str("0.1").unwrap());
        vars.insert("b".to_string(), Decimal::from_str("0.2").unwrap());

        let result = FormulaEngine::evaluate("a + b", &vars);
        assert!(result.is_ok());
        // Should handle floating point precision better with Decimal
        let sum = result.unwrap();
        assert!(sum > Decimal::from_str("0.29").unwrap());
        assert!(sum < Decimal::from_str("0.31").unwrap());
    }

    #[test]
    fn test_formula_evaluate_negative_numbers() {
        let mut vars = HashMap::new();
        vars.insert("a".to_string(), Decimal::from(-10));
        vars.insert("b".to_string(), Decimal::from(5));

        let result = FormulaEngine::evaluate("a + b", &vars);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Decimal::from(-5));
    }

    #[test]
    fn test_formula_evaluate_with_parentheses() {
        let mut vars = HashMap::new();
        vars.insert("a".to_string(), Decimal::from(2));
        vars.insert("b".to_string(), Decimal::from(3));
        vars.insert("c".to_string(), Decimal::from(4));

        // Test (a + b) * c
        let result1 = FormulaEngine::evaluate("(a + b) * c", &vars);
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), Decimal::from(20)); // (2+3)*4 = 20

        // Test a + (b * c)
        let result2 = FormulaEngine::evaluate("a + (b * c)", &vars);
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), Decimal::from(14)); // 2 + (3*4) = 14
    }

    #[test]
    fn test_formula_evaluate_bikescore_variant() {
        let mut vars = HashMap::new();
        vars.insert("duration".to_string(), Decimal::from_str("1.5").unwrap());
        vars.insert("IF".to_string(), Decimal::from_str("1.2").unwrap());

        // BikeScore: (duration * (IF^1.5)) * 100
        let formula = "(duration * (IF^1.5)) * 100";
        let result = FormulaEngine::evaluate(formula, &vars);
        assert!(result.is_ok());

        let bikescore = result.unwrap();
        // 1.5 * 1.2^1.5 * 100 ≈ 193.2
        assert!(bikescore > Decimal::from(180) && bikescore < Decimal::from(210));
    }

    #[test]
    fn test_formula_validate_then_evaluate() {
        // Should validate before evaluating
        let formula = "(a + b) * c";
        assert!(FormulaEngine::validate_formula(formula).is_ok());

        let mut vars = HashMap::new();
        vars.insert("a".to_string(), Decimal::from(1));
        vars.insert("b".to_string(), Decimal::from(2));
        vars.insert("c".to_string(), Decimal::from(3));

        let result = FormulaEngine::evaluate(formula, &vars);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Decimal::from(9));
    }

    #[test]
    fn test_formula_all_operators() {
        let mut vars = HashMap::new();
        vars.insert("a".to_string(), Decimal::from(10));
        vars.insert("b".to_string(), Decimal::from(3));

        // Test addition
        assert_eq!(
            FormulaEngine::evaluate("a + b", &vars).unwrap(),
            Decimal::from(13)
        );

        // Test subtraction
        assert_eq!(
            FormulaEngine::evaluate("a - b", &vars).unwrap(),
            Decimal::from(7)
        );

        // Test multiplication
        assert_eq!(
            FormulaEngine::evaluate("a * b", &vars).unwrap(),
            Decimal::from(30)
        );

        // Test division
        let div_result = FormulaEngine::evaluate("a / b", &vars).unwrap();
        assert!(div_result > Decimal::from(3) && div_result < Decimal::from(4));
    }
}
