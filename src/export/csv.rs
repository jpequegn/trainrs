#![allow(dead_code)]

use crate::models::Workout;
use crate::pmc::PmcMetrics;
use super::ExportError;
use rust_decimal::Decimal;
use std::path::Path;
use std::io::Write;

/// Export workout summaries to CSV format
pub fn export_workout_summaries<P: AsRef<Path>>(
    workouts: &[&Workout],
    output_path: P,
) -> Result<(), ExportError> {
    let mut file = std::fs::File::create(output_path)?;

    // Write CSV header
    writeln!(file, "Date,Sport,Duration_Hours,Workout_Type,Data_Source,TSS,Avg_HR,Max_HR,Avg_Power,Normalized_Power,Avg_Pace,Intensity_Factor,Distance_KM,Elevation_Gain_M,Avg_Cadence,Calories,Notes,Athlete_ID,Source")?;

    // Write workout data
    for workout in workouts {
        let duration_hours = Decimal::from(workout.duration_seconds) / Decimal::from(3600);
        let distance_km = workout.summary.total_distance
            .map(|d| d / Decimal::from(1000));

        writeln!(
            file,
            "{},{:?},{},{:?},{:?},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            workout.date.format("%Y-%m-%d"),
            workout.sport,
            duration_hours,
            workout.workout_type,
            workout.data_source,
            workout.summary.tss.map_or("".to_string(), |v| v.to_string()),
            workout.summary.avg_heart_rate.map_or("".to_string(), |v| v.to_string()),
            workout.summary.max_heart_rate.map_or("".to_string(), |v| v.to_string()),
            workout.summary.avg_power.map_or("".to_string(), |v| v.to_string()),
            workout.summary.normalized_power.map_or("".to_string(), |v| v.to_string()),
            workout.summary.avg_pace.map_or("".to_string(), |v| v.to_string()),
            workout.summary.intensity_factor.map_or("".to_string(), |v| v.to_string()),
            distance_km.map_or("".to_string(), |v| v.to_string()),
            workout.summary.elevation_gain.map_or("".to_string(), |v| v.to_string()),
            workout.summary.avg_cadence.map_or("".to_string(), |v| v.to_string()),
            workout.summary.calories.map_or("".to_string(), |v| v.to_string()),
            workout.notes.as_ref().map_or("".to_string(), |n| format!("\"{}\"", n.replace("\"", "\"\""))),
            workout.athlete_id.as_ref().map_or("".to_string(), |id| id.to_string()),
            workout.source.as_ref().map_or("".to_string(), |s| s.to_string())
        )?;
    }

    Ok(())
}

/// Export PMC data to CSV format (suitable for Excel plotting)
pub fn export_pmc_data<P: AsRef<Path>>(
    pmc_data: &[PmcMetrics],
    output_path: P,
) -> Result<(), ExportError> {
    let mut file = std::fs::File::create(output_path)?;

    // Write CSV header
    writeln!(file, "Date,CTL,ATL,TSB,Daily_TSS,CTL_Ramp_Rate,ATL_Spike")?;

    // Write PMC data
    for metrics in pmc_data {
        writeln!(
            file,
            "{},{},{},{},{},{},{}",
            metrics.date.format("%Y-%m-%d"),
            metrics.ctl,
            metrics.atl,
            metrics.tsb,
            metrics.daily_tss,
            metrics.ctl_ramp_rate.map_or("".to_string(), |v| v.to_string()),
            if metrics.atl_spike { "1" } else { "0" }
        )?;
    }

    Ok(())
}

/// Export zone analysis to CSV format
pub fn export_zone_analysis<P: AsRef<Path>>(
    zone_data: &[super::ZoneAnalysis],
    output_path: P,
) -> Result<(), ExportError> {
    let mut file = std::fs::File::create(output_path)?;

    // Write CSV header
    writeln!(file, "Date,Workout_ID,Sport,Zone_1_Time,Zone_2_Time,Zone_3_Time,Zone_4_Time,Zone_5_Time,Zone_6_Time,Zone_7_Time,Dominant_Zone")?;

    // Write zone analysis data
    for analysis in zone_data {
        let mut zone_times = vec![String::new(); 7]; // Support up to 7 zones

        for (zone, time_data) in &analysis.time_in_zones {
            if *zone >= 1 && *zone <= 7 {
                zone_times[(*zone as usize) - 1] = format!("{}", time_data);
            }
        }

        writeln!(
            file,
            "{},{},{},{},{},{},{},{},{},{},{}",
            analysis.date.format("%Y-%m-%d"),
            analysis.workout_id,
            analysis.sport,
            zone_times[0], zone_times[1], zone_times[2], zone_times[3],
            zone_times[4], zone_times[5], zone_times[6],
            analysis.dominant_zone.map_or("".to_string(), |z| z.to_string())
        )?;
    }

    Ok(())
}

/// Export weekly summaries to CSV format
pub fn export_weekly_summaries<P: AsRef<Path>>(
    weekly_summaries: &[super::WeeklySummary],
    output_path: P,
) -> Result<(), ExportError> {
    let mut file = std::fs::File::create(output_path)?;

    // Write CSV header
    writeln!(file, "Week_Start,Week_End,Year,Week_Number,Total_TSS,Workout_Count,Duration_Hours,Distance_KM,Avg_Daily_TSS")?;

    // Write weekly summary data
    for summary in weekly_summaries {
        writeln!(
            file,
            "{},{},{},{},{},{},{},{},{}",
            summary.week_start.format("%Y-%m-%d"),
            summary.week_end.format("%Y-%m-%d"),
            summary.year,
            summary.week_number,
            summary.total_tss,
            summary.workout_count,
            summary.total_duration_hours,
            summary.total_distance_km.map_or("".to_string(), |v| v.to_string()),
            summary.avg_daily_tss
        )?;
    }

    Ok(())
}

/// Export monthly summaries to CSV format
pub fn export_monthly_summaries<P: AsRef<Path>>(
    monthly_summaries: &[super::MonthlySummary],
    output_path: P,
) -> Result<(), ExportError> {
    let mut file = std::fs::File::create(output_path)?;

    // Write CSV header
    writeln!(file, "Year,Month,Month_Name,Total_TSS,Workout_Count,Duration_Hours,Distance_KM,Avg_Daily_TSS")?;

    // Write monthly summary data
    for summary in monthly_summaries {
        writeln!(
            file,
            "{},{},{},{},{},{},{},{}",
            summary.year,
            summary.month,
            summary.month_name,
            summary.total_tss,
            summary.workout_count,
            summary.total_duration_hours,
            summary.total_distance_km.map_or("".to_string(), |v| v.to_string()),
            summary.avg_daily_tss
        )?;
    }

    Ok(())
}

/// Export data in TrainingPeaks compatible format
pub fn export_training_peaks_format<P: AsRef<Path>>(
    workouts: &[&Workout],
    output_path: P,
) -> Result<(), ExportError> {
    let mut file = std::fs::File::create(output_path)?;

    // TrainingPeaks CSV format header
    // Based on TrainingPeaks WKO file format specifications
    writeln!(file, "Date,Time,Duration,Distance,TSS,IF,NP,Work,Title,Sport")?;

    // Write workout data in TrainingPeaks format
    for workout in workouts {
        let duration_formatted = format_duration_for_training_peaks(workout.duration_seconds);
        let distance_km = workout.summary.total_distance
            .map(|d| d / Decimal::from(1000))
            .unwrap_or(Decimal::ZERO);
        let work_kj = workout.summary.avg_power
            .map(|p| Decimal::from(p) * Decimal::from(workout.duration_seconds) / Decimal::from(1000))
            .unwrap_or(Decimal::ZERO);

        writeln!(
            file,
            "{},12:00:00,{},{},{},{},{},{},\"{:?} {:?}\",{}",
            workout.date.format("%m/%d/%Y"),
            duration_formatted,
            distance_km,
            workout.summary.tss.unwrap_or(Decimal::ZERO),
            workout.summary.intensity_factor.unwrap_or(Decimal::ZERO),
            workout.summary.normalized_power.map(|p| Decimal::from(p)).unwrap_or(Decimal::ZERO),
            work_kj,
            workout.workout_type,
            workout.sport,
            map_sport_to_training_peaks(&workout.sport)
        )?;
    }

    Ok(())
}

/// Format duration for TrainingPeaks (HH:MM:SS)
fn format_duration_for_training_peaks(duration_seconds: u32) -> String {
    let hours = duration_seconds / 3600;
    let minutes = (duration_seconds % 3600) / 60;
    let seconds = duration_seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

/// Map our sport enum to TrainingPeaks sport codes
fn map_sport_to_training_peaks(sport: &crate::models::Sport) -> &'static str {
    match sport {
        crate::models::Sport::Running => "Run",
        crate::models::Sport::Cycling => "Bike",
        crate::models::Sport::Swimming => "Swim",
        crate::models::Sport::Triathlon => "Brick",
        crate::models::Sport::Rowing => "Row",
        crate::models::Sport::CrossTraining => "Other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DataSource, Sport, WorkoutSummary, WorkoutType};
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;
    use tempfile::NamedTempFile;

    fn create_test_workout(date: NaiveDate, tss: Decimal) -> Workout {
        Workout {
            id: format!("workout_{}", date.format("%Y%m%d")),
            date,
            sport: Sport::Cycling,
            duration_seconds: 3600,
            workout_type: WorkoutType::Endurance,
            data_source: DataSource::Power,
            raw_data: None,
            summary: WorkoutSummary {
                tss: Some(tss),
                avg_heart_rate: Some(150),
                max_heart_rate: Some(180),
                avg_power: Some(220),
                normalized_power: Some(235),
                total_distance: Some(dec!(25000)), // 25km in meters
                ..WorkoutSummary::default()
            },
            notes: Some("Test workout".to_string()),
            athlete_id: Some("test_athlete".to_string()),
            source: Some("test_source".to_string()),
        }
    }

    #[test]
    fn test_export_workout_summaries() {
        let workouts = vec![
            create_test_workout(NaiveDate::from_ymd_opt(2024, 9, 23).unwrap(), dec!(85)),
        ];
        let workout_refs: Vec<&Workout> = workouts.iter().collect();

        let temp_file = NamedTempFile::new().unwrap();
        let result = export_workout_summaries(&workout_refs, temp_file.path());

        assert!(result.is_ok());

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(content.contains("Date,Sport,Duration_Hours"));
        assert!(content.contains("2024-09-23,Cycling"));
        assert!(content.contains("85")); // TSS value
    }

    #[test]
    fn test_export_pmc_data() {
        let pmc_data = vec![
            PmcMetrics {
                date: NaiveDate::from_ymd_opt(2024, 9, 23).unwrap(),
                ctl: dec!(45.5),
                atl: dec!(65.2),
                tsb: dec!(-19.7),
                daily_tss: dec!(85),
                ctl_ramp_rate: Some(dec!(3.2)),
                atl_spike: false,
            }
        ];

        let temp_file = NamedTempFile::new().unwrap();
        let result = export_pmc_data(&pmc_data, temp_file.path());

        assert!(result.is_ok());

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(content.contains("Date,CTL,ATL,TSB"));
        assert!(content.contains("2024-09-23,45.5,65.2,-19.7"));
        assert!(content.contains("3.2")); // CTL ramp rate
        assert!(content.contains("0")); // ATL spike false
    }

    #[test]
    fn test_format_duration_for_training_peaks() {
        assert_eq!(format_duration_for_training_peaks(3661), "01:01:01");
        assert_eq!(format_duration_for_training_peaks(3600), "01:00:00");
        assert_eq!(format_duration_for_training_peaks(90), "00:01:30");
    }

    #[test]
    fn test_map_sport_to_training_peaks() {
        assert_eq!(map_sport_to_training_peaks(&Sport::Running), "Run");
        assert_eq!(map_sport_to_training_peaks(&Sport::Cycling), "Bike");
        assert_eq!(map_sport_to_training_peaks(&Sport::Swimming), "Swim");
    }

    #[test]
    fn test_export_training_peaks_format() {
        let workouts = vec![
            create_test_workout(NaiveDate::from_ymd_opt(2024, 9, 23).unwrap(), dec!(85)),
        ];
        let workout_refs: Vec<&Workout> = workouts.iter().collect();

        let temp_file = NamedTempFile::new().unwrap();
        let result = export_training_peaks_format(&workout_refs, temp_file.path());

        assert!(result.is_ok());

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(content.contains("Date,Time,Duration,Distance,TSS"));
        assert!(content.contains("09/23/2024,12:00:00"));
        assert!(content.contains("01:00:00")); // Duration
        assert!(content.contains("25")); // Distance in km
        assert!(content.contains("Bike")); // Sport mapping
    }
}