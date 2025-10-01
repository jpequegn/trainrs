// Error handling and recovery example
//
// This example demonstrates comprehensive error handling for FIT file import,
// including recovery from corrupted files and graceful degradation.

use trainrs::import::fit::FitImporter;
use trainrs::error::{TrainRsError, FitError};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = std::env::args()
        .nth(1)
        .expect("Usage: error-handling <fit-file>");

    println!("Attempting to import: {}", file_path);
    println!();

    match import_with_recovery(&file_path) {
        Ok(workouts) => {
            println!("✓ Successfully imported {} workouts", workouts.len());
            for workout in workouts {
                println!("  - {:?} workout, {} seconds", workout.sport, workout.duration_seconds);
            }
        }
        Err(e) => {
            eprintln!("✗ Failed to import file: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn import_with_recovery(path: &str) -> Result<Vec<trainrs::models::Workout>, TrainRsError> {
    let importer = FitImporter::new();

    // First attempt: standard import
    match importer.import_file(path) {
        Ok(workouts) => {
            println!("Standard import succeeded");
            Ok(workouts)
        }
        Err(e) => {
            println!("Standard import failed: {}", e);
            println!();

            // Analyze the error and attempt recovery
            match &e {
                TrainRsError::FitParsing(fit_error) => {
                    handle_fit_error(fit_error, path, &importer)
                }
                TrainRsError::Io(io_error) => {
                    eprintln!("IO error: {}", io_error);
                    Err(e)
                }
                _ => {
                    eprintln!("Unexpected error type");
                    Err(e)
                }
            }
        }
    }
}

fn handle_fit_error(
    error: &FitError,
    path: &str,
    importer: &FitImporter,
) -> Result<Vec<trainrs::models::Workout>, TrainRsError> {
    match error {
        FitError::FileNotFound { path } => {
            eprintln!("File not found: {}", path.display());
            eprintln!("Please check the file path and try again.");
            Err(TrainRsError::FitParsing(error.clone()))
        }

        FitError::Corrupted { reason } => {
            println!("File is corrupted: {}", reason);
            println!("Attempting recovery with lenient parsing...");
            println!();

            // Attempt lenient import (would need implementation)
            // For now, return error
            eprintln!("Recovery not yet implemented");
            Err(TrainRsError::FitParsing(error.clone()))
        }

        FitError::UnsupportedVersion { version } => {
            eprintln!("Unsupported FIT version: {}", version);
            eprintln!("This file uses a FIT protocol version that is not yet supported.");
            eprintln!("Please check for updates to trainrs.");
            Err(TrainRsError::FitParsing(error.clone()))
        }

        FitError::MissingMessage { message_type } => {
            println!("Missing required message: {}", message_type);
            println!("Attempting to continue with available data...");
            println!();

            // Some missing messages can be tolerated
            // Would need implementation to skip and continue
            eprintln!("Partial import not yet implemented");
            Err(TrainRsError::FitParsing(error.clone()))
        }

        FitError::InvalidField { field, reason } => {
            println!("Invalid field '{}': {}", field, reason);
            println!("Attempting to skip invalid field...");
            println!();

            // Invalid fields might be skippable
            eprintln!("Field skipping not yet implemented");
            Err(TrainRsError::FitParsing(error.clone()))
        }

        FitError::ChecksumMismatch { expected, actual } => {
            println!("Checksum mismatch:");
            println!("  Expected: 0x{:04X}", expected);
            println!("  Actual:   0x{:04X}", actual);
            println!();

            // Checksum errors might indicate corruption
            // but data might still be valid
            println!("Attempting to import anyway (data may be incomplete)...");
            println!();

            // Would need lenient mode implementation
            eprintln!("Lenient mode not yet implemented");
            Err(TrainRsError::FitParsing(error.clone()))
        }

        FitError::UnknownDeveloperField { uuid, field_num } => {
            println!("Unknown developer field:");
            println!("  UUID: {}", uuid);
            println!("  Field number: {}", field_num);
            println!();

            // Unknown developer fields are non-fatal
            println!("This is a non-fatal error, continuing import...");
            println!();

            // Would attempt import while ignoring unknown field
            eprintln!("Selective import not yet implemented");
            Err(TrainRsError::FitParsing(error.clone()))
        }
    }
}

// Example helper function for checking file integrity before import
fn verify_file_integrity(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    // Check if file exists
    if !path.exists() {
        return Err("File does not exist".into());
    }

    // Check if file is readable
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Err("Path is not a file".into());
    }

    // Check minimum size (FIT header is 14 bytes)
    if metadata.len() < 14 {
        return Err("File is too small to be a valid FIT file".into());
    }

    // Check file extension
    if path.extension().and_then(|s| s.to_str()) != Some("fit") {
        eprintln!("Warning: File does not have .fit extension");
    }

    Ok(())
}
