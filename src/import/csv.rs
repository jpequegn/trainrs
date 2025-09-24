use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, Utc};
use csv::ReaderBuilder;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use std::path::Path;

use crate::import::{validation::WorkoutValidator, ImportFormat};
use crate::models::{DataPoint, DataSource, Sport, Workout, WorkoutSummary, WorkoutType};

/// CSV importer with flexible column mapping
pub struct CsvImporter {
    column_mapping: HashMap<String, String>,
}

impl CsvImporter {
    pub fn new() -> Self {
        let mut column_mapping = HashMap::new();

        // Common column name variations
        Self::add_mapping(
            &mut column_mapping,
            "timestamp",
            &["timestamp", "time", "elapsed_time", "elapsed", "duration"],
        );
        Self::add_mapping(
            &mut column_mapping,
            "heart_rate",
            &["heart_rate", "hr", "heartrate", "bpm"],
        );
        Self::add_mapping(
            &mut column_mapping,
            "power",
            &["power", "watts", "power_watts"],
        );
        Self::add_mapping(
            &mut column_mapping,
            "pace",
            &["pace", "min_per_km", "pace_min_km"],
        );
        Self::add_mapping(
            &mut column_mapping,
            "speed",
            &["speed", "velocity", "speed_ms", "speed_kmh"],
        );
        Self::add_mapping(
            &mut column_mapping,
            "elevation",
            &["elevation", "altitude", "alt", "elev", "height"],
        );
        Self::add_mapping(
            &mut column_mapping,
            "cadence",
            &["cadence", "rpm", "steps_per_minute", "spm"],
        );
        Self::add_mapping(
            &mut column_mapping,
            "distance",
            &["distance", "dist", "total_distance", "cumulative_distance"],
        );
        Self::add_mapping(
            &mut column_mapping,
            "latitude",
            &["latitude", "lat", "position_lat"],
        );
        Self::add_mapping(
            &mut column_mapping,
            "longitude",
            &["longitude", "lng", "lon", "position_long"],
        );

        Self { column_mapping }
    }

    fn add_mapping(mapping: &mut HashMap<String, String>, standard: &str, variations: &[&str]) {
        for variation in variations {
            mapping.insert(variation.to_lowercase(), standard.to_string());
        }
    }

    fn parse_datetime(date_str: &str) -> Result<DateTime<Utc>> {
        // Try different datetime formats
        let formats = [
            "%Y-%m-%d %H:%M:%S",
            "%Y-%m-%dT%H:%M:%S",
            "%Y-%m-%d %H:%M:%S%.f",
            "%Y-%m-%dT%H:%M:%S%.f",
            "%Y-%m-%dT%H:%M:%SZ",
            "%Y-%m-%dT%H:%M:%S%.fZ",
            "%d/%m/%Y %H:%M:%S",
            "%m/%d/%Y %H:%M:%S",
        ];

        for format in &formats {
            if let Ok(naive_dt) = NaiveDateTime::parse_from_str(date_str, format) {
                return Ok(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
            }
        }

        // Try parsing as timestamp (seconds since epoch)
        if let Ok(timestamp) = date_str.parse::<i64>() {
            if let Some(dt) = DateTime::from_timestamp(timestamp, 0) {
                return Ok(dt);
            }
        }

        anyhow::bail!("Unable to parse datetime: {}", date_str);
    }

    fn normalize_column_name(&self, name: &str) -> String {
        let normalized = name.to_lowercase().replace([' ', '-'], "_");

        self.column_mapping
            .get(&normalized)
            .cloned()
            .unwrap_or(normalized)
    }

    #[allow(dead_code)]
    fn parse_sport(sport_str: &str) -> Sport {
        let sport_lower = sport_str.to_lowercase();
        match sport_lower.as_str() {
            "running" | "run" | "jog" | "jogging" => Sport::Running,
            "cycling" | "bike" | "biking" | "cycle" => Sport::Cycling,
            "swimming" | "swim" => Sport::Swimming,
            "triathlon" | "tri" => Sport::Triathlon,
            "rowing" | "row" => Sport::Rowing,
            _ => Sport::CrossTraining,
        }
    }

    fn create_workout_from_csv(
        &self,
        file_path: &Path,
        data_points: Vec<DataPoint>,
        start_time: DateTime<Utc>,
    ) -> Result<Workout> {
        if data_points.is_empty() {
            anyhow::bail!("No valid data points found in CSV file");
        }

        // Calculate basic summary statistics
        let duration_seconds =
            if let (Some(first), Some(last)) = (data_points.first(), data_points.last()) {
                last.timestamp.saturating_sub(first.timestamp)
            } else {
                data_points.len() as u32 // Fallback: assume 1 second per point
            };

        let total_distance = data_points.last().and_then(|dp| dp.distance);

        let avg_heart_rate = if !data_points.is_empty() {
            let hr_sum: u32 = data_points
                .iter()
                .filter_map(|dp| dp.heart_rate)
                .map(|hr| hr as u32)
                .sum();
            let hr_count = data_points.iter().filter_map(|dp| dp.heart_rate).count();

            if hr_count > 0 {
                Some((hr_sum / hr_count as u32) as u16)
            } else {
                None
            }
        } else {
            None
        };

        let avg_power = if !data_points.is_empty() {
            let power_sum: u32 = data_points
                .iter()
                .filter_map(|dp| dp.power)
                .map(|p| p as u32)
                .sum();
            let power_count = data_points.iter().filter_map(|dp| dp.power).count();

            if power_count > 0 {
                Some((power_sum / power_count as u32) as u16)
            } else {
                None
            }
        } else {
            None
        };

        let elevation_gain = Self::calculate_elevation_gain(&data_points);

        let summary = WorkoutSummary {
            avg_heart_rate,
            max_heart_rate: data_points.iter().filter_map(|dp| dp.heart_rate).max(),
            avg_power,
            normalized_power: None, // Would require complex calculation
            avg_pace: Self::calculate_avg_pace(&data_points),
            intensity_factor: None, // Would require FTP
            tss: None,              // Would require NP and IF
            total_distance,
            elevation_gain,
            avg_cadence: if !data_points.is_empty() {
                let cadence_sum: u32 = data_points
                    .iter()
                    .filter_map(|dp| dp.cadence)
                    .map(|c| c as u32)
                    .sum();
                let cadence_count = data_points.iter().filter_map(|dp| dp.cadence).count();

                if cadence_count > 0 {
                    Some((cadence_sum / cadence_count as u32) as u16)
                } else {
                    None
                }
            } else {
                None
            },
            calories: None, // Not typically in raw data
        };

        let workout = Workout {
            id: format!("csv_import_{}", chrono::Utc::now().timestamp()),
            date: start_time.date_naive(),
            sport: Sport::CrossTraining, // Default, can be overridden
            duration_seconds,
            workout_type: WorkoutType::Endurance, // Default
            data_source: DataSource::HeartRate,   // Default assumption
            raw_data: Some(data_points),
            summary,
            notes: Some(format!(
                "Imported from CSV: {}",
                file_path.file_name().unwrap_or_default().to_string_lossy()
            )),
            athlete_id: None,
            source: Some(file_path.to_string_lossy().to_string()),
        };

        Ok(workout)
    }

    fn calculate_elevation_gain(data_points: &[DataPoint]) -> Option<u16> {
        let elevations: Vec<i16> = data_points.iter().filter_map(|dp| dp.elevation).collect();

        if elevations.len() < 2 {
            return None;
        }

        let mut gain = 0i32;
        for i in 1..elevations.len() {
            let diff = elevations[i] - elevations[i - 1];
            if diff > 0 {
                gain += diff as i32;
            }
        }

        Some(gain.max(0) as u16)
    }

    fn calculate_avg_pace(data_points: &[DataPoint]) -> Option<Decimal> {
        let paces: Vec<Decimal> = data_points.iter().filter_map(|dp| dp.pace).collect();

        if paces.is_empty() {
            None
        } else {
            let sum = paces.iter().sum::<Decimal>();
            Some(sum / Decimal::from(paces.len()))
        }
    }
}

impl ImportFormat for CsvImporter {
    fn can_import(&self, file_path: &Path) -> bool {
        file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase() == "csv")
            .unwrap_or(false)
    }

    fn import_file(&self, file_path: &Path) -> Result<Vec<Workout>> {
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .from_path(file_path)?;

        let headers = reader.headers()?.clone();
        let mut data_points = Vec::new();

        // Map headers to standard names
        let header_mapping: HashMap<usize, String> = headers
            .iter()
            .enumerate()
            .map(|(i, header)| (i, self.normalize_column_name(header)))
            .collect();

        // Track workout start time
        let mut workout_start_time = None;

        let mut current_timestamp = 0u32;

        for result in reader.records() {
            let record = result?;
            let mut data_point = DataPoint {
                timestamp: current_timestamp,
                heart_rate: None,
                power: None,
                pace: None,
                elevation: None,
                cadence: None,
                speed: None,
                distance: None,
            };

            // Parse each field based on column mapping
            for (i, value) in record.iter().enumerate() {
                if value.trim().is_empty() {
                    continue;
                }

                if let Some(column_name) = header_mapping.get(&i) {
                    match column_name.as_str() {
                        "timestamp" => {
                            match Self::parse_datetime(value) {
                                Ok(dt) => {
                                    if workout_start_time.is_none() {
                                        workout_start_time = Some(dt);
                                        data_point.timestamp = 0;
                                    } else if let Some(start) = workout_start_time {
                                        data_point.timestamp =
                                            (dt - start).num_seconds().max(0) as u32;
                                    }
                                }
                                Err(_) => {
                                    // Try parsing as elapsed seconds
                                    if let Ok(elapsed) = value.parse::<f64>() {
                                        data_point.timestamp = elapsed as u32;
                                        if workout_start_time.is_none() {
                                            workout_start_time = Some(Utc::now());
                                        }
                                    }
                                }
                            }
                        }
                        "heart_rate" => {
                            if let Ok(hr) = value.parse::<u16>() {
                                data_point.heart_rate = Some(hr);
                            }
                        }
                        "power" => {
                            if let Ok(power) = value.parse::<u16>() {
                                data_point.power = Some(power);
                            }
                        }
                        "pace" => {
                            if let Ok(pace) = value.parse::<f64>() {
                                data_point.pace =
                                    Some(Decimal::try_from(pace).unwrap_or(dec!(0.0)));
                            }
                        }
                        "speed" => {
                            if let Ok(speed) = value.parse::<f64>() {
                                data_point.speed =
                                    Some(Decimal::try_from(speed).unwrap_or(dec!(0.0)));
                            }
                        }
                        "elevation" => {
                            if let Ok(elevation) = value.parse::<i16>() {
                                data_point.elevation = Some(elevation);
                            }
                        }
                        "cadence" => {
                            if let Ok(cadence) = value.parse::<u16>() {
                                data_point.cadence = Some(cadence);
                            }
                        }
                        "distance" => {
                            if let Ok(distance) = value.parse::<f64>() {
                                data_point.distance =
                                    Some(Decimal::try_from(distance).unwrap_or(dec!(0.0)));
                            }
                        }
                        _ => {} // Ignore unknown columns
                    }
                }
            }

            data_points.push(data_point);
            current_timestamp += 1; // Default increment if no timestamp
        }

        if data_points.is_empty() {
            anyhow::bail!("No valid data points found in CSV file");
        }

        // Set default start time if none found
        let start_time = workout_start_time.unwrap_or_else(Utc::now);

        // Create workout from data points
        let mut workout = self.create_workout_from_csv(file_path, data_points, start_time)?;

        // Validate and clean the workout data
        WorkoutValidator::validate_workout(&mut workout)?;

        Ok(vec![workout])
    }

    fn get_format_name(&self) -> &'static str {
        "CSV"
    }
}
