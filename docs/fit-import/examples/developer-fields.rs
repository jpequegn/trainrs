// Developer field extraction example
//
// This example demonstrates how to work with custom developer fields
// from applications like Xert, TrainerRoad, and Wahoo SYSTM.

use trainrs::import::fit::FitImporter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = std::env::args()
        .nth(1)
        .expect("Usage: developer-fields <fit-file>");

    // Create importer with default registry
    let importer = FitImporter::new();

    // Import the file
    let workouts = importer.import_file(&file_path)?;

    // Access the developer field registry
    let registry = importer.registry();

    println!("Developer Field Registry");
    println!("========================");
    println!();

    // Display all registered applications
    println!("Registered Applications:");
    let apps = registry.list_applications();
    for app in apps {
        println!("  - {}", app);

        // Show fields for this app
        if let Some(fields) = registry.get_fields_by_app_name(&app) {
            for field in fields {
                println!("    * {} ({}): {}",
                    field.field_name,
                    field.field_type,
                    field.units.as_deref().unwrap_or("no units")
                );

                if let Some(desc) = &field.description {
                    println!("      {}", desc);
                }
            }
        }
        println!();
    }

    // Check for specific applications
    println!("Checking for specific applications...");
    println!();

    check_for_xert(&registry);
    check_for_trainerroad(&registry);
    check_for_wahoo(&registry);

    // Display workout information
    println!("Workout Information");
    println!("===================");
    println!();

    for (i, workout) in workouts.iter().enumerate() {
        println!("Workout {}", i + 1);
        println!("  Sport: {:?}", workout.sport);
        println!("  Duration: {}s", workout.duration_seconds);
        println!("  Data points: {}", workout.data_points.len());
        println!();

        // Note: Actual developer field data extraction would require
        // additional implementation in the DataPoint structure
        // This example shows the registry inspection capabilities
    }

    Ok(())
}

fn check_for_xert(registry: &trainrs::import::fit::DeveloperFieldRegistry) {
    if let Some(fields) = registry.get_fields_by_app_name("Xert") {
        println!("✓ Xert fields found ({} fields)", fields.len());

        // Xert provides workout difficulty, focus, and recommendations
        for field in fields {
            if field.field_name.contains("difficulty") {
                println!("  Found difficulty metric: {}", field.field_name);
            }
        }
    } else {
        println!("✗ No Xert fields found");
    }
    println!();
}

fn check_for_trainerroad(registry: &trainrs::import::fit::DeveloperFieldRegistry) {
    if let Some(fields) = registry.get_fields_by_app_name("TrainerRoad") {
        println!("✓ TrainerRoad fields found ({} fields)", fields.len());

        // TrainerRoad provides structured workout details
        for field in fields {
            if field.field_name.contains("workout") {
                println!("  Found workout field: {}", field.field_name);
            }
        }
    } else {
        println!("✗ No TrainerRoad fields found");
    }
    println!();
}

fn check_for_wahoo(registry: &trainrs::import::fit::DeveloperFieldRegistry) {
    if let Some(fields) = registry.get_fields_by_app_name("Wahoo") {
        println!("✓ Wahoo SYSTM fields found ({} fields)", fields.len());

        // Wahoo provides training targets and zones
        for field in fields {
            if field.field_name.contains("target") || field.field_name.contains("zone") {
                println!("  Found target/zone field: {}", field.field_name);
            }
        }
    } else {
        println!("✗ No Wahoo SYSTM fields found");
    }
    println!();
}
