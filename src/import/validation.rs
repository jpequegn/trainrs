use crate::models::{DataPoint, Sport, Workout};
use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;

/// Validate and clean workout data
pub struct WorkoutValidator;

impl WorkoutValidator {
    /// Validate a complete workout
    pub fn validate_workout(workout: &mut Workout) -> Result<()> {
        // Validate basic workout fields
        Self::validate_basic_fields(workout)?;

        // Validate and clean data points if present
        if let Some(ref mut raw_data) = workout.raw_data {
            Self::validate_data_points(raw_data)?;
            Self::validate_sport_specific_data(raw_data, &workout.sport)?;
        }

        Ok(())
    }

    /// Validate basic workout fields
    fn validate_basic_fields(workout: &Workout) -> Result<()> {
        // Check duration is positive
        if workout.duration_seconds == 0 {
            anyhow::bail!(
                "Workout duration must be positive, got: {}",
                workout.duration_seconds
            );
        }

        // Check date is reasonable (not in future, not too far in past)
        let now = Utc::now().date_naive();
        let min_date = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();

        if workout.date > now {
            anyhow::bail!("Workout date cannot be in the future: {}", workout.date);
        }

        if workout.date < min_date {
            anyhow::bail!("Workout date is too far in the past: {}", workout.date);
        }

        Ok(())
    }

    /// Validate and clean data points
    fn validate_data_points(data_points: &mut Vec<DataPoint>) -> Result<()> {
        if data_points.is_empty() {
            return Ok(());
        }

        // Sort by timestamp
        data_points.sort_by_key(|dp| dp.timestamp);

        // Remove duplicates by timestamp
        data_points.dedup_by_key(|dp| dp.timestamp);

        // Validate each data point
        for point in data_points.iter_mut() {
            Self::validate_data_point(point)?;
        }

        Ok(())
    }

    /// Validate and clean a single data point
    fn validate_data_point(point: &mut DataPoint) -> Result<()> {
        // Clean heart rate (30-220 bpm reasonable range)
        if let Some(hr) = point.heart_rate {
            if !(30..=220).contains(&hr) {
                point.heart_rate = None; // Remove invalid heart rate
            }
        }

        // Clean power (0-2000W reasonable range for cycling)
        if let Some(power) = point.power {
            if power > 2000 {
                point.power = None; // Remove invalid power
            }
        }

        // Clean pace (reasonable running/cycling speeds)
        if let Some(pace) = point.pace {
            if pace <= Decimal::ZERO || pace > Decimal::new(1000, 0) {
                point.pace = None; // Remove invalid pace
            }
        }

        // Clean speed (reasonable range for human activities)
        if let Some(speed) = point.speed {
            if speed < Decimal::ZERO || speed > Decimal::new(100, 0) {
                // 100 m/s = 360 km/h
                point.speed = None; // Remove invalid speed
            }
        }

        // Clean cadence (0-300 reasonable range)
        if let Some(cadence) = point.cadence {
            if cadence > 300 {
                point.cadence = None; // Remove invalid cadence
            }
        }

        // Clean distance (must be non-negative)
        if let Some(distance) = point.distance {
            if distance < Decimal::ZERO {
                point.distance = None; // Remove negative distance
            }
        }

        // Clean running dynamics fields
        if let Some(gct) = point.ground_contact_time {
            // Ground contact time should be between 100-500ms for humans
            if !(100..=500).contains(&gct) {
                point.ground_contact_time = None;
            }
        }

        if let Some(vo) = point.vertical_oscillation {
            // Vertical oscillation should be between 40-200mm (4-20cm)
            if !(40..=200).contains(&vo) {
                point.vertical_oscillation = None;
            }
        }

        if let Some(stride) = point.stride_length {
            // Stride length should be between 0.5-3.0 meters for humans
            if stride < Decimal::new(5, 1) || stride > Decimal::new(30, 1) {
                point.stride_length = None;
            }
        }

        // Clean swimming fields
        if let Some(strokes) = point.stroke_count {
            // Stroke count should be reasonable (0-100 per length/lap)
            if strokes > 100 {
                point.stroke_count = None;
            }
        }

        if let Some(stroke_type) = point.stroke_type {
            // Stroke type should be valid enum value (0-6)
            if stroke_type > 6 {
                point.stroke_type = None;
            }
        }

        if let Some(lap) = point.lap_number {
            // Lap number should be positive and reasonable (max 1000 laps)
            if lap == 0 || lap > 1000 {
                point.lap_number = None;
            }
        }

        Ok(())
    }

    /// Validate sport-specific data requirements and consistency
    fn validate_sport_specific_data(data_points: &mut [DataPoint], sport: &Sport) -> Result<()> {
        for point in data_points.iter_mut() {
            match sport {
                Sport::Running => {
                    Self::validate_running_data(point)?;
                }
                Sport::Swimming => {
                    Self::validate_swimming_data(point)?;
                }
                Sport::Cycling => {
                    Self::validate_cycling_data(point)?;
                }
                Sport::Triathlon => {
                    // Triathlon can have mixed data types, so validate all
                    Self::validate_running_data(point)?;
                    Self::validate_swimming_data(point)?;
                    Self::validate_cycling_data(point)?;
                }
                _ => {
                    // For other sports, apply general validation
                    Self::validate_general_sport_data(point)?;
                }
            }
        }

        Ok(())
    }

    /// Validate running-specific data
    fn validate_running_data(point: &mut DataPoint) -> Result<()> {
        // Running cadence should typically be 120-220 steps per minute
        if let Some(cadence) = point.cadence {
            if !(120..=220).contains(&cadence) {
                // Don't remove, but could log warning
                // For now, keep the value as it might be valid in edge cases
            }
        }

        // Running speed should be reasonable (1-15 m/s for humans)
        if let Some(speed) = point.speed {
            if speed < Decimal::new(1, 0) || speed > Decimal::new(15, 0) {
                point.speed = None; // Remove unrealistic running speeds
            }
        }

        // Running dynamics should only be present for running
        // (ground_contact_time, vertical_oscillation, stride_length are already validated above)

        // Remove swimming-specific data that shouldn't be in running workouts
        if point.stroke_count.is_some() || point.stroke_type.is_some() {
            point.stroke_count = None;
            point.stroke_type = None;
        }

        Ok(())
    }

    /// Validate swimming-specific data
    fn validate_swimming_data(point: &mut DataPoint) -> Result<()> {
        // Swimming speed should be much slower than running (0.5-3 m/s)
        if let Some(speed) = point.speed {
            if speed < Decimal::new(5, 1) || speed > Decimal::new(3, 0) {
                point.speed = None; // Remove unrealistic swimming speeds
            }
        }

        // Swimming typically doesn't have running dynamics
        if point.ground_contact_time.is_some() ||
           point.vertical_oscillation.is_some() ||
           point.stride_length.is_some() {
            point.ground_contact_time = None;
            point.vertical_oscillation = None;
            point.stride_length = None;
        }

        // Swimming power is uncommon but possible (swimming power meters exist)
        // Keep power data if present

        Ok(())
    }

    /// Validate cycling-specific data
    fn validate_cycling_data(point: &mut DataPoint) -> Result<()> {
        // Cycling power can be quite high (professional cyclists can exceed 1500W)
        // Already validated in general validation with 2000W limit

        // Cycling cadence should typically be 40-150 RPM
        if let Some(cadence) = point.cadence {
            if !(40..=150).contains(&cadence) {
                // Don't remove, but could log warning
                // Keep the value as it might be valid
            }
        }

        // Cycling speed should be reasonable (2-30 m/s)
        if let Some(speed) = point.speed {
            if speed < Decimal::new(2, 0) || speed > Decimal::new(30, 0) {
                point.speed = None; // Remove unrealistic cycling speeds
            }
        }

        // Remove sport-specific data that shouldn't be in cycling workouts
        if point.ground_contact_time.is_some() ||
           point.vertical_oscillation.is_some() ||
           point.stride_length.is_some() {
            point.ground_contact_time = None;
            point.vertical_oscillation = None;
            point.stride_length = None;
        }

        if point.stroke_count.is_some() || point.stroke_type.is_some() {
            point.stroke_count = None;
            point.stroke_type = None;
        }

        Ok(())
    }

    /// Validate general sport data (for rowing, cross training, etc.)
    fn validate_general_sport_data(point: &mut DataPoint) -> Result<()> {
        // For general sports, apply conservative speed limits
        if let Some(speed) = point.speed {
            if speed < Decimal::ZERO || speed > Decimal::new(20, 0) {
                point.speed = None; // Remove unrealistic speeds
            }
        }

        // Power can be present for rowing and other sports
        // Already validated in general validation

        // Remove sport-specific data that shouldn't be in general sports
        if point.ground_contact_time.is_some() ||
           point.vertical_oscillation.is_some() ||
           point.stride_length.is_some() {
            point.ground_contact_time = None;
            point.vertical_oscillation = None;
            point.stride_length = None;
        }

        if point.stroke_count.is_some() || point.stroke_type.is_some() {
            point.stroke_count = None;
            point.stroke_type = None;
        }

        Ok(())
    }

    /// Validate multi-sport transitions for triathlon workouts
    #[allow(dead_code)]
    pub fn validate_sport_transitions(data_points: &[DataPoint]) -> Result<()> {
        let mut transition_count = 0;

        for point in data_points {
            if point.sport_transition == Some(true) {
                transition_count += 1;
            }
        }

        // Triathlon should have 0-2 transitions (swim->bike, bike->run)
        // More than 5 transitions might indicate data quality issues
        if transition_count > 5 {
            anyhow::bail!(
                "Too many sport transitions detected ({}). This may indicate data quality issues.",
                transition_count
            );
        }

        Ok(())
    }

    /// Validate data consistency across the workout
    #[allow(dead_code)]
    pub fn validate_workout_consistency(workout: &Workout) -> Result<()> {
        if let Some(ref raw_data) = workout.raw_data {
            // Check for sport transitions in triathlon workouts
            if workout.sport == Sport::Triathlon {
                Self::validate_sport_transitions(raw_data)?;
            }

            // Validate lap progression
            Self::validate_lap_progression(raw_data)?;

            // Validate timestamp progression
            Self::validate_timestamp_progression(raw_data)?;
        }

        Ok(())
    }

    /// Validate lap number progression
    #[allow(dead_code)]
    fn validate_lap_progression(data_points: &[DataPoint]) -> Result<()> {
        let lap_numbers: Vec<u16> = data_points.iter()
            .filter_map(|dp| dp.lap_number)
            .collect();

        if lap_numbers.is_empty() {
            return Ok(()); // No lap data to validate
        }

        // Check for reasonable lap progression
        let min_lap = *lap_numbers.iter().min().unwrap();
        let max_lap = *lap_numbers.iter().max().unwrap();

        if min_lap == 0 {
            anyhow::bail!("Lap numbers should start from 1, found lap 0");
        }

        // Allow some flexibility in lap numbering but catch obvious errors
        if max_lap - min_lap > 500 {
            anyhow::bail!(
                "Unrealistic lap range: {} to {}. This may indicate data corruption.",
                min_lap, max_lap
            );
        }

        Ok(())
    }

    /// Validate timestamp progression
    #[allow(dead_code)]
    fn validate_timestamp_progression(data_points: &[DataPoint]) -> Result<()> {
        if data_points.len() < 2 {
            return Ok(());
        }

        let mut prev_timestamp = data_points[0].timestamp;

        for (i, point) in data_points.iter().enumerate().skip(1) {
            // Allow equal timestamps (multiple sensors at same time)
            // but catch major timestamp issues
            if point.timestamp < prev_timestamp && (prev_timestamp - point.timestamp) > 300 {
                anyhow::bail!(
                    "Major timestamp regression at data point {}: {} -> {}",
                    i, prev_timestamp, point.timestamp
                );
            }

            prev_timestamp = point.timestamp;
        }

        Ok(())
    }

    /// Clean elevation data by removing outliers
    #[allow(dead_code)]
    pub fn clean_elevation_data(data_points: &mut [DataPoint]) {
        if data_points.len() < 3 {
            return;
        }

        // Calculate median elevation for outlier detection
        let mut elevations: Vec<i16> = data_points.iter().filter_map(|dp| dp.elevation).collect();

        if elevations.is_empty() {
            return;
        }

        elevations.sort();
        let median = elevations[elevations.len() / 2];

        // Calculate median absolute deviation
        let mad: f64 = elevations
            .iter()
            .map(|&x| (x - median).abs() as f64)
            .sum::<f64>()
            / elevations.len() as f64;

        // Remove elevation outliers (more than 5 MADs from median)
        let threshold = 5.0 * mad;
        for point in data_points.iter_mut() {
            if let Some(elevation) = point.elevation {
                if (elevation - median).abs() as f64 > threshold {
                    point.elevation = None;
                }
            }
        }
    }

    /// Interpolate missing timestamps
    #[allow(dead_code)]
    pub fn interpolate_timestamps(data_points: &mut [DataPoint], _start_time: DateTime<Utc>) {
        if data_points.is_empty() {
            return;
        }

        // Check if timestamps are sequential
        let mut needs_interpolation = false;
        for i in 1..data_points.len() {
            if data_points[i].timestamp <= data_points[i - 1].timestamp {
                needs_interpolation = true;
                break;
            }
        }

        // If timestamps need fixing, assign sequential timestamps
        if needs_interpolation {
            for (i, point) in data_points.iter_mut().enumerate() {
                point.timestamp = i as u32;
            }
        }
    }
}
