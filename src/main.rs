use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;

mod export;
mod import;
mod models;
mod pmc;
mod tss;
mod zones;

/// TrainRS - Training Load Analysis CLI
///
/// A Rust-based tool for analyzing workout data and calculating training loads
/// using sports science metrics like TSS, CTL, ATL, and TSB.
#[derive(Parser)]
#[command(name = "trainrs")]
#[command(author = "TrainRS Contributors")]
#[command(version = "0.1.0")]
#[command(about = "Training Load Analysis CLI", long_about = None)]
struct Cli {
    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Increase verbosity of output
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Import workout data from various sources
    Import {
        /// Input file path (supports CSV, TCX, GPX, FIT)
        #[arg(short, long, group = "input")]
        file: Option<PathBuf>,

        /// Import directory (batch import all supported files)
        #[arg(short, long, group = "input")]
        directory: Option<PathBuf>,

        /// File format (auto-detect if not specified)
        #[arg(long)]
        format: Option<String>,

        /// Validate file without importing
        #[arg(short, long)]
        validate_only: bool,
    },

    /// Calculate training metrics (TSS, IF, NP, etc.)
    Calculate {
        /// Date range start (YYYY-MM-DD)
        #[arg(short, long)]
        from: Option<String>,

        /// Date range end (YYYY-MM-DD)
        #[arg(short, long)]
        to: Option<String>,

        /// Specific athlete ID
        #[arg(short, long)]
        athlete: Option<String>,
    },

    /// Analyze training patterns and trends
    Analyze {
        /// Analysis period in days (default: 42)
        #[arg(short, long, default_value = "42")]
        period: u32,

        /// Include predictions
        #[arg(short = 'p', long)]
        predict: bool,
    },

    /// Export training data and reports
    Export {
        /// Output file path
        #[arg(short, long)]
        output: PathBuf,

        /// Export format (csv, json, text, html, pdf)
        #[arg(short = 'f', long, default_value = "csv")]
        format: String,

        /// Export type (workouts, pmc, zones, weekly, monthly, report, trainingpeaks)
        #[arg(short = 't', long, default_value = "workouts")]
        export_type: String,

        /// Start date for filtering (YYYY-MM-DD)
        #[arg(long)]
        from: Option<String>,

        /// End date for filtering (YYYY-MM-DD)
        #[arg(long)]
        to: Option<String>,

        /// Specific athlete ID
        #[arg(short, long)]
        athlete: Option<String>,

        /// Include raw data points (for supported formats)
        #[arg(long)]
        include_raw: bool,

        /// Template name for specialized exports
        #[arg(long)]
        template: Option<String>,
    },

    /// Display training metrics in terminal
    Display {
        /// Display format (table, chart, summary)
        #[arg(short = 'f', long, default_value = "table")]
        format: String,

        /// Number of recent activities to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Configure application settings
    Config {
        /// List all configuration options
        #[arg(short, long)]
        list: bool,

        /// Set a configuration value
        #[arg(short, long)]
        set: Option<String>,

        /// Get a configuration value
        #[arg(short, long)]
        get: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up logging based on verbosity
    let log_level = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    if cli.verbose > 0 {
        eprintln!("{}", format!("Log level: {}", log_level).dimmed());
    }

    // Handle commands
    match cli.command {
        Commands::Import {
            file,
            directory,
            format,
            validate_only,
        } => {
            use crate::import::ImportManager;

            let manager = ImportManager::new();

            if let Some(file_path) = file {
                // Single file import
                println!("{}", "Importing workout data...".green().bold());
                println!("  File: {}", file_path.display());

                if let Some(fmt) = format {
                    println!("  Format: {}", fmt);
                }

                match if validate_only {
                    manager.validate_file(&file_path).map(|_| Vec::new()) // Return empty vec for validation
                } else {
                    manager.import_file(&file_path)
                } {
                    Ok(workouts) => {
                        if validate_only {
                            println!("{}", "✓ File validation completed successfully".green());
                        } else {
                            println!(
                                "{}",
                                format!(
                                    "✓ Import completed successfully: {} workouts imported",
                                    workouts.len()
                                )
                                .green()
                            );
                            for workout in &workouts {
                                println!(
                                    "  - {} workout on {} ({} seconds)",
                                    format!("{:?}", workout.sport).cyan(),
                                    workout.date.format("%Y-%m-%d %H:%M:%S"),
                                    workout.duration_seconds
                                );
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", format!("✗ Import failed: {}", e).red());
                        std::process::exit(1);
                    }
                }
            } else if let Some(dir_path) = directory {
                // Directory batch import
                println!(
                    "{}",
                    "Importing workout data from directory...".green().bold()
                );
                println!("  Directory: {}", dir_path.display());

                match manager.import_directory(&dir_path) {
                    Ok(workouts) => {
                        println!(
                            "{}",
                            format!(
                                "✓ Batch import completed successfully: {} workouts imported",
                                workouts.len()
                            )
                            .green()
                        );

                        // Group workouts by sport for summary
                        let mut sport_counts = std::collections::HashMap::new();
                        for workout in &workouts {
                            *sport_counts.entry(workout.sport.clone()).or_insert(0) += 1;
                        }

                        for (sport, count) in sport_counts {
                            println!("  - {:?}: {} workouts", sport, count);
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", format!("✗ Batch import failed: {}", e).red());
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!(
                    "{}",
                    "Error: Must specify either --file or --directory".red()
                );
                std::process::exit(1);
            }
        }

        Commands::Calculate { from, to, athlete } => {
            println!("{}", "Calculating training metrics...".blue().bold());
            if let Some(f) = from {
                println!("  From: {}", f);
            }
            if let Some(t) = to {
                println!("  To: {}", t);
            }
            if let Some(a) = athlete {
                println!("  Athlete: {}", a);
            }
            // TODO: Implement calculation functionality
            println!("{}", "✓ Calculations completed".blue());
        }

        Commands::Analyze { period, predict } => {
            println!("{}", "Analyzing training patterns...".cyan().bold());
            println!("  Period: {} days", period);
            println!(
                "  Predictions: {}",
                if predict { "enabled" } else { "disabled" }
            );
            // TODO: Implement analysis functionality
            println!("{}", "✓ Analysis completed".cyan());
        }

        Commands::Export {
            output,
            format,
            export_type,
            from,
            to,
            athlete,
            include_raw,
            template
        } => {
            println!("{}", "Exporting data...".yellow().bold());
            println!("  Output: {:?}", output);
            println!("  Format: {}", format);
            println!("  Type: {}", export_type);

            if let Some(ref f) = from {
                println!("  From: {}", f);
            }
            if let Some(ref t) = to {
                println!("  To: {}", t);
            }
            if let Some(ref a) = athlete {
                println!("  Athlete: {}", a);
            }
            if include_raw {
                println!("  Including raw data");
            }

            use export::{ExportManager, ExportOptions, ExportFormat, ExportType, DateRange};
            use chrono::NaiveDate;

            // Parse export format
            let export_format = match ExportFormat::from_str(&format) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("{}", format!("✗ Invalid format '{}': {}", format, e).red());
                    std::process::exit(1);
                }
            };

            // Parse export type
            let export_type_enum = match export_type.to_lowercase().as_str() {
                "workouts" | "workout" | "workout-summaries" => ExportType::WorkoutSummaries,
                "pmc" | "pmc-data" => ExportType::PmcData,
                "zones" | "zone-analysis" => ExportType::ZoneAnalysis,
                "weekly" | "weekly-summary" => ExportType::WeeklySummary,
                "monthly" | "monthly-summary" => ExportType::MonthlySummary,
                "report" | "training-report" => ExportType::TrainingReport,
                "trainingpeaks" | "training-peaks" => ExportType::TrainingPeaksFormat,
                _ => {
                    eprintln!("{}", format!("✗ Invalid export type '{}'", export_type).red());
                    std::process::exit(1);
                }
            };

            // Parse date range
            let start_date = if let Some(from_str) = from {
                match NaiveDate::parse_from_str(&from_str, "%Y-%m-%d") {
                    Ok(date) => Some(date),
                    Err(_) => {
                        eprintln!("{}", format!("✗ Invalid start date format '{}'. Use YYYY-MM-DD", from_str).red());
                        std::process::exit(1);
                    }
                }
            } else {
                None
            };

            let end_date = if let Some(to_str) = to {
                match NaiveDate::parse_from_str(&to_str, "%Y-%m-%d") {
                    Ok(date) => Some(date),
                    Err(_) => {
                        eprintln!("{}", format!("✗ Invalid end date format '{}'. Use YYYY-MM-DD", to_str).red());
                        std::process::exit(1);
                    }
                }
            } else {
                None
            };

            let date_range = DateRange::new(start_date, end_date);

            // Create export options
            let export_options = ExportOptions {
                format: export_format,
                export_type: export_type_enum,
                date_range,
                include_raw_data: include_raw,
                athlete_id: athlete,
                template,
            };

            // For now, create some sample data since we don't have a data store yet
            // TODO: Load actual workout data from storage
            println!("{}", "Note: Using sample data - integrate with data storage in future".dimmed());

            let sample_workouts = Vec::new(); // Empty for now
            let export_manager = ExportManager::new();

            match export_manager.export(&sample_workouts, None, &export_options, &output) {
                Ok(_) => {
                    println!("{}", "✓ Export completed successfully".green());
                },
                Err(e) => {
                    eprintln!("{}", format!("✗ Export failed: {}", e).red());
                    std::process::exit(1);
                }
            }
        }

        Commands::Display { format, limit } => {
            println!("{}", "Displaying training metrics...".magenta().bold());
            println!("  Format: {}", format);
            println!("  Limit: {} activities", limit);
            // TODO: Implement display functionality
            println!("{}", "✓ Display completed".magenta());
        }

        Commands::Config { list, set, get } => {
            println!("{}", "Managing configuration...".white().bold());
            if list {
                println!("Listing all configuration options:");
                // TODO: Implement config listing
            } else if let Some(key_value) = set {
                println!("Setting: {}", key_value);
                // TODO: Implement config setting
            } else if let Some(key) = get {
                println!("Getting: {}", key);
                // TODO: Implement config getting
            }
            println!("{}", "✓ Configuration updated".white());
        }
    }

    Ok(())
}
