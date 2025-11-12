//! Configuration file parsing and management for formula system
//!
//! This module provides TOML-based configuration for formula definitions,
//! allowing users to define custom formulas, TSS methods, and calculation
//! parameters without modifying code.

use crate::formulas::{
    CalculationConfig, CustomFormula, FormulaError, FormulaResult, FtpMethod,
    NormalizedPowerConfig, SmoothingAlgorithm, TssFormula,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;

/// Serializable configuration format for TOML files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlConfig {
    /// Global calculation settings
    #[serde(default)]
    pub calculation: CalculationSection,

    /// Custom formula definitions
    #[serde(default)]
    pub custom_formulas: Vec<TomlFormula>,
}

/// Calculation settings section of TOML config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculationSection {
    /// TSS formula variant: "Classic", "BikeScore", or custom expression
    #[serde(default = "default_tss")]
    pub tss_formula: String,

    /// Normalized power window size in seconds
    #[serde(default = "default_np_window")]
    pub np_window_seconds: u32,

    /// Smoothing algorithm: "RollingAverage", "ExponentialMovingAverage", "WeightedMovingAverage"
    #[serde(default = "default_smoothing")]
    pub smoothing_algorithm: String,

    /// FTP detection method
    #[serde(default = "default_ftp")]
    pub ftp_method: String,
}

fn default_tss() -> String {
    "Classic".to_string()
}

fn default_np_window() -> u32 {
    30
}

fn default_smoothing() -> String {
    "RollingAverage".to_string()
}

fn default_ftp() -> String {
    "TwentyMinute".to_string()
}

impl Default for CalculationSection {
    fn default() -> Self {
        Self {
            tss_formula: default_tss(),
            np_window_seconds: default_np_window(),
            smoothing_algorithm: default_smoothing(),
            ftp_method: default_ftp(),
        }
    }
}

/// Custom formula definition in TOML format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlFormula {
    /// Formula name (must be unique)
    pub name: String,

    /// Mathematical expression
    pub expression: String,

    /// Human-readable description
    #[serde(default)]
    pub description: Option<String>,
}

/// Configuration loader for TOML files
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load configuration from a TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> FormulaResult<CalculationConfig> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .map_err(|e| FormulaError::ConfigError(
                format!("Failed to read config file {:?}: {}", path, e)
            ))?;

        Self::load_from_string(&content)
    }

    /// Load configuration from a TOML string
    pub fn load_from_string(content: &str) -> FormulaResult<CalculationConfig> {
        let toml_config: TomlConfig = toml::from_str(content)
            .map_err(|e| FormulaError::ConfigError(
                format!("Invalid TOML syntax: {}", e)
            ))?;

        Self::build_config(toml_config)
    }

    /// Build CalculationConfig from parsed TOML
    fn build_config(toml_config: TomlConfig) -> FormulaResult<CalculationConfig> {
        // Parse TSS formula
        let tss_formula = Self::parse_tss_formula(&toml_config.calculation.tss_formula)?;

        // Parse FTP method
        let ftp_method = Self::parse_ftp_method(&toml_config.calculation.ftp_method)?;

        // Build normalized power config
        let np_config = NormalizedPowerConfig {
            window_seconds: toml_config.calculation.np_window_seconds,
            smoothing: Self::parse_smoothing_algorithm(&toml_config.calculation.smoothing_algorithm)?,
        };
        np_config.validate()?;

        // Build custom formulas
        let mut custom_formulas = HashMap::new();
        for toml_formula in toml_config.custom_formulas {
            let formula = CustomFormula::new(&toml_formula.name, &toml_formula.expression);
            let formula = if let Some(desc) = toml_formula.description {
                formula.with_description(&desc)
            } else {
                formula
            };

            formula.validate()?;
            custom_formulas.insert(toml_formula.name.clone(), formula);
        }

        // Build final config
        let mut config = CalculationConfig {
            tss_formula,
            np_config,
            ftp_method,
            custom_formulas,
        };

        // Validate complete configuration
        config.validate()?;

        Ok(config)
    }

    /// Parse TSS formula from string
    fn parse_tss_formula(formula_str: &str) -> FormulaResult<TssFormula> {
        match formula_str.trim() {
            "Classic" => Ok(TssFormula::Classic),
            "BikeScore" => Ok(TssFormula::BikeScore),
            expr => {
                // Assume it's a custom expression
                // Validate syntax
                crate::formulas::FormulaEngine::validate_formula(expr)?;
                Ok(TssFormula::Custom(expr.to_string()))
            }
        }
    }

    /// Parse smoothing algorithm from string
    fn parse_smoothing_algorithm(algo_str: &str) -> FormulaResult<SmoothingAlgorithm> {
        match algo_str.trim() {
            "RollingAverage" => Ok(SmoothingAlgorithm::RollingAverage),
            "ExponentialMovingAverage" => Ok(SmoothingAlgorithm::ExponentialMovingAverage { alpha: 0.3 }),
            "WeightedMovingAverage" => {
                // Default weights (uniform)
                Ok(SmoothingAlgorithm::WeightedMovingAverage { weights: vec![] })
            }
            other => Err(FormulaError::ConfigError(format!(
                "Unknown smoothing algorithm: '{}'. Must be one of: RollingAverage, ExponentialMovingAverage, WeightedMovingAverage",
                other
            ))),
        }
    }

    /// Parse FTP method from string
    fn parse_ftp_method(method_str: &str) -> FormulaResult<FtpMethod> {
        match method_str.trim() {
            "TwentyMinute" => Ok(FtpMethod::TwentyMinute { factor: 0.95 }),
            "EightMinute" => Ok(FtpMethod::EightMinute { factor: 0.98 }),
            "CriticalPower" => Ok(FtpMethod::CriticalPower),
            expr => {
                // Assume it's a custom expression
                crate::formulas::FormulaEngine::validate_formula(expr)?;
                Ok(FtpMethod::Custom(expr.to_string()))
            }
        }
    }

    /// Export CalculationConfig to TOML string
    pub fn export_to_string(config: &CalculationConfig) -> FormulaResult<String> {
        let toml_config = Self::config_to_toml(config)?;
        toml::to_string_pretty(&toml_config)
            .map_err(|e| FormulaError::ConfigError(format!("Failed to serialize config: {}", e)))
    }

    /// Export CalculationConfig to TOML file
    pub fn export_to_file<P: AsRef<Path>>(config: &CalculationConfig, path: P) -> FormulaResult<()> {
        let content = Self::export_to_string(config)?;
        fs::write(&path, content)
            .map_err(|e| FormulaError::ConfigError(
                format!("Failed to write config file: {}", e)
            ))?;
        Ok(())
    }

    /// Convert CalculationConfig to TOML representation
    fn config_to_toml(config: &CalculationConfig) -> FormulaResult<TomlConfig> {
        let calculation = CalculationSection {
            tss_formula: match &config.tss_formula {
                TssFormula::Classic => "Classic".to_string(),
                TssFormula::BikeScore => "BikeScore".to_string(),
                TssFormula::Custom(expr) => expr.clone(),
            },
            np_window_seconds: config.np_config.window_seconds,
            smoothing_algorithm: match &config.np_config.smoothing {
                SmoothingAlgorithm::RollingAverage => "RollingAverage".to_string(),
                SmoothingAlgorithm::ExponentialMovingAverage { .. } => {
                    "ExponentialMovingAverage".to_string()
                }
                SmoothingAlgorithm::WeightedMovingAverage { .. } => "WeightedMovingAverage".to_string(),
            },
            ftp_method: match &config.ftp_method {
                FtpMethod::TwentyMinute { .. } => "TwentyMinute".to_string(),
                FtpMethod::EightMinute { .. } => "EightMinute".to_string(),
                FtpMethod::CriticalPower => "CriticalPower".to_string(),
                FtpMethod::Custom(expr) => expr.clone(),
            },
        };

        let custom_formulas = config
            .custom_formulas
            .values()
            .map(|formula| TomlFormula {
                name: formula.name.clone(),
                expression: formula.expression.clone(),
                description: formula.description.clone(),
            })
            .collect();

        Ok(TomlConfig {
            calculation,
            custom_formulas,
        })
    }

    /// Load configuration with defaults merged
    ///
    /// If a config file doesn't specify all values, defaults are used.
    pub fn load_with_defaults<P: AsRef<Path>>(path: P) -> FormulaResult<CalculationConfig> {
        // Start with defaults
        let mut config = CalculationConfig::new();

        // Try to load and merge user config
        if path.as_ref().exists() {
            let user_config = Self::load_from_file(path)?;
            config = user_config;
        }

        Ok(config)
    }
}

/// Pre-built configuration templates for common use cases
pub struct ConfigTemplates;

impl ConfigTemplates {
    /// Template for cycling-focused training
    pub fn cycling() -> CalculationConfig {
        CalculationConfig {
            tss_formula: TssFormula::Classic,
            np_config: NormalizedPowerConfig::new(),
            ftp_method: FtpMethod::TwentyMinute { factor: 0.95 },
            custom_formulas: HashMap::new(),
        }
    }

    /// Template for running-focused training
    pub fn running() -> CalculationConfig {
        let mut config = CalculationConfig::new();
        config.ftp_method = FtpMethod::EightMinute { factor: 0.98 };
        config
    }

    /// Template for triathlon/multisport training
    pub fn triathlon() -> CalculationConfig {
        CalculationConfig::new()
    }

    /// Template as TOML string for cycling
    pub fn cycling_toml() -> &'static str {
        r#"
[calculation]
tss_formula = "Classic"
np_window_seconds = 30
smoothing_algorithm = "RollingAverage"
ftp_method = "TwentyMinute"

# Cycling-specific custom formulas can be added here
# [[custom_formulas]]
# name = "power_to_weight"
# expression = "avg_power / weight"
# description = "Power-to-weight ratio in watts per kilogram"
"#
    }

    /// Template as TOML string for running
    pub fn running_toml() -> &'static str {
        r#"
[calculation]
tss_formula = "Classic"
np_window_seconds = 30
smoothing_algorithm = "RollingAverage"
ftp_method = "EightMinute"

# Running-specific custom formulas
# [[custom_formulas]]
# name = "pace_to_vo2max"
# expression = "100 / (pace * 0.06)"
# description = "Estimated VO2max from running pace"
"#
    }

    /// Template as TOML string for triathlon
    pub fn triathlon_toml() -> &'static str {
        r#"
[calculation]
tss_formula = "Classic"
np_window_seconds = 30
smoothing_algorithm = "RollingAverage"
ftp_method = "TwentyMinute"

# Triathlon-specific custom formulas
[[custom_formulas]]
name = "weekly_load"
expression = "cycling_tss + (running_tss * 1.1) + (swimming_tss * 1.2)"
description = "Total weekly training load with sport weighting"
"#
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_calculation_section() {
        let section = CalculationSection::default();
        assert_eq!(section.tss_formula, "Classic");
        assert_eq!(section.np_window_seconds, 30);
        assert_eq!(section.smoothing_algorithm, "RollingAverage");
        assert_eq!(section.ftp_method, "TwentyMinute");
    }

    #[test]
    fn test_parse_tss_formula_classic() {
        let formula = ConfigLoader::parse_tss_formula("Classic").unwrap();
        assert_eq!(formula, TssFormula::Classic);
    }

    #[test]
    fn test_parse_tss_formula_bikescore() {
        let formula = ConfigLoader::parse_tss_formula("BikeScore").unwrap();
        assert_eq!(formula, TssFormula::BikeScore);
    }

    #[test]
    fn test_parse_tss_formula_custom() {
        let formula =
            ConfigLoader::parse_tss_formula("(duration * IF^1.8) * 100").unwrap();
        assert!(matches!(formula, TssFormula::Custom(_)));
    }

    #[test]
    fn test_parse_smoothing_algorithm() {
        assert_eq!(
            ConfigLoader::parse_smoothing_algorithm("RollingAverage").unwrap(),
            SmoothingAlgorithm::RollingAverage
        );
        let ema = ConfigLoader::parse_smoothing_algorithm("ExponentialMovingAverage")
            .unwrap();
        assert!(matches!(ema, SmoothingAlgorithm::ExponentialMovingAverage { .. }));

        let wma = ConfigLoader::parse_smoothing_algorithm("WeightedMovingAverage")
            .unwrap();
        assert!(matches!(wma, SmoothingAlgorithm::WeightedMovingAverage { .. }));
    }

    #[test]
    fn test_parse_smoothing_algorithm_invalid() {
        let result = ConfigLoader::parse_smoothing_algorithm("InvalidAlgo");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ftp_method() {
        let method = ConfigLoader::parse_ftp_method("TwentyMinute").unwrap();
        assert!(matches!(method, FtpMethod::TwentyMinute { factor: 0.95 }));

        let method = ConfigLoader::parse_ftp_method("EightMinute").unwrap();
        assert!(matches!(method, FtpMethod::EightMinute { factor: 0.98 }));
    }

    #[test]
    fn test_load_minimal_config() {
        let toml_str = r#"
[calculation]
"#;
        let config = ConfigLoader::load_from_string(toml_str).unwrap();
        assert_eq!(config.tss_formula, TssFormula::Classic);
        assert_eq!(config.np_config.window_seconds, 30);
    }

    #[test]
    fn test_load_config_with_custom_formulas() {
        let toml_str = r#"
[calculation]
tss_formula = "Classic"

[[custom_formulas]]
name = "test_formula"
expression = "a + b"
description = "A test formula"
"#;
        let config = ConfigLoader::load_from_string(toml_str).unwrap();
        assert_eq!(config.custom_formulas.len(), 1);
        assert!(config.get_custom_formula("test_formula").is_some());
    }

    #[test]
    fn test_load_config_invalid_toml() {
        let toml_str = r#"
[calculation
tss_formula = "Classic"
"#;
        let result = ConfigLoader::load_from_string(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_invalid_smoothing() {
        let toml_str = r#"
[calculation]
smoothing_algorithm = "InvalidAlgo"
"#;
        let result = ConfigLoader::load_from_string(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_export_to_string() {
        let config = CalculationConfig::new();
        let toml_str = ConfigLoader::export_to_string(&config).unwrap();
        assert!(toml_str.contains("[calculation]"));
        assert!(toml_str.contains("tss_formula"));
    }

    #[test]
    fn test_export_and_reload() {
        let original = CalculationConfig::new();
        let toml_str = ConfigLoader::export_to_string(&original).unwrap();
        let reloaded = ConfigLoader::load_from_string(&toml_str).unwrap();

        assert_eq!(original.tss_formula, reloaded.tss_formula);
        assert_eq!(
            original.np_config.window_seconds,
            reloaded.np_config.window_seconds
        );
        assert_eq!(
            original.np_config.smoothing,
            reloaded.np_config.smoothing
        );
    }

    #[test]
    fn test_cycling_template() {
        let config = ConfigTemplates::cycling();
        assert_eq!(config.tss_formula, TssFormula::Classic);
        assert_eq!(config.np_config.window_seconds, 30);
    }

    #[test]
    fn test_cycling_template_toml() {
        let toml_str = ConfigTemplates::cycling_toml();
        assert!(toml_str.contains("tss_formula = \"Classic\""));
        assert!(toml_str.contains("ftp_method = \"TwentyMinute\""));
    }

    #[test]
    fn test_running_template() {
        let config = ConfigTemplates::running();
        assert_eq!(config.tss_formula, TssFormula::Classic);
    }

    #[test]
    fn test_triathlon_template_toml() {
        let toml_str = ConfigTemplates::triathlon_toml();
        assert!(toml_str.contains("weekly_load"));
    }
}
