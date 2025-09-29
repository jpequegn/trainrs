use crate::models::{Workout, AthleteProfile};
use crate::pmc::{PmcMetrics, PmcCalculator};
use chrono::{NaiveDate, Datelike};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use thiserror::Error;

pub mod csv;
pub mod json;
pub mod text;

/// Export format types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    Csv,
    Json,
    Text,
    Html,
    Pdf,
}

impl ExportFormat {
    pub fn from_str(s: &str) -> Result<Self, ExportError> {
        match s.to_lowercase().as_str() {
            "csv" => Ok(ExportFormat::Csv),
            "json" => Ok(ExportFormat::Json),
            "text" | "txt" => Ok(ExportFormat::Text),
            "html" => Ok(ExportFormat::Html),
            "pdf" => Ok(ExportFormat::Pdf),
            _ => Err(ExportError::UnsupportedFormat(s.to_string())),
        }
    }
}

/// Export data type categories
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportType {
    WorkoutSummaries,
    PmcData,
    ZoneAnalysis,
    WeeklySummary,
    MonthlySummary,
    TrainingReport,
    TrainingPeaksFormat,
}

/// Date range filter for exports
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DateRange {
    pub start: Option<NaiveDate>,
    pub end: Option<NaiveDate>,
}

impl DateRange {
    pub fn new(start: Option<NaiveDate>, end: Option<NaiveDate>) -> Self {
        DateRange { start, end }
    }

    /// Check if a date falls within this range
    pub fn contains(&self, date: &NaiveDate) -> bool {
        let after_start = self.start.map_or(true, |start| date >= &start);
        let before_end = self.end.map_or(true, |end| date <= &end);
        after_start && before_end
    }

    /// Filter workouts by date range
    pub fn filter_workouts<'a>(&self, workouts: &'a [Workout]) -> Vec<&'a Workout> {
        workouts.iter().filter(|w| self.contains(&w.date)).collect()
    }
}

/// Export configuration options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportOptions {
    pub format: ExportFormat,
    pub export_type: ExportType,
    pub date_range: DateRange,
    pub include_raw_data: bool,
    pub athlete_id: Option<String>,
    pub template: Option<String>,
}

impl Default for ExportOptions {
    fn default() -> Self {
        ExportOptions {
            format: ExportFormat::Csv,
            export_type: ExportType::WorkoutSummaries,
            date_range: DateRange::new(None, None),
            include_raw_data: false,
            athlete_id: None,
            template: None,
        }
    }
}

/// Export errors
#[derive(Error, Debug)]
pub enum ExportError {
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Date parsing error: {0}")]
    #[allow(dead_code)]
    DateParseError(String),
    #[error("Insufficient data: {0}")]
    InsufficientData(String),
    #[error("Configuration error: {0}")]
    #[allow(dead_code)]
    ConfigurationError(String),
    #[error("PMC calculation error: {0}")]
    PmcError(#[from] crate::pmc::PmcError),
}

/// Weekly training summary
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeeklySummary {
    pub week_start: NaiveDate,
    pub week_end: NaiveDate,
    pub year: i32,
    pub week_number: u32,
    pub total_tss: Decimal,
    pub workout_count: u16,
    pub total_duration_hours: Decimal,
    pub total_distance_km: Option<Decimal>,
    pub sports_breakdown: BTreeMap<String, SportSummary>,
    pub avg_daily_tss: Decimal,
}

/// Monthly training summary
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonthlySummary {
    pub year: i32,
    pub month: u32,
    pub month_name: String,
    pub total_tss: Decimal,
    pub workout_count: u16,
    pub total_duration_hours: Decimal,
    pub total_distance_km: Option<Decimal>,
    pub sports_breakdown: BTreeMap<String, SportSummary>,
    pub avg_daily_tss: Decimal,
    pub weekly_summaries: Vec<WeeklySummary>,
}

/// Sport-specific summary data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SportSummary {
    pub workout_count: u16,
    pub total_tss: Decimal,
    pub total_duration_hours: Decimal,
    pub total_distance_km: Option<Decimal>,
    pub avg_intensity_factor: Option<Decimal>,
}

/// Zone analysis data for export
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZoneAnalysis {
    pub date: NaiveDate,
    pub workout_id: String,
    pub sport: String,
    pub zone_distribution: BTreeMap<u8, ZoneTimeData>,
    pub dominant_zone: Option<u8>,
    pub time_in_zones: BTreeMap<u8, u32>, // Zone -> seconds
}

/// Time spent in a specific zone
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZoneTimeData {
    pub zone_number: u8,
    pub time_seconds: u32,
    pub percentage: Decimal,
    pub zone_name: String,
}

/// Training report containing comprehensive analysis
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingReport {
    pub athlete_id: Option<String>,
    pub date_range: DateRange,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub summary_stats: TrainingReportSummary,
    pub pmc_analysis: Option<PmcAnalysisReport>,
    pub weekly_summaries: Vec<WeeklySummary>,
    pub monthly_summaries: Vec<MonthlySummary>,
    pub zone_analysis: Vec<ZoneAnalysis>,
    pub training_recommendations: Vec<String>,
}

/// High-level training statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingReportSummary {
    pub total_workouts: u16,
    pub total_tss: Decimal,
    pub total_duration_hours: Decimal,
    pub avg_tss_per_workout: Decimal,
    pub most_frequent_sport: String,
    pub date_range_days: u16,
    pub training_consistency: Decimal, // Percentage of days with training
}

/// PMC analysis for reports
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PmcAnalysisReport {
    pub current_ctl: Decimal,
    pub current_atl: Decimal,
    pub current_tsb: Decimal,
    pub fitness_trend: String,
    pub fatigue_trend: String,
    pub form_trend: String,
    pub recommendations: Vec<String>,
}

/// Main export manager
pub struct ExportManager {
    pmc_calculator: PmcCalculator,
}

impl ExportManager {
    pub fn new() -> Self {
        ExportManager {
            pmc_calculator: PmcCalculator::new(),
        }
    }

    /// Export data based on options
    pub fn export<P: AsRef<Path>>(
        &self,
        workouts: &[Workout],
        athlete_profile: Option<&AthleteProfile>,
        options: &ExportOptions,
        output_path: P,
    ) -> Result<(), ExportError> {
        let filtered_workouts = options.date_range.filter_workouts(workouts);

        match (&options.format, &options.export_type) {
            (ExportFormat::Csv, ExportType::WorkoutSummaries) => {
                csv::export_workout_summaries(&filtered_workouts, output_path)
            }
            (ExportFormat::Csv, ExportType::PmcData) => {
                let pmc_data = self.calculate_pmc_data(&filtered_workouts, &options.date_range)?;
                csv::export_pmc_data(&pmc_data, output_path)
            }
            (ExportFormat::Json, ExportType::TrainingReport) => {
                let report = self.generate_training_report(&filtered_workouts, athlete_profile, &options.date_range)?;
                json::export_training_report(&report, output_path)
            }
            (ExportFormat::Text, ExportType::TrainingReport) => {
                let report = self.generate_training_report(&filtered_workouts, athlete_profile, &options.date_range)?;
                text::export_training_report(&report, output_path)
            }
            _ => Err(ExportError::UnsupportedFormat(format!(
                "{:?} format for {:?} export type not yet implemented",
                options.format, options.export_type
            ))),
        }
    }

    /// Calculate PMC data for the given workouts and date range
    fn calculate_pmc_data(
        &self,
        workouts: &[&Workout],
        _date_range: &DateRange,
    ) -> Result<Vec<PmcMetrics>, ExportError> {
        let owned_workouts: Vec<Workout> = workouts.iter().map(|&w| w.clone()).collect();
        let daily_tss = self.pmc_calculator.aggregate_daily_tss(&owned_workouts);

        let start_date = _date_range.start.or_else(|| {
            workouts.iter().map(|w| w.date).min()
        }).ok_or(ExportError::InsufficientData("No workouts to analyze".to_string()))?;

        let end_date = _date_range.end.or_else(|| {
            workouts.iter().map(|w| w.date).max()
        }).ok_or(ExportError::InsufficientData("No workouts to analyze".to_string()))?;

        let pmc_series = self.pmc_calculator.calculate_pmc_series(&daily_tss, start_date, end_date)?;
        Ok(pmc_series)
    }

    /// Generate comprehensive training report
    fn generate_training_report(
        &self,
        workouts: &[&Workout],
        athlete_profile: Option<&AthleteProfile>,
        _date_range: &DateRange,
    ) -> Result<TrainingReport, ExportError> {
        let summary_stats = self.calculate_summary_stats(workouts, _date_range)?;
        let weekly_summaries = self.generate_weekly_summaries(workouts)?;
        let monthly_summaries = self.generate_monthly_summaries(workouts)?;

        let pmc_analysis = if !workouts.is_empty() {
            let pmc_data = self.calculate_pmc_data(workouts, _date_range)?;
            if let Some(latest_pmc) = pmc_data.last() {
                Some(PmcAnalysisReport {
                    current_ctl: latest_pmc.ctl,
                    current_atl: latest_pmc.atl,
                    current_tsb: latest_pmc.tsb,
                    fitness_trend: "Stable".to_string(), // TODO: Implement trend analysis
                    fatigue_trend: "Stable".to_string(),
                    form_trend: "Neutral".to_string(),
                    recommendations: self.pmc_calculator.generate_recommendations(latest_pmc),
                })
            } else {
                None
            }
        } else {
            None
        };

        Ok(TrainingReport {
            athlete_id: athlete_profile.map(|p| p.id.clone()),
            date_range: _date_range.clone(),
            generated_at: chrono::Utc::now(),
            summary_stats,
            pmc_analysis,
            weekly_summaries,
            monthly_summaries,
            zone_analysis: Vec::new(), // TODO: Implement zone analysis
            training_recommendations: Vec::new(), // TODO: Implement recommendations
        })
    }

    /// Calculate high-level summary statistics
    fn calculate_summary_stats(
        &self,
        workouts: &[&Workout],
        _date_range: &DateRange,
    ) -> Result<TrainingReportSummary, ExportError> {
        if workouts.is_empty() {
            return Err(ExportError::InsufficientData("No workouts to analyze".to_string()));
        }

        let total_workouts = workouts.len() as u16;
        let total_tss: Decimal = workouts
            .iter()
            .filter_map(|w| w.summary.tss)
            .sum();

        let total_duration_hours: Decimal = workouts
            .iter()
            .map(|w| Decimal::from(w.duration_seconds) / Decimal::from(3600))
            .sum();

        let avg_tss_per_workout = if total_workouts > 0 {
            total_tss / Decimal::from(total_workouts)
        } else {
            Decimal::ZERO
        };

        // Find most frequent sport
        let mut sport_counts = BTreeMap::new();
        for workout in workouts {
            *sport_counts.entry(format!("{:?}", workout.sport)).or_insert(0) += 1;
        }

        let most_frequent_sport = sport_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(sport, _)| sport.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        // Calculate date range and training consistency
        let first_date = workouts.iter().map(|w| w.date).min().unwrap();
        let last_date = workouts.iter().map(|w| w.date).max().unwrap();
        let date_range_days = (last_date - first_date).num_days() as u16 + 1;

        let training_days = workouts.iter().map(|w| w.date).collect::<std::collections::HashSet<_>>().len();
        let training_consistency = Decimal::from(training_days) / Decimal::from(date_range_days) * Decimal::from(100);

        Ok(TrainingReportSummary {
            total_workouts,
            total_tss,
            total_duration_hours,
            avg_tss_per_workout,
            most_frequent_sport,
            date_range_days,
            training_consistency,
        })
    }

    /// Generate weekly training summaries
    fn generate_weekly_summaries(&self, workouts: &[&Workout]) -> Result<Vec<WeeklySummary>, ExportError> {
        let mut weekly_summaries = Vec::new();
        let mut weekly_data: BTreeMap<(i32, u32), Vec<&Workout>> = BTreeMap::new();

        // Group workouts by week
        for workout in workouts {
            let year = workout.date.year();
            let week = workout.date.iso_week().week();
            weekly_data.entry((year, week)).or_insert_with(Vec::new).push(workout);
        }

        // Generate summary for each week
        for ((year, week_num), week_workouts) in weekly_data {
            let week_start = chrono::NaiveDate::from_isoywd_opt(year, week_num, chrono::Weekday::Mon).unwrap();
            let week_end = chrono::NaiveDate::from_isoywd_opt(year, week_num, chrono::Weekday::Sun).unwrap();

            let total_tss: Decimal = week_workouts.iter().filter_map(|w| w.summary.tss).sum();
            let workout_count = week_workouts.len() as u16;
            let total_duration_hours: Decimal = week_workouts
                .iter()
                .map(|w| Decimal::from(w.duration_seconds) / Decimal::from(3600))
                .sum();

            let total_distance_km: Option<Decimal> = {
                let distances: Vec<Decimal> = week_workouts
                    .iter()
                    .filter_map(|w| w.summary.total_distance)
                    .map(|d| d / Decimal::from(1000)) // Convert meters to km
                    .collect();
                if distances.is_empty() { None } else { Some(distances.iter().sum()) }
            };

            let avg_daily_tss = total_tss / Decimal::from(7);

            // Generate sports breakdown
            let mut sports_breakdown = BTreeMap::new();
            for workout in &week_workouts {
                let sport_name = format!("{:?}", workout.sport);
                let entry = sports_breakdown.entry(sport_name).or_insert(SportSummary {
                    workout_count: 0,
                    total_tss: Decimal::ZERO,
                    total_duration_hours: Decimal::ZERO,
                    total_distance_km: None,
                    avg_intensity_factor: None,
                });

                entry.workout_count += 1;
                entry.total_tss += workout.summary.tss.unwrap_or(Decimal::ZERO);
                entry.total_duration_hours += Decimal::from(workout.duration_seconds) / Decimal::from(3600);
            }

            weekly_summaries.push(WeeklySummary {
                week_start,
                week_end,
                year,
                week_number: week_num,
                total_tss,
                workout_count,
                total_duration_hours,
                total_distance_km,
                sports_breakdown,
                avg_daily_tss,
            });
        }

        weekly_summaries.sort_by_key(|w| (w.year, w.week_number));
        Ok(weekly_summaries)
    }

    /// Generate monthly training summaries
    fn generate_monthly_summaries(&self, workouts: &[&Workout]) -> Result<Vec<MonthlySummary>, ExportError> {
        let mut monthly_summaries = Vec::new();
        let mut monthly_data: BTreeMap<(i32, u32), Vec<&Workout>> = BTreeMap::new();

        // Group workouts by month
        for workout in workouts {
            let year = workout.date.year();
            let month = workout.date.month();
            monthly_data.entry((year, month)).or_insert_with(Vec::new).push(workout);
        }

        let weekly_summaries = self.generate_weekly_summaries(workouts)?;

        // Generate summary for each month
        for ((year, month_num), month_workouts) in monthly_data {
            let month_name = match month_num {
                1 => "January", 2 => "February", 3 => "March", 4 => "April",
                5 => "May", 6 => "June", 7 => "July", 8 => "August",
                9 => "September", 10 => "October", 11 => "November", 12 => "December",
                _ => "Unknown",
            }.to_string();

            let total_tss: Decimal = month_workouts.iter().filter_map(|w| w.summary.tss).sum();
            let workout_count = month_workouts.len() as u16;
            let total_duration_hours: Decimal = month_workouts
                .iter()
                .map(|w| Decimal::from(w.duration_seconds) / Decimal::from(3600))
                .sum();

            let total_distance_km: Option<Decimal> = {
                let distances: Vec<Decimal> = month_workouts
                    .iter()
                    .filter_map(|w| w.summary.total_distance)
                    .map(|d| d / Decimal::from(1000))
                    .collect();
                if distances.is_empty() { None } else { Some(distances.iter().sum()) }
            };

            let days_in_month = chrono::NaiveDate::from_ymd_opt(year, month_num, 1)
                .unwrap()
                .with_month(month_num + 1)
                .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap())
                .pred_opt()
                .unwrap()
                .day() as u16;

            let avg_daily_tss = total_tss / Decimal::from(days_in_month);

            // Generate sports breakdown
            let mut sports_breakdown = BTreeMap::new();
            for workout in &month_workouts {
                let sport_name = format!("{:?}", workout.sport);
                let entry = sports_breakdown.entry(sport_name).or_insert(SportSummary {
                    workout_count: 0,
                    total_tss: Decimal::ZERO,
                    total_duration_hours: Decimal::ZERO,
                    total_distance_km: None,
                    avg_intensity_factor: None,
                });

                entry.workout_count += 1;
                entry.total_tss += workout.summary.tss.unwrap_or(Decimal::ZERO);
                entry.total_duration_hours += Decimal::from(workout.duration_seconds) / Decimal::from(3600);
            }

            // Filter weekly summaries for this month
            let month_weekly_summaries = weekly_summaries
                .iter()
                .filter(|w| w.year == year && w.week_start.month() == month_num)
                .cloned()
                .collect();

            monthly_summaries.push(MonthlySummary {
                year,
                month: month_num,
                month_name,
                total_tss,
                workout_count,
                total_duration_hours,
                total_distance_km,
                sports_breakdown,
                avg_daily_tss,
                weekly_summaries: month_weekly_summaries,
            });
        }

        monthly_summaries.sort_by_key(|m| (m.year, m.month));
        Ok(monthly_summaries)
    }
}

impl Default for ExportManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DataSource, Sport, WorkoutSummary, WorkoutType};
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

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
                ..WorkoutSummary::default()
            },
            notes: None,
            athlete_id: Some("test_athlete".to_string()),
            source: None,
        }
    }

    #[test]
    fn test_date_range_contains() {
        let range = DateRange::new(
            Some(NaiveDate::from_ymd_opt(2024, 9, 1).unwrap()),
            Some(NaiveDate::from_ymd_opt(2024, 9, 30).unwrap()),
        );

        assert!(range.contains(&NaiveDate::from_ymd_opt(2024, 9, 15).unwrap()));
        assert!(!range.contains(&NaiveDate::from_ymd_opt(2024, 8, 31).unwrap()));
        assert!(!range.contains(&NaiveDate::from_ymd_opt(2024, 10, 1).unwrap()));
    }

    #[test]
    fn test_date_range_filter_workouts() {
        let workouts = vec![
            create_test_workout(NaiveDate::from_ymd_opt(2024, 8, 31).unwrap(), dec!(50)),
            create_test_workout(NaiveDate::from_ymd_opt(2024, 9, 15).unwrap(), dec!(75)),
            create_test_workout(NaiveDate::from_ymd_opt(2024, 10, 1).unwrap(), dec!(60)),
        ];

        let range = DateRange::new(
            Some(NaiveDate::from_ymd_opt(2024, 9, 1).unwrap()),
            Some(NaiveDate::from_ymd_opt(2024, 9, 30).unwrap()),
        );

        let filtered = range.filter_workouts(&workouts);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].date, NaiveDate::from_ymd_opt(2024, 9, 15).unwrap());
    }

    #[test]
    fn test_export_format_from_str() {
        assert_eq!(ExportFormat::from_str("csv").unwrap(), ExportFormat::Csv);
        assert_eq!(ExportFormat::from_str("JSON").unwrap(), ExportFormat::Json);
        assert_eq!(ExportFormat::from_str("text").unwrap(), ExportFormat::Text);
        assert_eq!(ExportFormat::from_str("txt").unwrap(), ExportFormat::Text);

        assert!(ExportFormat::from_str("invalid").is_err());
    }
}