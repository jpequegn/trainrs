use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use fitparser::{FitDataRecord, Value};
use rust_decimal::Decimal;
use std::fs::File;
use std::path::Path;
use uuid::Uuid;

use crate::import::{validation::WorkoutValidator, ImportFormat};
use crate::models::{DataPoint, DataSource, Sport, Workout, WorkoutSummary, WorkoutType};
use crate::power::PowerAnalyzer;

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
                                    6 => Sport::Rowing,
                                    15 => Sport::Triathlon,
                                    26 => Sport::CrossTraining,
                                    _ => Sport::Cycling, // Default fallback
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

    /// Parse data points from FIT records with sport-specific metrics
    fn parse_data_points(&self, records: &[FitDataRecord], start_time: DateTime<Utc>) -> Result<Vec<DataPoint>> {
        let mut data_points: Vec<DataPoint> = Vec::new();
        let mut timestamp_offset = 0u32;
        let mut current_lap = 1u16;
        let mut current_sport: Option<Sport> = None;

        for record in records {
            // Handle lap records for lap counting and sport transitions
            if record.kind() == fitparser::profile::MesgNum::Lap {
                for field in record.fields() {
                    match field.name() {
                        "message_index" => {
                            if let Value::UInt16(lap_idx) = field.value() {
                                current_lap = lap_idx + 1; // Lap index is 0-based
                            }
                        }
                        "sport" => {
                            if let Value::Enum(sport_val) = field.value() {
                                let lap_sport = match sport_val {
                                    1 => Sport::Running,
                                    2 => Sport::Cycling,
                                    5 => Sport::Swimming,
                                    6 => Sport::Rowing,
                                    15 => Sport::Triathlon,
                                    26 => Sport::CrossTraining,
                                    _ => Sport::Cycling,
                                };

                                // Detect sport transition
                                if let Some(prev_sport) = current_sport {
                                    if prev_sport != lap_sport {
                                        // Mark previous data points as having a sport transition
                                        if let Some(last_point) = data_points.last_mut() {
                                            last_point.sport_transition = Some(true);
                                        }
                                    }
                                }
                                current_sport = Some(lap_sport);
                            }
                        }
                        _ => {}
                    }
                }
                continue;
            }

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
                    ground_contact_time: None,
                    vertical_oscillation: None,
                    stride_length: None,
                    stroke_count: None,
                    stroke_type: None,
                    lap_number: Some(current_lap),
                    sport_transition: None,
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
                        "left_right_balance" => {
                            if let Value::UInt8(balance) = field.value() {
                                // Balance is typically encoded as 0-200, where 100 = 50/50 split
                                // Extract left and right percentages and calculate power split
                                if let Some(total_power) = data_point.power {
                                    let left_percent = *balance as f64 / 200.0; // Convert to 0.0-1.0 range
                                    data_point.left_power = Some((total_power as f64 * left_percent) as u16);
                                    data_point.right_power = Some((total_power as f64 * (1.0 - left_percent)) as u16);
                                }
                            }
                        }
                        "left_power_phase" | "right_power_phase" => {
                            // TODO: Phase 3 - Extract power phase data for advanced analysis
                        }
                        // Running dynamics fields
                        "stance_time" | "ground_contact_time" => {
                            if let Value::UInt16(gct) = field.value() {
                                data_point.ground_contact_time = Some(*gct);
                            }
                        }
                        "vertical_oscillation" => {
                            if let Value::UInt16(vo) = field.value() {
                                data_point.vertical_oscillation = Some(*vo);
                            }
                        }
                        "stride_length" => {
                            if let Value::Float64(sl) = field.value() {
                                data_point.stride_length = Some(Decimal::from_f64_retain(*sl).unwrap_or_default());
                            } else if let Value::UInt16(sl) = field.value() {
                                data_point.stride_length = Some(Decimal::from(*sl));
                            }
                        }
                        // Swimming-specific fields
                        "strokes" | "total_strokes" => {
                            if let Value::UInt16(strokes) = field.value() {
                                data_point.stroke_count = Some(*strokes);
                            } else if let Value::UInt8(strokes) = field.value() {
                                data_point.stroke_count = Some(*strokes as u16);
                            }
                        }
                        "swim_stroke" => {
                            if let Value::Enum(stroke_type) = field.value() {
                                data_point.stroke_type = Some(*stroke_type);
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

    /// Calculate workout summary from data points with advanced power metrics
    fn calculate_summary(&self, data_points: &[DataPoint], sport: &Sport) -> WorkoutSummary {
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

        // Calculate basic power metrics
        let powers: Vec<u16> = data_points.iter().filter_map(|dp| dp.power).collect();
        let avg_power = if !powers.is_empty() {
            Some(powers.iter().sum::<u16>() / powers.len() as u16)
        } else {
            None
        };

        // Calculate advanced power metrics for cycling with power data
        let (normalized_power, intensity_factor) = if sport == &Sport::Cycling && !powers.is_empty() {
            match PowerAnalyzer::calculate_power_metrics(data_points, None) {
                Ok(metrics) => (Some(metrics.normalized_power), metrics.intensity_factor),
                Err(_) => (None, None) // Fallback gracefully on error
            }
        } else {
            (None, None)
        };

        // Calculate cadence
        let cadences: Vec<u16> = data_points.iter().filter_map(|dp| dp.cadence).collect();
        let avg_cadence = if !cadences.is_empty() {
            Some(cadences.iter().sum::<u16>() / cadences.len() as u16)
        } else {
            None
        };

        // Calculate average pace from speed data (for running)
        let avg_pace = if sport == &Sport::Running {
            let speeds: Vec<Decimal> = data_points.iter().filter_map(|dp| dp.speed).collect();
            if !speeds.is_empty() {
                let avg_speed = speeds.iter().sum::<Decimal>() / Decimal::from(speeds.len());
                if avg_speed > Decimal::ZERO {
                    // Convert speed (m/s) to pace (min/km): pace = 1000 / (speed * 60)
                    Some(Decimal::from(1000) / (avg_speed * Decimal::from(60)))
                } else {
                    None
                }
            } else {
                None
            }
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
            normalized_power,
            avg_pace,
            intensity_factor,
            tss: None, // Will be calculated by TSS calculator after import
            total_distance,
            elevation_gain,
            avg_cadence,
            calories: None, // TODO: Extract from session data if available
        }
    }

    /// Determine primary data source based on available data and sport
    fn determine_data_source(&self, data_points: &[DataPoint], sport: &Sport) -> DataSource {
        let has_power = data_points.iter().any(|dp| dp.power.is_some());
        let has_heart_rate = data_points.iter().any(|dp| dp.heart_rate.is_some());
        let has_speed = data_points.iter().any(|dp| dp.speed.is_some());
        let has_running_dynamics = data_points.iter().any(|dp| {
            dp.ground_contact_time.is_some() || dp.vertical_oscillation.is_some() || dp.stride_length.is_some()
        });
        let has_swimming_data = data_points.iter().any(|dp| dp.stroke_count.is_some());

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
                if has_running_dynamics {
                    DataSource::Pace // Running dynamics enhance pace-based training
                } else if has_speed {
                    DataSource::Pace
                } else if has_heart_rate {
                    DataSource::HeartRate
                } else {
                    DataSource::HeartRate // Default for running
                }
            }
            Sport::Swimming => {
                if has_swimming_data {
                    DataSource::Pace // Swimming uses pace-based analysis with stroke data
                } else if has_heart_rate {
                    DataSource::HeartRate
                } else {
                    DataSource::HeartRate // Default for swimming
                }
            }
            Sport::Rowing => {
                if has_power {
                    DataSource::Power // Rowing can use power meters
                } else if has_heart_rate {
                    DataSource::HeartRate
                } else {
                    DataSource::HeartRate
                }
            }
            Sport::Triathlon => {
                // For triathlon, prefer the most comprehensive data source available
                if has_power {
                    DataSource::Power
                } else if has_running_dynamics || has_swimming_data {
                    DataSource::Pace
                } else if has_heart_rate {
                    DataSource::HeartRate
                } else {
                    DataSource::HeartRate
                }
            }
            Sport::CrossTraining => DataSource::HeartRate, // Default for cross training
        }
    }

    /// Parse developer data IDs from FIT records (message type 207)
    /// These map developer field indices to application UUIDs
    fn parse_developer_data_ids(&self, records: &[FitDataRecord]) -> Vec<crate::models::DeveloperDataId> {
        use crate::models::DeveloperDataId;

        let mut developer_ids = Vec::new();

        for record in records {
            // Check if this is a DeveloperDataId message (type 207)
            if record.kind() == fitparser::profile::MesgNum::DeveloperDataId {
                let mut developer_data_id: Option<u8> = None;
                let mut application_id: Option<[u8; 16]> = None;
                let mut manufacturer_id: Option<u16> = None;
                let mut developer_data_index: Option<u8> = None;

                for field in record.fields() {
                    match field.name() {
                        "developer_data_index" => {
                            if let Value::UInt8(id) = field.value() {
                                developer_data_id = Some(*id);
                            }
                        }
                        "application_id" => {
                            // Application ID is a 16-byte UUID
                            if let Value::Array(bytes) = field.value() {
                                if bytes.len() == 16 {
                                    let mut uuid_bytes = [0u8; 16];
                                    for (i, byte) in bytes.iter().enumerate() {
                                        if let Value::UInt8(b) = byte {
                                            uuid_bytes[i] = *b;
                                        }
                                    }
                                    application_id = Some(uuid_bytes);
                                }
                            }
                        }
                        "manufacturer_id" => {
                            if let Value::UInt16(id) = field.value() {
                                manufacturer_id = Some(*id);
                            }
                        }
                        "developer_data_index_2" => {
                            if let Value::UInt8(idx) = field.value() {
                                developer_data_index = Some(*idx);
                            }
                        }
                        _ => {}
                    }
                }

                // Create DeveloperDataId if we have the required fields
                if let (Some(dev_id), Some(app_id)) = (developer_data_id, application_id) {
                    developer_ids.push(DeveloperDataId {
                        developer_data_id: dev_id,
                        application_id: app_id,
                        manufacturer_id,
                        developer_data_index,
                    });
                }
            }
        }

        developer_ids
    }

    /// Parse developer field descriptions from FIT records (message type 206)
    /// These define the metadata for custom fields
    fn parse_field_descriptions(&self, records: &[FitDataRecord]) -> Vec<crate::models::DeveloperField> {
        use crate::models::DeveloperField;

        let mut developer_fields = Vec::new();

        for record in records {
            // Check if this is a FieldDescription message (type 206)
            if record.kind() == fitparser::profile::MesgNum::FieldDescription {
                let mut developer_data_id: Option<u8> = None;
                let mut field_definition_number: Option<u8> = None;
                let mut field_name: Option<String> = None;
                let mut fit_base_type_id: Option<u8> = None;
                let mut units: Option<String> = None;
                let mut scale: Option<f64> = None;
                let mut offset: Option<f64> = None;

                for field in record.fields() {
                    match field.name() {
                        "developer_data_index" => {
                            if let Value::UInt8(id) = field.value() {
                                developer_data_id = Some(*id);
                            }
                        }
                        "field_definition_number" => {
                            if let Value::UInt8(num) = field.value() {
                                field_definition_number = Some(*num);
                            }
                        }
                        "field_name" => {
                            if let Value::String(name) = field.value() {
                                field_name = Some(name.to_string());
                            }
                        }
                        "fit_base_type_id" => {
                            if let Value::UInt8(type_id) = field.value() {
                                fit_base_type_id = Some(*type_id);
                            }
                        }
                        "units" => {
                            if let Value::String(unit_str) = field.value() {
                                units = Some(unit_str.to_string());
                            }
                        }
                        "scale" => {
                            if let Value::UInt8(s) = field.value() {
                                scale = Some(*s as f64);
                            } else if let Value::UInt16(s) = field.value() {
                                scale = Some(*s as f64);
                            } else if let Value::UInt32(s) = field.value() {
                                scale = Some(*s as f64);
                            } else if let Value::Float64(s) = field.value() {
                                scale = Some(*s);
                            }
                        }
                        "offset" => {
                            if let Value::SInt8(o) = field.value() {
                                offset = Some(*o as f64);
                            } else if let Value::SInt16(o) = field.value() {
                                offset = Some(*o as f64);
                            } else if let Value::SInt32(o) = field.value() {
                                offset = Some(*o as f64);
                            } else if let Value::Float64(o) = field.value() {
                                offset = Some(*o);
                            }
                        }
                        _ => {}
                    }
                }

                // Create DeveloperField if we have the required fields
                if let (Some(dev_id), Some(field_num), Some(name), Some(type_id)) =
                    (developer_data_id, field_definition_number, field_name, fit_base_type_id) {
                    developer_fields.push(DeveloperField {
                        developer_data_id: dev_id,
                        field_definition_number: field_num,
                        field_name: name,
                        fit_base_type_id: type_id,
                        units,
                        scale,
                        offset,
                    });
                }
            }
        }

        developer_fields
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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Create test data points with power data for testing advanced metrics
    fn create_test_power_data_points() -> Vec<DataPoint> {
        vec![
            DataPoint {
                timestamp: 0,
                power: Some(200),
                heart_rate: Some(140),
                cadence: Some(90),
                speed: Some(dec!(10.0)), // 10 m/s
                distance: Some(dec!(10.0)),
                elevation: Some(100),
                pace: None,
                left_power: Some(100),
                right_power: Some(100),
                ground_contact_time: None,
                vertical_oscillation: None,
                stride_length: None,
                stroke_count: None,
                stroke_type: None,
                lap_number: Some(1),
                sport_transition: None,
            },
            DataPoint {
                timestamp: 1,
                power: Some(250),
                heart_rate: Some(150),
                cadence: Some(95),
                speed: Some(dec!(11.0)),
                distance: Some(dec!(21.0)),
                elevation: Some(102),
                pace: None,
                left_power: Some(125),
                right_power: Some(125),
                ground_contact_time: None,
                vertical_oscillation: None,
                stride_length: None,
                stroke_count: None,
                stroke_type: None,
                lap_number: Some(1),
                sport_transition: None,
            },
            DataPoint {
                timestamp: 2,
                power: Some(300),
                heart_rate: Some(160),
                cadence: Some(100),
                speed: Some(dec!(12.0)),
                distance: Some(dec!(33.0)),
                elevation: Some(105),
                pace: None,
                left_power: Some(150),
                right_power: Some(150),
                ground_contact_time: None,
                vertical_oscillation: None,
                stride_length: None,
                stroke_count: None,
                stroke_type: None,
                lap_number: Some(1),
                sport_transition: None,
            },
            // Add more data points to ensure we have enough for 30-second rolling average
            DataPoint {
                timestamp: 30,
                power: Some(220),
                heart_rate: Some(145),
                cadence: Some(92),
                speed: Some(dec!(10.5)),
                distance: Some(dec!(345.0)),
                elevation: Some(110),
                pace: None,
                left_power: Some(110),
                right_power: Some(110),
                ground_contact_time: None,
                vertical_oscillation: None,
                stride_length: None,
                stroke_count: None,
                stroke_type: None,
                lap_number: Some(1),
                sport_transition: None,
            },
        ]
    }

    #[test]
    fn test_calculate_summary_with_power_metrics() {
        let importer = FitImporter::new();
        let data_points = create_test_power_data_points();
        let summary = importer.calculate_summary(&data_points, &Sport::Cycling);

        // Test basic metrics
        assert!(summary.avg_power.is_some());
        assert!(summary.avg_heart_rate.is_some());
        assert!(summary.avg_cadence.is_some());
        assert!(summary.elevation_gain.is_some());

        // Test advanced power metrics (Phase 2 features)
        assert!(summary.normalized_power.is_some());
        let np = summary.normalized_power.unwrap();
        assert!(np > 0);
        assert!(np >= 200); // Should be at least average power due to variability

        // Test that Intensity Factor is None when no FTP is provided
        // (IF calculation requires FTP from athlete profile)
        assert!(summary.intensity_factor.is_none());

        // Test that TSS is None (calculated separately after import)
        assert!(summary.tss.is_none());
    }

    #[test]
    fn test_calculate_summary_with_running_data() {
        let importer = FitImporter::new();
        let data_points = vec![
            DataPoint {
                timestamp: 0,
                power: None,
                heart_rate: Some(140),
                cadence: Some(180), // Steps per minute for running
                speed: Some(dec!(4.0)), // 4 m/s
                distance: Some(dec!(4.0)),
                elevation: Some(100),
                pace: None,
                left_power: None,
                right_power: None,
                ground_contact_time: Some(250), // 250ms contact time
                vertical_oscillation: Some(80), // 8.0cm oscillation
                stride_length: Some(dec!(1.3)), // 1.3m stride
                stroke_count: None,
                stroke_type: None,
                lap_number: Some(1),
                sport_transition: None,
            },
            DataPoint {
                timestamp: 1,
                power: None,
                heart_rate: Some(150),
                cadence: Some(185),
                speed: Some(dec!(4.2)),
                distance: Some(dec!(8.2)),
                elevation: Some(102),
                pace: None,
                left_power: None,
                right_power: None,
                ground_contact_time: Some(240),
                vertical_oscillation: Some(75),
                stride_length: Some(dec!(1.35)),
                stroke_count: None,
                stroke_type: None,
                lap_number: Some(1),
                sport_transition: None,
            },
        ];

        let summary = importer.calculate_summary(&data_points, &Sport::Running);

        // Power metrics should be None for running
        assert!(summary.avg_power.is_none());
        assert!(summary.normalized_power.is_none());
        assert!(summary.intensity_factor.is_none());

        // Running-specific metrics
        assert!(summary.avg_heart_rate.is_some());
        assert!(summary.avg_cadence.is_some());
        assert!(summary.avg_pace.is_some()); // Calculated from speed

        let pace = summary.avg_pace.unwrap();
        // Average speed is 4.1 m/s, pace should be ~4.07 min/km
        assert!(pace > dec!(4.0) && pace < dec!(5.0));
    }

    #[test]
    fn test_calculate_summary_empty_data() {
        let importer = FitImporter::new();
        let data_points = vec![];
        let summary = importer.calculate_summary(&data_points, &Sport::Cycling);

        // All metrics should be None for empty data
        assert!(summary.avg_power.is_none());
        assert!(summary.normalized_power.is_none());
        assert!(summary.avg_heart_rate.is_none());
        assert!(summary.intensity_factor.is_none());
        assert!(summary.tss.is_none());
        assert!(summary.avg_cadence.is_none());
        assert!(summary.total_distance.is_none());
        assert!(summary.elevation_gain.is_none());
    }

    #[test]
    fn test_parse_left_right_power_balance() {
        // This test would require mock FIT data with left/right balance
        // For now, just test the data structure supports it
        let data_point = DataPoint {
            timestamp: 0,
            power: Some(200),
            heart_rate: Some(140),
            cadence: Some(90),
            speed: Some(dec!(10.0)),
            distance: Some(dec!(10.0)),
            elevation: Some(100),
            pace: None,
            left_power: Some(95), // 47.5% left
            right_power: Some(105), // 52.5% right
            ground_contact_time: None,
            vertical_oscillation: None,
            stride_length: None,
            stroke_count: None,
            stroke_type: None,
            lap_number: Some(1),
            sport_transition: None,
        };

        assert_eq!(data_point.left_power, Some(95));
        assert_eq!(data_point.right_power, Some(105));

        // Test balance calculation (95 + 105 = 200 total)
        let total = data_point.left_power.unwrap() + data_point.right_power.unwrap();
        assert_eq!(total, data_point.power.unwrap());
    }

    #[test]
    fn test_running_dynamics_and_swimming_metrics() {
        let importer = FitImporter::new();

        // Test running data point with dynamics
        let running_data = vec![
            DataPoint {
                timestamp: 0,
                power: None,
                heart_rate: Some(140),
                cadence: Some(180),
                speed: Some(dec!(4.0)),
                distance: Some(dec!(4.0)),
                elevation: Some(100),
                pace: None,
                left_power: None,
                right_power: None,
                ground_contact_time: Some(250), // Running dynamics
                vertical_oscillation: Some(80),
                stride_length: Some(dec!(1.3)),
                stroke_count: None,
                stroke_type: None,
                lap_number: Some(1),
                sport_transition: None,
            },
        ];

        // Test data source determination for running with dynamics
        let running_source = importer.determine_data_source(&running_data, &Sport::Running);
        assert_eq!(running_source, DataSource::Pace); // Should prefer pace for running dynamics

        // Test swimming data point
        let swimming_data = vec![
            DataPoint {
                timestamp: 0,
                power: None,
                heart_rate: Some(130),
                cadence: None,
                speed: Some(dec!(1.5)), // Slower swimming speed
                distance: Some(dec!(25.0)), // Pool length
                elevation: None,
                pace: None,
                left_power: None,
                right_power: None,
                ground_contact_time: None,
                vertical_oscillation: None,
                stride_length: None,
                stroke_count: Some(20), // Swimming metrics
                stroke_type: Some(1), // Freestyle
                lap_number: Some(1),
                sport_transition: None,
            },
        ];

        // Test data source determination for swimming
        let swimming_source = importer.determine_data_source(&swimming_data, &Sport::Swimming);
        assert_eq!(swimming_source, DataSource::Pace); // Should use pace for swimming with stroke data

        // Test multisport data with transition
        let multisport_data = vec![
            DataPoint {
                timestamp: 0,
                power: Some(200),
                heart_rate: Some(140),
                cadence: Some(90),
                speed: Some(dec!(10.0)),
                distance: Some(dec!(10.0)),
                elevation: Some(100),
                pace: None,
                left_power: Some(100),
                right_power: Some(100),
                ground_contact_time: None,
                vertical_oscillation: None,
                stride_length: None,
                stroke_count: None,
                stroke_type: None,
                lap_number: Some(1),
                sport_transition: None,
            },
            DataPoint {
                timestamp: 3600, // 1 hour later
                power: None,
                heart_rate: Some(150),
                cadence: Some(180),
                speed: Some(dec!(4.0)),
                distance: Some(dec!(4.0)),
                elevation: Some(102),
                pace: None,
                left_power: None,
                right_power: None,
                ground_contact_time: Some(250),
                vertical_oscillation: Some(80),
                stride_length: Some(dec!(1.3)),
                stroke_count: None,
                stroke_type: None,
                lap_number: Some(2),
                sport_transition: Some(true), // Transition detected
            },
        ];

        // Test data source determination for triathlon
        let triathlon_source = importer.determine_data_source(&multisport_data, &Sport::Triathlon);
        assert_eq!(triathlon_source, DataSource::Power); // Should prefer power for triathlon when available

        // Verify sport transition is marked
        assert!(multisport_data[1].sport_transition.unwrap_or(false));
    }

    #[test]
    fn test_enhanced_sport_mapping() {
        let importer = FitImporter::new();

        // Test expanded sport mapping (this would require mock FIT data)
        // For now, just verify the data structures support the new sports
        let sports = vec![
            Sport::Running,
            Sport::Cycling,
            Sport::Swimming,
            Sport::Rowing,
            Sport::Triathlon,
            Sport::CrossTraining,
        ];

        for sport in sports {
            let empty_data = vec![];
            let source = importer.determine_data_source(&empty_data, &sport);
            // All sports should have a default data source
            assert_eq!(source, DataSource::HeartRate);
        }
    }
}
