// Example: Adding Stryd Running Power support
//
// This example demonstrates how to add support for Stryd running power
// developer fields to TrainRS.

use trainrs::import::fit::{DeveloperFieldRegistry, ApplicationInfo, KnownField};
use trainrs::import::fit::FitImporter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Stryd Running Power Integration Example");
    println!("========================================\n");

    // Step 1: Create registry with Stryd support
    println!("Step 1: Setting up Stryd field definitions");

    let mut registry = DeveloperFieldRegistry::new();

    let stryd = ApplicationInfo {
        uuid: "a42b5e01-d5e9-4eb6-9f42-91234567890a".to_string(),
        name: "Stryd Running Power".to_string(),
        manufacturer: "Stryd".to_string(),
        version: Some("1.0".to_string()),
        fields: vec![
            KnownField {
                field_number: 0,
                name: "running_power".to_string(),
                data_type: "uint16".to_string(),
                units: Some("watts".to_string()),
                scale: Some(1.0),
                offset: None,
                description: Some("Instantaneous running power output".to_string()),
            },
            KnownField {
                field_number: 1,
                name: "form_power".to_string(),
                data_type: "uint16".to_string(),
                units: Some("watts".to_string()),
                scale: Some(1.0),
                offset: None,
                description: Some("Power required to overcome oscillation".to_string()),
            },
            KnownField {
                field_number: 2,
                name: "leg_spring_stiffness".to_string(),
                data_type: "uint16".to_string(),
                units: Some("kN/m".to_string()),
                scale: Some(10.0),
                offset: None,
                description: Some("Leg spring stiffness coefficient".to_string()),
            },
            KnownField {
                field_number: 3,
                name: "air_power".to_string(),
                data_type: "uint16".to_string(),
                units: Some("watts".to_string()),
                scale: Some(1.0),
                offset: None,
                description: Some("Power to overcome air resistance".to_string()),
            },
        ],
    };

    registry.register_application(stryd);
    println!("✓ Registered Stryd with {} fields\n", stryd.fields.len());

    // Step 2: Verify registration
    println!("Step 2: Verifying registration");

    let uuid = "a42b5e01-d5e9-4eb6-9f42-91234567890a";
    assert!(registry.is_registered(uuid));
    println!("✓ UUID is registered");

    let app = registry.get_application(uuid).unwrap();
    println!("✓ Application: {}", app.name);
    println!("✓ Manufacturer: {}", app.manufacturer);
    println!("✓ Fields defined: {}\n", app.fields.len());

    // Step 3: Verify field definitions
    println!("Step 3: Verifying field definitions");

    for field in &app.fields {
        println!("  Field {}: {}", field.field_number, field.name);
        println!("    Type: {}", field.data_type);
        if let Some(units) = &field.units {
            println!("    Units: {}", units);
        }
        if let Some(scale) = field.scale {
            println!("    Scale: {}", scale);
        }
        if let Some(desc) = &field.description {
            println!("    Description: {}", desc);
        }
        println!();
    }

    // Step 4: Test field lookups
    println!("Step 4: Testing field lookups");

    let running_power = registry.get_field(uuid, 0).unwrap();
    assert_eq!(running_power.name, "running_power");
    println!("✓ Found field 0: {}", running_power.name);

    let form_power = registry.get_field(uuid, 1).unwrap();
    assert_eq!(form_power.name, "form_power");
    println!("✓ Found field 1: {}", form_power.name);

    let stiffness = registry.get_field(uuid, 2).unwrap();
    assert_eq!(stiffness.name, "leg_spring_stiffness");
    assert_eq!(stiffness.scale.unwrap(), 10.0);
    println!("✓ Found field 2: {} (scale={})",
        stiffness.name, stiffness.scale.unwrap());

    println!("\n✓ All verifications passed!");

    // Step 5: Use registry with importer
    println!("\nStep 5: Creating importer with Stryd support");

    let importer = FitImporter::with_registry(registry);
    println!("✓ Importer created with custom registry");

    // Now the importer will automatically recognize and parse Stryd fields
    // when importing FIT files

    println!("\n✓ Integration complete!");
    println!("\nNext steps:");
    println!("1. Add the field definitions to src/import/developer_registry.json");
    println!("2. Test with a real Stryd FIT file");
    println!("3. Verify fields are extracted correctly");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stryd_registry() {
        let mut registry = DeveloperFieldRegistry::new();

        let stryd = create_stryd_application();
        registry.register_application(stryd);

        let uuid = "a42b5e01-d5e9-4eb6-9f42-91234567890a";
        assert!(registry.is_registered(uuid));

        let app = registry.get_application(uuid).unwrap();
        assert_eq!(app.name, "Stryd Running Power");
        assert_eq!(app.fields.len(), 4);

        let field = registry.get_field(uuid, 0).unwrap();
        assert_eq!(field.name, "running_power");
    }

    #[test]
    fn test_stryd_field_definitions() {
        let stryd = create_stryd_application();

        // Verify all fields
        assert_eq!(stryd.fields[0].name, "running_power");
        assert_eq!(stryd.fields[0].field_number, 0);
        assert_eq!(stryd.fields[0].data_type, "uint16");

        assert_eq!(stryd.fields[2].name, "leg_spring_stiffness");
        assert_eq!(stryd.fields[2].scale.unwrap(), 10.0);
    }

    fn create_stryd_application() -> ApplicationInfo {
        ApplicationInfo {
            uuid: "a42b5e01-d5e9-4eb6-9f42-91234567890a".to_string(),
            name: "Stryd Running Power".to_string(),
            manufacturer: "Stryd".to_string(),
            version: Some("1.0".to_string()),
            fields: vec![
                KnownField {
                    field_number: 0,
                    name: "running_power".to_string(),
                    data_type: "uint16".to_string(),
                    units: Some("watts".to_string()),
                    scale: Some(1.0),
                    offset: None,
                    description: Some("Running power".to_string()),
                },
                KnownField {
                    field_number: 1,
                    name: "form_power".to_string(),
                    data_type: "uint16".to_string(),
                    units: Some("watts".to_string()),
                    scale: Some(1.0),
                    offset: None,
                    description: Some("Form power".to_string()),
                },
                KnownField {
                    field_number: 2,
                    name: "leg_spring_stiffness".to_string(),
                    data_type: "uint16".to_string(),
                    units: Some("kN/m".to_string()),
                    scale: Some(10.0),
                    offset: None,
                    description: Some("Leg spring stiffness".to_string()),
                },
                KnownField {
                    field_number: 3,
                    name: "air_power".to_string(),
                    data_type: "uint16".to_string(),
                    units: Some("watts".to_string()),
                    scale: Some(1.0),
                    offset: None,
                    description: Some("Air power".to_string()),
                },
            ],
        }
    }
}
