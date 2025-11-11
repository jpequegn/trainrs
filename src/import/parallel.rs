//! Parallel processing for batch FIT imports using rayon
//!
//! Provides high-performance multi-threaded import capabilities with:
//! - Configurable thread pool sizing
//! - Progress tracking and reporting
//! - Error collection and recovery
//! - Memory-efficient processing

use crate::models::Workout;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use rayon::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};
use tracing::{info, warn, debug};

use super::fit::FitImporter;
use super::ImportFormat;

/// Configuration for parallel import operations
#[derive(Debug, Clone)]
pub struct ParallelImportConfig {
    /// Number of threads for parallel processing
    pub num_threads: Option<usize>,
    /// Show progress bar during import
    pub show_progress: bool,
    /// Maximum files to process in parallel
    pub batch_size: Option<usize>,
    /// Continue processing on errors
    pub continue_on_error: bool,
}

impl Default for ParallelImportConfig {
    fn default() -> Self {
        Self {
            num_threads: None, // Use rayon default (number of CPUs)
            show_progress: true,
            batch_size: None,  // No limit
            continue_on_error: true,
        }
    }
}

/// Result of a single file import in parallel context
#[derive(Debug, Clone)]
pub struct FileImportResult {
    /// Path to the file that was processed
    pub file_path: PathBuf,
    /// Workouts imported from this file
    pub workouts: Vec<Workout>,
    /// Duration in milliseconds for this file
    pub duration_ms: u128,
    /// Whether import succeeded
    pub success: bool,
    /// Error message if import failed
    pub error: Option<String>,
}

/// Summary of parallel import operation
#[derive(Debug, Clone)]
pub struct ParallelImportSummary {
    /// Total files processed
    pub total_files: usize,
    /// Files successfully imported
    pub successful_files: usize,
    /// Files with errors
    pub failed_files: usize,
    /// Total workouts imported
    pub total_workouts: usize,
    /// Total duration in milliseconds
    pub total_duration_ms: u128,
    /// Per-file results
    pub results: Vec<FileImportResult>,
    /// Errors encountered
    pub errors: Vec<(PathBuf, String)>,
}

impl ParallelImportSummary {
    /// Get throughput (files per second)
    pub fn throughput_files_per_sec(&self) -> f64 {
        if self.total_duration_ms == 0 {
            return 0.0;
        }
        (self.successful_files as f64 / self.total_duration_ms as f64) * 1000.0
    }

    /// Get average time per file
    pub fn avg_time_per_file_ms(&self) -> f64 {
        if self.successful_files == 0 {
            return 0.0;
        }
        self.total_duration_ms as f64 / self.successful_files as f64
    }

    /// Check if import was completely successful
    pub fn is_fully_successful(&self) -> bool {
        self.failed_files == 0
    }

    /// Get human-readable summary
    pub fn to_string_pretty(&self) -> String {
        format!(
            "Parallel Import Summary\n  \
             Total Files: {}\n  \
             Successful: {}\n  \
             Failed: {}\n  \
             Total Workouts: {}\n  \
             Total Time: {:.2}s\n  \
             Throughput: {:.2} files/sec\n  \
             Avg Time/File: {:.2}ms",
            self.total_files,
            self.successful_files,
            self.failed_files,
            self.total_workouts,
            self.total_duration_ms as f64 / 1000.0,
            self.throughput_files_per_sec(),
            self.avg_time_per_file_ms()
        )
    }
}

/// Parallel import manager
pub struct ParallelImporter {
    pub config: ParallelImportConfig,
    importer: FitImporter,
}

impl ParallelImporter {
    /// Create new parallel importer with default config
    pub fn new() -> Self {
        Self::with_config(ParallelImportConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: ParallelImportConfig) -> Self {
        Self {
            config,
            importer: FitImporter::new(),
        }
    }

    /// Import multiple files in parallel
    pub fn import_files(&self, file_paths: &[PathBuf]) -> Result<(Vec<Workout>, ParallelImportSummary)> {
        let start_time = std::time::Instant::now();

        info!("Starting parallel import of {} files", file_paths.len());

        // Create progress bar if requested
        let progress = if self.config.show_progress {
            Some(ProgressBar::new(file_paths.len() as u64))
        } else {
            None
        };

        if let Some(ref pb) = progress {
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({msg})")
                    .unwrap()
                    .progress_chars("#>-"),
            );
        }

        // Thread-safe shared state for error collection
        let results = Arc::new(Mutex::new(Vec::new()));
        let errors = Arc::new(Mutex::new(Vec::new()));

        // Setup thread pool if thread count is specified
        let result = if let Some(num_threads) = self.config.num_threads {
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build_global()
                .map_err(|e| anyhow::anyhow!("Failed to create thread pool: {}", e))?;

            self.process_files_parallel(file_paths, &progress, &results, &errors)
        } else {
            self.process_files_parallel(file_paths, &progress, &results, &errors)
        };

        // Finish progress bar
        if let Some(pb) = progress {
            pb.finish_with_message("Complete");
        }

        let total_duration_ms = start_time.elapsed().as_millis();

        // Collect results
        let results_vec = Arc::try_unwrap(results)
            .map(|m| m.lock().unwrap().clone())
            .unwrap_or_else(|arc| arc.lock().unwrap().clone());

        let errors_vec = Arc::try_unwrap(errors)
            .map(|m| m.lock().unwrap().clone())
            .unwrap_or_else(|arc| arc.lock().unwrap().clone());

        // Build summary
        let (successful, failed) = results_vec.iter().fold((0, 0), |(s, f), r| {
            if r.success {
                (s + 1, f)
            } else {
                (s, f + 1)
            }
        });

        let total_workouts: usize = results_vec.iter().map(|r| r.workouts.len()).sum();
        let mut all_workouts = Vec::new();

        for result in &results_vec {
            all_workouts.extend(result.workouts.clone());
        }

        let summary = ParallelImportSummary {
            total_files: file_paths.len(),
            successful_files: successful,
            failed_files: failed,
            total_workouts,
            total_duration_ms,
            results: results_vec,
            errors: errors_vec,
        };

        info!("{}", summary.to_string_pretty());

        result?;
        Ok((all_workouts, summary))
    }

    /// Import directory in parallel
    pub fn import_directory(&self, dir_path: &Path) -> Result<(Vec<Workout>, ParallelImportSummary)> {
        debug!("Scanning directory for importable files: {:?}", dir_path);

        let mut files = Vec::new();

        if !dir_path.is_dir() {
            anyhow::bail!("Path is not a directory: {}", dir_path.display());
        }

        for entry in std::fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().map(|e| e == "fit").unwrap_or(false) {
                files.push(path);
            }
        }

        if files.is_empty() {
            warn!("No FIT files found in directory: {}", dir_path.display());
            return Ok((
                Vec::new(),
                ParallelImportSummary {
                    total_files: 0,
                    successful_files: 0,
                    failed_files: 0,
                    total_workouts: 0,
                    total_duration_ms: 0,
                    results: Vec::new(),
                    errors: Vec::new(),
                },
            ));
        }

        info!("Found {} FIT files in directory", files.len());
        self.import_files(&files)
    }

    /// Process files in parallel
    fn process_files_parallel(
        &self,
        file_paths: &[PathBuf],
        progress: &Option<ProgressBar>,
        results: &Arc<Mutex<Vec<FileImportResult>>>,
        errors: &Arc<Mutex<Vec<(PathBuf, String)>>>,
    ) -> Result<()> {
        file_paths
            .par_iter()
            .for_each_with((progress.clone(), results.clone(), errors.clone()),
                |(pb, res, err), file_path| {

                let file_start = std::time::Instant::now();

                match self.importer.import_file(file_path) {
                    Ok(workouts) => {
                        let duration_ms = file_start.elapsed().as_millis();
                        debug!(
                            "Successfully imported {:?} ({} workouts, {:.2}ms)",
                            file_path,
                            workouts.len(),
                            duration_ms
                        );

                        let result = FileImportResult {
                            file_path: file_path.clone(),
                            workouts,
                            duration_ms,
                            success: true,
                            error: None,
                        };

                        if let Ok(mut r) = res.lock() {
                            r.push(result);
                        }
                    }
                    Err(e) => {
                        let duration_ms = file_start.elapsed().as_millis();
                        let error_msg = e.to_string();
                        warn!(
                            "Failed to import {:?}: {} ({:.2}ms)",
                            file_path, error_msg, duration_ms
                        );

                        let result = FileImportResult {
                            file_path: file_path.clone(),
                            workouts: Vec::new(),
                            duration_ms,
                            success: false,
                            error: Some(error_msg.clone()),
                        };

                        if let Ok(mut r) = res.lock() {
                            r.push(result);
                        }

                        if let Ok(mut e) = err.lock() {
                            e.push((file_path.clone(), error_msg));
                        }
                    }
                }

                if let Some(p) = pb {
                    p.inc(1);
                }
            });

        Ok(())
    }
}

impl Default for ParallelImporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_config_default() {
        let config = ParallelImportConfig::default();
        assert_eq!(config.num_threads, None);
        assert!(config.show_progress);
        assert!(config.continue_on_error);
    }

    #[test]
    fn test_parallel_importer_creation() {
        let importer = ParallelImporter::new();
        assert!(importer.config.show_progress);
    }

    #[test]
    fn test_summary_throughput_calculation() {
        let summary = ParallelImportSummary {
            total_files: 10,
            successful_files: 10,
            failed_files: 0,
            total_workouts: 50,
            total_duration_ms: 1000,
            results: Vec::new(),
            errors: Vec::new(),
        };

        let throughput = summary.throughput_files_per_sec();
        assert_eq!(throughput, 10.0); // 10 files/sec

        let avg_time = summary.avg_time_per_file_ms();
        assert_eq!(avg_time, 100.0); // 100ms per file
    }

    #[test]
    fn test_summary_with_errors() {
        let summary = ParallelImportSummary {
            total_files: 10,
            successful_files: 8,
            failed_files: 2,
            total_workouts: 40,
            total_duration_ms: 1000,
            results: Vec::new(),
            errors: vec![(PathBuf::from("file1.fit"), "Parse error".to_string())],
        };

        assert!(!summary.is_fully_successful());
        assert_eq!(summary.failed_files, 2);
    }

    #[test]
    fn test_file_import_result() {
        let result = FileImportResult {
            file_path: PathBuf::from("test.fit"),
            workouts: vec![],
            duration_ms: 100,
            success: true,
            error: None,
        };

        assert!(result.success);
        assert_eq!(result.duration_ms, 100);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_summary_pretty_print() {
        let summary = ParallelImportSummary {
            total_files: 5,
            successful_files: 5,
            failed_files: 0,
            total_workouts: 25,
            total_duration_ms: 500,
            results: Vec::new(),
            errors: Vec::new(),
        };

        let pretty = summary.to_string_pretty();
        assert!(pretty.contains("Parallel Import Summary"));
        assert!(pretty.contains("Total Files: 5"));
        assert!(pretty.contains("Successful: 5"));
        assert!(pretty.contains("Workouts: 25"));
    }
}
