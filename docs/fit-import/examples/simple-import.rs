// Simple FIT file import example
//
// This example demonstrates the most basic usage of the FIT importer.

use trainrs::import::fit::FitImporter;
use trainrs::models::Sport;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create importer with default settings
    let importer = FitImporter::new();

    // Import a FIT file
    let workouts = importer.import_file("workout.fit")?;

    // Display basic information about each workout
    for (i, workout) in workouts.iter().enumerate() {
        println!("Workout {}", i + 1);
        println!("  Date: {}", workout.date);
        println!("  Sport: {:?}", workout.sport);
        println!("  Duration: {}s ({}m)",
            workout.duration_seconds,
            workout.duration_seconds / 60
        );

        // Display cycling-specific metrics
        if workout.sport == Sport::Cycling {
            if let Some(avg_power) = workout.summary.avg_power {
                println!("  Average Power: {}W", avg_power);
            }

            if let Some(np) = workout.summary.normalized_power {
                println!("  Normalized Power: {}W", np);
            }

            if let Some(tss) = workout.summary.tss {
                println!("  Training Stress Score: {:.1}", tss);
            }
        }

        // Display heart rate metrics (all sports)
        if let Some(avg_hr) = workout.summary.avg_heart_rate {
            println!("  Average Heart Rate: {} bpm", avg_hr);
        }

        // Display distance (running/cycling)
        if let Some(distance) = workout.summary.total_distance {
            println!("  Distance: {:.2} km", distance / 1000.0);
        }

        println!();
    }

    Ok(())
}
