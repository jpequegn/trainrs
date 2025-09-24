use super::{TrainingReport, ExportError};
use std::path::Path;
use std::io::Write;

/// Export training report to JSON format
pub fn export_training_report<P: AsRef<Path>>(
    report: &TrainingReport,
    output_path: P,
) -> Result<(), ExportError> {
    let json_data = serde_json::to_string_pretty(report)
        .map_err(|e| ExportError::SerializationError(e.to_string()))?;

    let mut file = std::fs::File::create(output_path)?;
    file.write_all(json_data.as_bytes())?;

    Ok(())
}

/// Export any serializable data structure to JSON
pub fn export_json<T, P>(data: &T, output_path: P) -> Result<(), ExportError>
where
    T: serde::Serialize,
    P: AsRef<Path>,
{
    let json_data = serde_json::to_string_pretty(data)
        .map_err(|e| ExportError::SerializationError(e.to_string()))?;

    let mut file = std::fs::File::create(output_path)?;
    file.write_all(json_data.as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::{TrainingReport, TrainingReportSummary, DateRange};
    use chrono::{NaiveDate, Utc};
    use rust_decimal_macros::dec;
    use tempfile::NamedTempFile;

    #[test]
    fn test_export_training_report() {
        let report = TrainingReport {
            athlete_id: Some("test_athlete".to_string()),
            date_range: DateRange::new(
                Some(NaiveDate::from_ymd_opt(2024, 9, 1).unwrap()),
                Some(NaiveDate::from_ymd_opt(2024, 9, 30).unwrap()),
            ),
            generated_at: Utc::now(),
            summary_stats: TrainingReportSummary {
                total_workouts: 20,
                total_tss: dec!(1500),
                total_duration_hours: dec!(25.5),
                avg_tss_per_workout: dec!(75),
                most_frequent_sport: "Cycling".to_string(),
                date_range_days: 30,
                training_consistency: dec!(66.7),
            },
            pmc_analysis: None,
            weekly_summaries: Vec::new(),
            monthly_summaries: Vec::new(),
            zone_analysis: Vec::new(),
            training_recommendations: vec!["Continue current training".to_string()],
        };

        let temp_file = NamedTempFile::new().unwrap();
        let result = export_training_report(&report, temp_file.path());

        assert!(result.is_ok());

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(content.contains("\"athlete_id\": \"test_athlete\""));
        assert!(content.contains("\"total_workouts\": 20"));
        assert!(content.contains("\"total_tss\": \"1500\""));
        assert!(content.contains("\"most_frequent_sport\": \"Cycling\""));
    }

    #[test]
    fn test_export_json_generic() {
        #[derive(serde::Serialize)]
        struct TestData {
            name: String,
            value: u32,
        }

        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        let temp_file = NamedTempFile::new().unwrap();
        let result = export_json(&data, temp_file.path());

        assert!(result.is_ok());

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(content.contains("\"name\": \"test\""));
        assert!(content.contains("\"value\": 42"));
    }
}