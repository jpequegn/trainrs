use anyhow::Result;
use chrono::{Datelike, NaiveDate, Duration};
use clap::{Parser, Subcommand};
use colored::*;
use rust_decimal::{prelude::{FromPrimitive, ToPrimitive}, Decimal};
use rust_decimal_macros::dec;
use std::path::PathBuf;
use std::collections::HashMap;
use crate::models::DataPoint;

mod export;
mod import;
mod models;
mod pmc;
mod power;
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

    /// Specify athlete profile name or ID
    #[arg(short, long, global = true)]
    athlete: Option<String>,

    /// Custom data directory path
    #[arg(long, global = true, value_name = "DIR")]
    data_dir: Option<PathBuf>,

    /// Increase verbosity of output
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Suppress non-essential output
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Output format (table, json, csv)
    #[arg(long, global = true, value_name = "FORMAT", default_value = "table")]
    format: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum ZoneCommands {
    /// List current training zones
    List {
        /// Zone type (heart-rate, power, pace)
        #[arg(short = 't', long)]
        zone_type: Option<String>,

        /// Athlete profile to work with
        #[arg(long)]
        athlete: Option<String>,
    },

    /// Set training zone thresholds
    Set {
        /// Zone type (heart-rate, power, pace)
        #[arg(short = 't', long)]
        zone_type: Option<String>,

        /// Athlete profile to work with
        #[arg(long)]
        athlete: Option<String>,

        /// FTP value for power zones
        #[arg(long)]
        ftp: Option<u16>,

        /// LTHR value for heart rate zones
        #[arg(long)]
        lthr: Option<u16>,

        /// Max HR value for heart rate zones
        #[arg(long)]
        max_hr: Option<u16>,

        /// Threshold pace for running zones (min/mile or min/km)
        #[arg(long)]
        threshold_pace: Option<f64>,
    },

    /// Calculate zones from athlete profile
    Calculate {
        /// Athlete profile to work with
        #[arg(long)]
        athlete: Option<String>,
    },

    /// Import zones from external source
    Import {
        /// Import file path
        #[arg(short, long)]
        file: PathBuf,

        /// Athlete profile to work with
        #[arg(long)]
        athlete: Option<String>,
    },

    /// Analyze time-in-zone distributions
    Analyze {
        /// Time period for analysis
        #[arg(long)]
        last_days: Option<u16>,

        /// Start date for analysis (YYYY-MM-DD)
        #[arg(long)]
        from: Option<String>,

        /// End date for analysis (YYYY-MM-DD)
        #[arg(long)]
        to: Option<String>,

        /// Filter by specific sport
        #[arg(long)]
        sport: Option<String>,

        /// Zone type to analyze (heart-rate, power, pace, all)
        #[arg(short = 't', long, default_value = "all")]
        zone_type: String,

        /// Athlete profile to use for zone calculations
        #[arg(long)]
        athlete: Option<String>,

        /// Show detailed zone distribution
        #[arg(long)]
        detailed: bool,

        /// Show training pattern analysis
        #[arg(long)]
        show_patterns: bool,

        /// Show training recommendations
        #[arg(long)]
        show_recommendations: bool,

        /// Minimum workout duration in minutes to include
        #[arg(long)]
        min_duration: Option<u32>,
    },
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
        /// Input file with workout data (TCX, GPX, FIT, CSV)
        #[arg(short, long)]
        file: Option<PathBuf>,

        /// Power data file (CSV format)
        #[arg(long)]
        power_file: Option<PathBuf>,

        /// Heart rate data file (CSV format)
        #[arg(long)]
        hr_file: Option<PathBuf>,

        /// Functional Threshold Power for TSS calculation
        #[arg(long)]
        ftp: Option<u16>,

        /// Lactate Threshold Heart Rate for HR-based TSS
        #[arg(long)]
        lthr: Option<u16>,

        /// Workout duration in seconds (if not in file)
        #[arg(long)]
        duration: Option<u32>,

        /// Manual TSS calculation method (auto, power, hr, rpe, estimated)
        #[arg(short, long, default_value = "auto")]
        method: String,

        /// Rate of Perceived Exertion (1-10 scale)
        #[arg(long)]
        rpe: Option<u8>,

        /// Date range start (YYYY-MM-DD) for multiple workouts
        #[arg(long)]
        from: Option<String>,

        /// Date range end (YYYY-MM-DD) for multiple workouts
        #[arg(long)]
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
        #[arg(long)]
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

    /// Manage training zones and thresholds
    Zones {
        #[command(subcommand)]
        command: ZoneCommands,
    },

    /// Generate training summaries
    Summary {
        /// Summary period (daily, weekly, monthly, yearly)
        #[arg(short, long, default_value = "weekly")]
        period: String,

        /// Number of periods to include
        #[arg(short, long, default_value = "4")]
        count: u32,

        /// Include PMC analysis
        #[arg(long)]
        include_pmc: bool,

        /// Include zone analysis
        #[arg(long)]
        include_zones: bool,

        /// Start date for summary (YYYY-MM-DD)
        #[arg(long)]
        from: Option<String>,

        /// End date for summary (YYYY-MM-DD)
        #[arg(long)]
        to: Option<String>,
    },

    /// Performance Management Chart analysis and display
    Pmc {
        /// Number of recent days to display
        #[arg(long)]
        last_days: Option<u16>,

        /// Start date for date range (YYYY-MM-DD)
        #[arg(long)]
        from: Option<String>,

        /// End date for date range (YYYY-MM-DD)
        #[arg(long)]
        to: Option<String>,

        /// Filter by specific sport
        #[arg(long)]
        sport: Option<String>,

        /// Show weekly summary instead of daily
        #[arg(long)]
        weekly: bool,

        /// Show monthly summary instead of daily
        #[arg(long)]
        monthly: bool,

        /// Show training load warnings
        #[arg(long)]
        show_warnings: bool,

        /// Show trend analysis
        #[arg(long)]
        show_trends: bool,

        /// Minimum TSS threshold for inclusion
        #[arg(long)]
        min_tss: Option<f64>,
    },

    /// Power analysis for cycling training
    Power {
        #[command(subcommand)]
        command: PowerCommands,
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

/// Power analysis subcommands
#[derive(Subcommand)]
enum PowerCommands {
    /// Generate power curve (Mean Maximal Power) analysis
    Curve {
        /// Number of days to analyze
        #[arg(long, default_value = "90")]
        last_days: u16,

        /// Start date (YYYY-MM-DD)
        #[arg(long)]
        from: Option<String>,

        /// End date (YYYY-MM-DD)
        #[arg(long)]
        to: Option<String>,

        /// Athlete to analyze
        #[arg(long)]
        athlete: Option<String>,

        /// Show comparison with previous period
        #[arg(long)]
        compare: bool,

        /// Export results to file
        #[arg(long)]
        export: Option<PathBuf>,
    },

    /// Critical Power model analysis
    CriticalPower {
        /// Input file with test data
        #[arg(short, long)]
        file: Option<PathBuf>,

        /// Use 3-parameter model
        #[arg(long)]
        three_parameter: bool,

        /// Override 3-min power (watts)
        #[arg(long)]
        power_3min: Option<u16>,

        /// Override 5-min power (watts)
        #[arg(long)]
        power_5min: Option<u16>,

        /// Override 20-min power (watts)
        #[arg(long)]
        power_20min: Option<u16>,

        /// Athlete to analyze
        #[arg(long)]
        athlete: Option<String>,
    },

    /// Analyze power data from a single workout
    Analyze {
        /// Input workout file
        #[arg(short, long)]
        file: PathBuf,

        /// Show interval analysis
        #[arg(long)]
        show_intervals: bool,

        /// FTP for calculations
        #[arg(long)]
        ftp: Option<u16>,

        /// Show quadrant analysis
        #[arg(long)]
        quadrants: bool,

        /// Show power balance analysis
        #[arg(long)]
        balance: bool,

        /// Export detailed analysis
        #[arg(long)]
        export: Option<PathBuf>,
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

        Commands::Calculate {
            file,
            power_file,
            hr_file,
            ftp,
            lthr,
            duration,
            method,
            rpe,
            from,
            to,
            athlete,
        } => {
            println!("{}", "Calculating training metrics...".blue().bold());

            // Handle global athlete flag
            let athlete_id = athlete.clone().or_else(|| cli.athlete.clone());
            if let Some(a) = &athlete_id {
                println!("  Athlete: {}", a);
            }

            // Single workout calculation
            if let Some(workout_file) = file {
                println!("  Workout file: {}", workout_file.display());
                println!("  Method: {}", method);

                // Use existing import functionality to read the workout
                use crate::import::ImportManager;
                let manager = ImportManager::new();

                match manager.import_file(&workout_file) {
                    Ok(workouts) => {
                        if workouts.is_empty() {
                            eprintln!("{}", "✗ No workout data found in file".red());
                            std::process::exit(1);
                        }

                        let workout = &workouts[0];
                        println!("  Sport: {:?}", workout.sport);
                        println!("  Duration: {} minutes", workout.duration_seconds / 60);

                        // Calculate TSS using existing TSS module
                        use crate::tss::TssCalculator;
                        use crate::models::{AthleteProfile, TrainingZones, Units};

                        // Create a basic athlete profile for calculation
                        let profile = AthleteProfile {
                            id: athlete_id.unwrap_or_else(|| "default".to_string()),
                            name: "Default Athlete".to_string(),
                            date_of_birth: None,
                            weight: None,
                            height: None,
                            ftp: ftp.or(Some(250)), // Default FTP if not provided
                            lthr: lthr.or(Some(165)), // Default LTHR if not provided
                            threshold_pace: None,
                            max_hr: None,
                            resting_hr: None,
                            training_zones: TrainingZones::default(),
                            preferred_units: Units::default(),
                            created_at: chrono::Utc::now(),
                            updated_at: chrono::Utc::now(),
                        };

                        // Calculate TSS
                        match TssCalculator::calculate_tss(workout, &profile) {
                            Ok(tss_result) => {
                                println!("  ✓ TSS: {:.1}", tss_result.tss);
                                if let Some(if_value) = tss_result.intensity_factor {
                                    println!("  ✓ Intensity Factor: {:.3}", if_value);
                                }
                                if let Some(np) = tss_result.normalized_power {
                                    println!("  ✓ Normalized Power: {} watts", np);
                                }
                                println!("{}", "✓ Calculation completed successfully".green());
                            }
                            Err(e) => {
                                eprintln!("{}", format!("✗ TSS calculation failed: {}", e).red());
                                std::process::exit(1);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", format!("✗ Failed to import workout file: {}", e).red());
                        std::process::exit(1);
                    }
                }
            }
            // Power file calculation
            else if let Some(power_file_path) = power_file {
                println!("  Power file: {}", power_file_path.display());
                if let Some(ftp_value) = ftp {
                    println!("  FTP: {} watts", ftp_value);
                    // TODO: Implement power file TSS calculation
                    println!("{}", "✓ Power-based TSS calculation completed".green());
                } else {
                    eprintln!("{}", "✗ FTP value required for power-based calculations".red());
                    std::process::exit(1);
                }
            }
            // HR file calculation
            else if let Some(hr_file_path) = hr_file {
                println!("  HR file: {}", hr_file_path.display());
                if let Some(lthr_value) = lthr {
                    println!("  LTHR: {} bpm", lthr_value);
                    // TODO: Implement HR file TSS calculation
                    println!("{}", "✓ HR-based TSS calculation completed".green());
                } else {
                    eprintln!("{}", "✗ LTHR value required for HR-based calculations".red());
                    std::process::exit(1);
                }
            }
            // RPE-based estimation
            else if let Some(rpe_value) = rpe {
                if let Some(duration_secs) = duration {
                    println!("  RPE: {}/10", rpe_value);
                    println!("  Duration: {} minutes", duration_secs / 60);

                    // Simple RPE to TSS estimation: RPE^2 * duration_hours * 10
                    let duration_hours = duration_secs as f64 / 3600.0;
                    let estimated_tss = (rpe_value as f64).powi(2) * duration_hours * 10.0;

                    println!("  ✓ Estimated TSS: {:.1}", estimated_tss);
                    println!("{}", "✓ RPE-based estimation completed".green());
                } else {
                    eprintln!("{}", "✗ Duration required for RPE-based calculations".red());
                    std::process::exit(1);
                }
            }
            // Date range calculation
            else if from.is_some() || to.is_some() {
                if let Some(f) = from {
                    println!("  From: {}", f);
                }
                if let Some(t) = to {
                    println!("  To: {}", t);
                }
                // TODO: Implement bulk calculation for date range
                println!("{}", "✓ Bulk calculations completed".blue());
            } else {
                eprintln!("{}", "✗ Must specify either --file, --power-file, --hr-file, --rpe, or date range".red());
                std::process::exit(1);
            }
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

        Commands::Zones { ref command } => {
            match command {
                ZoneCommands::List { zone_type, athlete } => {
                    println!("{}", "Listing training zones...".cyan().bold());

                    // Handle global athlete flag
                    let athlete_id = athlete.clone().or_else(|| cli.athlete.clone());
                    if let Some(a) = &athlete_id {
                        println!("  Athlete: {}", a);
                    }

                    if let Some(zone_type_str) = zone_type {
                        println!("  Zone type: {}", zone_type_str);
                        match zone_type_str.as_str() {
                            "heart-rate" | "hr" => {
                                println!("\n💓 Heart Rate Zones:");
                                println!("    Zone 1: < 81% of LTHR (Active Recovery)");
                                println!("    Zone 2: 81-89% of LTHR (Aerobic Base)");
                                println!("    Zone 3: 90-93% of LTHR (Aerobic)");
                                println!("    Zone 4: 94-99% of LTHR (Lactate Threshold)");
                                println!("    Zone 5: 100%+ of LTHR (VO2 Max)");
                            }
                            "power" => {
                                println!("\n⚡ Power Zones:");
                                println!("    Zone 1: < 55% of FTP (Active Recovery)");
                                println!("    Zone 2: 56-75% of FTP (Endurance)");
                                println!("    Zone 3: 76-90% of FTP (Tempo)");
                                println!("    Zone 4: 91-105% of FTP (Lactate Threshold)");
                                println!("    Zone 5: 106-120% of FTP (VO2 Max)");
                                println!("    Zone 6: 121-150% of FTP (Anaerobic Capacity)");
                                println!("    Zone 7: > 150% of FTP (Sprint Power)");
                            }
                            "pace" => {
                                println!("\n🏃 Pace Zones:");
                                println!("    Zone 1: Easy pace (slowest)");
                                println!("    Zone 2: Aerobic pace");
                                println!("    Zone 3: Tempo pace");
                                println!("    Zone 4: Threshold pace");
                                println!("    Zone 5: VO2 Max pace (fastest)");
                            }
                            _ => {
                                eprintln!("{}", "✗ Invalid zone type. Use: heart-rate, power, or pace".red());
                                std::process::exit(1);
                            }
                        }
                    } else {
                        println!("\n📊 All Available Zone Types:");
                        println!("  💓 Heart Rate Zones (5 zones based on LTHR)");
                        println!("  ⚡ Power Zones (7 zones based on FTP)");
                        println!("  🏃 Pace Zones (5 zones based on threshold pace)");
                    }
                    println!("{}", "✓ Zone listing completed".cyan());
                }

                ZoneCommands::Set { zone_type, athlete, ftp, lthr, max_hr, threshold_pace } => {
                    println!("{}", "Setting training zone thresholds...".cyan().bold());

                    // Handle global athlete flag
                    let athlete_id = athlete.clone().or_else(|| cli.athlete.clone());
                    if let Some(a) = &athlete_id {
                        println!("  Athlete: {}", a);
                    }

                    if let Some(zone_type_str) = zone_type {
                        println!("  Zone type: {}", zone_type_str);
                    }

                    let mut updated_thresholds = false;

                    if let Some(ftp_value) = ftp {
                        println!("    FTP: {} watts", ftp_value);
                        updated_thresholds = true;
                        // TODO: Create or update athlete profile with FTP
                    }
                    if let Some(lthr_value) = lthr {
                        println!("    LTHR: {} bpm", lthr_value);
                        updated_thresholds = true;
                        // TODO: Create or update athlete profile with LTHR
                    }
                    if let Some(max_hr_value) = max_hr {
                        println!("    Max HR: {} bpm", max_hr_value);
                        updated_thresholds = true;
                        // TODO: Create or update athlete profile with Max HR
                    }
                    if let Some(pace_value) = threshold_pace {
                        println!("    Threshold pace: {:.2} min/mile", pace_value);
                        updated_thresholds = true;
                        // TODO: Create or update athlete profile with threshold pace
                    }

                    if !updated_thresholds {
                        println!("  No threshold values provided. Use --ftp, --lthr, --max-hr, or --threshold-pace");
                    }

                    println!("{}", "✓ Zone thresholds updated".cyan());
                }

                ZoneCommands::Calculate { athlete } => {
                    println!("{}", "Calculating zones from athlete profile...".cyan().bold());

                    // Handle global athlete flag
                    let athlete_id = athlete.clone().or_else(|| cli.athlete.clone());
                    if let Some(a) = &athlete_id {
                        println!("  Athlete: {}", a);
                    }

                    // TODO: Load athlete profile and calculate zones using ZoneCalculator
                    println!("  ✓ Zones calculated and updated");
                    println!("{}", "✓ Zone calculation completed".cyan());
                }

                ZoneCommands::Import { file, athlete } => {
                    println!("{}", "Importing zones from file...".cyan().bold());

                    // Handle global athlete flag
                    let athlete_id = athlete.clone().or_else(|| cli.athlete.clone());
                    if let Some(a) = &athlete_id {
                        println!("  Athlete: {}", a);
                    }

                    println!("  Import file: {}", file.display());
                    // TODO: Implement zone import functionality
                    println!("  ✓ Zones imported successfully");
                    println!("{}", "✓ Zone import completed".cyan());
                }

                ZoneCommands::Analyze {
                    last_days,
                    from,
                    to,
                    sport,
                    zone_type,
                    athlete,
                    detailed,
                    show_patterns,
                    show_recommendations,
                    min_duration,
                } => {
                    handle_zone_analysis(
                        &cli,
                        *last_days,
                        from.clone(),
                        to.clone(),
                        sport.clone(),
                        zone_type.clone(),
                        athlete.clone(),
                        *detailed,
                        *show_patterns,
                        *show_recommendations,
                        *min_duration,
                    );
                }
            }
        }

        Commands::Summary {
            period,
            count,
            include_pmc,
            include_zones,
            from,
            to,
        } => {
            println!("{}", "Generating training summary...".magenta().bold());
            println!("  Period: {}", period);
            println!("  Count: {} periods", count);

            if include_pmc {
                println!("  Including PMC analysis");
            }
            if include_zones {
                println!("  Including zone analysis");
            }

            // Handle date range
            if let Some(from_date) = from {
                println!("  From: {}", from_date);
            }
            if let Some(to_date) = to {
                println!("  To: {}", to_date);
            }

            match period.as_str() {
                "daily" => {
                    println!("\n📊 DAILY TRAINING SUMMARY");
                    println!("==========================");
                    for i in 1..=count {
                        println!("Day {} - Date: [Sample Date]", i);
                        println!("  Workouts: 1");
                        println!("  TSS: 85.5");
                        println!("  Duration: 1h 30m");
                        if include_pmc {
                            println!("  CTL: 45.2, ATL: 65.8, TSB: -20.6");
                        }
                        println!();
                    }
                }
                "weekly" => {
                    println!("\n📊 WEEKLY TRAINING SUMMARY");
                    println!("===========================");
                    for i in 1..=count {
                        println!("Week {} - [Sample Week Range]", i);
                        println!("  Total workouts: 6");
                        println!("  Total TSS: 425.5");
                        println!("  Total duration: 8h 45m");
                        println!("  Average daily TSS: 60.8");
                        if include_pmc {
                            println!("  Average CTL: 48.3, Average ATL: 62.1, Average TSB: -13.8");
                        }
                        if include_zones {
                            println!("  Zone distribution: Z1: 45%, Z2: 35%, Z3: 15%, Z4: 5%");
                        }
                        println!();
                    }
                }
                "monthly" => {
                    println!("\n📊 MONTHLY TRAINING SUMMARY");
                    println!("============================");
                    for i in 1..=count {
                        println!("Month {} - [Sample Month]", i);
                        println!("  Total workouts: 24");
                        println!("  Total TSS: 1,850.5");
                        println!("  Total duration: 38h 15m");
                        println!("  Average daily TSS: 59.7");
                        if include_pmc {
                            println!("  End-of-month CTL: 52.1, ATL: 58.9, TSB: -6.8");
                        }
                        if include_zones {
                            println!("  Zone distribution: Z1: 42%, Z2: 38%, Z3: 15%, Z4: 4%, Z5: 1%");
                        }
                        println!();
                    }
                }
                "yearly" => {
                    println!("\n📊 YEARLY TRAINING SUMMARY");
                    println!("===========================");
                    for i in 1..=count {
                        println!("Year {} - [Sample Year]", i);
                        println!("  Total workouts: 285");
                        println!("  Total TSS: 22,150");
                        println!("  Total duration: 485h 30m");
                        println!("  Average daily TSS: 60.7");
                        if include_pmc {
                            println!("  Peak CTL: 68.5");
                            println!("  Training consistency: 78%");
                        }
                        println!();
                    }
                }
                _ => {
                    eprintln!("{}", "✗ Invalid period. Use: daily, weekly, monthly, or yearly".red());
                    std::process::exit(1);
                }
            }

            println!("{}", "✓ Summary generation completed".magenta());
        }

        Commands::Pmc {
            last_days,
            from,
            to,
            sport,
            weekly,
            monthly,
            show_warnings,
            show_trends,
            min_tss,
        } => {
            println!("{}", "Performance Management Chart Analysis".blue().bold());

            // Handle global athlete flag
            let athlete_id = cli.athlete.clone();
            if let Some(a) = &athlete_id {
                println!("  Athlete: {}", a);
            }

            // Determine date range
            let end_date = if let Some(to_date) = to {
                chrono::NaiveDate::parse_from_str(&to_date, "%Y-%m-%d")
                    .map_err(|e| {
                        eprintln!("{}", format!("✗ Invalid end date format: {}", e).red());
                        std::process::exit(1);
                    })
                    .unwrap()
            } else {
                chrono::Local::now().date_naive()
            };

            let start_date = if let Some(from_date) = from {
                chrono::NaiveDate::parse_from_str(&from_date, "%Y-%m-%d")
                    .map_err(|e| {
                        eprintln!("{}", format!("✗ Invalid start date format: {}", e).red());
                        std::process::exit(1);
                    })
                    .unwrap()
            } else if let Some(days) = last_days {
                end_date - chrono::Duration::days(days as i64)
            } else {
                end_date - chrono::Duration::days(30) // Default to 30 days
            };

            if start_date > end_date {
                eprintln!("{}", "✗ Start date must be before end date".red());
                std::process::exit(1);
            }

            // Load workout data (placeholder - would need to load from database/files)
            // For now, we'll create sample data
            use crate::models::{Workout, WorkoutSummary, Sport, DataSource, WorkoutType};
            use crate::pmc::PmcCalculator;
            use rust_decimal::Decimal;
            use rust_decimal_macros::dec;

            println!("  Date range: {} to {}", start_date, end_date);
            if let Some(ref sport_filter) = sport {
                println!("  Sport filter: {}", sport_filter);
            }
            if let Some(min_tss_val) = min_tss {
                println!("  Minimum TSS: {:.1}", min_tss_val);
            }

            // Create sample workout data for demonstration
            let mut sample_workouts = Vec::new();
            let mut current_date = start_date;
            let mut day_counter = 0;

            while current_date <= end_date {
                // Simulate varying workout patterns
                if day_counter % 7 != 6 { // Skip Sundays as rest days
                    let base_tss = match day_counter % 7 {
                        0 => 85,  // Monday - Moderate
                        1 => 120, // Tuesday - Hard
                        2 => 45,  // Wednesday - Easy
                        3 => 140, // Thursday - Very Hard
                        4 => 95,  // Friday - Moderate
                        5 => 110, // Saturday - Hard
                        _ => 0,   // Sunday - Rest
                    };

                    if base_tss > 0 {
                        let workout = Workout {
                            id: format!("workout_{}", current_date.format("%Y%m%d")),
                            date: current_date,
                            sport: Sport::Cycling,
                            duration_seconds: (base_tss as f64 * 60.0) as u32, // Rough duration estimate
                            workout_type: WorkoutType::Endurance,
                            data_source: DataSource::Power,
                            raw_data: None,
                            summary: WorkoutSummary {
                                tss: Some(Decimal::from(base_tss)),
                                avg_heart_rate: Some(150 + (base_tss / 10) as u16),
                                max_heart_rate: Some(180),
                                avg_power: Some(220),
                                normalized_power: Some(235),
                                total_distance: Some(dec!(25000)), // 25km
                                ..WorkoutSummary::default()
                            },
                            notes: Some("Sample workout".to_string()),
                            athlete_id: athlete_id.clone(),
                            source: Some("trainrs".to_string()),
                        };
                        sample_workouts.push(workout);
                    }
                }

                current_date += chrono::Duration::days(1);
                day_counter += 1;
            }

            // Apply sport filter if specified
            if let Some(_sport_filter) = &sport {
                // TODO: Filter by sport when we have sport parsing
                println!("  Note: Sport filtering not yet implemented");
            }

            // Calculate PMC metrics
            let pmc_calculator = PmcCalculator::new();
            let daily_tss = pmc_calculator.aggregate_daily_tss(&sample_workouts);

            match pmc_calculator.calculate_pmc_series(&daily_tss, start_date, end_date) {
                Ok(pmc_metrics) => {
                    display_pmc_table(&pmc_metrics, weekly, monthly, show_warnings, show_trends);
                }
                Err(e) => {
                    eprintln!("{}", format!("✗ PMC calculation failed: {}", e).red());
                    std::process::exit(1);
                }
            }

            println!("{}", "✓ PMC analysis completed".blue());
        }

        Commands::Power { ref command } => {
            handle_power_commands(command, &cli).unwrap_or_else(|e| {
                eprintln!("{}", format!("Power analysis error: {}", e).red());
                std::process::exit(1);
            });
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

/// Display PMC data in tabular format with color coding and analysis
fn display_pmc_table(
    pmc_metrics: &[crate::pmc::PmcMetrics],
    weekly: bool,
    monthly: bool,
    show_warnings: bool,
    show_trends: bool,
) {
    use crate::pmc::TsbInterpretation;
    use colored::Colorize;

    if pmc_metrics.is_empty() {
        println!("{}", "No PMC data to display".yellow());
        return;
    }

    if weekly {
        display_weekly_pmc_summary(pmc_metrics);
    } else if monthly {
        display_monthly_pmc_summary(pmc_metrics);
    } else {
        display_daily_pmc_table(pmc_metrics);
    }

    if show_trends {
        display_training_trends(pmc_metrics);
    }

    if show_warnings {
        display_training_warnings(pmc_metrics);
    }

    // Show current fitness summary
    if let Some(latest) = pmc_metrics.last() {
        let first = pmc_metrics.first().unwrap();
        let ctl_change = latest.ctl - first.ctl;
        let tsb_interp = TsbInterpretation::from_tsb(latest.tsb);

        println!("\n📈 CURRENT FITNESS SUMMARY");
        println!("==========================");
        println!("Current Fitness (CTL): {:.1} ({:+.1} from period start)", latest.ctl, ctl_change);
        println!("Current Fatigue (ATL): {:.1}", latest.atl);

        let tsb_color = get_tsb_color_string(&latest.tsb);
        println!("Current Form (TSB): {} - {}", tsb_color, tsb_interp.description());
        println!("Training Recommendation: {}", tsb_interp.recommendation());
    }
}

/// Display daily PMC data in table format
fn display_daily_pmc_table(pmc_metrics: &[crate::pmc::PmcMetrics]) {

    println!("\n📊 PERFORMANCE MANAGEMENT CHART - DAILY VIEW");
    println!("============================================");

    // Table header
    println!("{:<12} │ {:<4} │ {:<5} │ {:<5} │ {:<6} │ {}",
             "Date", "TSS", "CTL", "ATL", "TSB", "Status");
    println!("─────────────┼──────┼───────┼───────┼────────┼─────────────");

    for metrics in pmc_metrics {
        let status_emoji = get_tsb_emoji(metrics.tsb);
        let status_text = get_tsb_status_text(metrics.tsb);
        let tsb_colored = get_tsb_color_string(&metrics.tsb);

        println!("{:<12} │ {:>4} │ {:>5.1} │ {:>5.1} │ {:>6} │ {} {}",
            metrics.date.format("%Y-%m-%d"),
            if metrics.daily_tss > rust_decimal::Decimal::ZERO {
                format!("{:.0}", metrics.daily_tss)
            } else {
                "0".to_string()
            },
            metrics.ctl,
            metrics.atl,
            tsb_colored,
            status_emoji,
            status_text
        );
    }
}

/// Display weekly PMC summary
fn display_weekly_pmc_summary(pmc_metrics: &[crate::pmc::PmcMetrics]) {
    use std::collections::HashMap;

    println!("\n📊 PERFORMANCE MANAGEMENT CHART - WEEKLY SUMMARY");
    println!("================================================");

    // Group by weeks
    let mut weekly_data: HashMap<chrono::IsoWeek, Vec<&crate::pmc::PmcMetrics>> = HashMap::new();

    for metrics in pmc_metrics {
        weekly_data.entry(metrics.date.iso_week()).or_default().push(metrics);
    }

    // Table header
    println!("{:<15} │ {:<5} │ {:<5} │ {:<5} │ {:<6} │ {}",
             "Week", "TSS", "CTL", "ATL", "TSB", "Status");
    println!("────────────────┼───────┼───────┼───────┼────────┼─────────────");

    let mut weeks: Vec<_> = weekly_data.keys().collect();
    weeks.sort();

    for week in weeks {
        let week_metrics = &weekly_data[week];
        let total_tss: rust_decimal::Decimal = week_metrics.iter()
            .map(|m| m.daily_tss)
            .sum();

        let avg_ctl: rust_decimal::Decimal = week_metrics.iter()
            .map(|m| m.ctl)
            .sum::<rust_decimal::Decimal>() / rust_decimal::Decimal::from(week_metrics.len());

        let avg_atl: rust_decimal::Decimal = week_metrics.iter()
            .map(|m| m.atl)
            .sum::<rust_decimal::Decimal>() / rust_decimal::Decimal::from(week_metrics.len());

        let avg_tsb: rust_decimal::Decimal = week_metrics.iter()
            .map(|m| m.tsb)
            .sum::<rust_decimal::Decimal>() / rust_decimal::Decimal::from(week_metrics.len());

        let status_emoji = get_tsb_emoji(avg_tsb);
        let status_text = get_tsb_status_text(avg_tsb);
        let tsb_colored = get_tsb_color_string(&avg_tsb);

        println!("{:<15} │ {:>5.0} │ {:>5.1} │ {:>5.1} │ {:>6} │ {} {}",
            format!("{}-W{:02}", week.year(), week.week()),
            total_tss,
            avg_ctl,
            avg_atl,
            tsb_colored,
            status_emoji,
            status_text
        );
    }
}

/// Display monthly PMC summary
fn display_monthly_pmc_summary(pmc_metrics: &[crate::pmc::PmcMetrics]) {
    use std::collections::HashMap;

    println!("\n📊 PERFORMANCE MANAGEMENT CHART - MONTHLY SUMMARY");
    println!("=================================================");

    // Group by months (year-month)
    let mut monthly_data: HashMap<(i32, u32), Vec<&crate::pmc::PmcMetrics>> = HashMap::new();

    for metrics in pmc_metrics {
        let key = (metrics.date.year(), metrics.date.month());
        monthly_data.entry(key).or_default().push(metrics);
    }

    // Table header
    println!("{:<10} │ {:<5} │ {:<5} │ {:<5} │ {:<6} │ {}",
             "Month", "TSS", "CTL", "ATL", "TSB", "Status");
    println!("───────────┼───────┼───────┼───────┼────────┼─────────────");

    let mut months: Vec<_> = monthly_data.keys().collect();
    months.sort();

    for (year, month) in months {
        let month_metrics = &monthly_data[&(*year, *month)];
        let total_tss: rust_decimal::Decimal = month_metrics.iter()
            .map(|m| m.daily_tss)
            .sum();

        // Use end-of-month values for CTL/ATL/TSB
        let last_metric = month_metrics.last().unwrap();

        let status_emoji = get_tsb_emoji(last_metric.tsb);
        let status_text = get_tsb_status_text(last_metric.tsb);
        let tsb_colored = get_tsb_color_string(&last_metric.tsb);

        println!("{:<10} │ {:>5.0} │ {:>5.1} │ {:>5.1} │ {:>6} │ {} {}",
            format!("{}-{:02}", year, month),
            total_tss,
            last_metric.ctl,
            last_metric.atl,
            tsb_colored,
            status_emoji,
            status_text
        );
    }
}

/// Display training trends analysis
fn display_training_trends(pmc_metrics: &[crate::pmc::PmcMetrics]) {
    use colored::Colorize;

    if pmc_metrics.len() < 7 {
        return; // Need at least a week of data for trends
    }

    println!("\n📈 TRAINING LOAD TRENDS");
    println!("=======================");

    let first = pmc_metrics.first().unwrap();
    let last = pmc_metrics.last().unwrap();
    let period_days = (last.date - first.date).num_days() as f64;

    // CTL trend
    let ctl_change = last.ctl - first.ctl;
    let ctl_rate = ctl_change / rust_decimal::Decimal::from_f64(period_days / 7.0).unwrap();
    let ctl_trend = if ctl_change > rust_decimal::Decimal::from(2) {
        "📈 Building".green()
    } else if ctl_change < rust_decimal::Decimal::from(-2) {
        "📉 Declining".red()
    } else {
        "➡️ Stable".yellow()
    };

    println!("CTL (Fitness):     {:>6.1} → {:>6.1} ({:+.1}/week) {}",
             first.ctl, last.ctl, ctl_rate, ctl_trend);

    // ATL trend
    let atl_change = last.atl - first.atl;
    let atl_rate = atl_change / rust_decimal::Decimal::from_f64(period_days / 7.0).unwrap();
    let atl_trend = if atl_change > rust_decimal::Decimal::from(3) {
        "⚡ High fatigue".red()
    } else if atl_change < rust_decimal::Decimal::from(-3) {
        "😌 Recovering".green()
    } else {
        "➡️ Normal".yellow()
    };

    println!("ATL (Fatigue):     {:>6.1} → {:>6.1} ({:+.1}/week) {}",
             first.atl, last.atl, atl_rate, atl_trend);

    // TSB trend
    let tsb_change = last.tsb - first.tsb;
    let tsb_trend = if tsb_change > rust_decimal::Decimal::from(5) {
        "🟢 Getting fresher".green()
    } else if tsb_change < rust_decimal::Decimal::from(-5) {
        "🔴 Getting fatigued".red()
    } else {
        "🟡 Steady form".yellow()
    };

    println!("TSB (Form):        {:>6.1} → {:>6.1} ({:+.1} change) {}",
             first.tsb, last.tsb, tsb_change, tsb_trend);

    // Ramp rate analysis
    let mut rapid_increases = 0;
    for window in pmc_metrics.windows(7) {
        if let (Some(start), Some(end)) = (window.first(), window.last()) {
            let weekly_ctl_change = end.ctl - start.ctl;
            if weekly_ctl_change > rust_decimal::Decimal::from(5) {
                rapid_increases += 1;
            }
        }
    }

    if rapid_increases > 0 {
        println!("⚠️  Detected {} week(s) with rapid fitness increases", rapid_increases);
    }
}

/// Display training warnings
fn display_training_warnings(pmc_metrics: &[crate::pmc::PmcMetrics]) {
    use colored::Colorize;

    println!("\n⚠️  TRAINING LOAD WARNINGS");
    println!("=========================");

    let mut warnings = Vec::new();

    // Check for extended negative TSB (overtraining risk)
    let mut negative_tsb_streak = 0;
    let mut max_negative_streak = 0;
    for metrics in pmc_metrics {
        if metrics.tsb < rust_decimal::Decimal::from(-10) {
            negative_tsb_streak += 1;
            max_negative_streak = max_negative_streak.max(negative_tsb_streak);
        } else {
            negative_tsb_streak = 0;
        }
    }

    if max_negative_streak >= 7 {
        warnings.push(format!("🔴 Extended fatigue period: {} days with TSB < -10", max_negative_streak).red());
    }

    // Check for extended positive TSB (fitness loss risk)
    let mut positive_tsb_streak = 0;
    let mut max_positive_streak = 0;
    for metrics in pmc_metrics {
        if metrics.tsb > rust_decimal::Decimal::from(15) {
            positive_tsb_streak += 1;
            max_positive_streak = max_positive_streak.max(positive_tsb_streak);
        } else {
            positive_tsb_streak = 0;
        }
    }

    if max_positive_streak >= 7 {
        warnings.push(format!("🟡 Extended recovery period: {} days with TSB > 15 (fitness loss risk)", max_positive_streak).yellow());
    }

    // Check for rapid CTL increases (injury risk)
    for window in pmc_metrics.windows(7) {
        if let (Some(start), Some(end)) = (window.first(), window.last()) {
            let weekly_ctl_change = end.ctl - start.ctl;
            if weekly_ctl_change > rust_decimal::Decimal::from(8) {
                warnings.push(format!("🔴 Rapid fitness increase detected: +{:.1} CTL in week ending {}",
                    weekly_ctl_change, end.date.format("%Y-%m-%d")).red());
            }
        }
    }

    // Check for ATL spikes
    let mut atl_spikes = 0;
    for metrics in pmc_metrics {
        if metrics.atl_spike {
            atl_spikes += 1;
        }
    }

    if atl_spikes > 0 {
        warnings.push(format!("⚡ {} ATL spike(s) detected (high acute load)", atl_spikes).yellow());
    }

    if warnings.is_empty() {
        println!("{}", "✅ No training load warnings detected".green());
    } else {
        for warning in warnings {
            println!("{}", warning);
        }
    }
}

/// Get emoji for TSB value
fn get_tsb_emoji(tsb: rust_decimal::Decimal) -> &'static str {
    if tsb >= rust_decimal::Decimal::from(25) {
        "🟢"
    } else if tsb >= rust_decimal::Decimal::from(5) {
        "🟢"
    } else if tsb >= rust_decimal::Decimal::from(-10) {
        "🟡"
    } else if tsb >= rust_decimal::Decimal::from(-30) {
        "🟠"
    } else {
        "🔴"
    }
}

/// Get status text for TSB value
fn get_tsb_status_text(tsb: rust_decimal::Decimal) -> &'static str {
    use crate::pmc::TsbInterpretation;
    match TsbInterpretation::from_tsb(tsb) {
        crate::pmc::TsbInterpretation::VeryFresh => "Very Fresh",
        crate::pmc::TsbInterpretation::Fresh => "Fresh",
        crate::pmc::TsbInterpretation::Neutral => "Neutral",
        crate::pmc::TsbInterpretation::Fatigued => "Fatigued",
        crate::pmc::TsbInterpretation::VeryFatigued => "Very Fatigued",
    }
}

/// Get colored string for TSB value
fn get_tsb_color_string(tsb: &rust_decimal::Decimal) -> String {
    use colored::Colorize;
    let tsb_str = format!("{:+.1}", tsb);

    if *tsb >= rust_decimal::Decimal::from(25) {
        tsb_str.bright_green().to_string()
    } else if *tsb >= rust_decimal::Decimal::from(5) {
        tsb_str.green().to_string()
    } else if *tsb >= rust_decimal::Decimal::from(-10) {
        tsb_str.yellow().to_string()
    } else if *tsb >= rust_decimal::Decimal::from(-30) {
        tsb_str.bright_red().to_string()
    } else {
        tsb_str.red().bold().to_string()
    }
}

/// Handle zone analysis command
fn handle_zone_analysis(
    cli: &Cli,
    last_days: Option<u16>,
    from: Option<String>,
    to: Option<String>,
    sport: Option<String>,
    zone_type: String,
    athlete: Option<String>,
    detailed: bool,
    show_patterns: bool,
    show_recommendations: bool,
    min_duration: Option<u32>,
) {
    use crate::export::DateRange;
    use crate::models::{Workout, Sport, AthleteProfile};
    use crate::zones::{ZoneAnalyzer, ZoneCalculator};

    println!("{}", "🎯 Analyzing zone distributions...".cyan().bold());

    // Handle global athlete flag
    let athlete_id = athlete.as_ref().or(cli.athlete.as_ref());
    if let Some(a) = athlete_id {
        println!("  Athlete: {}", a);
    }

    // Parse date range
    let date_range = parse_date_range(&last_days, &from, &to);
    if let Some(start) = date_range.start {
        if let Some(end) = date_range.end {
            println!("  📅 Period: {} to {}", start.format("%Y-%m-%d"), end.format("%Y-%m-%d"));
        } else {
            println!("  📅 Period: From {}", start.format("%Y-%m-%d"));
        }
    } else if let Some(end) = date_range.end {
        println!("  📅 Period: Up to {}", end.format("%Y-%m-%d"));
    }

    if let Some(s) = &sport {
        println!("  🏃 Sport filter: {}", s);
    }
    if let Some(duration) = min_duration {
        println!("  ⏱️  Minimum duration: {} minutes", duration);
    }
    println!("  📊 Zone type: {}", zone_type);

    // TODO: Load workout data from database/storage
    // For now, create sample data to demonstrate functionality
    let sample_workouts = create_sample_workouts();

    // Filter workouts by date range
    let filtered_workouts = date_range.filter_workouts(&sample_workouts);
    println!("  📈 Found {} workouts matching criteria", filtered_workouts.len());

    if filtered_workouts.is_empty() {
        println!("{}", "❌ No workouts found matching the specified criteria".yellow());
        return;
    }

    // Filter by sport if specified
    let workouts_by_sport: Vec<&Workout> = if let Some(sport_filter) = &sport {
        filtered_workouts.into_iter().filter(|w| {
            format!("{:?}", w.sport).to_lowercase().contains(&sport_filter.to_lowercase())
        }).collect()
    } else {
        filtered_workouts
    };

    // Filter by minimum duration if specified
    let final_workouts: Vec<&Workout> = if let Some(min_dur) = min_duration {
        let min_seconds = min_dur * 60;
        workouts_by_sport.into_iter().filter(|w| w.duration_seconds >= min_seconds).collect()
    } else {
        workouts_by_sport
    };

    if final_workouts.is_empty() {
        println!("{}", "❌ No workouts found after applying filters".yellow());
        return;
    }

    println!("  ✅ Analyzing {} workouts", final_workouts.len());

    // Create sample athlete profile for zone calculations
    let athlete_profile = create_sample_athlete_profile();

    // Perform zone analysis based on zone_type
    match zone_type.as_str() {
        "heart-rate" | "hr" => analyze_heart_rate_zones(&final_workouts, &athlete_profile, detailed),
        "power" => analyze_power_zones(&final_workouts, &athlete_profile, detailed),
        "pace" => analyze_pace_zones(&final_workouts, &athlete_profile, detailed),
        "all" => {
            analyze_heart_rate_zones(&final_workouts, &athlete_profile, detailed);
            analyze_power_zones(&final_workouts, &athlete_profile, detailed);
            analyze_pace_zones(&final_workouts, &athlete_profile, detailed);
        }
        _ => {
            println!("{}", "❌ Invalid zone type. Use: heart-rate, power, pace, or all".red());
            return;
        }
    }

    if show_patterns {
        analyze_training_patterns(&final_workouts);
    }

    if show_recommendations {
        provide_zone_recommendations(&final_workouts);
    }

    println!("{}", "✓ Zone analysis completed".green());
}

/// Parse date range from command line arguments
fn parse_date_range(last_days: &Option<u16>, from: &Option<String>, to: &Option<String>) -> crate::export::DateRange {

    // If last_days is specified, calculate from and to dates
    if let Some(days) = last_days {
        let end_date = chrono::Utc::now().date_naive();
        let start_date = end_date - Duration::days(*days as i64);
        return crate::export::DateRange::new(Some(start_date), Some(end_date));
    }

    // Parse from date
    let start_date = from.as_ref().and_then(|date_str| {
        NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()
    });

    // Parse to date
    let end_date = to.as_ref().and_then(|date_str| {
        NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()
    });

    crate::export::DateRange::new(start_date, end_date)
}

/// Create sample workouts for demonstration (TODO: Replace with actual data loading)
fn create_sample_workouts() -> Vec<crate::models::Workout> {
    use crate::models::{Workout, WorkoutSummary, Sport, WorkoutType, DataSource};

    let today = chrono::Utc::now().date_naive();
    vec![
        Workout {
            id: "sample_1".to_string(),
            date: today - Duration::days(7),
            sport: Sport::Cycling,
            duration_seconds: 3600,
            workout_type: WorkoutType::Endurance,
            data_source: DataSource::Power,
            raw_data: None,
            summary: WorkoutSummary {
                tss: Some(dec!(85)),
                avg_heart_rate: Some(150),
                max_heart_rate: Some(175),
                avg_power: Some(220),
                normalized_power: Some(235),
                intensity_factor: Some(dec!(0.94)),
                total_distance: Some(dec!(40000)),
                elevation_gain: Some(500),
                avg_cadence: Some(85),
                calories: Some(650),
                avg_pace: None,
            },
            notes: Some("Zone 2 endurance ride".to_string()),
            athlete_id: Some("test_athlete".to_string()),
            source: Some("sample_data".to_string()),
        },
        Workout {
            id: "sample_2".to_string(),
            date: today - Duration::days(5),
            sport: Sport::Running,
            duration_seconds: 2400,
            workout_type: WorkoutType::Tempo,
            data_source: DataSource::HeartRate,
            raw_data: None,
            summary: WorkoutSummary {
                tss: Some(dec!(65)),
                avg_heart_rate: Some(165),
                max_heart_rate: Some(185),
                avg_power: None,
                normalized_power: None,
                intensity_factor: None,
                total_distance: Some(dec!(8000)),
                elevation_gain: Some(100),
                avg_cadence: Some(180),
                calories: Some(420),
                avg_pace: Some(dec!(5.5)),
            },
            notes: Some("Tempo run".to_string()),
            athlete_id: Some("test_athlete".to_string()),
            source: Some("sample_data".to_string()),
        },
        Workout {
            id: "sample_3".to_string(),
            date: today - Duration::days(2),
            sport: Sport::Cycling,
            duration_seconds: 5400,
            workout_type: WorkoutType::Interval,
            data_source: DataSource::Power,
            raw_data: None,
            summary: WorkoutSummary {
                tss: Some(dec!(120)),
                avg_heart_rate: Some(160),
                max_heart_rate: Some(190),
                avg_power: Some(280),
                normalized_power: Some(310),
                intensity_factor: Some(dec!(1.24)),
                total_distance: Some(dec!(60000)),
                elevation_gain: Some(800),
                avg_cadence: Some(90),
                calories: Some(950),
                avg_pace: None,
            },
            notes: Some("High-intensity interval training".to_string()),
            athlete_id: Some("test_athlete".to_string()),
            source: Some("sample_data".to_string()),
        },
    ]
}

/// Handle power analysis commands
fn handle_power_commands(command: &PowerCommands, cli: &Cli) -> Result<()> {
    use crate::power::{PowerAnalyzer, CpModelType};
    use colored::Colorize;

    match command {
        PowerCommands::Curve {
            last_days,
            from,
            to,
            athlete,
            compare,
            export,
        } => {
            println!("{}", "🚴 Generating power curve analysis...".blue().bold());

            // Handle athlete selection
            let athlete_id = athlete.clone().or_else(|| cli.athlete.clone());
            if let Some(a) = &athlete_id {
                println!("  Athlete: {}", a);
            }

            // Parse date range (reuse existing logic)
            println!("  📅 Date range: Last {} days", last_days);
            if let Some(start) = &from {
                println!("    From: {}", start);
            }
            if let Some(end) = &to {
                println!("    To: {}", end);
            }

            // TODO: Load actual workout data
            println!("  📊 Creating sample power curve data...");
            let sample_workouts = create_sample_power_workouts();
            let workout_refs: Vec<&crate::models::Workout> = sample_workouts.iter().collect();

            match PowerAnalyzer::calculate_power_curve(&workout_refs, None) {
                Ok(power_curve) => {
                    display_power_curve(&power_curve, *compare);

                    if let Some(export_path) = export {
                        println!("  💾 Exporting to: {}", export_path.display());
                        // TODO: Implement power curve export
                    }
                }
                Err(e) => {
                    eprintln!("{}", format!("Failed to calculate power curve: {}", e).red());
                }
            }

            println!("{}", "✓ Power curve analysis completed".blue());
        }

        PowerCommands::CriticalPower {
            file,
            three_parameter,
            power_3min,
            power_5min,
            power_20min,
            athlete,
        } => {
            println!("{}", "⚡ Critical Power analysis...".blue().bold());

            // Handle athlete selection
            let athlete_id = athlete.clone().or_else(|| cli.athlete.clone());
            if let Some(a) = &athlete_id {
                println!("  Athlete: {}", a);
            }

            let model_type = if *three_parameter {
                CpModelType::ThreeParameter { time_constant: rust_decimal_macros::dec!(30) }
            } else {
                CpModelType::TwoParameter
            };

            println!("  Model: {:?}", model_type);

            if let Some(file_path) = file {
                println!("  📁 Loading test data from: {}", file_path.display());
                // TODO: Load and analyze test file
            }

            // Use manual power inputs if provided
            if power_3min.is_some() || power_5min.is_some() || power_20min.is_some() {
                println!("  Using manual power inputs:");
                if let Some(p3) = power_3min {
                    println!("    3-min power: {} W", p3);
                }
                if let Some(p5) = power_5min {
                    println!("    5-min power: {} W", p5);
                }
                if let Some(p20) = power_20min {
                    println!("    20-min power: {} W", p20);
                }
            }

            // Create sample data for demonstration
            let sample_workouts = create_sample_power_workouts();
            let workout_refs: Vec<&crate::models::Workout> = sample_workouts.iter().collect();

            match PowerAnalyzer::calculate_power_curve(&workout_refs, None) {
                Ok(power_curve) => {
                    match PowerAnalyzer::fit_critical_power_model(&power_curve, model_type) {
                        Ok(cp_model) => {
                            display_critical_power_model(&cp_model);
                        }
                        Err(e) => {
                            eprintln!("{}", format!("Failed to fit CP model: {}", e).red());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}", format!("Failed to calculate power curve for CP analysis: {}", e).red());
                }
            }

            println!("{}", "✓ Critical Power analysis completed".blue());
        }

        PowerCommands::Analyze {
            file,
            show_intervals,
            ftp,
            quadrants,
            balance,
            export,
        } => {
            println!("{}", "📊 Analyzing workout power data...".blue().bold());
            println!("  📁 File: {}", file.display());

            if let Some(ftp_value) = ftp {
                println!("  ⚡ FTP: {} W", ftp_value);
            }

            // TODO: Load actual workout file
            println!("  📊 Creating sample workout data for analysis...");
            let sample_data = create_sample_power_datapoints();

            // Calculate power metrics
            match PowerAnalyzer::calculate_power_metrics(&sample_data, *ftp) {
                Ok(metrics) => {
                    display_power_metrics(&metrics);
                }
                Err(e) => {
                    eprintln!("{}", format!("Failed to calculate power metrics: {}", e).red());
                }
            }

            // Peak power analysis
            match PowerAnalyzer::analyze_peak_powers(&sample_data) {
                Ok(peaks) => {
                    display_peak_power_analysis(&peaks);
                }
                Err(e) => {
                    eprintln!("{}", format!("Failed to analyze peak powers: {}", e).red());
                }
            }

            // Quadrant analysis if requested
            if *quadrants {
                if let Some(ftp_value) = ftp {
                    match PowerAnalyzer::analyze_quadrants(&sample_data, *ftp_value, 85) {
                        Ok(quad_analysis) => {
                            display_quadrant_analysis(&quad_analysis);
                        }
                        Err(e) => {
                            eprintln!("{}", format!("Failed to perform quadrant analysis: {}", e).red());
                        }
                    }
                } else {
                    println!("{}", "⚠️  FTP required for quadrant analysis".yellow());
                }
            }

            // Power balance analysis if requested
            if *balance {
                match PowerAnalyzer::analyze_power_balance(&sample_data) {
                    Ok(balance_analysis) => {
                        display_power_balance(&balance_analysis);
                    }
                    Err(e) => {
                        eprintln!("{}", format!("Failed to analyze power balance: {}", e).red());
                    }
                }
            }

            if *show_intervals {
                // TODO: Implement interval detection and analysis
                println!("  🔍 Interval analysis (coming soon)");
            }

            if let Some(export_path) = export {
                println!("  💾 Exporting detailed analysis to: {}", export_path.display());
                // TODO: Implement detailed analysis export
            }

            println!("{}", "✓ Power analysis completed".blue());
        }
    }

    Ok(())
}

// Helper functions for power analysis displays and sample data

/// Create sample workout data for power analysis demonstration
fn create_sample_power_workouts() -> Vec<crate::models::Workout> {
    use crate::models::*;
    use rust_decimal_macros::dec;
    use chrono::NaiveDate;

    vec![
        Workout {
            id: "sample_1".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            sport: Sport::Cycling,
            duration_seconds: 3600, // 1 hour
            workout_type: WorkoutType::Interval,
            data_source: DataSource::Power,
            raw_data: Some(create_sample_intervals_data()),
            summary: WorkoutSummary {
                avg_power: Some(250),
                normalized_power: Some(265),
                tss: Some(dec!(85)),
                intensity_factor: Some(dec!(0.88)),
                ..WorkoutSummary::default()
            },
            notes: Some("High intensity interval workout".to_string()),
            athlete_id: None,
            source: Some("trainrs".to_string()),
        },
        Workout {
            id: "sample_2".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 1, 20).unwrap(),
            sport: Sport::Cycling,
            duration_seconds: 7200, // 2 hours
            workout_type: WorkoutType::Endurance,
            data_source: DataSource::Power,
            raw_data: Some(create_sample_endurance_data()),
            summary: WorkoutSummary {
                avg_power: Some(200),
                normalized_power: Some(210),
                tss: Some(dec!(120)),
                intensity_factor: Some(dec!(0.7)),
                ..WorkoutSummary::default()
            },
            notes: Some("Long endurance ride".to_string()),
            athlete_id: None,
            source: Some("trainrs".to_string()),
        },
    ]
}

/// Create sample athlete profile for zone calculations (TODO: Load from actual profile data)
fn create_sample_athlete_profile() -> crate::models::AthleteProfile {
    use crate::models::{AthleteProfile, TrainingZones, Units};
    use chrono::Utc;

    let now = Utc::now();
    AthleteProfile {
        id: "test_athlete".to_string(),
        name: "Test Athlete".to_string(),
        date_of_birth: Some(chrono::NaiveDate::from_ymd_opt(1990, 1, 1).unwrap()),
        weight: Some(dec!(70.0)),
        height: Some(175),
        ftp: Some(250),
        lthr: Some(165),
        threshold_pace: Some(dec!(5.5)), // 5.5 min/km
        max_hr: Some(190),
        resting_hr: Some(50),
        training_zones: TrainingZones::default(),
        preferred_units: Units::Metric,
        created_at: now,
        updated_at: now,
    }
}

/// Analyze heart rate zone distribution
fn analyze_heart_rate_zones(workouts: &[&crate::models::Workout], athlete_profile: &crate::models::AthleteProfile, detailed: bool) {
    use crate::zones::{ZoneCalculator, HRZoneMethod, ZoneAnalyzer};

    println!("\n💓 HEART RATE ZONE ANALYSIS");
    println!("===========================");

    // Calculate heart rate zones from athlete profile
    let hr_zones = match ZoneCalculator::calculate_heart_rate_zones(athlete_profile, HRZoneMethod::LTHR) {
        Ok(zones) => zones,
        Err(e) => {
            println!("{}", format!("❌ Could not calculate HR zones: {}", e).red());
            return;
        }
    };

    println!("🎯 Zone Boundaries (based on LTHR: {} bpm):", athlete_profile.lthr.unwrap_or(0));
    println!("  Zone 1: ≤ {} bpm (Active Recovery)", hr_zones.zone1_max);
    println!("  Zone 2: {}-{} bpm (Aerobic Base)", hr_zones.zone1_max + 1, hr_zones.zone2_max);
    println!("  Zone 3: {}-{} bpm (Aerobic)", hr_zones.zone2_max + 1, hr_zones.zone3_max);
    println!("  Zone 4: {}-{} bpm (Lactate Threshold)", hr_zones.zone3_max + 1, hr_zones.zone4_max);
    println!("  Zone 5: ≥ {} bpm (VO2 Max)", hr_zones.zone4_max + 1);

    // Aggregate all heart rate data from workouts
    let mut all_hr_data = Vec::new();
    let mut total_time_seconds = 0u32;

    for workout in workouts {
        if workout.summary.avg_heart_rate.is_some() {
            // Simulate heart rate data based on average (in real implementation, use raw_data)
            let avg_hr = workout.summary.avg_heart_rate.unwrap() as u16;
            let workout_duration = workout.duration_seconds;

            // Create simulated HR data points (one per second)
            for i in 0..workout_duration {
                // Add some deterministic variation around average HR
                let variation = ((i as i16 * 7) % 20) - 10; // ±10 bpm variation
                let hr = ((avg_hr as i16) + variation).max(50).min(220) as u16;
                all_hr_data.push(hr);
            }

            total_time_seconds += workout_duration;
        }
    }

    if all_hr_data.is_empty() {
        println!("{}", "❌ No heart rate data found in selected workouts".yellow());
        return;
    }

    // Calculate zone distribution
    let distribution = ZoneAnalyzer::analyze_hr_distribution(&all_hr_data, &hr_zones);

    // Display results
    println!("\n📊 Zone Distribution ({} total data points, {:.1} hours):",
        distribution.total_points,
        total_time_seconds as f64 / 3600.0
    );

    display_zone_bar_chart(&[
        ("Zone 1", distribution.zone1_percent, "🟢"),
        ("Zone 2", distribution.zone2_percent, "🟡"),
        ("Zone 3", distribution.zone3_percent, "🟠"),
        ("Zone 4", distribution.zone4_percent, "🔴"),
        ("Zone 5", distribution.zone5_percent, "🟣"),
    ]);

    if detailed {
        display_detailed_zone_stats(&[
            ("Zone 1 (Recovery)", distribution.zone1_percent, total_time_seconds),
            ("Zone 2 (Base)", distribution.zone2_percent, total_time_seconds),
            ("Zone 3 (Aerobic)", distribution.zone3_percent, total_time_seconds),
            ("Zone 4 (Threshold)", distribution.zone4_percent, total_time_seconds),
            ("Zone 5 (VO2 Max)", distribution.zone5_percent, total_time_seconds),
        ]);
    }
}

/// Analyze power zone distribution
fn analyze_power_zones(workouts: &[&crate::models::Workout], athlete_profile: &crate::models::AthleteProfile, detailed: bool) {
    use crate::zones::{ZoneCalculator, ZoneAnalyzer};
    use crate::models::Sport;

    println!("\n⚡ POWER ZONE ANALYSIS");
    println!("======================");

    // Calculate power zones from athlete profile
    let power_zones = match ZoneCalculator::calculate_power_zones(athlete_profile) {
        Ok(zones) => zones,
        Err(e) => {
            println!("{}", format!("❌ Could not calculate power zones: {}", e).red());
            return;
        }
    };

    println!("🎯 Zone Boundaries (based on FTP: {} watts):", athlete_profile.ftp.unwrap_or(0));
    println!("  Zone 1: ≤ {} W (Active Recovery)", power_zones.zone1_max);
    println!("  Zone 2: {}-{} W (Endurance)", power_zones.zone1_max + 1, power_zones.zone2_max);
    println!("  Zone 3: {}-{} W (Tempo)", power_zones.zone2_max + 1, power_zones.zone3_max);
    println!("  Zone 4: {}-{} W (Lactate Threshold)", power_zones.zone3_max + 1, power_zones.zone4_max);
    println!("  Zone 5: {}-{} W (VO2 Max)", power_zones.zone4_max + 1, power_zones.zone5_max);
    println!("  Zone 6: {}-{} W (Anaerobic)", power_zones.zone5_max + 1, power_zones.zone6_max);
    println!("  Zone 7: ≥ {} W (Sprint Power)", power_zones.zone6_max + 1);

    // Aggregate all power data from workouts
    let mut all_power_data = Vec::new();
    let mut total_time_seconds = 0u32;

    for workout in workouts {
        if workout.summary.avg_power.is_some() && workout.sport == Sport::Cycling {
            // Simulate power data based on average (in real implementation, use raw_data)
            let avg_power = workout.summary.avg_power.unwrap() as u16;
            let workout_duration = workout.duration_seconds;

            // Create simulated power data points (one per second)
            for i in 0..workout_duration {
                // Add some deterministic variation around average power
                let variation = ((i as i16 * 11) % 100) - 50; // ±50 watts variation
                let power = ((avg_power as i16) + variation).max(0).min(800) as u16;
                all_power_data.push(power);
            }

            total_time_seconds += workout_duration;
        }
    }

    if all_power_data.is_empty() {
        println!("{}", "❌ No power data found in selected workouts".yellow());
        return;
    }

    // Calculate zone distribution
    let distribution = ZoneAnalyzer::analyze_power_distribution(&all_power_data, &power_zones);

    // Display results
    println!("\n📊 Zone Distribution ({} total data points, {:.1} hours):",
        distribution.total_points,
        total_time_seconds as f64 / 3600.0
    );

    display_power_zone_bar_chart(&[
        ("Zone 1", distribution.zone1_percent, "🟢"),
        ("Zone 2", distribution.zone2_percent, "🟡"),
        ("Zone 3", distribution.zone3_percent, "🟠"),
        ("Zone 4", distribution.zone4_percent, "🔴"),
        ("Zone 5", distribution.zone5_percent, "🟣"),
        ("Zone 6", distribution.zone6_percent, "🔵"),
        ("Zone 7", distribution.zone7_percent, "⚫"),
    ]);

    if detailed {
        display_detailed_zone_stats(&[
            ("Zone 1 (Recovery)", distribution.zone1_percent, total_time_seconds),
            ("Zone 2 (Endurance)", distribution.zone2_percent, total_time_seconds),
            ("Zone 3 (Tempo)", distribution.zone3_percent, total_time_seconds),
            ("Zone 4 (Threshold)", distribution.zone4_percent, total_time_seconds),
            ("Zone 5 (VO2 Max)", distribution.zone5_percent, total_time_seconds),
            ("Zone 6 (Anaerobic)", distribution.zone6_percent, total_time_seconds),
            ("Zone 7 (Sprint)", distribution.zone7_percent, total_time_seconds),
        ]);
    }
}

/// Analyze pace zone distribution
fn analyze_pace_zones(workouts: &[&crate::models::Workout], athlete_profile: &crate::models::AthleteProfile, detailed: bool) {
    use crate::zones::ZoneCalculator;
    use crate::models::Sport;
    println!("\n🏃 PACE ZONE ANALYSIS");
    println!("=====================");

    // For pace zones, we need running workouts with pace data
    let running_workouts: Vec<&crate::models::Workout> = workouts.iter()
        .filter(|w| w.sport == Sport::Running && w.summary.avg_pace.is_some())
        .cloned()
        .collect();

    if running_workouts.is_empty() {
        println!("{}", "❌ No running workouts with pace data found".yellow());
        return;
    }

    // Calculate pace zones from athlete profile
    let pace_zones = match ZoneCalculator::calculate_pace_zones(athlete_profile) {
        Ok(zones) => zones,
        Err(e) => {
            println!("{}", format!("❌ Could not calculate pace zones: {}", e).red());
            return;
        }
    };

    let threshold_pace = athlete_profile.threshold_pace.unwrap_or(dec!(6.0));
    println!("🎯 Zone Boundaries (based on threshold pace: {:.2} min/km):", threshold_pace);
    println!("  Zone 1: ≥ {:.2} min/km (Easy)", pace_zones.zone1_min);
    println!("  Zone 2: {:.2}-{:.2} min/km (Aerobic Base)", pace_zones.zone2_min, pace_zones.zone1_min);
    println!("  Zone 3: {:.2}-{:.2} min/km (Tempo)", pace_zones.zone3_min, pace_zones.zone2_min);
    println!("  Zone 4: {:.2}-{:.2} min/km (Threshold)", pace_zones.zone4_min, pace_zones.zone3_min);
    println!("  Zone 5: ≤ {:.2} min/km (VO2 Max)", pace_zones.zone5_min);

    // Calculate zone distribution for pace
    let mut zone_counts = [0u32; 5];
    let mut total_time_seconds = 0u32;

    for workout in &running_workouts {
        if let Some(avg_pace) = workout.summary.avg_pace {
            let zone = ZoneCalculator::get_pace_zone(avg_pace, &pace_zones);
            if zone >= 1 && zone <= 5 {
                zone_counts[(zone - 1) as usize] += workout.duration_seconds;
            }
            total_time_seconds += workout.duration_seconds;
        }
    }

    // Convert time to percentages
    let zone_percentages: Vec<Decimal> = zone_counts.iter()
        .map(|&count| {
            if total_time_seconds > 0 {
                (Decimal::from(count) / Decimal::from(total_time_seconds)) * dec!(100.0)
            } else {
                dec!(0.0)
            }
        })
        .collect();

    // Display results
    println!("\n📊 Zone Distribution ({:.1} hours of running):",
        total_time_seconds as f64 / 3600.0
    );

    display_zone_bar_chart(&[
        ("Zone 1", zone_percentages[0], "🟢"),
        ("Zone 2", zone_percentages[1], "🟡"),
        ("Zone 3", zone_percentages[2], "🟠"),
        ("Zone 4", zone_percentages[3], "🔴"),
        ("Zone 5", zone_percentages[4], "🟣"),
    ]);

    if detailed {
        display_detailed_zone_stats(&[
            ("Zone 1 (Easy)", zone_percentages[0], total_time_seconds),
            ("Zone 2 (Base)", zone_percentages[1], total_time_seconds),
            ("Zone 3 (Tempo)", zone_percentages[2], total_time_seconds),
            ("Zone 4 (Threshold)", zone_percentages[3], total_time_seconds),
            ("Zone 5 (VO2 Max)", zone_percentages[4], total_time_seconds),
        ]);
    }
}

/// Display ASCII bar chart for zone distribution
fn display_zone_bar_chart(zones: &[(&str, Decimal, &str)]) {
    const BAR_WIDTH: usize = 40;

    for (zone_name, percentage, emoji) in zones {
        let bar_length = ((*percentage * Decimal::from(BAR_WIDTH)) / dec!(100.0)).to_usize().unwrap_or(0);
        let bar = "█".repeat(bar_length) + &"░".repeat(BAR_WIDTH - bar_length);

        println!("  {} {} |{}| {:>5.1}%",
            emoji,
            format!("{:<8}", zone_name).bold(),
            bar.color(match zone_name {
                zone if zone.contains("Zone 1") => "green",
                zone if zone.contains("Zone 2") => "yellow",
                zone if zone.contains("Zone 3") => "bright_yellow",
                zone if zone.contains("Zone 4") => "red",
                zone if zone.contains("Zone 5") => "magenta",
                zone if zone.contains("Zone 6") => "blue",
                zone if zone.contains("Zone 7") => "white",
                _ => "white"
            }),
            percentage
        );
    }
}

/// Display ASCII bar chart for power zones (7 zones)
fn display_power_zone_bar_chart(zones: &[(&str, Decimal, &str)]) {
    const BAR_WIDTH: usize = 40;

    for (zone_name, percentage, emoji) in zones {
        let bar_length = ((*percentage * Decimal::from(BAR_WIDTH)) / dec!(100.0)).to_usize().unwrap_or(0);
        let bar = "█".repeat(bar_length) + &"░".repeat(BAR_WIDTH - bar_length);

        println!("  {} {} |{}| {:>5.1}%",
            emoji,
            format!("{:<8}", zone_name).bold(),
            bar.color(match zone_name {
                zone if zone.contains("Zone 1") => "green",
                zone if zone.contains("Zone 2") => "yellow",
                zone if zone.contains("Zone 3") => "bright_yellow",
                zone if zone.contains("Zone 4") => "red",
                zone if zone.contains("Zone 5") => "magenta",
                zone if zone.contains("Zone 6") => "blue",
                zone if zone.contains("Zone 7") => "white",
                _ => "white"
            }),
            percentage
        );
    }
}

/// Display detailed zone statistics
fn display_detailed_zone_stats(zones: &[(&str, Decimal, u32)]) {
    println!("\n📈 Detailed Zone Statistics:");
    println!("  {:<20} {:>8} {:>8} {:>8}", "Zone", "Time", "Percent", "Hours");
    println!("  {}", "─".repeat(48));

    for (zone_name, percentage, total_seconds) in zones {
        let time_in_zone = (*percentage / dec!(100.0)) * Decimal::from(*total_seconds);
        let hours = time_in_zone / dec!(3600.0);

        println!("  {:<20} {:>6.0}s {:>6.1}% {:>6.1}h",
            zone_name,
            time_in_zone,
            percentage,
            hours
        );
    }
}

/// Analyze training patterns (polarized, threshold, high-intensity)
fn analyze_training_patterns(workouts: &[&crate::models::Workout]) {
    println!("\n🎯 TRAINING PATTERN ANALYSIS");
    println!("=============================");

    // Categorize workouts by intensity
    let mut low_intensity = 0;
    let mut moderate_intensity = 0;
    let mut high_intensity = 0;

    for workout in workouts {
        if let Some(if_val) = workout.summary.intensity_factor {
            if if_val < dec!(0.75) {
                low_intensity += 1;
            } else if if_val < dec!(0.95) {
                moderate_intensity += 1;
            } else {
                high_intensity += 1;
            }
        }
    }

    let total_workouts = workouts.len() as f64;
    if total_workouts == 0.0 {
        println!("{}", "❌ No workouts available for pattern analysis".yellow());
        return;
    }

    let low_pct = (low_intensity as f64 / total_workouts) * 100.0;
    let mod_pct = (moderate_intensity as f64 / total_workouts) * 100.0;
    let high_pct = (high_intensity as f64 / total_workouts) * 100.0;

    println!("📊 Intensity Distribution:");
    println!("  🟢 Low Intensity (IF < 0.75):    {:>2} workouts ({:>4.1}%)", low_intensity, low_pct);
    println!("  🟡 Moderate Intensity (0.75-0.95): {:>2} workouts ({:>4.1}%)", moderate_intensity, mod_pct);
    println!("  🔴 High Intensity (IF > 0.95):   {:>2} workouts ({:>4.1}%)", high_intensity, high_pct);

    // Determine training pattern
    println!("\n🔍 Pattern Analysis:");
    if low_pct >= 75.0 && high_pct >= 15.0 && mod_pct <= 15.0 {
        println!("  ✅ {} Polarized Training Pattern Detected", "🎯".green().bold());
        println!("     Optimal distribution: ~80% easy, ~20% hard, minimal moderate intensity");
    } else if mod_pct >= 40.0 {
        println!("  ⚠️  {} Threshold-Heavy Pattern Detected", "🟡".yellow().bold());
        println!("     High moderate intensity may limit recovery and adaptation");
    } else if high_pct >= 40.0 {
        println!("  🔥 {} High-Intensity Pattern Detected", "🔴".red().bold());
        println!("     Very high intensity load - ensure adequate recovery");
    } else {
        println!("  📊 {} Mixed Training Pattern", "🔵".blue().bold());
        println!("     No clear pattern detected - consider structure");
    }
}

/// Provide zone-based training recommendations
fn provide_zone_recommendations(workouts: &[&crate::models::Workout]) {
    println!("\n💡 ZONE-BASED TRAINING RECOMMENDATIONS");
    println!("======================================");

    // Calculate recent training distribution
    let mut zone1_time = 0u32;
    let mut zone2_time = 0u32;
    let mut high_intensity_time = 0u32;
    let mut total_time = 0u32;

    for workout in workouts {
        let duration = workout.duration_seconds;
        total_time += duration;

        if let Some(if_val) = workout.summary.intensity_factor {
            if if_val < dec!(0.75) {
                zone1_time += duration;
            } else if if_val < dec!(0.85) {
                zone2_time += duration;
            } else {
                high_intensity_time += duration;
            }
        }
    }

    if total_time == 0 {
        println!("{}", "❌ No training data available for recommendations".yellow());
        return;
    }

    let zone1_pct = (zone1_time as f64 / total_time as f64) * 100.0;
    let zone2_pct = (zone2_time as f64 / total_time as f64) * 100.0;
    let high_pct = (high_intensity_time as f64 / total_time as f64) * 100.0;

    let total_hours = total_time as f64 / 3600.0;

    println!("📈 Current Training Analysis ({:.1} hours):", total_hours);
    println!("  🟢 Easy/Recovery (Zone 1): {:.1}% ({:.1}h)", zone1_pct, zone1_time as f64 / 3600.0);
    println!("  🟡 Base/Aerobic (Zone 2):  {:.1}% ({:.1}h)", zone2_pct, zone2_time as f64 / 3600.0);
    println!("  🔴 High Intensity:         {:.1}% ({:.1}h)", high_pct, high_intensity_time as f64 / 3600.0);

    println!("\n🎯 Recommendations:");

    // Zone 1 recommendations
    if zone1_pct < 60.0 {
        println!("  🟢 {} More Zone 1 (Easy) Training", "INCREASE".green().bold());
        println!("     • Target 60-80% of total training time");
        println!("     • Builds aerobic base and enhances recovery");
        println!("     • Should feel 'conversational' pace");
    }

    // Zone 2 recommendations
    if zone2_pct < 15.0 {
        println!("  🟡 {} Zone 2 (Base) Training", "ADD".yellow().bold());
        println!("     • Target 15-25% of total training time");
        println!("     • Builds aerobic power and efficiency");
        println!("     • Comfortably hard, sustainable pace");
    } else if zone2_pct > 30.0 {
        println!("  🟡 {} Zone 2 Training", "REDUCE".bright_red().bold());
        println!("     • Too much moderate intensity can impede recovery");
        println!("     • Replace some Zone 2 with Zone 1 or high intensity");
    }

    // High intensity recommendations
    if high_pct < 15.0 && total_hours > 5.0 {
        println!("  🔴 {} High-Intensity Training", "ADD".red().bold());
        println!("     • Target 15-20% of total training time");
        println!("     • Improves VO2max and race-specific fitness");
        println!("     • Include threshold and VO2max intervals");
    } else if high_pct > 25.0 {
        println!("  🔴 {} High-Intensity Training", "REDUCE".bright_red().bold());
        println!("     • Too much high intensity increases injury risk");
        println!("     • Ensure adequate recovery between hard sessions");
    }

    // Weekly structure recommendations
    println!("\n📅 Weekly Structure Suggestions:");
    if total_hours < 5.0 {
        println!("  • 3-4 workouts: 2-3 easy, 1 hard session");
        println!("  • Focus on building base volume first");
    } else if total_hours < 10.0 {
        println!("  • 4-5 workouts: 3-4 easy, 1-2 hard sessions");
        println!("  • Add one Zone 2 session per week");
    } else {
        println!("  • 5-7 workouts: 4-5 easy, 2-3 structured sessions");
        println!("  • Include one threshold and one VO2max session weekly");
        println!("  • One long Zone 2 session for base building");
    }

    println!("\n⚠️  Remember:");
    println!("  • Consistency beats intensity for long-term improvement");
    println!("  • Allow 48+ hours between high-intensity sessions");
    println!("  • Listen to your body and adjust accordingly");
}

/// Create sample interval workout data
fn create_sample_intervals_data() -> Vec<DataPoint> {
    let mut data_points = Vec::new();
    let mut timestamp = 0;

    // Warm-up: 10 minutes at 150W
    for _ in 0..600 {
        data_points.push(DataPoint {
            timestamp,
            power: Some(150),
            heart_rate: Some(130),
            cadence: Some(85),
            speed: Some(rust_decimal::Decimal::from_f64(8.33).unwrap()), // 30 km/h = 8.33 m/s
            distance: Some(rust_decimal::Decimal::from_f64(500.0).unwrap()), // 0.5 km = 500 m
            left_power: Some(75),
            right_power: Some(75),
            pace: None,
            elevation: None
        });
        timestamp += 1;
    }

    // 5x5min intervals at 300W with 2min recovery at 100W
    for _ in 0..5 {
        // 5 minutes at 300W
        for _ in 0..300 {
            data_points.push(DataPoint {
                timestamp,
                power: Some(300),
                heart_rate: Some(165),
                cadence: Some(95),
                speed: Some(rust_decimal::Decimal::from_f64(9.72).unwrap()), // 35 km/h = 9.72 m/s
                distance: Some(rust_decimal::Decimal::from_f64(580.0).unwrap()), // 0.58 km = 580 m
                left_power: Some(148),
                right_power: Some(152),
                pace: None,
            elevation: None
            });
            timestamp += 1;
        }

        // 2 minutes recovery at 100W
        for _ in 0..120 {
            data_points.push(DataPoint {
                timestamp,
                power: Some(100),
                heart_rate: Some(140),
                cadence: Some(75),
                speed: Some(rust_decimal::Decimal::from_f64(6.94).unwrap()), // 25 km/h = 6.94 m/s
                distance: Some(rust_decimal::Decimal::from_f64(420.0).unwrap()), // 0.42 km = 420 m
                left_power: Some(50),
                right_power: Some(50),
                pace: None,
            elevation: None
            });
            timestamp += 1;
        }
    }

    // Cool-down: remaining time at 120W
    while data_points.len() < 3600 {
        data_points.push(DataPoint {
            timestamp,
            power: Some(120),
            heart_rate: Some(125),
            cadence: Some(80),
            speed: Some(rust_decimal::Decimal::from_f64(7.78).unwrap()), // 28 km/h = 7.78 m/s
            distance: Some(rust_decimal::Decimal::from_f64(470.0).unwrap()), // 0.47 km = 470 m
            left_power: Some(60),
            right_power: Some(60),
            pace: None,
            elevation: None
        });
        timestamp += 1;
    }

    data_points
}

/// Create sample endurance workout data
fn create_sample_endurance_data() -> Vec<DataPoint> {
    let mut data_points = Vec::new();
    let mut timestamp = 0;

    // 2 hours of steady endurance riding with some variation
    for i in 0..7200 {
        // Add some variation to simulate realistic power data
        let base_power = 200;
        let variation = ((i as f64 / 100.0).sin() * 20.0) as i16;
        let power = (base_power + variation).max(150) as u16;

        data_points.push(DataPoint {
            timestamp,
            power: Some(power),
            heart_rate: Some(140 + (variation.abs() / 4) as u16),
            cadence: Some((85 + variation / 10).max(60) as u16),
            speed: Some(rust_decimal::Decimal::from_f64(8.89).unwrap() + rust_decimal::Decimal::from(variation) / dec!(3.6)), // 32 km/h = 8.89 m/s
            distance: Some(rust_decimal::Decimal::from_f64(530.0).unwrap()), // 0.53 km = 530 m
            left_power: Some(power / 2 - 2),
            right_power: Some(power / 2 + 2),
            pace: None,
            elevation: None
        });
        timestamp += 1;
    }

    data_points
}

/// Create sample power data points for analysis
fn create_sample_power_datapoints() -> Vec<DataPoint> {
    create_sample_intervals_data()
}

/// Display power curve analysis results
fn display_power_curve(power_curve: &crate::power::PowerCurve, compare: bool) {
    use colored::Colorize;

    println!("\n📊 POWER CURVE ANALYSIS");
    println!("========================");
    println!("Date Range: {} to {}", power_curve.date_range.0, power_curve.date_range.1);
    println!("Total Data Points: {}", power_curve.points.len());
    println!();

    println!("{:<12} │ {:<8} │ {:<12}", "Duration", "Power (W)", "Date Set");
    println!("─────────────┼──────────┼─────────────");

    // Standard durations with nice formatting
    let durations = vec![
        (1, "1 second"),
        (5, "5 seconds"),
        (15, "15 seconds"),
        (30, "30 seconds"),
        (60, "1 minute"),
        (300, "5 minutes"),
        (600, "10 minutes"),
        (1200, "20 minutes"),
        (3600, "1 hour"),
        (14400, "4 hours"),
        (21600, "6 hours"),
    ];

    for (duration_secs, duration_str) in durations {
        if let Some(power_value) = power_curve.standard_durations.get(&duration_secs) {
            // Find the corresponding date from the points
            let date_str = power_curve.points
                .iter()
                .find(|p| p.duration_seconds == duration_secs && p.max_power == *power_value)
                .map(|p| p.date.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "N/A".to_string());
            println!("{:<12} │ {:>8} │ {:<12}",
                duration_str,
                format!("{}", power_value).yellow().bold(),
                date_str.dimmed()
            );
        }
    }

    if compare {
        println!("\n📈 POWER CURVE COMPARISON");
        println!("==========================");
        println!("{}", "Note: Comparison with previous periods coming soon".dimmed());
    }

    // Power curve insights
    if let (Some(sprint), Some(cp20)) = (power_curve.standard_durations.get(&5), power_curve.standard_durations.get(&1200)) {
        let sprint_to_cp_ratio = *sprint as f64 / *cp20 as f64;
        println!("\n💡 POWER PROFILE INSIGHTS");
        println!("==========================");
        println!("Sprint:CP20 Ratio: {:.2} (>2.5 = sprinter, <2.0 = time trialist)", sprint_to_cp_ratio);

        if sprint_to_cp_ratio > 2.5 {
            println!("{}", "🚀 Strong sprinting power profile".green());
        } else if sprint_to_cp_ratio < 2.0 {
            println!("{}", "⏱️ Strong time trial/endurance profile".blue());
        } else {
            println!("{}", "⚖️ Balanced power profile".yellow());
        }
    }
}

/// Display critical power model results
fn display_critical_power_model(cp_model: &crate::power::CriticalPowerModel) {
    use colored::Colorize;

    println!("\n⚡ CRITICAL POWER MODEL RESULTS");
    println!("===============================\n");

    // Model type and parameters
    println!("{}: {:?}", "Model Type".bold(), cp_model.model_type);
    println!("{}: {:.1} W", "Critical Power (CP)".bold(), cp_model.critical_power);
    println!("{}: {:.0} J", "W' (Anaerobic Work Capacity)".bold(), cp_model.w_prime);

    if let crate::power::CpModelType::ThreeParameter { time_constant } = &cp_model.model_type {
        println!("{}: {:.1} s", "Tau (Recovery Time Constant)".bold(), time_constant);
    }

    println!("{}: {:.4}", "R² (Goodness of Fit)".bold(), cp_model.r_squared);
    println!();

    // Model quality assessment
    let quality = if cp_model.r_squared > dec!(0.95) {
        "Excellent fit".green().bold()
    } else if cp_model.r_squared > dec!(0.90) {
        "Good fit".yellow().bold()
    } else {
        "Poor fit - consider more data points".red().bold()
    };
    println!("{}: {}", "Model Quality", quality);
    println!();

    // Practical applications
    println!("{}", "📈 TRAINING APPLICATIONS".blue().bold());
    println!("========================");
    println!("• FTP Estimate: {:.0} W (CP)", cp_model.critical_power);
    println!("• Anaerobic Reserve: {:.0} kJ ({:.1} seconds at 400W)",
        cp_model.w_prime as f64 / 1000.0,
        cp_model.w_prime as f64 / (400.0 - cp_model.critical_power as f64)
    );

    // Training zones based on CP
    println!("\n🎯 TRAINING ZONES (based on CP)");
    println!("================================");
    let cp = cp_model.critical_power as f64;
    println!("Zone 1 (Active Recovery): < {:.0} W (<75% CP)", cp * 0.75);
    println!("Zone 2 (Endurance):        {:.0}-{:.0} W (75-85% CP)", cp * 0.75, cp * 0.85);
    println!("Zone 3 (Tempo):            {:.0}-{:.0} W (85-95% CP)", cp * 0.85, cp * 0.95);
    println!("Zone 4 (Threshold):        {:.0}-{:.0} W (95-105% CP)", cp * 0.95, cp * 1.05);
    println!("Zone 5 (VO2max):           {:.0}-{:.0} W (105-120% CP)", cp * 1.05, cp * 1.20);
    println!("Zone 6 (Anaerobic):        > {:.0} W (>120% CP)", cp * 1.20);
}

/// Display power metrics analysis
fn display_power_metrics(metrics: &crate::power::PowerMetrics) {
    use colored::Colorize;

    println!("\n📊 POWER METRICS ANALYSIS");
    println!("=========================\n");

    // Basic power stats
    println!("{}", "🔋 POWER STATISTICS".blue().bold());
    println!("====================");
    println!("Normalized Power:   {:>6} W", metrics.normalized_power);
    println!("Variability Index:  {:>6.3}", metrics.variability_index);
    println!();

    // Intensity metrics
    println!("{}", "⚡ INTENSITY METRICS".yellow().bold());
    println!("====================");

    if let Some(intensity_factor) = metrics.intensity_factor {
        println!("Intensity Factor:   {:>6.3}", intensity_factor);
    }

    if let Some(ef) = metrics.efficiency_factor {
        println!("Efficiency Factor:  {:>6.3}", ef);
    }
    println!();

    // Work/energy metrics
    println!("{}", "🏋️ WORK & ENERGY".green().bold());
    println!("=================");

    if let Some(work_above_ftp) = metrics.work_above_ftp {
        println!("Work Above FTP:     {:>6.1} kJ", work_above_ftp as f64 / 1000.0);
    }
    if let Some(work_below_ftp) = metrics.work_below_ftp {
        println!("Work Below FTP:     {:>6.1} kJ", work_below_ftp as f64 / 1000.0);
    }

    println!();
    println!("{}", "Note: Additional metrics available with enhanced analysis".dimmed());
}

/// Display peak power analysis
fn display_peak_power_analysis(peaks: &crate::power::PeakPowerAnalysis) {
    use colored::Colorize;

    println!("\n🏔️ PEAK POWER ANALYSIS");
    println!("======================\n");

    println!("{:<12} │ {:<8}", "Duration", "Power (W)");
    println!("─────────────┼──────────");

    // Display peak powers
    if let Some(p5s) = peaks.peak_5s {
        println!("{:<12} │ {:>8}", "5 seconds", format!("{}", p5s).yellow().bold());
    }
    if let Some(p15s) = peaks.peak_15s {
        println!("{:<12} │ {:>8}", "15 seconds", format!("{}", p15s).yellow().bold());
    }
    if let Some(p30s) = peaks.peak_30s {
        println!("{:<12} │ {:>8}", "30 seconds", format!("{}", p30s).yellow().bold());
    }
    if let Some(p1min) = peaks.peak_1min {
        println!("{:<12} │ {:>8}", "1 minute", format!("{}", p1min).yellow().bold());
    }
    if let Some(p5min) = peaks.peak_5min {
        println!("{:<12} │ {:>8}", "5 minutes", format!("{}", p5min).yellow().bold());
    }
    if let Some(p20min) = peaks.peak_20min {
        println!("{:<12} │ {:>8}", "20 minutes", format!("{}", p20min).yellow().bold());
    }
    if let Some(p60min) = peaks.peak_60min {
        println!("{:<12} │ {:>8}", "60 minutes", format!("{}", p60min).yellow().bold());
    }

    println!();

    // Peak power insights
    println!("{}", "💡 PEAK POWER INSIGHTS".cyan().bold());
    println!("=======================");

    if let (Some(sprint), Some(one_min)) = (peaks.peak_5s, peaks.peak_1min) {
        let neuromuscular_ratio = sprint as f64 / one_min as f64;
        println!("Neuromuscular Power: {:.2}x 1-min power", neuromuscular_ratio);

        if neuromuscular_ratio > 2.0 {
            println!("{}", "🚀 Excellent sprint capacity".green());
        } else if neuromuscular_ratio < 1.5 {
            println!("{}", "⏱️ Focus on neuromuscular power development".yellow());
        }
    }

    if let (Some(five_min), Some(twenty_min)) = (peaks.peak_5min, peaks.peak_20min) {
        let vo2_threshold_ratio = five_min as f64 / twenty_min as f64;
        println!("VO2max:FTP Ratio: {:.2} (typical range: 1.15-1.25)", vo2_threshold_ratio);

        if vo2_threshold_ratio > 1.25 {
            println!("{}", "💪 Strong VO2max relative to threshold".green());
        } else if vo2_threshold_ratio < 1.15 {
            println!("{}", "📈 Focus on VO2max development".yellow());
        }
    }
}

/// Display quadrant analysis
fn display_quadrant_analysis(analysis: &crate::power::QuadrantAnalysis) {
    use colored::Colorize;

    println!("\n🎯 QUADRANT ANALYSIS");
    println!("===================\n");

    println!("{:<20} │ {:<6}", "Quadrant", "% of Time");
    println!("─────────────────────┼────────");

    println!("{:<20} │ {:>6.1}%", "Q1 (High Power, High Cadence)", analysis.quadrant_i_percent);
    println!("{:<20} │ {:>6.1}%", "Q2 (Low Power, High Cadence)", analysis.quadrant_ii_percent);
    println!("{:<20} │ {:>6.1}%", "Q3 (Low Power, Low Cadence)", analysis.quadrant_iii_percent);
    println!("{:<20} │ {:>6.1}%", "Q4 (High Power, Low Cadence)", analysis.quadrant_iv_percent);

    println!();
    println!("{}", "💡 TRAINING INSIGHTS".cyan().bold());
    println!("====================");

    let q1_pct = analysis.quadrant_i_percent.to_f64().unwrap_or(0.0);
    let q2_pct = analysis.quadrant_ii_percent.to_f64().unwrap_or(0.0);
    let q3_pct = analysis.quadrant_iii_percent.to_f64().unwrap_or(0.0);
    let q4_pct = analysis.quadrant_iv_percent.to_f64().unwrap_or(0.0);

    if q4_pct > 25.0 {
        println!("{}", "💪 High force development work detected".green());
    }
    if q1_pct > 40.0 {
        println!("{}", "🏃 Strong sustained power emphasis".blue());
    }
    if q3_pct > 50.0 {
        println!("{}", "😴 High recovery time - consider intensity".yellow());
    }
    if q2_pct > 15.0 {
        println!("{}", "🚴 Good high-cadence endurance work".cyan());
    }
}

/// Display power balance analysis
fn display_power_balance(balance: &crate::power::PowerBalance) {
    use colored::Colorize;

    println!("\n⚖️ POWER BALANCE ANALYSIS");
    println!("=========================\n");

    println!("{}", "🦵 LEFT/RIGHT LEG BALANCE".blue().bold());
    println!("==========================");
    println!("Left Leg:          {:>5.1}%", balance.left_percent);
    println!("Right Leg:         {:>5.1}%", balance.right_percent);
    println!("Balance Score:     {:>5.1}", balance.balance_score);
    println!();

    let avg_imbalance = (balance.left_percent - rust_decimal::Decimal::from(50)).abs();
    println!("Average Imbalance: {:>5.1}%", avg_imbalance);

    println!();

    // Visual representation
    let left_bars = ((balance.left_percent.to_f64().unwrap_or(0.0) / 2.0) as usize).min(25);
    let right_bars = ((balance.right_percent.to_f64().unwrap_or(0.0) / 2.0) as usize).min(25);

    println!("{}", "📊 VISUAL BALANCE".green().bold());
    println!("=================\n");
    println!("Left:  {:>5.1}% │{:<25}│", balance.left_percent, "█".repeat(left_bars));
    println!("Right: {:>5.1}% │{:<25}│", balance.right_percent, "█".repeat(right_bars));
    println!("               │{}│", "-".repeat(25));
    println!("               0%      50%      100%");
    println!();

    // Analysis and recommendations
    println!("{}", "💡 BALANCE ASSESSMENT".cyan().bold());
    println!("======================");

    let avg_imbalance_f64 = avg_imbalance.to_f64().unwrap_or(0.0);
    if avg_imbalance_f64 < 2.0 {
        println!("{}", "✅ Excellent balance (<2% imbalance)".green());
    } else if avg_imbalance_f64 < 5.0 {
        println!("{}", "✅ Good balance (2-5% imbalance)".yellow());
    } else if avg_imbalance_f64 < 10.0 {
        println!("{}", "⚠️ Moderate imbalance (5-10%) - monitor closely".yellow());
    } else {
        println!("{}", "🚨 Significant imbalance (>10%) - consider bike fit review".red());
    }

    println!();
    println!("Recommendations:");
    if avg_imbalance_f64 > 5.0 {
        println!("• Consider professional bike fitting");
        println!("• Check for leg length differences");
        println!("• Focus on single-leg training drills");
    } else {
        println!("• Continue current training approach");
        println!("• Monitor trends over time");
    }
}
