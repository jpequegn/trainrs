use super::{TrainingReport, ExportError};
use std::path::Path;
use std::io::Write;

/// Export training report to human-readable text format
pub fn export_training_report<P: AsRef<Path>>(
    report: &TrainingReport,
    output_path: P,
) -> Result<(), ExportError> {
    let mut file = std::fs::File::create(output_path)?;

    // Header
    writeln!(file, "=")?;
    writeln!(file, "TRAINING REPORT")?;
    writeln!(file, "=")?;
    writeln!(file)?;

    // Basic info
    if let Some(athlete_id) = &report.athlete_id {
        writeln!(file, "Athlete: {}", athlete_id)?;
    }

    writeln!(file, "Generated: {}", report.generated_at.format("%Y-%m-%d %H:%M:%S UTC"))?;

    if let Some(start) = report.date_range.start {
        if let Some(end) = report.date_range.end {
            writeln!(file, "Period: {} to {}", start.format("%Y-%m-%d"), end.format("%Y-%m-%d"))?;
        } else {
            writeln!(file, "Period: From {}", start.format("%Y-%m-%d"))?;
        }
    } else if let Some(end) = report.date_range.end {
        writeln!(file, "Period: Up to {}", end.format("%Y-%m-%d"))?;
    }
    writeln!(file)?;

    // Summary statistics
    writeln!(file, "TRAINING SUMMARY")?;
    writeln!(file, "-")?;
    writeln!(file, "Total Workouts: {}", report.summary_stats.total_workouts)?;
    writeln!(file, "Total TSS: {}", report.summary_stats.total_tss)?;
    writeln!(file, "Total Duration: {:.1} hours", report.summary_stats.total_duration_hours)?;
    writeln!(file, "Average TSS per Workout: {:.1}", report.summary_stats.avg_tss_per_workout)?;
    writeln!(file, "Most Frequent Sport: {}", report.summary_stats.most_frequent_sport)?;
    writeln!(file, "Training Consistency: {:.1}% of days", report.summary_stats.training_consistency)?;
    writeln!(file)?;

    // PMC Analysis
    if let Some(pmc) = &report.pmc_analysis {
        writeln!(file, "PERFORMANCE MANAGEMENT CHART (PMC)")?;
        writeln!(file, "-")?;
        writeln!(file, "Current Chronic Training Load (CTL): {:.1}", pmc.current_ctl)?;
        writeln!(file, "Current Acute Training Load (ATL): {:.1}", pmc.current_atl)?;
        writeln!(file, "Current Training Stress Balance (TSB): {:.1}", pmc.current_tsb)?;

        let tsb_interpretation = if pmc.current_tsb >= rust_decimal::Decimal::from(25) {
            "Very fresh (may be losing fitness)"
        } else if pmc.current_tsb >= rust_decimal::Decimal::from(5) {
            "Fresh and ready for hard training/racing"
        } else if pmc.current_tsb >= rust_decimal::Decimal::from(-10) {
            "Neutral (normal training)"
        } else if pmc.current_tsb >= rust_decimal::Decimal::from(-30) {
            "Fatigued (monitor closely)"
        } else {
            "Very fatigued (rest needed)"
        };

        writeln!(file, "Form Status: {}", tsb_interpretation)?;
        writeln!(file, "Fitness Trend: {}", pmc.fitness_trend)?;
        writeln!(file, "Fatigue Trend: {}", pmc.fatigue_trend)?;
        writeln!(file, "Form Trend: {}", pmc.form_trend)?;
        writeln!(file)?;

        if !pmc.recommendations.is_empty() {
            writeln!(file, "PMC Recommendations:")?;
            for recommendation in &pmc.recommendations {
                writeln!(file, "• {}", recommendation)?;
            }
            writeln!(file)?;
        }
    }

    // Weekly summaries
    if !report.weekly_summaries.is_empty() {
        writeln!(file, "WEEKLY SUMMARIES")?;
        writeln!(file, "-")?;
        writeln!(file, "{:<12} {:<8} {:<12} {:<10} {:<15} {:<15}", "Week", "TSS", "Workouts", "Hours", "Distance (km)", "Avg Daily TSS")?;
        writeln!(file, "{:-<80}", "")?;

        for week in &report.weekly_summaries {
            let distance = week.total_distance_km
                .map(|d| format!("{:.1}", d))
                .unwrap_or_else(|| "-".to_string());

            writeln!(
                file,
                "{}/{:02} {:>8} {:>8} {:>10.1} {:>15} {:>15.1}",
                week.year,
                week.week_number,
                week.total_tss,
                week.workout_count,
                week.total_duration_hours,
                distance,
                week.avg_daily_tss
            )?;
        }
        writeln!(file)?;
    }

    // Monthly summaries
    if !report.monthly_summaries.is_empty() {
        writeln!(file, "MONTHLY SUMMARIES")?;
        writeln!(file, "-")?;
        writeln!(file, "{:<15} {:<8} {:<12} {:<10} {:<15} {:<15}", "Month", "TSS", "Workouts", "Hours", "Distance (km)", "Avg Daily TSS")?;
        writeln!(file, "{:-<80}", "")?;

        for month in &report.monthly_summaries {
            let distance = month.total_distance_km
                .map(|d| format!("{:.1}", d))
                .unwrap_or_else(|| "-".to_string());

            writeln!(
                file,
                "{} {} {:>8} {:>8} {:>10.1} {:>15} {:>15.1}",
                month.year,
                month.month_name,
                month.total_tss,
                month.workout_count,
                month.total_duration_hours,
                distance,
                month.avg_daily_tss
            )?;
        }
        writeln!(file)?;

        // Sport breakdown for latest month
        if let Some(latest_month) = report.monthly_summaries.last() {
            if !latest_month.sports_breakdown.is_empty() {
                writeln!(file, "SPORT BREAKDOWN - {} {}", latest_month.month_name, latest_month.year)?;
                writeln!(file, "-")?;
                writeln!(file, "{:<15} {:<8} {:<12} {:<10}", "Sport", "TSS", "Workouts", "Hours")?;
                writeln!(file, "{:-<50}", "")?;

                for (sport, summary) in &latest_month.sports_breakdown {
                    writeln!(
                        file,
                        "{:<15} {:>8} {:>8} {:>10.1}",
                        sport,
                        summary.total_tss,
                        summary.workout_count,
                        summary.total_duration_hours
                    )?;
                }
                writeln!(file)?;
            }
        }
    }

    // Training recommendations
    if !report.training_recommendations.is_empty() {
        writeln!(file, "TRAINING RECOMMENDATIONS")?;
        writeln!(file, "-")?;
        for recommendation in &report.training_recommendations {
            writeln!(file, "• {}", recommendation)?;
        }
        writeln!(file)?;
    }

    // Footer
    writeln!(file, "=")?;
    writeln!(file, "End of Report")?;

    Ok(())
}

/// Export workout summaries in a simple text table format
pub fn export_workout_summaries_text<P: AsRef<Path>>(
    workouts: &[&crate::models::Workout],
    output_path: P,
) -> Result<(), ExportError> {
    let mut file = std::fs::File::create(output_path)?;

    writeln!(file, "WORKOUT SUMMARIES")?;
    writeln!(file, "=")?;
    writeln!(file)?;

    writeln!(file, "{:<12} {:<10} {:<10} {:<8} {:<8} {:<8} {:<8} {:<15}",
             "Date", "Sport", "Type", "Hours", "TSS", "Avg HR", "Avg Pwr", "Distance (km)")?;
    writeln!(file, "{:-<80}", "")?;

    for workout in workouts {
        let duration_hours = rust_decimal::Decimal::from(workout.duration_seconds) / rust_decimal::Decimal::from(3600);
        let distance_km = workout.summary.total_distance
            .map(|d| d / rust_decimal::Decimal::from(1000));

        writeln!(
            file,
            "{:<12} {:<10} {:<10} {:>8.1} {:>8} {:>8} {:>8} {:>15}",
            workout.date.format("%Y-%m-%d"),
            format!("{:?}", workout.sport),
            format!("{:?}", workout.workout_type),
            duration_hours,
            workout.summary.tss.map_or("-".to_string(), |v| v.to_string()),
            workout.summary.avg_heart_rate.map_or("-".to_string(), |v| v.to_string()),
            workout.summary.avg_power.map_or("-".to_string(), |v| v.to_string()),
            distance_km.map_or("-".to_string(), |v| format!("{:.1}", v))
        )?;
    }

    writeln!(file)?;
    writeln!(file, "Total workouts: {}", workouts.len())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::{TrainingReport, TrainingReportSummary, DateRange, PmcAnalysisReport};
    use chrono::{NaiveDate, Utc};
    use rust_decimal_macros::dec;
    use tempfile::NamedTempFile;

    #[test]
    fn test_export_training_report_text() {
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
            pmc_analysis: Some(PmcAnalysisReport {
                current_ctl: dec!(45.5),
                current_atl: dec!(65.2),
                current_tsb: dec!(-19.7),
                fitness_trend: "Stable".to_string(),
                fatigue_trend: "Increasing".to_string(),
                form_trend: "Declining".to_string(),
                recommendations: vec!["Consider reducing training load".to_string()],
            }),
            weekly_summaries: Vec::new(),
            monthly_summaries: Vec::new(),
            zone_analysis: Vec::new(),
            training_recommendations: vec!["Focus on recovery".to_string()],
        };

        let temp_file = NamedTempFile::new().unwrap();
        let result = export_training_report(&report, temp_file.path());

        assert!(result.is_ok());

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(content.contains("TRAINING REPORT"));
        assert!(content.contains("Athlete: test_athlete"));
        assert!(content.contains("Total Workouts: 20"));
        assert!(content.contains("Total TSS: 1500"));
        assert!(content.contains("PERFORMANCE MANAGEMENT CHART"));
        assert!(content.contains("Current Chronic Training Load (CTL): 45.5"));
        assert!(content.contains("Fatigued (monitor closely)"));
        assert!(content.contains("TRAINING RECOMMENDATIONS"));
        assert!(content.contains("Focus on recovery"));
    }

    #[test]
    fn test_export_workout_summaries_text() {
        use crate::models::{Workout, Sport, WorkoutType, DataSource, WorkoutSummary};

        let workout = Workout {
            id: "test_workout".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 9, 23).unwrap(),
            sport: Sport::Cycling,
            duration_seconds: 3600,
            workout_type: WorkoutType::Endurance,
            data_source: DataSource::Power,
            raw_data: None,
            summary: WorkoutSummary {
                tss: Some(dec!(85)),
                avg_heart_rate: Some(150),
                avg_power: Some(220),
                total_distance: Some(dec!(25000)),
                ..WorkoutSummary::default()
            },
            notes: None,
            athlete_id: Some("test_athlete".to_string()),
            source: None,
        };

        let workouts = vec![&workout];
        let temp_file = NamedTempFile::new().unwrap();
        let result = export_workout_summaries_text(&workouts, temp_file.path());

        assert!(result.is_ok());

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(content.contains("WORKOUT SUMMARIES"));
        assert!(content.contains("2024-09-23"));
        assert!(content.contains("Cycling"));
        assert!(content.contains("85")); // TSS
        assert!(content.contains("150")); // Avg HR
        assert!(content.contains("220")); // Avg Power
        assert!(content.contains("25.0")); // Distance in km
    }
}