use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use fitparser::{FitDataRecord, Value};
use rust_decimal::Decimal;
use std::fs::File;
use std::path::Path;
use uuid::Uuid;

use crate::import::{validation::WorkoutValidator, ImportFormat};
use crate::models::{DataPoint, DataSource, Sport, Workout, WorkoutSummary, WorkoutType};

/// FIT file importer for Garmin native format
/// Supports parsing of FIT files from Garmin devices with focus on cycling power data
pub struct FitImporter;

impl FitImporter {
    pub fn new() -> Self {
        Self
    }

    /// Parse session information from FIT file
    fn parse_session_info(&self, records: &[FitDataRecord]) -> Result<(Sport, WorkoutType, DateTime<Utc>, u32)> {
        for record in records {
            if record.kind() == fitparser::profile::MesgNum::Session {
                let mut sport = Sport::Cycling;
                let mut workout_type = WorkoutType::Endurance;
                let mut start_time = Utc::now();
                let mut duration = 0u32;

                for field in record.fields() {
                    match field.name() {
                        "sport" => {
                            if let Value::Enum(sport_val) = field.value() {
                                sport = match sport_val {
                                    1 => Sport::Running,
                                    2 => Sport::Cycling,
                                    5 => Sport::Swimming,
                                    _ => Sport::Cycling,
                                };
                            }
                        }
                        "start_time" => {
                            if let Value::Timestamp(dt) = field.value() {
                                start_time = dt.with_timezone(&Utc);
                            }
                        }
                        "total_timer_time" => {
                            if let Value::Float64(dur) = field.value() {
                                duration = (*dur * 1000.0) as u32 / 1000; // Convert from seconds
                            } else if let Value::UInt32(dur) = field.value() {
                                duration = *dur / 1000; // Convert from milliseconds
                            }
                        }
                        "sub_sport" => {
                            if let Value::Enum(sub_sport_val) = field.value() {
                                workout_type = match sub_sport_val {
                                    1 => WorkoutType::Interval, // Track
                                    11 => WorkoutType::Endurance, // Road
                                    12 => WorkoutType::Endurance, // Mountain
                                    _ => WorkoutType::Endurance,
                                };
                            }
                        }
                        _ => {}
                    }
                }

                return Ok((sport, workout_type, start_time, duration));
            }
        }

        // Fallback if no session record found
        Ok((Sport::Cycling, WorkoutType::Endurance, Utc::now(), 0))
    }

    /// Parse data points from FIT records focusing on cycling power data
    fn parse_data_points(&self, records: &[FitDataRecord], start_time: DateTime<Utc>) -> Result<Vec<DataPoint>> {
        let mut data_points = Vec::new();
        let mut timestamp_offset = 0u32;

        for record in records {
            if record.kind() == fitparser::profile::MesgNum::Record {
                let mut data_point = DataPoint {
                    timestamp: timestamp_offset,
                    heart_rate: None,
                    power: None,
                    pace: None,
                    elevation: None,
                    cadence: None,
                    speed: None,
                    distance: None,
                    left_power: None,
                    right_power: None,
                };

                for field in record.fields() {
                    match field.name() {
                        "timestamp" => {
                            if let Value::Timestamp(record_time) = field.value() {
                                let record_time_utc = record_time.with_timezone(&Utc);
                                timestamp_offset = (record_time_utc - start_time).num_seconds().max(0) as u32;
                                data_point.timestamp = timestamp_offset;
                            }
                        }
                        "heart_rate" => {
                            if let Value::UInt8(hr) = field.value() {
                                data_point.heart_rate = Some(*hr as u16);
                            }
                        }
                        "power" => {
                            if let Value::UInt16(power) = field.value() {
                                data_point.power = Some(*power);
                            }
                        }
                        "altitude" | "enhanced_altitude" => {
                            if let Value::Float64(alt) = field.value() {
                                data_point.elevation = Some(*alt as i16);
                            } else if let Value::UInt16(alt) = field.value() {
                                data_point.elevation = Some(*alt as i16);
                            }
                        }
                        "cadence" => {
                            if let Value::UInt8(cad) = field.value() {
                                data_point.cadence = Some(*cad as u16);
                            }
                        }
                        "speed" | "enhanced_speed" => {
                            if let Value::Float64(speed) = field.value() {
                                data_point.speed = Some(Decimal::from_f64_retain(*speed).unwrap_or_default());
                            }
                        }
                        "distance" => {
                            if let Value::Float64(dist) = field.value() {
                                data_point.distance = Some(Decimal::from_f64_retain(*dist).unwrap_or_default());
                            }
                        }
                        _ => {}
                    }
                }

                data_points.push(data_point);
            }
        }

        Ok(data_points)
    }

    /// Calculate workout summary from data points
    fn calculate_summary(&self, data_points: &[DataPoint], _sport: &Sport) -> WorkoutSummary {
        if data_points.is_empty() {
            return WorkoutSummary {
                avg_heart_rate: None,
                max_heart_rate: None,
                avg_power: None,
                normalized_power: None,
                avg_pace: None,
                intensity_factor: None,
                tss: None,
                total_distance: None,
                elevation_gain: None,
                avg_cadence: None,
                calories: None,
            };
        }

        // Calculate heart rate metrics
        let heart_rates: Vec<u16> = data_points.iter().filter_map(|dp| dp.heart_rate).collect();
        let avg_heart_rate = if !heart_rates.is_empty() {
            Some(heart_rates.iter().sum::<u16>() / heart_rates.len() as u16)
        } else {
            None
        };
        let max_heart_rate = heart_rates.iter().max().copied();

        // Calculate power metrics (for cycling)
        let powers: Vec<u16> = data_points.iter().filter_map(|dp| dp.power).collect();
        let avg_power = if !powers.is_empty() {
            Some(powers.iter().sum::<u16>() / powers.len() as u16)
        } else {
            None
        };

        // Calculate cadence
        let cadences: Vec<u16> = data_points.iter().filter_map(|dp| dp.cadence).collect();
        let avg_cadence = if !cadences.is_empty() {
            Some(cadences.iter().sum::<u16>() / cadences.len() as u16)
        } else {
            None
        };

        // Calculate distance (take the last distance value as total)
        let total_distance = data_points.iter().filter_map(|dp| dp.distance).last();

        // Calculate elevation gain
        let elevations: Vec<i16> = data_points.iter().filter_map(|dp| dp.elevation).collect();
        let elevation_gain = if elevations.len() > 1 {
            let mut gain = 0i32;
            for window in elevations.windows(2) {
                let diff = window[1] as i32 - window[0] as i32;
                if diff > 0 {
                    gain += diff;
                }
            }
            Some(gain as u16)
        } else {
            None
        };

        WorkoutSummary {
            avg_heart_rate,
            max_heart_rate,
            avg_power,
            normalized_power: None, // TODO: Calculate normalized power in future phase
            avg_pace: None, // TODO: Calculate from speed data
            intensity_factor: None,
            tss: None, // Will be calculated by TSS calculator after import
            total_distance,
            elevation_gain,
            avg_cadence,
            calories: None, // TODO: Extract from session data if available
        }
    }

    /// Determine primary data source based on available data
    fn determine_data_source(&self, data_points: &[DataPoint], sport: &Sport) -> DataSource {
        let has_power = data_points.iter().any(|dp| dp.power.is_some());
        let has_heart_rate = data_points.iter().any(|dp| dp.heart_rate.is_some());
        let has_speed = data_points.iter().any(|dp| dp.speed.is_some());

        match sport {
            Sport::Cycling => {
                if has_power {
                    DataSource::Power
                } else if has_heart_rate {
                    DataSource::HeartRate
                } else {
                    DataSource::HeartRate // Default for cycling
                }
            }
            Sport::Running => {
                if has_speed {
                    DataSource::Pace
                } else if has_heart_rate {
                    DataSource::HeartRate
                } else {
                    DataSource::HeartRate // Default for running
                }
            }
            _ => DataSource::HeartRate, // Default for other sports
        }
    }
}

impl ImportFormat for FitImporter {
    fn can_import(&self, file_path: &Path) -> bool {
        file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase() == "fit")
            .unwrap_or(false)
    }

    fn import_file(&self, file_path: &Path) -> Result<Vec<Workout>> {
        // Parse the FIT file
        let mut file = File::open(file_path)
            .with_context(|| format!("Failed to open FIT file: {}", file_path.display()))?;

        let records: Vec<FitDataRecord> = fitparser::from_reader(&mut file)
            .map_err(|e| anyhow::anyhow!("Failed to parse FIT file records: {:?}", e))?;

        if records.is_empty() {
            anyhow::bail!("FIT file contains no data records");
        }

        // Parse session information
        let (sport, workout_type, start_time, duration) = self.parse_session_info(&records)
            .with_context(|| "Failed to parse session information from FIT file")?;

        // Parse data points
        let data_points = self.parse_data_points(&records, start_time)
            .with_context(|| "Failed to parse data points from FIT file")?;

        if data_points.is_empty() {
            anyhow::bail!("FIT file contains no usable data points");
        }

        // Calculate summary metrics
        let summary = self.calculate_summary(&data_points, &sport);

        // Determine primary data source
        let data_source = self.determine_data_source(&data_points, &sport);

        // Create workout object
        let workout = Workout {
            id: Uuid::new_v4().to_string(),
            date: start_time.date_naive(),
            sport,
            duration_seconds: duration,
            workout_type,
            data_source,
            raw_data: Some(data_points),
            summary,
            notes: Some(format!("Imported from FIT file: {}", file_path.display())),
            athlete_id: None, // TODO: Extract from FIT file if available
            source: Some(file_path.to_string_lossy().to_string()),
        };

        // Validate the workout
        let mut workout = workout;
        WorkoutValidator::validate_workout(&mut workout)
            .with_context(|| "Workout validation failed")?;

        Ok(vec![workout])
    }

    fn get_format_name(&self) -> &'static str {
        "FIT"
    }
}
