use crate::models::{DataPoint, Workout};
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
