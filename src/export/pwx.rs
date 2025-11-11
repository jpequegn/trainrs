use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::Path;

use chrono::{NaiveTime, Utc};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use rust_decimal::Decimal;

use crate::models::{DataSource, Sport, Workout};
use crate::power::MmpAnalyzer;

use super::ExportError;

const PWX_NAMESPACE: &str = "http://www.peaksware.com/PWX/1/0";

impl From<quick_xml::Error> for ExportError {
    fn from(err: quick_xml::Error) -> Self {
        ExportError::SerializationError(err.to_string())
    }
}

pub struct PwxExporter;

impl PwxExporter {
    pub fn export_workout(workout: &Workout, path: &Path) -> Result<(), ExportError> {
        let xml = Self::generate_pwx_xml(workout)?;
        fs::write(path, xml)?;
        Ok(())
    }

    pub fn generate_pwx_xml(workout: &Workout) -> Result<String, ExportError> {
        let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);

        writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
        writer.write_event(Event::Text(BytesText::new("\n")))?;

        let mut pwx = BytesStart::new("pwx");
        pwx.push_attribute(("version", "1.0"));
        pwx.push_attribute(("xmlns", PWX_NAMESPACE));
        writer.write_event(Event::Start(pwx))?;

        Self::write_workout(&mut writer, workout)?;

        writer.write_event(Event::End(BytesEnd::new("pwx")))?;

        let bytes = writer.into_inner();
        String::from_utf8(bytes).map_err(|err| ExportError::SerializationError(err.to_string()))
    }

    fn write_workout<W: Write>(
        writer: &mut Writer<W>,
        workout: &Workout,
    ) -> Result<(), ExportError> {
        writer.write_event(Event::Start(BytesStart::new("workout")))?;

        Self::write_athlete(writer, workout)?;
        Self::write_metadata(writer, workout)?;
        Self::write_summary(writer, workout)?;
        Self::write_segments(writer, workout)?;
        Self::write_samples(writer, workout)?;
        Self::write_power_curve(writer, workout)?;

        if let Some(notes) = &workout.notes {
            if !notes.trim().is_empty() {
                Self::write_text_element(writer, "notes", notes)?;
            }
        }

        writer.write_event(Event::End(BytesEnd::new("workout")))?;
        Ok(())
    }

    fn write_athlete<W: Write>(
        writer: &mut Writer<W>,
        workout: &Workout,
    ) -> Result<(), ExportError> {
        writer.write_event(Event::Start(BytesStart::new("athlete")))?;
        let name = workout.athlete_id.as_deref().unwrap_or("Unknown Athlete");
        Self::write_text_element(writer, "name", name)?;
        writer.write_event(Event::End(BytesEnd::new("athlete")))?;
        Ok(())
    }

    fn write_metadata<W: Write>(
        writer: &mut Writer<W>,
        workout: &Workout,
    ) -> Result<(), ExportError> {
        let title = workout
            .notes
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .or(workout.source.as_deref())
            .unwrap_or(workout.id.as_str());
        Self::write_text_element(writer, "title", title)?;
        Self::write_text_element(writer, "sportType", map_sport(&workout.sport))?;

        let start_time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
        let datetime = chrono::NaiveDateTime::new(workout.date, start_time);
        let time_iso =
            chrono::DateTime::<Utc>::from_naive_utc_and_offset(datetime, Utc).to_rfc3339();
        Self::write_text_element(writer, "time", &time_iso)?;

        Self::write_text_element(writer, "duration", &workout.duration_seconds.to_string())?;
        Self::write_text_element(writer, "durationType", "time")?;

        if let Some(source) = &workout.source {
            Self::write_text_element(writer, "device", source)?;
        }

        Self::write_text_element(writer, "dataSource", map_data_source(workout))?;
        Ok(())
    }

    fn write_summary<W: Write>(
        writer: &mut Writer<W>,
        workout: &Workout,
    ) -> Result<(), ExportError> {
        writer.write_event(Event::Start(BytesStart::new("summary_data")))?;
        Self::write_text_element(writer, "duration", &workout.duration_seconds.to_string())?;

        if let Some(distance) = workout.summary.total_distance {
            Self::write_text_element(writer, "distance", &decimal_to_string(distance))?;
        }

        if let Some(tss) = workout.summary.tss {
            Self::write_text_element(writer, "tss", &decimal_to_string(tss))?;
        }

        if let Some(intensity_factor) = workout.summary.intensity_factor {
            Self::write_text_element(
                writer,
                "intensityFactor",
                &decimal_to_string(intensity_factor),
            )?;
        }

        if let Some(normalized_power) = workout.summary.normalized_power {
            Self::write_text_element(writer, "normalizedPower", &normalized_power.to_string())?;
        }

        if let Some(avg_power) = workout.summary.avg_power {
            Self::write_text_element(writer, "avgPower", &avg_power.to_string())?;
        }

        if let Some(avg_hr) = workout.summary.avg_heart_rate {
            Self::write_text_element(writer, "avgHeartRate", &avg_hr.to_string())?;
        }

        if let Some(calories) = workout.summary.calories {
            Self::write_text_element(writer, "calories", &calories.to_string())?;
        }

        writer.write_event(Event::End(BytesEnd::new("summary_data")))?;
        Ok(())
    }

    fn write_segments<W: Write>(
        writer: &mut Writer<W>,
        workout: &Workout,
    ) -> Result<(), ExportError> {
        let raw_data = match &workout.raw_data {
            Some(data) if !data.is_empty() => data,
            _ => return Ok(()),
        };

        let lap_numbers: BTreeSet<u16> = raw_data.iter().filter_map(|dp| dp.lap_number).collect();
        if lap_numbers.is_empty() {
            return Ok(());
        }

        writer.write_event(Event::Start(BytesStart::new("segments")))?;
        for lap in lap_numbers {
            writer.write_event(Event::Start(BytesStart::new("segment")))?;
            Self::write_text_element(writer, "name", &format!("Lap {}", lap))?;
            Self::write_text_element(writer, "type", "Lap")?;
            writer.write_event(Event::End(BytesEnd::new("segment")))?;
        }
        writer.write_event(Event::End(BytesEnd::new("segments")))?;
        Ok(())
    }

    fn write_samples<W: Write>(
        writer: &mut Writer<W>,
        workout: &Workout,
    ) -> Result<(), ExportError> {
        let raw_data = match &workout.raw_data {
            Some(data) if !data.is_empty() => data,
            _ => return Ok(()),
        };

        let max_samples = 10_000usize;
        let len = raw_data.len();
        let step = if len > max_samples {
            (len / max_samples).max(1)
        } else {
            1
        };

        let mut indices: Vec<usize> = (0..len).step_by(step).collect();
        if let Some(last_index) = len.checked_sub(1) {
            if indices.last().copied() != Some(last_index) {
                indices.push(last_index);
            }
        }

        for idx in indices {
            let dp = &raw_data[idx];
            writer.write_event(Event::Start(BytesStart::new("sample")))?;
            Self::write_text_element(writer, "timeoffset", &dp.timestamp.to_string())?;

            if let Some(hr) = dp.heart_rate {
                Self::write_text_element(writer, "hr", &hr.to_string())?;
            }
            if let Some(power) = dp.power {
                Self::write_text_element(writer, "pwr", &power.to_string())?;
            }
            if let Some(cadence) = dp.cadence {
                Self::write_text_element(writer, "cad", &cadence.to_string())?;
            }
            if let Some(speed) = dp.speed {
                Self::write_text_element(writer, "spd", &decimal_to_string(speed))?;
            }
            if let Some(distance) = dp.distance {
                Self::write_text_element(writer, "dist", &decimal_to_string(distance))?;
            }
            if let Some(elevation) = dp.elevation {
                Self::write_text_element(writer, "alt", &elevation.to_string())?;
            }
            if let Some(left) = dp.left_power {
                Self::write_text_element(writer, "leftBalance", &left.to_string())?;
            }
            if let Some(right) = dp.right_power {
                Self::write_text_element(writer, "rightBalance", &right.to_string())?;
            }

            writer.write_event(Event::End(BytesEnd::new("sample")))?;
        }

        Ok(())
    }

    fn write_power_curve<W: Write>(
        writer: &mut Writer<W>,
        workout: &Workout,
    ) -> Result<(), ExportError> {
        let raw_data = match &workout.raw_data {
            Some(data) if data.iter().any(|dp| dp.power.is_some()) => data,
            _ => return Ok(()),
        };

        let curve = match MmpAnalyzer::calculate_mmp(raw_data) {
            Ok(curve) => curve,
            Err(_) => return Ok(()),
        };

        if curve.duration_seconds.is_empty() {
            return Ok(());
        }

        writer.write_event(Event::Start(BytesStart::new("metrics")))?;
        writer.write_event(Event::Start(BytesStart::new("powerCurve")))?;

        let total = curve.duration_seconds.len();
        let step = if total > 300 { (total / 300).max(1) } else { 1 };

        let mut idx = 0;
        while idx < total {
            let duration = curve.duration_seconds[idx];
            let power = curve.max_power[idx];
            writer.write_event(Event::Start(BytesStart::new("peak")))?;
            Self::write_text_element(writer, "seconds", &duration.to_string())?;
            Self::write_text_element(writer, "watts", &power.to_string())?;
            writer.write_event(Event::End(BytesEnd::new("peak")))?;
            idx += step;
        }

        if (total - 1) % step != 0 {
            let duration = *curve.duration_seconds.last().unwrap();
            let power = *curve.max_power.last().unwrap();
            writer.write_event(Event::Start(BytesStart::new("peak")))?;
            Self::write_text_element(writer, "seconds", &duration.to_string())?;
            Self::write_text_element(writer, "watts", &power.to_string())?;
            writer.write_event(Event::End(BytesEnd::new("peak")))?;
        }

        writer.write_event(Event::End(BytesEnd::new("powerCurve")))?;
        writer.write_event(Event::End(BytesEnd::new("metrics")))?;
        Ok(())
    }

    fn write_text_element<W: Write>(
        writer: &mut Writer<W>,
        name: &str,
        value: &str,
    ) -> Result<(), ExportError> {
        let start = BytesStart::new(name);
        writer.write_event(Event::Start(start))?;
        writer.write_event(Event::Text(BytesText::new(value)))?;
        writer.write_event(Event::End(BytesEnd::new(name)))?;
        Ok(())
    }
}

fn map_sport(sport: &Sport) -> &'static str {
    match sport {
        Sport::Running => "Run",
        Sport::Cycling => "Bike",
        Sport::Swimming => "Swim",
        Sport::Triathlon => "Tri",
        Sport::Rowing => "Row",
        Sport::CrossTraining => "Cross",
    }
}

fn map_data_source(workout: &Workout) -> &'static str {
    match workout.data_source {
        DataSource::HeartRate => "HeartRate",
        DataSource::Power => "Power",
        DataSource::Pace => "Pace",
        DataSource::Rpe => "RPE",
    }
}

fn decimal_to_string(value: Decimal) -> String {
    value.normalize().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DataPoint, WorkoutSummary, WorkoutType};
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;
    use std::fs;
    use tempfile::NamedTempFile;

    fn sample_data_point(
        timestamp: u32,
        power: u16,
        heart_rate: u16,
        distance: Decimal,
    ) -> DataPoint {
        DataPoint {
            timestamp,
            heart_rate: Some(heart_rate),
            power: Some(power),
            pace: None,
            elevation: Some(100),
            cadence: Some(90),
            speed: Some(dec!(8.5)),
            distance: Some(distance),
            left_power: None,
            right_power: None,
            ground_contact_time: None,
            vertical_oscillation: None,
            stride_length: None,
            stroke_count: None,
            stroke_type: None,
            lap_number: Some(1),
            sport_transition: None,
        }
    }

    fn build_test_workout() -> Workout {
        let raw_data = vec![
            sample_data_point(0, 220, 145, dec!(0)),
            sample_data_point(30, 230, 150, dec!(250)),
            sample_data_point(60, 240, 152, dec!(500)),
        ];

        Workout {
            id: "workout_20240930".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 9, 30).unwrap(),
            sport: Sport::Cycling,
            duration_seconds: 3600,
            workout_type: WorkoutType::Endurance,
            data_source: DataSource::Power,
            raw_data: Some(raw_data),
            summary: WorkoutSummary {
                avg_heart_rate: Some(148),
                max_heart_rate: Some(165),
                avg_power: Some(215),
                normalized_power: Some(230),
                intensity_factor: Some(dec!(0.85)),
                tss: Some(dec!(85.2)),
                total_distance: Some(dec!(25000)),
                calories: Some(780),
                ..WorkoutSummary::default()
            },
            notes: Some("Morning ride".to_string()),
            athlete_id: Some("athlete_a".to_string()),
            source: Some("Garmin Edge".to_string()),
        }
    }

    #[test]
    fn generates_pwx_xml_with_expected_sections() {
        let workout = build_test_workout();
        let xml = PwxExporter::generate_pwx_xml(&workout).unwrap();

        assert!(xml.contains("<pwx"));
        assert!(xml.contains("<sportType>Bike</sportType>"));
        assert!(xml.contains("<summary_data>"));
        assert!(xml.contains("<sample>"));
        assert!(xml.contains("<powerCurve>"));
    }

    #[test]
    fn exports_pwx_file_to_disk() {
        let workout = build_test_workout();
        let file = NamedTempFile::new().unwrap();

        PwxExporter::export_workout(&workout, file.path()).unwrap();

        let content = fs::read_to_string(file.path()).unwrap();
        assert!(content.contains("Morning ride"));
    }
}
