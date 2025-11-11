use crate::models::Workout;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

pub mod csv;
pub mod developer_registry;
pub mod fit;
pub mod fit_recovery;
pub mod fit_streaming;
pub mod gpx;
pub mod streaming;
pub mod tcx;
pub mod validation;
pub mod validation_rules;

/// Trait for importing workout data from different file formats
pub trait ImportFormat {
    /// Check if this importer can handle the given file
    fn can_import(&self, file_path: &Path) -> bool;

    /// Import workout data from the file
    fn import_file(&self, file_path: &Path) -> Result<Vec<Workout>>;

    /// Get the format name for this importer
    fn get_format_name(&self) -> &'static str;
}

/// Manager for coordinating different import formats
pub struct ImportManager {
    importers: Vec<Box<dyn ImportFormat>>,
}

impl ImportManager {
    /// Create a new import manager with all available importers
    pub fn new() -> Self {
        let importers: Vec<Box<dyn ImportFormat>> = vec![
            Box::new(csv::CsvImporter::new()),
            Box::new(tcx::TcxImporter::new()),
            Box::new(gpx::GpxImporter::new()),
            Box::new(fit::FitImporter::new()),
        ];

        Self { importers }
    }

    /// Import a single file, auto-detecting the format
    pub fn import_file(&self, file_path: &Path) -> Result<Vec<Workout>> {
        // Find an importer that can handle this file
        for importer in &self.importers {
            if importer.can_import(file_path) {
                println!(
                    "Importing {} using {} format...",
                    file_path.display(),
                    importer.get_format_name()
                );
                return importer.import_file(file_path);
            }
        }

        anyhow::bail!("No importer found for file: {}", file_path.display());
    }

    /// Import all files from a directory
    pub fn import_directory(&self, dir_path: &Path) -> Result<Vec<Workout>> {
        let mut all_workouts = Vec::new();

        // Collect all supported files
        let files = self.collect_importable_files(dir_path)?;

        if files.is_empty() {
            println!("No importable files found in {}", dir_path.display());
            return Ok(all_workouts);
        }

        // Set up progress bar
        let pb = ProgressBar::new(files.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({msg})",
                )
                .unwrap()
                .progress_chars("#>-"),
        );

        // Import each file
        for file_path in files {
            pb.set_message(format!(
                "Processing {}",
                file_path.file_name().unwrap_or_default().to_string_lossy()
            ));

            match self.import_file(&file_path) {
                Ok(mut workouts) => {
                    all_workouts.append(&mut workouts);
                    pb.println(format!(
                        "✓ Imported {} workouts from {}",
                        workouts.len(),
                        file_path.file_name().unwrap_or_default().to_string_lossy()
                    ));
                }
                Err(e) => {
                    pb.println(format!(
                        "✗ Failed to import {}: {}",
                        file_path.file_name().unwrap_or_default().to_string_lossy(),
                        e
                    ));
                }
            }

            pb.inc(1);
        }

        pb.finish_with_message("Import complete");
        Ok(all_workouts)
    }

    /// Collect all files that can be imported from a directory
    fn collect_importable_files(&self, dir_path: &Path) -> Result<Vec<std::path::PathBuf>> {
        let mut files = Vec::new();

        if !dir_path.is_dir() {
            anyhow::bail!("Path is not a directory: {}", dir_path.display());
        }

        for entry in std::fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                // Check if any importer can handle this file
                for importer in &self.importers {
                    if importer.can_import(&path) {
                        files.push(path);
                        break;
                    }
                }
            }
        }

        Ok(files)
    }

    /// Validate a file without importing
    pub fn validate_file(&self, file_path: &Path) -> Result<()> {
        // Find an importer that can handle this file
        for importer in &self.importers {
            if importer.can_import(file_path) {
                println!(
                    "Validating {} using {} format...",
                    file_path.display(),
                    importer.get_format_name()
                );

                // Try to import but don't return the data
                match importer.import_file(file_path) {
                    Ok(workouts) => {
                        println!("✓ File is valid: {} workouts found", workouts.len());
                        return Ok(());
                    }
                    Err(e) => {
                        anyhow::bail!("Validation failed: {}", e);
                    }
                }
            }
        }

        anyhow::bail!("No importer found for file: {}", file_path.display());
    }

    /// Check if this manager can import a given file (helper for streaming)
    pub fn can_import_file(&self, file_path: &Path) -> bool {
        self.importers.iter().any(|importer| importer.can_import(file_path))
    }

    /// Get reference to importers for external use
    #[allow(dead_code)]
    pub fn get_importers(&self) -> &[Box<dyn ImportFormat>] {
        &self.importers
    }
}

impl Default for ImportManager {
    fn default() -> Self {
        Self::new()
    }
}
