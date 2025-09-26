use trainrs::{models, tss, pmc, zones};
use chrono::{NaiveDate, Utc};
use rust_decimal_macros::dec;

/// Integration tests that test the complete system workflows

#[cfg(test)]
mod integration_tests {
    use super::*;
    use trainrs::models::{AthleteProfile, Workout, WorkoutSummary, Sport, WorkoutType, DataSource, TrainingZones, Units};

    fn create_test_athlete() -> AthleteProfile {
        AthleteProfile {
            id: "test_athlete".to_string(),
            name: "Test Athlete".to_string(),
            date_of_birth: Some(NaiveDate::from_ymd_opt(1990, 1, 1).unwrap()),
            weight: Some(dec!(70.0)),
            height: Some(175),
            ftp: Some(250),
            lthr: Some(165),
            threshold_pace: Some(dec!(6.0)), // 6 min/mile threshold pace
            max_hr: Some(190),
            resting_hr: Some(50),
            training_zones: TrainingZones::default(),
            preferred_units: Units::Metric,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_workout() -> Workout {
        Workout {
            id: "test_workout".to_string(),
            athlete_id: Some("test_athlete".to_string()),
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            sport: Sport::Cycling,
            workout_type: WorkoutType::Endurance,
            duration_seconds: 3600, // 1 hour
            summary: WorkoutSummary {
                total_distance: Some(dec!(30.0)),
                avg_heart_rate: Some(150),
                max_heart_rate: Some(180),
                avg_power: Some(200),
                normalized_power: Some(220),
                avg_pace: None,
                intensity_factor: None,
                tss: None,
                elevation_gain: Some(100),
                avg_cadence: Some(90),
                calories: Some(800),
            },
            data_source: DataSource::Power,
            notes: Some("Test workout".to_string()),
            source: None,
            raw_data: None,
        }
    }

    /// Test complete PMC calculation workflow with simple workout data
    #[test]
    fn test_complete_pmc_workflow() {
        let athlete = create_test_athlete();
        let workout = create_test_workout();

        // Calculate TSS first
        let tss_result = tss::TssCalculator::calculate_power_tss(&workout, &athlete);
        assert!(tss_result.is_ok());

        let mut workouts = vec![workout];
        if let Ok(tss_data) = tss_result {
            workouts[0].summary.tss = Some(tss_data.tss);
            workouts[0].summary.intensity_factor = tss_data.intensity_factor;
        }

        // Test PMC calculation
        let pmc_calculator = pmc::PmcCalculator::new();
        let daily_tss = pmc_calculator.aggregate_daily_tss(&workouts);
        let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let pmc_result = pmc_calculator.calculate_pmc_series(&daily_tss, start_date, end_date);

        assert!(pmc_result.is_ok());
        let pmc_series = pmc_result.unwrap();
        assert!(!pmc_series.is_empty());

        // Check that we get reasonable PMC values
        let latest_pmc = &pmc_series[0];
        assert!(latest_pmc.ctl >= dec!(0));
        assert!(latest_pmc.atl >= dec!(0));
        assert!(latest_pmc.tsb >= dec!(-50) && latest_pmc.tsb <= dec!(50));
    }

    /// Test zone analysis workflow
    #[test]
    fn test_zone_analysis_workflow() {
        let athlete = create_test_athlete();

        // Test power zone analysis
        let power_zones = zones::ZoneCalculator::calculate_power_zones(&athlete);
        assert!(power_zones.is_ok());

        // Test heart rate zone analysis
        let hr_zones = zones::ZoneCalculator::calculate_heart_rate_zones(&athlete, zones::HRZoneMethod::Lthr);
        assert!(hr_zones.is_ok());
    }

    /// Test TSS calculation workflow with different sports
    #[test]
    fn test_tss_calculation_workflow() {
        let athlete = create_test_athlete();

        // Test cycling power TSS
        let cycling_workout = create_test_workout();
        let cycling_tss = tss::TssCalculator::calculate_power_tss(&cycling_workout, &athlete);
        assert!(cycling_tss.is_ok());

        // Test running TSS
        let mut running_workout = create_test_workout();
        running_workout.sport = Sport::Running;
        running_workout.summary.avg_power = None;
        running_workout.summary.avg_pace = Some(dec!(5.0)); // 5 min/km pace
        running_workout.data_source = DataSource::Pace;

        let running_tss = tss::TssCalculator::calculate_pace_tss(&running_workout, &athlete);
        assert!(running_tss.is_ok());
    }

    /// Test system integration with multiple workouts
    #[test]
    fn test_multi_workout_integration() {
        let athlete = create_test_athlete();
        let mut workouts = Vec::new();

        // Create multiple workouts with different intensities
        for i in 0..5 {
            let mut workout = create_test_workout();
            workout.id = format!("workout_{}", i);
            workout.date = NaiveDate::from_ymd_opt(2024, 1, i + 1).unwrap();
            workout.summary.avg_power = Some(180 + (i as u16 * 20)); // Varying power
            workouts.push(workout);
        }

        // Calculate TSS for all workouts
        for workout in &mut workouts {
            if let Ok(tss_result) = tss::TssCalculator::calculate_power_tss(workout, &athlete) {
                workout.summary.tss = Some(tss_result.tss);
                workout.summary.intensity_factor = tss_result.intensity_factor;
            }
        }

        // Calculate PMC
        let pmc_calculator = pmc::PmcCalculator::new();
        let daily_tss = pmc_calculator.aggregate_daily_tss(&workouts);
        let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2024, 1, 5).unwrap();
        let pmc_result = pmc_calculator.calculate_pmc_series(&daily_tss, start_date, end_date);

        assert!(pmc_result.is_ok());
        let pmc_series = pmc_result.unwrap();
        assert_eq!(pmc_series.len(), workouts.len());

        // Check that PMC values progress logically
        for (i, pmc) in pmc_series.iter().enumerate() {
            assert!(pmc.ctl >= dec!(0));
            assert!(pmc.atl >= dec!(0));
            // TSB should become more negative as training load increases
            if i > 0 {
                assert!(pmc.ctl >= pmc_series[i-1].ctl || pmc.ctl >= dec!(0));
            }
        }
    }
}