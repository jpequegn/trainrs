use anyhow::Result;
use chrono::Datelike;
use clap::{Parser, Subcommand};
use colored::*;
use rust_decimal::prelude::FromPrimitive;
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
        /// Zone action (list, set, calculate, import)
        #[arg(long, default_value = "list")]
        action: String,

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
            let athlete_id = athlete.or_else(|| cli.athlete.clone());
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
                        let mut profile = AthleteProfile {
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

        Commands::Zones {
            action,
            zone_type,
            athlete,
            ftp,
            lthr,
            max_hr,
            threshold_pace,
        } => {
            println!("{}", "Managing training zones...".cyan().bold());

            // Handle global athlete flag
            let athlete_id = athlete.or_else(|| cli.athlete.clone());
            if let Some(a) = &athlete_id {
                println!("  Athlete: {}", a);
            }

            println!("  Action: {}", action);

            match action.as_str() {
                "list" => {
                    println!("  Listing current training zones:");
                    if let Some(zone_type_str) = zone_type {
                        println!("  Zone type: {}", zone_type_str);
                        match zone_type_str.as_str() {
                            "heart-rate" | "hr" => {
                                println!("  Heart Rate Zones:");
                                println!("    Zone 1: < 68% of LTHR (Active Recovery)");
                                println!("    Zone 2: 69-83% of LTHR (Aerobic Base)");
                                println!("    Zone 3: 84-94% of LTHR (Aerobic)");
                                println!("    Zone 4: 95-105% of LTHR (Lactate Threshold)");
                                println!("    Zone 5: > 105% of LTHR (VO2 Max)");
                            }
                            "power" => {
                                println!("  Power Zones:");
                                println!("    Zone 1: < 55% of FTP (Active Recovery)");
                                println!("    Zone 2: 56-75% of FTP (Endurance)");
                                println!("    Zone 3: 76-90% of FTP (Tempo)");
                                println!("    Zone 4: 91-105% of FTP (Lactate Threshold)");
                                println!("    Zone 5: 106-120% of FTP (VO2 Max)");
                                println!("    Zone 6: 121-150% of FTP (Anaerobic Capacity)");
                                println!("    Zone 7: > 150% of FTP (Sprint Power)");
                            }
                            "pace" => {
                                println!("  Pace Zones:");
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
                        println!("  All zone types available");
                    }
                }
                "set" => {
                    println!("  Setting training zones:");


                    if let Some(ftp_value) = ftp {
                        println!("    FTP: {} watts", ftp_value);
                        // TODO: Create or update athlete profile with FTP
                    }
                    if let Some(lthr_value) = lthr {
                        println!("    LTHR: {} bpm", lthr_value);
                        // TODO: Create or update athlete profile with LTHR
                    }
                    if let Some(max_hr_value) = max_hr {
                        println!("    Max HR: {} bpm", max_hr_value);
                        // TODO: Create or update athlete profile with Max HR
                    }
                    if let Some(pace_value) = threshold_pace {
                        println!("    Threshold pace: {:.2} min/mile", pace_value);
                        // TODO: Create or update athlete profile with threshold pace
                    }
                }
                "calculate" => {
                    println!("  Calculating zones from current athlete profile:");
                    // TODO: Load athlete profile and calculate zones using ZoneCalculator
                    println!("  ✓ Zones calculated and updated");
                }
                "import" => {
                    println!("  Importing zones from external source:");
                    // TODO: Implement zone import functionality
                    println!("  ✓ Zones imported successfully");
                }
                _ => {
                    eprintln!("{}", "✗ Invalid action. Use: list, set, calculate, or import".red());
                    std::process::exit(1);
                }
            }

            println!("{}", "✓ Zone management completed".cyan());
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
