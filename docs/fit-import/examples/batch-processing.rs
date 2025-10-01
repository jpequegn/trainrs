// Batch FIT file processing example
//
// This example shows how to process multiple FIT files in a directory,
// handling errors gracefully and collecting statistics.

use trainrs::import::fit::FitImporter;
use trainrs::models::{Sport, Workout};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args()
        .nth(1)
        .unwrap_or_else(|| ".".to_string());

    println!("Processing FIT files in: {}", directory);
    println!();

    let results = process_directory(&directory)?;

    // Display statistics
    println!("Import Summary");
    println!("==============");
    println!("Total files processed: {}", results.total_files);
    println!("Successful imports: {}", results.successful);
    println!("Failed imports: {}", results.failed);
    println!("Total workouts: {}", results.workouts.len());
    println!();

    // Display sport breakdown
    println!("Sport Breakdown");
    println!("===============");
    let sport_counts = count_by_sport(&results.workouts);
    for (sport, count) in sport_counts {
        println!("{:?}: {} workouts", sport, count);
    }
    println!();

    // Display total training load
    let total_tss: f64 = results.workouts
        .iter()
        .filter_map(|w| w.summary.tss)
        .map(|tss| tss.to_f64().unwrap_or(0.0))
        .sum();

    println!("Total Training Load: {:.1} TSS", total_tss);

    Ok(())
}

struct ImportResults {
    total_files: usize,
    successful: usize,
    failed: usize,
    workouts: Vec<Workout>,
    errors: Vec<(PathBuf, String)>,
}

fn process_directory<P: AsRef<Path>>(dir: P) -> Result<ImportResults, Box<dyn std::error::Error>> {
    let importer = FitImporter::new();
    let mut results = ImportResults {
        total_files: 0,
        successful: 0,
        failed: 0,
        workouts: Vec::new(),
        errors: Vec::new(),
    };

    // Find all FIT files
    let fit_files = find_fit_files(dir)?;
    results.total_files = fit_files.len();

    println!("Found {} FIT files", fit_files.len());
    println!();

    // Process each file
    for (i, path) in fit_files.iter().enumerate() {
        print!("[{}/{}] Processing {} ... ",
            i + 1,
            fit_files.len(),
            path.display()
        );

        match importer.import_file(path) {
            Ok(mut workouts) => {
                println!("OK ({} workouts)", workouts.len());
                results.successful += 1;
                results.workouts.append(&mut workouts);
            }
            Err(e) => {
                println!("FAILED: {}", e);
                results.failed += 1;
                results.errors.push((path.clone(), e.to_string()));
            }
        }
    }

    println!();

    Ok(results)
}

fn find_fit_files<P: AsRef<Path>>(dir: P) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut fit_files = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("fit") {
            fit_files.push(path);
        }
    }

    fit_files.sort();
    Ok(fit_files)
}

fn count_by_sport(workouts: &[Workout]) -> HashMap<Sport, usize> {
    let mut counts = HashMap::new();

    for workout in workouts {
        *counts.entry(workout.sport).or_insert(0) += 1;
    }

    counts
}
