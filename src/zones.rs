use crate::models::{AthleteProfile, HeartRateZones, PowerZones, PaceZones, TrainingZones};
use anyhow::{anyhow, Result};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;

/// Errors that can occur during zone calculations
#[derive(Debug, thiserror::Error)]
pub enum ZoneError {
    #[error("Missing threshold value: {0}")]
    MissingThreshold(String),
    #[error("Invalid threshold value: {0}")]
    InvalidThreshold(String),
    #[error("Calculation error: {0}")]
    CalculationError(String),
}

/// Heart rate zone calculation methods
pub enum HRZoneMethod {
    /// Based on Lactate Threshold Heart Rate (preferred)
    LTHR,
    /// Based on Maximum Heart Rate (age-predicted or tested)
    MaxHR,
}

/// Zone calculation utilities and algorithms
pub struct ZoneCalculator;

impl ZoneCalculator {
    /// Calculate heart rate zones based on athlete profile
    pub fn calculate_heart_rate_zones(
        profile: &AthleteProfile,
        method: HRZoneMethod,
    ) -> Result<HeartRateZones> {
        match method {
            HRZoneMethod::LTHR => Self::hr_zones_from_lthr(profile),
            HRZoneMethod::MaxHR => Self::hr_zones_from_max_hr(profile),
        }
    }

    /// Calculate heart rate zones based on LTHR (Lactate Threshold Heart Rate)
    ///
    /// Zone calculations based on LTHR:
    /// - Z1: < 81% LTHR (Active Recovery)
    /// - Z2: 81-89% LTHR (Aerobic Base)
    /// - Z3: 90-93% LTHR (Aerobic)
    /// - Z4: 94-99% LTHR (Lactate Threshold)
    /// - Z5: 100%+ LTHR (VO2 Max)
    fn hr_zones_from_lthr(profile: &AthleteProfile) -> Result<HeartRateZones> {
        let lthr = profile.lthr.ok_or_else(|| {
            ZoneError::MissingThreshold("LTHR required for heart rate zone calculation".to_string())
        })?;

        Self::validate_heart_rate(lthr, "LTHR")?;

        let lthr_decimal = Decimal::from(lthr);

        Ok(HeartRateZones {
            zone1_max: Self::calculate_percentage(lthr_decimal, dec!(0.81))?,
            zone2_max: Self::calculate_percentage(lthr_decimal, dec!(0.89))?,
            zone3_max: Self::calculate_percentage(lthr_decimal, dec!(0.93))?,
            zone4_max: Self::calculate_percentage(lthr_decimal, dec!(0.99))?,
            zone5_max: lthr + 20, // Upper bound for Z5
        })
    }

    /// Calculate heart rate zones based on Maximum Heart Rate
    ///
    /// Zone calculations based on MaxHR (alternative method):
    /// - Z1: < 68% MaxHR (Active Recovery)
    /// - Z2: 69-83% MaxHR (Aerobic Base)
    /// - Z3: 84-94% MaxHR (Aerobic)
    /// - Z4: 95-105% MaxHR (Lactate Threshold)
    /// - Z5: 106%+ MaxHR (VO2 Max)
    fn hr_zones_from_max_hr(profile: &AthleteProfile) -> Result<HeartRateZones> {
        let max_hr = profile.max_hr.ok_or_else(|| {
            ZoneError::MissingThreshold("Max HR required for heart rate zone calculation".to_string())
        })?;

        Self::validate_heart_rate(max_hr, "Max HR")?;

        let max_hr_decimal = Decimal::from(max_hr);

        Ok(HeartRateZones {
            zone1_max: Self::calculate_percentage(max_hr_decimal, dec!(0.68))?,
            zone2_max: Self::calculate_percentage(max_hr_decimal, dec!(0.83))?,
            zone3_max: Self::calculate_percentage(max_hr_decimal, dec!(0.94))?,
            zone4_max: Self::calculate_percentage(max_hr_decimal, dec!(1.05))?,
            zone5_max: max_hr + 10, // Upper bound for Z5
        })
    }

    /// Calculate power zones based on FTP (Functional Threshold Power)
    ///
    /// Zone calculations based on FTP:
    /// - Z1: < 55% FTP (Active Recovery)
    /// - Z2: 55-74% FTP (Endurance)
    /// - Z3: 75-89% FTP (Tempo)
    /// - Z4: 90-104% FTP (Lactate Threshold)
    /// - Z5: 105-120% FTP (VO2 Max)
    /// - Z6: 121-150% FTP (Anaerobic Capacity)
    /// - Z7: > 150% FTP (Sprint Power)
    pub fn calculate_power_zones(profile: &AthleteProfile) -> Result<PowerZones> {
        let ftp = profile.ftp.ok_or_else(|| {
            ZoneError::MissingThreshold("FTP required for power zone calculation".to_string())
        })?;

        Self::validate_power(ftp)?;

        let ftp_decimal = Decimal::from(ftp);

        Ok(PowerZones {
            zone1_max: Self::calculate_percentage(ftp_decimal, dec!(0.55))?,
            zone2_max: Self::calculate_percentage(ftp_decimal, dec!(0.74))?,
            zone3_max: Self::calculate_percentage(ftp_decimal, dec!(0.89))?,
            zone4_max: Self::calculate_percentage(ftp_decimal, dec!(1.04))?,
            zone5_max: Self::calculate_percentage(ftp_decimal, dec!(1.20))?,
            zone6_max: Self::calculate_percentage(ftp_decimal, dec!(1.50))?,
            zone7_max: ftp * 3, // Upper bound for sprints
        })
    }

    /// Calculate pace zones based on threshold pace
    ///
    /// Zone calculations based on threshold pace (5K-10K race pace):
    /// - Z1: > 129% threshold pace (Easy/Recovery - slower pace)
    /// - Z2: 114-129% threshold pace (Aerobic Base)
    /// - Z3: 106-113% threshold pace (Tempo)
    /// - Z4: 100-105% threshold pace (Threshold)
    /// - Z5: < 100% threshold pace (VO2 Max - faster pace)
    pub fn calculate_pace_zones(profile: &AthleteProfile) -> Result<PaceZones> {
        let threshold_pace = profile.threshold_pace.ok_or_else(|| {
            ZoneError::MissingThreshold("Threshold pace required for pace zone calculation".to_string())
        })?;

        Self::validate_pace(threshold_pace)?;

        // For pace, slower is higher value (higher minutes per mile/km)
        // So we multiply by percentages > 1.0 for easier zones
        Ok(PaceZones {
            zone1_min: threshold_pace * dec!(1.29), // Slowest (easiest)
            zone2_min: threshold_pace * dec!(1.14), // Aerobic base
            zone3_min: threshold_pace * dec!(1.06), // Tempo
            zone4_min: threshold_pace * dec!(1.00), // Threshold
            zone5_min: threshold_pace * dec!(0.95), // Fastest (hardest)
        })
    }

    /// Calculate all training zones for an athlete
    pub fn calculate_all_zones(profile: &AthleteProfile) -> TrainingZones {
        let heart_rate_zones = if profile.lthr.is_some() {
            Self::calculate_heart_rate_zones(profile, HRZoneMethod::LTHR).ok()
        } else if profile.max_hr.is_some() {
            Self::calculate_heart_rate_zones(profile, HRZoneMethod::MaxHR).ok()
        } else {
            None
        };

        let power_zones = if profile.ftp.is_some() {
            Self::calculate_power_zones(profile).ok()
        } else {
            None
        };

        let pace_zones = if profile.threshold_pace.is_some() {
            Self::calculate_pace_zones(profile).ok()
        } else {
            None
        };

        TrainingZones {
            heart_rate_zones,
            power_zones,
            pace_zones,
        }
    }

    /// Determine which heart rate zone a given HR falls into
    pub fn get_heart_rate_zone(hr: u16, zones: &HeartRateZones) -> u8 {
        if hr <= zones.zone1_max {
            1
        } else if hr <= zones.zone2_max {
            2
        } else if hr <= zones.zone3_max {
            3
        } else if hr <= zones.zone4_max {
            4
        } else {
            5
        }
    }

    /// Determine which power zone a given power falls into
    pub fn get_power_zone(power: u16, zones: &PowerZones) -> u8 {
        if power <= zones.zone1_max {
            1
        } else if power <= zones.zone2_max {
            2
        } else if power <= zones.zone3_max {
            3
        } else if power <= zones.zone4_max {
            4
        } else if power <= zones.zone5_max {
            5
        } else if power <= zones.zone6_max {
            6
        } else {
            7
        }
    }

    /// Determine which pace zone a given pace falls into
    pub fn get_pace_zone(pace: Decimal, zones: &PaceZones) -> u8 {
        // For pace, slower is easier (higher zone numbers mean slower paces)
        if pace >= zones.zone1_min {
            1
        } else if pace >= zones.zone2_min {
            2
        } else if pace >= zones.zone3_min {
            3
        } else if pace >= zones.zone4_min {
            4
        } else {
            5
        }
    }

    // Helper methods for calculations and validation

    fn calculate_percentage(value: Decimal, percentage: Decimal) -> Result<u16> {
        let result = value * percentage;
        let rounded = result.round();

        // Convert to u16, ensuring it's within valid range
        if rounded < Decimal::ZERO {
            return Err(anyhow!(ZoneError::CalculationError("Negative result".to_string())));
        }

        let as_u32 = rounded.to_u32().ok_or_else(|| {
            ZoneError::CalculationError("Result too large for u16".to_string())
        })?;

        if as_u32 > u16::MAX as u32 {
            return Err(anyhow!(ZoneError::CalculationError("Result exceeds u16 range".to_string())));
        }

        Ok(as_u32 as u16)
    }

    fn validate_heart_rate(hr: u16, field_name: &str) -> Result<()> {
        if hr < 30 || hr > 220 {
            return Err(anyhow!(ZoneError::InvalidThreshold(
                format!("{} must be between 30 and 220 bpm, got {}", field_name, hr)
            )));
        }
        Ok(())
    }

    fn validate_power(power: u16) -> Result<()> {
        if power < 50 || power > 800 {
            return Err(anyhow!(ZoneError::InvalidThreshold(
                format!("FTP must be between 50 and 800 watts, got {}", power)
            )));
        }
        Ok(())
    }

    fn validate_pace(pace: Decimal) -> Result<()> {
        if pace <= Decimal::ZERO || pace > dec!(20.0) {
            return Err(anyhow!(ZoneError::InvalidThreshold(
                format!("Threshold pace must be between 0 and 20 min/mile or min/km, got {}", pace)
            )));
        }
        Ok(())
    }
}

/// Threshold testing and estimation utilities
pub struct ThresholdEstimator;

impl ThresholdEstimator {
    /// Estimate FTP from 20-minute power test (multiply by 0.95)
    pub fn estimate_ftp_from_20min_test(power_20min: u16) -> Result<u16> {
        ZoneCalculator::validate_power(power_20min)?;
        let ftp_decimal = Decimal::from(power_20min) * dec!(0.95);
        Ok(ftp_decimal.round().to_u16().unwrap_or(power_20min))
    }

    /// Estimate FTP from 1-hour power test (use directly)
    pub fn estimate_ftp_from_1hour_test(power_1hour: u16) -> Result<u16> {
        ZoneCalculator::validate_power(power_1hour)?;
        Ok(power_1hour)
    }

    /// Estimate LTHR from 30-minute time trial average heart rate
    pub fn estimate_lthr_from_30min_test(avg_hr_30min: u16) -> Result<u16> {
        ZoneCalculator::validate_heart_rate(avg_hr_30min, "30-min test HR")?;
        // LTHR is typically very close to 30-min TT average HR
        Ok(avg_hr_30min)
    }

    /// Estimate max heart rate from age (220 - age formula)
    pub fn estimate_max_hr_from_age(age: u8) -> Result<u16> {
        if age < 10 || age > 100 {
            return Err(anyhow!(ZoneError::InvalidThreshold(
                format!("Age must be between 10 and 100, got {}", age)
            )));
        }
        let max_hr = 220u16.saturating_sub(age as u16);
        Ok(max_hr)
    }

    /// Estimate threshold pace from 5K race time (in minutes)
    pub fn estimate_threshold_pace_from_5k(race_time_minutes: Decimal) -> Result<Decimal> {
        if race_time_minutes <= Decimal::ZERO || race_time_minutes > dec!(60.0) {
            return Err(anyhow!(ZoneError::InvalidThreshold(
                "5K time must be between 0 and 60 minutes".to_string()
            )));
        }

        // 5K threshold pace is approximately the average pace
        let pace_per_km = race_time_minutes / dec!(5.0);
        Ok(pace_per_km)
    }

    /// Estimate threshold pace from 10K race time (in minutes)
    pub fn estimate_threshold_pace_from_10k(race_time_minutes: Decimal) -> Result<Decimal> {
        if race_time_minutes <= Decimal::ZERO || race_time_minutes > dec!(120.0) {
            return Err(anyhow!(ZoneError::InvalidThreshold(
                "10K time must be between 0 and 120 minutes".to_string()
            )));
        }

        // 10K pace is slightly slower than threshold, so we adjust by ~2%
        let pace_per_km = race_time_minutes / dec!(10.0);
        let threshold_pace = pace_per_km * dec!(0.98); // Slightly faster than 10K pace
        Ok(threshold_pace)
    }
}

/// Zone distribution analysis utilities
pub struct ZoneAnalyzer;

impl ZoneAnalyzer {
    /// Calculate zone distribution from heart rate data
    pub fn analyze_hr_distribution(hr_data: &[u16], zones: &HeartRateZones) -> ZoneDistribution {
        let mut zone_counts = [0u32; 5];
        let total_points = hr_data.len() as u32;

        for &hr in hr_data {
            let zone = ZoneCalculator::get_heart_rate_zone(hr, zones);
            if zone >= 1 && zone <= 5 {
                zone_counts[(zone - 1) as usize] += 1;
            }
        }

        ZoneDistribution {
            zone1_percent: calculate_percentage(zone_counts[0], total_points),
            zone2_percent: calculate_percentage(zone_counts[1], total_points),
            zone3_percent: calculate_percentage(zone_counts[2], total_points),
            zone4_percent: calculate_percentage(zone_counts[3], total_points),
            zone5_percent: calculate_percentage(zone_counts[4], total_points),
            total_points,
        }
    }

    /// Calculate zone distribution from power data
    pub fn analyze_power_distribution(power_data: &[u16], zones: &PowerZones) -> PowerZoneDistribution {
        let mut zone_counts = [0u32; 7];
        let total_points = power_data.len() as u32;

        for &power in power_data {
            let zone = ZoneCalculator::get_power_zone(power, zones);
            if zone >= 1 && zone <= 7 {
                zone_counts[(zone - 1) as usize] += 1;
            }
        }

        PowerZoneDistribution {
            zone1_percent: calculate_percentage(zone_counts[0], total_points),
            zone2_percent: calculate_percentage(zone_counts[1], total_points),
            zone3_percent: calculate_percentage(zone_counts[2], total_points),
            zone4_percent: calculate_percentage(zone_counts[3], total_points),
            zone5_percent: calculate_percentage(zone_counts[4], total_points),
            zone6_percent: calculate_percentage(zone_counts[5], total_points),
            zone7_percent: calculate_percentage(zone_counts[6], total_points),
            total_points,
        }
    }
}

/// Zone distribution results for heart rate
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneDistribution {
    pub zone1_percent: Decimal,
    pub zone2_percent: Decimal,
    pub zone3_percent: Decimal,
    pub zone4_percent: Decimal,
    pub zone5_percent: Decimal,
    pub total_points: u32,
}

/// Zone distribution results for power
#[derive(Debug, Clone, PartialEq)]
pub struct PowerZoneDistribution {
    pub zone1_percent: Decimal,
    pub zone2_percent: Decimal,
    pub zone3_percent: Decimal,
    pub zone4_percent: Decimal,
    pub zone5_percent: Decimal,
    pub zone6_percent: Decimal,
    pub zone7_percent: Decimal,
    pub total_points: u32,
}

fn calculate_percentage(count: u32, total: u32) -> Decimal {
    if total == 0 {
        Decimal::ZERO
    } else {
        (Decimal::from(count) / Decimal::from(total)) * dec!(100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AthleteProfile, Units};
    use chrono::Utc;
    use rust_decimal_macros::dec;

    fn create_test_profile() -> AthleteProfile {
        let now = Utc::now();
        AthleteProfile {
            id: "test_athlete".to_string(),
            name: "Test Athlete".to_string(),
            date_of_birth: Some(chrono::NaiveDate::from_ymd_opt(1990, 1, 1).unwrap()),
            weight: Some(dec!(70.0)),
            height: Some(175),
            ftp: Some(250),
            lthr: Some(165),
            threshold_pace: Some(dec!(6.0)), // 6 min/mile or min/km
            max_hr: Some(190),
            resting_hr: Some(50),
            training_zones: TrainingZones::default(),
            preferred_units: Units::Metric,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_hr_zones_from_lthr() {
        let profile = create_test_profile();
        let zones = ZoneCalculator::hr_zones_from_lthr(&profile).unwrap();

        // Test LTHR-based zones (165 LTHR)
        assert_eq!(zones.zone1_max, 134); // 165 * 0.81 = 133.65 -> 134
        assert_eq!(zones.zone2_max, 147); // 165 * 0.89 = 146.85 -> 147
        assert_eq!(zones.zone3_max, 153); // 165 * 0.93 = 153.45 -> 153
        assert_eq!(zones.zone4_max, 163); // 165 * 0.99 = 163.35 -> 163
        assert_eq!(zones.zone5_max, 185); // 165 + 20 = 185
    }

    #[test]
    fn test_hr_zones_from_max_hr() {
        let profile = create_test_profile();
        let zones = ZoneCalculator::hr_zones_from_max_hr(&profile).unwrap();

        // Test MaxHR-based zones (190 MaxHR)
        assert_eq!(zones.zone1_max, 129); // 190 * 0.68 = 129.2 -> 129
        assert_eq!(zones.zone2_max, 158); // 190 * 0.83 = 157.7 -> 158
        assert_eq!(zones.zone3_max, 179); // 190 * 0.94 = 178.6 -> 179
        assert_eq!(zones.zone4_max, 200); // 190 * 1.05 = 199.5 -> 200
        assert_eq!(zones.zone5_max, 200); // 190 + 10 = 200
    }

    #[test]
    fn test_power_zones_calculation() {
        let profile = create_test_profile();
        let zones = ZoneCalculator::calculate_power_zones(&profile).unwrap();

        // Test FTP-based zones (250 FTP)
        assert_eq!(zones.zone1_max, 138); // 250 * 0.55 = 137.5 -> 138
        assert_eq!(zones.zone2_max, 185); // 250 * 0.74 = 185
        assert_eq!(zones.zone3_max, 222); // 250 * 0.89 = 222.5 -> 222
        assert_eq!(zones.zone4_max, 260); // 250 * 1.04 = 260
        assert_eq!(zones.zone5_max, 300); // 250 * 1.20 = 300
        assert_eq!(zones.zone6_max, 375); // 250 * 1.50 = 375
        assert_eq!(zones.zone7_max, 750); // 250 * 3 = 750
    }

    #[test]
    fn test_pace_zones_calculation() {
        let profile = create_test_profile();
        let zones = ZoneCalculator::calculate_pace_zones(&profile).unwrap();

        // Test threshold pace-based zones (6.0 min/mile threshold)
        assert_eq!(zones.zone1_min, dec!(7.74)); // 6.0 * 1.29 = 7.74
        assert_eq!(zones.zone2_min, dec!(6.84)); // 6.0 * 1.14 = 6.84
        assert_eq!(zones.zone3_min, dec!(6.36)); // 6.0 * 1.06 = 6.36
        assert_eq!(zones.zone4_min, dec!(6.00)); // 6.0 * 1.00 = 6.00
        assert_eq!(zones.zone5_min, dec!(5.70)); // 6.0 * 0.95 = 5.70
    }

    #[test]
    fn test_missing_threshold_errors() {
        let mut profile = create_test_profile();

        // Test missing LTHR
        profile.lthr = None;
        profile.max_hr = None;
        let result = ZoneCalculator::hr_zones_from_lthr(&profile);
        assert!(result.is_err());

        // Test missing FTP
        profile.ftp = None;
        let result = ZoneCalculator::calculate_power_zones(&profile);
        assert!(result.is_err());

        // Test missing threshold pace
        profile.threshold_pace = None;
        let result = ZoneCalculator::calculate_pace_zones(&profile);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_threshold_validation() {
        let mut profile = create_test_profile();

        // Test invalid LTHR
        profile.lthr = Some(25); // Too low
        let result = ZoneCalculator::hr_zones_from_lthr(&profile);
        assert!(result.is_err());

        profile.lthr = Some(250); // Too high
        let result = ZoneCalculator::hr_zones_from_lthr(&profile);
        assert!(result.is_err());

        // Test invalid FTP
        profile.ftp = Some(25); // Too low
        let result = ZoneCalculator::calculate_power_zones(&profile);
        assert!(result.is_err());

        profile.ftp = Some(900); // Too high
        let result = ZoneCalculator::calculate_power_zones(&profile);
        assert!(result.is_err());

        // Test invalid threshold pace
        profile.threshold_pace = Some(dec!(0.0)); // Too low
        let result = ZoneCalculator::calculate_pace_zones(&profile);
        assert!(result.is_err());

        profile.threshold_pace = Some(dec!(25.0)); // Too high
        let result = ZoneCalculator::calculate_pace_zones(&profile);
        assert!(result.is_err());
    }

    #[test]
    fn test_zone_detection() {
        let profile = create_test_profile();
        let hr_zones = ZoneCalculator::hr_zones_from_lthr(&profile).unwrap();
        let power_zones = ZoneCalculator::calculate_power_zones(&profile).unwrap();
        let pace_zones = ZoneCalculator::calculate_pace_zones(&profile).unwrap();

        // Test heart rate zone detection
        assert_eq!(ZoneCalculator::get_heart_rate_zone(120, &hr_zones), 1);
        assert_eq!(ZoneCalculator::get_heart_rate_zone(140, &hr_zones), 2);
        assert_eq!(ZoneCalculator::get_heart_rate_zone(150, &hr_zones), 3);
        assert_eq!(ZoneCalculator::get_heart_rate_zone(160, &hr_zones), 4);
        assert_eq!(ZoneCalculator::get_heart_rate_zone(180, &hr_zones), 5);

        // Test power zone detection
        assert_eq!(ZoneCalculator::get_power_zone(100, &power_zones), 1);
        assert_eq!(ZoneCalculator::get_power_zone(150, &power_zones), 2);
        assert_eq!(ZoneCalculator::get_power_zone(200, &power_zones), 3);
        assert_eq!(ZoneCalculator::get_power_zone(250, &power_zones), 4);
        assert_eq!(ZoneCalculator::get_power_zone(310, &power_zones), 6); // 310 > 300 (zone5_max)
        assert_eq!(ZoneCalculator::get_power_zone(280, &power_zones), 5); // Within zone 5
        assert_eq!(ZoneCalculator::get_power_zone(400, &power_zones), 7); // 400 > 375 (zone6_max)
        assert_eq!(ZoneCalculator::get_power_zone(500, &power_zones), 7);

        // Test pace zone detection (slower pace = easier zone)
        assert_eq!(ZoneCalculator::get_pace_zone(dec!(8.0), &pace_zones), 1); // Slowest
        assert_eq!(ZoneCalculator::get_pace_zone(dec!(7.0), &pace_zones), 2);
        assert_eq!(ZoneCalculator::get_pace_zone(dec!(6.2), &pace_zones), 4); // Between zone3_min (6.36) and zone4_min (6.0)
        assert_eq!(ZoneCalculator::get_pace_zone(dec!(6.5), &pace_zones), 3); // Above zone3_min (6.36)
        assert_eq!(ZoneCalculator::get_pace_zone(dec!(6.0), &pace_zones), 4);
        assert_eq!(ZoneCalculator::get_pace_zone(dec!(5.5), &pace_zones), 5); // Fastest
    }

    #[test]
    fn test_calculate_all_zones() {
        let profile = create_test_profile();
        let zones = ZoneCalculator::calculate_all_zones(&profile);

        assert!(zones.heart_rate_zones.is_some());
        assert!(zones.power_zones.is_some());
        assert!(zones.pace_zones.is_some());

        // Test with missing thresholds
        let mut incomplete_profile = create_test_profile();
        incomplete_profile.ftp = None;
        incomplete_profile.lthr = None;
        incomplete_profile.max_hr = None;

        let zones = ZoneCalculator::calculate_all_zones(&incomplete_profile);
        assert!(zones.heart_rate_zones.is_none());
        assert!(zones.power_zones.is_none());
        assert!(zones.pace_zones.is_some()); // Still has threshold pace
    }

    #[test]
    fn test_threshold_estimators() {
        // Test FTP estimation from 20-min test
        let ftp = ThresholdEstimator::estimate_ftp_from_20min_test(260).unwrap();
        assert_eq!(ftp, 247); // 260 * 0.95 = 247

        // Test FTP estimation from 1-hour test
        let ftp = ThresholdEstimator::estimate_ftp_from_1hour_test(240).unwrap();
        assert_eq!(ftp, 240); // Direct use

        // Test LTHR estimation from 30-min test
        let lthr = ThresholdEstimator::estimate_lthr_from_30min_test(168).unwrap();
        assert_eq!(lthr, 168); // Direct use

        // Test max HR estimation from age
        let max_hr = ThresholdEstimator::estimate_max_hr_from_age(30).unwrap();
        assert_eq!(max_hr, 190); // 220 - 30 = 190

        // Test threshold pace from 5K
        let pace = ThresholdEstimator::estimate_threshold_pace_from_5k(dec!(25.0)).unwrap();
        assert_eq!(pace, dec!(5.0)); // 25 minutes / 5 km = 5 min/km

        // Test threshold pace from 10K
        let pace = ThresholdEstimator::estimate_threshold_pace_from_10k(dec!(50.0)).unwrap();
        assert_eq!(pace, dec!(4.9)); // (50 / 10) * 0.98 = 4.9 min/km
    }

    #[test]
    fn test_threshold_estimator_validation() {
        // Test invalid inputs
        assert!(ThresholdEstimator::estimate_ftp_from_20min_test(30).is_err());
        assert!(ThresholdEstimator::estimate_max_hr_from_age(5).is_err());
        assert!(ThresholdEstimator::estimate_threshold_pace_from_5k(dec!(0.0)).is_err());
        assert!(ThresholdEstimator::estimate_threshold_pace_from_10k(dec!(150.0)).is_err());
    }

    #[test]
    fn test_zone_distribution_analysis() {
        let hr_data = vec![120, 130, 140, 150, 160, 170, 180, 190];
        let profile = create_test_profile();
        let hr_zones = ZoneCalculator::hr_zones_from_lthr(&profile).unwrap();

        let distribution = ZoneAnalyzer::analyze_hr_distribution(&hr_data, &hr_zones);

        assert_eq!(distribution.total_points, 8);
        // Each zone should have roughly equal distribution based on test data
        assert!(distribution.zone1_percent > dec!(0.0));
        assert!(distribution.zone2_percent > dec!(0.0));
        assert!(distribution.zone3_percent > dec!(0.0));
        assert!(distribution.zone4_percent > dec!(0.0));
        assert!(distribution.zone5_percent > dec!(0.0));

        // Test power distribution
        let power_data = vec![100, 150, 200, 250, 300, 350, 400, 500];
        let power_zones = ZoneCalculator::calculate_power_zones(&profile).unwrap();

        let power_distribution = ZoneAnalyzer::analyze_power_distribution(&power_data, &power_zones);
        assert_eq!(power_distribution.total_points, 8);
        assert!(power_distribution.zone1_percent > dec!(0.0));
    }

    #[test]
    fn test_zone_distribution_empty_data() {
        let empty_data: Vec<u16> = vec![];
        let profile = create_test_profile();
        let hr_zones = ZoneCalculator::hr_zones_from_lthr(&profile).unwrap();

        let distribution = ZoneAnalyzer::analyze_hr_distribution(&empty_data, &hr_zones);

        assert_eq!(distribution.total_points, 0);
        assert_eq!(distribution.zone1_percent, dec!(0.0));
        assert_eq!(distribution.zone2_percent, dec!(0.0));
        assert_eq!(distribution.zone3_percent, dec!(0.0));
        assert_eq!(distribution.zone4_percent, dec!(0.0));
        assert_eq!(distribution.zone5_percent, dec!(0.0));
    }

    #[test]
    fn test_athlete_profile_integration() {
        let mut profile = create_test_profile();

        // Test threshold checking methods
        assert!(profile.has_heart_rate_thresholds());
        assert!(profile.has_power_thresholds());
        assert!(profile.has_pace_thresholds());

        // Test age calculation
        let age = profile.age().unwrap();
        assert!(age >= 30 && age <= 40); // Should be around 34 years old

        // Test zone calculation integration
        let zones_result = profile.calculate_zones();
        assert!(zones_result.is_ok());
        assert!(profile.training_zones.has_any_zones());

        // Test zone lookup methods
        let hr_zone = profile.training_zones.get_hr_zone(150);
        assert!(hr_zone.is_some());
        assert!(hr_zone.unwrap() >= 1 && hr_zone.unwrap() <= 5);

        let power_zone = profile.training_zones.get_power_zone(200);
        assert!(power_zone.is_some());
        assert!(power_zone.unwrap() >= 1 && power_zone.unwrap() <= 7);

        let pace_zone = profile.training_zones.get_pace_zone(dec!(6.5));
        assert!(pace_zone.is_some());
        assert!(pace_zone.unwrap() >= 1 && pace_zone.unwrap() <= 5);
    }

    #[test]
    fn test_estimate_missing_thresholds() {
        let mut profile = create_test_profile();
        profile.max_hr = None; // Remove max HR

        let result = profile.estimate_missing_thresholds();
        assert!(result.is_ok());
        assert!(profile.max_hr.is_some());

        // Should be around 220 - 34 = 186
        let estimated_max_hr = profile.max_hr.unwrap();
        assert!(estimated_max_hr >= 180 && estimated_max_hr <= 195);
    }
}