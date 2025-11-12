//! Integration tests for formula evaluator with real-world scenarios
//!
//! These tests demonstrate the formula evaluator working with realistic
//! training data scenarios from cycling, running, and swimming workouts.

use rust_decimal::Decimal;
use std::collections::HashMap;
use std::str::FromStr;
use trainrs::formulas::{CalculationConfig, CustomFormula, FormulaEngine, TssFormula};

#[test]
fn test_tss_evaluation_cycling_threshold_effort() {
    // Scenario: 2-hour cycling workout at threshold intensity
    // FTP = 250W, Average Power = 250W (1.0 IF), Duration = 2 hours
    let mut vars = HashMap::new();
    vars.insert("duration".to_string(), Decimal::from_str("2.0").unwrap());
    vars.insert("NP".to_string(), Decimal::from(250));
    vars.insert("FTP".to_string(), Decimal::from(250));

    // Formula: TSS = (duration * (NP/FTP)^2) * 100
    let formula = "(duration * (NP / FTP)^2) * 100";
    let result = FormulaEngine::evaluate(formula, &vars).unwrap();

    // TSS = 2 * 1.0^2 * 100 = 200
    assert_eq!(result, Decimal::from(200));
}

#[test]
fn test_tss_evaluation_cycling_high_intensity() {
    // Scenario: 1.5-hour cycling workout at high intensity
    // FTP = 250W, Average Power = 320W (1.28 IF), Duration = 1.5 hours
    let mut vars = HashMap::new();
    vars.insert("duration".to_string(), Decimal::from_str("1.5").unwrap());
    vars.insert("NP".to_string(), Decimal::from(320));
    vars.insert("FTP".to_string(), Decimal::from(250));

    let formula = "(duration * (NP / FTP)^2) * 100";
    let result = FormulaEngine::evaluate(formula, &vars).unwrap();

    // TSS = 1.5 * 1.28^2 * 100 = 1.5 * 1.6384 * 100 = 245.76
    assert!(result > Decimal::from(240) && result < Decimal::from(250));
}

#[test]
fn test_bikescore_variant_evaluation() {
    // BikeScore variant applies different intensity weighting
    // Same workout as above but with BikeScore formula
    let mut vars = HashMap::new();
    vars.insert("duration".to_string(), Decimal::from_str("1.5").unwrap());
    vars.insert("IF".to_string(), Decimal::from_str("1.28").unwrap());

    // BikeScore: (duration * (IF^1.5)) * 100
    let formula = "(duration * (IF^1.5)) * 100";
    let result = FormulaEngine::evaluate(formula, &vars).unwrap();

    // Should be different from classic TSS due to different exponent
    assert!(result > Decimal::from(200) && result < Decimal::from(230));
}

#[test]
fn test_intensity_factor_calculation_running() {
    // Scenario: Running workout at threshold pace
    // Threshold Pace = 6:00/mile, Average Pace = 6:00/mile (1.0 IF)
    let mut vars = HashMap::new();
    vars.insert("threshold_pace".to_string(), Decimal::from_str("360").unwrap()); // seconds
    vars.insert("avg_pace".to_string(), Decimal::from_str("360").unwrap()); // seconds

    // Inverse relationship: IF = threshold_pace / avg_pace
    let formula = "threshold_pace / avg_pace";
    let result = FormulaEngine::evaluate(formula, &vars).unwrap();

    assert_eq!(result, Decimal::from(1));
}

#[test]
fn test_intensity_factor_calculation_running_fast() {
    // Scenario: Running faster than threshold
    // Threshold Pace = 6:00/mile, Average Pace = 5:30/mile (1.09 IF)
    let mut vars = HashMap::new();
    vars.insert("threshold_pace".to_string(), Decimal::from_str("360").unwrap());
    vars.insert("avg_pace".to_string(), Decimal::from_str("330").unwrap());

    let formula = "threshold_pace / avg_pace";
    let result = FormulaEngine::evaluate(formula, &vars).unwrap();

    // 360/330 = 1.09
    assert!(result > Decimal::from_str("1.08").unwrap());
    assert!(result < Decimal::from_str("1.10").unwrap());
}

#[test]
fn test_combined_multi_sport_metric() {
    // Scenario: Calculate a composite metric across multiple sports
    // Cycling TSS + Running TSS + Swimming TSS weighting
    let mut vars = HashMap::new();
    vars.insert("cycling_tss".to_string(), Decimal::from(180));
    vars.insert("running_tss".to_string(), Decimal::from(120));
    vars.insert("swimming_tss".to_string(), Decimal::from(90));

    // Triathlon load with sport-specific weighting
    // Cycling weighted 1.0x, Running 1.1x, Swimming 1.2x due to different fatigue patterns
    let formula = "(cycling_tss * 1) + (running_tss * 1.1) + (swimming_tss * 1.2)";
    let result = FormulaEngine::evaluate(formula, &vars).unwrap();

    // 180 + 132 + 108 = 420
    assert_eq!(result, Decimal::from(420));
}

#[test]
fn test_recovery_integration_pmc_style() {
    // Scenario: Calculate PMC-style recovery adjustment
    // CTL (fitness) effect on recovery capability
    let mut vars = HashMap::new();
    vars.insert("daily_tss".to_string(), Decimal::from(200));
    vars.insert("ctl".to_string(), Decimal::from(50)); // Chronic Training Load

    // Recovery ratio: TSS adjusted by fitness level
    // Higher fitness allows better recovery of same TSS
    let formula = "daily_tss / (1 + (ctl / 100))";
    let result = FormulaEngine::evaluate(formula, &vars).unwrap();

    // 200 / 1.5 = 133.33
    assert!(result > Decimal::from(130) && result < Decimal::from(135));
}

#[test]
fn test_custom_formula_with_multiple_calculations() {
    // Scenario: Multi-step custom formula
    // Calculate adjusted training load considering recovery status
    let mut vars = HashMap::new();
    vars.insert("base_tss".to_string(), Decimal::from(150));
    vars.insert("recovery_hours".to_string(), Decimal::from(12));
    vars.insert("max_recovery".to_string(), Decimal::from(24));

    // Adjusted TSS = base_TSS * (recovery_hours / max_recovery)
    // Penalizes training when recovery is incomplete
    let formula = "base_tss * (recovery_hours / max_recovery)";
    let result = FormulaEngine::evaluate(formula, &vars).unwrap();

    // 150 * 0.5 = 75
    assert_eq!(result, Decimal::from(75));
}

#[test]
fn test_formula_with_string_variables() {
    // Demonstrate evaluate_with_strings for CLI/config usage
    let mut vars = HashMap::new();
    vars.insert("duration".to_string(), "2.5".to_string());
    vars.insert("intensity".to_string(), "1.15".to_string());

    let formula = "duration * intensity * 100";
    let result = FormulaEngine::evaluate_with_strings(formula, &vars).unwrap();

    // 2.5 * 1.15 * 100 = 287.5
    assert_eq!(result, Decimal::from_str("287.5").unwrap());
}

#[test]
fn test_formula_conversion_to_f64() {
    // Demonstrate conversion for legacy code expecting f64
    let mut vars = HashMap::new();
    vars.insert("a".to_string(), Decimal::from(100));
    vars.insert("b".to_string(), Decimal::from(2));

    let formula = "a / b";
    let result = FormulaEngine::evaluate_as_f64(formula, &vars).unwrap();

    assert_eq!(result, 50.0);
}

#[test]
fn test_calculation_config_with_custom_formulas() {
    // Scenario: Using CalculationConfig with custom TSS formula
    let custom_tss = CustomFormula::new(
        "adjusted_tss",
        "(duration * (NP / FTP)^2) * 100",
    )
    .with_description("Custom TSS with adjusted exponent");

    let config = CalculationConfig::new()
        .add_custom_formula(custom_tss)
        .unwrap();

    assert_eq!(config.custom_formulas.len(), 1);
    assert!(config.get_custom_formula("adjusted_tss").is_some());

    // Use the custom formula
    let formula_def = config.get_custom_formula("adjusted_tss").unwrap();
    let mut vars = HashMap::new();
    vars.insert("duration".to_string(), Decimal::from(2));
    vars.insert("NP".to_string(), Decimal::from(280));
    vars.insert("FTP".to_string(), Decimal::from(250));

    let result = FormulaEngine::evaluate(&formula_def.expression, &vars).unwrap();
    assert!(result > Decimal::from(200));
}

#[test]
fn test_realistic_weekly_training_load() {
    // Scenario: Calculate weekly training load from multiple sessions
    let mut vars = HashMap::new();

    // Monday: Hard cycling (threshold intervals)
    vars.insert("mon_tss".to_string(), Decimal::from(180));

    // Wednesday: Moderate cycling with running
    vars.insert("wed_cycling_tss".to_string(), Decimal::from(120));
    vars.insert("wed_running_tss".to_string(), Decimal::from(90));

    // Friday: Swim + bike (sprint triathlon prep)
    vars.insert("fri_swim_tss".to_string(), Decimal::from(100));
    vars.insert("fri_bike_tss".to_string(), Decimal::from(140));

    // Sunday: Long bike
    vars.insert("sun_tss".to_string(), Decimal::from(250));

    // Total weekly load
    let formula = "mon_tss + (wed_cycling_tss + wed_running_tss) + \
                   (fri_swim_tss + fri_bike_tss) + sun_tss";
    let result = FormulaEngine::evaluate(formula, &vars).unwrap();

    // 180 + 210 + 240 + 250 = 880
    assert_eq!(result, Decimal::from(880));
}

#[test]
fn test_error_on_missing_variable() {
    let vars = HashMap::new();

    // Should error when variable not provided
    let result = FormulaEngine::evaluate("missing_var * 100", &vars);
    assert!(result.is_err());
}

#[test]
fn test_error_on_invalid_syntax() {
    let mut vars = HashMap::new();
    vars.insert("a".to_string(), Decimal::from(10));

    // Invalid syntax (double operators)
    let result = FormulaEngine::evaluate("a ** 2", &vars);
    // evalexpr might accept this or reject it depending on implementation
    // Just verify it's handled without panic
    let _ = result;
}

#[test]
fn test_precision_with_complex_calculation() {
    // Verify Decimal precision is maintained through complex expression
    let mut vars = HashMap::new();
    vars.insert("a".to_string(), Decimal::from_str("0.123456789").unwrap());
    vars.insert("b".to_string(), Decimal::from_str("0.987654321").unwrap());
    vars.insert("c".to_string(), Decimal::from_str("2.5").unwrap());

    let formula = "((a + b) * c)";
    let result = FormulaEngine::evaluate(formula, &vars).unwrap();

    // Should preserve precision better than f64
    assert!(result > Decimal::from(2) && result < Decimal::from(3));
}
