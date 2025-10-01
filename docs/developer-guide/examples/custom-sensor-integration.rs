// Example: Custom Sensor Integration
//
// This example shows how to integrate a custom muscle oxygen sensor
// with scaling and validation.

use trainrs::import::fit::{DeveloperFieldRegistry, ApplicationInfo, KnownField};
use trainrs::import::fit::FitImporter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Custom Sensor Integration Example");
    println!("==================================\n");

    // Example: Moxy Muscle Oxygen Monitor
    println!("Integrating Moxy Muscle Oxygen Monitor\n");

    let mut registry = DeveloperFieldRegistry::new();

    // Define Moxy sensor fields
    let moxy = create_moxy_application();
    registry.register_application(moxy);

    println!("✓ Registered Moxy sensor");
    println!("  UUID: c4d5e6f7-8901-4bcd-ef01-234567890123");
    println!("  Fields: 2 (SmO2, tHb)\n");

    // Verify field definitions
    let uuid = "c4d5e6f7-8901-4bcd-ef01-234567890123";

    println!("Field Definitions:");
    println!("-----------------");

    let smo2 = registry.get_field(uuid, 0).unwrap();
    println!("SmO2 (Field 0):");
    println!("  Type: {}", smo2.data_type);
    println!("  Units: {}", smo2.units.as_ref().unwrap());
    println!("  Scale: {} (raw / 10 = %)", smo2.scale.unwrap());
    println!("  Example: raw value 75 → 75/10 = 7.5%\n");

    let thb = registry.get_field(uuid, 1).unwrap();
    println!("tHb (Field 1):");
    println!("  Type: {}", thb.data_type);
    println!("  Units: {}", thb.units.as_ref().unwrap());
    println!("  Scale: {} (raw / 100 = g/dL)", thb.scale.unwrap());
    println!("  Example: raw value 1250 → 1250/100 = 12.50 g/dL\n");

    // Demonstrate value conversion
    println!("Value Conversion Examples:");
    println!("-------------------------");

    demonstrate_smo2_conversion();
    demonstrate_thb_conversion();

    // Demonstrate validation
    println!("\nData Validation:");
    println!("----------------");

    demonstrate_validation();

    println!("\n✓ Custom sensor integration complete!");

    Ok(())
}

fn create_moxy_application() -> ApplicationInfo {
    ApplicationInfo {
        uuid: "c4d5e6f7-8901-4bcd-ef01-234567890123".to_string(),
        name: "Moxy Muscle Oxygen Monitor".to_string(),
        manufacturer: "Moxy Monitor".to_string(),
        version: Some("1.5".to_string()),
        fields: vec![
            KnownField {
                field_number: 0,
                name: "smo2".to_string(),
                data_type: "uint16".to_string(),
                units: Some("%".to_string()),
                scale: Some(10.0),
                offset: None,
                description: Some("Muscle oxygen saturation (SmO2)".to_string()),
            },
            KnownField {
                field_number: 1,
                name: "thb".to_string(),
                data_type: "uint16".to_string(),
                units: Some("g/dL".to_string()),
                scale: Some(100.0),
                offset: None,
                description: Some("Total hemoglobin concentration (tHb)".to_string()),
            },
        ],
    }
}

fn demonstrate_smo2_conversion() {
    println!("SmO2 Conversion:");

    let test_values = vec![
        (750, "75.0% (normal resting)"),
        (650, "65.0% (moderate exercise)"),
        (450, "45.0% (hard effort)"),
        (300, "30.0% (near anaerobic threshold)"),
    ];

    for (raw, description) in test_values {
        let smo2 = convert_smo2(raw);
        println!("  Raw: {:4} → {:.1}% ({})", raw, smo2, description);
    }
}

fn demonstrate_thb_conversion() {
    println!("\ntHb Conversion:");

    let test_values = vec![
        (1250, "12.50 g/dL (typical)"),
        (1300, "13.00 g/dL (well-perfused)"),
        (1150, "11.50 g/dL (reduced perfusion)"),
    ];

    for (raw, description) in test_values {
        let thb = convert_thb(raw);
        println!("  Raw: {:4} → {:.2} g/dL ({})", raw, thb, description);
    }
}

fn demonstrate_validation() {
    println!("Validating SmO2 values:");

    let test_cases = vec![
        (750, true, "normal value"),
        (1001, false, "exceeds 100%"),
        (0, false, "zero (likely sensor error)"),
        (200, true, "low but valid"),
    ];

    for (raw, should_pass, description) in test_cases {
        let smo2 = convert_smo2(raw);
        match validate_smo2(smo2) {
            Ok(_) => {
                if should_pass {
                    println!("  ✓ {:.1}% valid ({})", smo2, description);
                } else {
                    println!("  ✗ {:.1}% should have failed ({})", smo2, description);
                }
            }
            Err(e) => {
                if !should_pass {
                    println!("  ✓ {:.1}% rejected: {} ({})", smo2, e, description);
                } else {
                    println!("  ✗ {:.1}% incorrectly rejected ({})", smo2, description);
                }
            }
        }
    }
}

// Conversion functions
fn convert_smo2(raw: u16) -> f64 {
    raw as f64 / 10.0
}

fn convert_thb(raw: u16) -> f64 {
    raw as f64 / 100.0
}

// Validation functions
fn validate_smo2(smo2: f64) -> Result<f64, String> {
    match smo2 {
        s if s < 0.0 => Err("SmO2 cannot be negative".to_string()),
        s if s == 0.0 => Err("SmO2 of zero indicates sensor error".to_string()),
        s if s > 100.0 => Err(format!("SmO2 cannot exceed 100% (got {:.1}%)", s)),
        s => Ok(s),
    }
}

fn validate_thb(thb: f64) -> Result<f64, String> {
    match thb {
        t if t < 5.0 => Err("tHb too low (< 5 g/dL)".to_string()),
        t if t > 20.0 => Err("tHb too high (> 20 g/dL)".to_string()),
        t => Ok(t),
    }
}

// Analysis functions
fn analyze_smo2_trend(samples: &[f64]) -> String {
    if samples.is_empty() {
        return "No data".to_string();
    }

    let avg = samples.iter().sum::<f64>() / samples.len() as f64;
    let min = samples.iter().copied().fold(f64::INFINITY, f64::min);
    let max = samples.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    // Detect desaturation events (< 50%)
    let desaturations = samples.iter().filter(|&&s| s < 50.0).count();

    format!(
        "Avg: {:.1}%, Range: {:.1}-{:.1}%, Desaturations: {}",
        avg, min, max, desaturations
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smo2_conversion() {
        assert_eq!(convert_smo2(750), 75.0);
        assert_eq!(convert_smo2(100), 10.0);
        assert_eq!(convert_smo2(0), 0.0);
    }

    #[test]
    fn test_thb_conversion() {
        assert_eq!(convert_thb(1250), 12.50);
        assert_eq!(convert_thb(1000), 10.00);
    }

    #[test]
    fn test_smo2_validation() {
        assert!(validate_smo2(75.0).is_ok());
        assert!(validate_smo2(50.0).is_ok());
        assert!(validate_smo2(0.0).is_err());
        assert!(validate_smo2(101.0).is_err());
        assert!(validate_smo2(-1.0).is_err());
    }

    #[test]
    fn test_thb_validation() {
        assert!(validate_thb(12.5).is_ok());
        assert!(validate_thb(10.0).is_ok());
        assert!(validate_thb(4.0).is_err());
        assert!(validate_thb(25.0).is_err());
    }

    #[test]
    fn test_smo2_analysis() {
        let samples = vec![75.0, 70.0, 65.0, 45.0, 50.0];
        let analysis = analyze_smo2_trend(&samples);
        assert!(analysis.contains("Avg"));
        assert!(analysis.contains("Desaturations: 1"));
    }

    #[test]
    fn test_moxy_registry() {
        let mut registry = DeveloperFieldRegistry::new();
        registry.register_application(create_moxy_application());

        let uuid = "c4d5e6f7-8901-4bcd-ef01-234567890123";
        assert!(registry.is_registered(uuid));

        let app = registry.get_application(uuid).unwrap();
        assert_eq!(app.name, "Moxy Muscle Oxygen Monitor");
        assert_eq!(app.fields.len(), 2);
    }
}
