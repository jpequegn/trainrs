use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Sport types supported by the training analysis system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Sport {
    Running,
    Cycling,
    Swimming,
    Triathlon,
    Rowing,
    CrossTraining,
}

/// Workout types for categorizing training sessions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkoutType {
    Interval,
    Endurance,
    Recovery,
    Tempo,
    Threshold,
    VO2Max,
    Strength,
    Race,
    Test,
}

/// Data source types indicating the primary metric used for training load calculation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataSource {
    HeartRate,
    Power,
    Pace,
    Rpe, // Rate of Perceived Exertion
}

/// Individual data point in time-series workout data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataPoint {
    /// Timestamp in seconds from workout start
    pub timestamp: u32,

    /// Heart rate in beats per minute
    pub heart_rate: Option<u16>,

    /// Power output in watts (primarily for cycling)
    pub power: Option<u16>,

    /// Pace in minutes per mile or minutes per kilometer
    pub pace: Option<Decimal>,

    /// Elevation in meters above sea level
    pub elevation: Option<i16>,

    /// Cadence (steps per minute for running, revolutions per minute for cycling)
    pub cadence: Option<u16>,

    /// Speed in meters per second
    pub speed: Option<Decimal>,

    /// Distance covered at this point in meters
    pub distance: Option<Decimal>,

    /// Left leg power output in watts (power meter balance)
    pub left_power: Option<u16>,

    /// Right leg power output in watts (power meter balance)
    pub right_power: Option<u16>,
}

/// Summary metrics calculated from workout data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkoutSummary {
    /// Average heart rate during the workout
    pub avg_heart_rate: Option<u16>,

    /// Maximum heart rate reached during the workout
    pub max_heart_rate: Option<u16>,

    /// Average power output (cycling)
    pub avg_power: Option<u16>,

    /// Normalized power - power-based equivalent of steady-state effort
    pub normalized_power: Option<u16>,

    /// Average pace (running/swimming)
    pub avg_pace: Option<Decimal>,

    /// Intensity Factor - ratio of normalized power to functional threshold power
    pub intensity_factor: Option<Decimal>,

    /// Training Stress Score - quantifies training stress
    pub tss: Option<Decimal>,

    /// Total distance covered in meters
    pub total_distance: Option<Decimal>,

    /// Total elevation gain in meters
    pub elevation_gain: Option<u16>,

    /// Average cadence
    pub avg_cadence: Option<u16>,

    /// Calories burned (estimated)
    pub calories: Option<u16>,
}

/// Core workout data structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workout {
    /// Unique identifier for the workout
    pub id: String,

    /// Date of the workout
    pub date: NaiveDate,

    /// Sport/activity type
    pub sport: Sport,

    /// Duration of the workout in seconds
    pub duration_seconds: u32,

    /// Type/category of the workout
    pub workout_type: WorkoutType,

    /// Primary data source used for calculations
    pub data_source: DataSource,

    /// Raw time-series data points (optional for summary-only workouts)
    pub raw_data: Option<Vec<DataPoint>>,

    /// Calculated summary metrics
    pub summary: WorkoutSummary,

    /// Optional notes or description
    pub notes: Option<String>,

    /// Athlete identifier
    pub athlete_id: Option<String>,

    /// Original file name or source identifier
    pub source: Option<String>,
}

/// Training zones for different sports and metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingZones {
    /// Heart rate zones (5-zone model)
    pub heart_rate_zones: Option<HeartRateZones>,

    /// Power zones for cycling (7-zone model)
    pub power_zones: Option<PowerZones>,

    /// Pace zones for running (5-zone model)
    pub pace_zones: Option<PaceZones>,
}

/// Heart rate training zones
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeartRateZones {
    pub zone1_max: u16, // Active Recovery
    pub zone2_max: u16, // Aerobic Base
    pub zone3_max: u16, // Aerobic
    pub zone4_max: u16, // Lactate Threshold
    pub zone5_max: u16, // VO2 Max
}

/// Power training zones for cycling
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerZones {
    pub zone1_max: u16, // Active Recovery
    pub zone2_max: u16, // Endurance
    pub zone3_max: u16, // Tempo
    pub zone4_max: u16, // Lactate Threshold
    pub zone5_max: u16, // VO2 Max
    pub zone6_max: u16, // Anaerobic Capacity
    pub zone7_max: u16, // Sprint Power
}

/// Pace training zones for running
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaceZones {
    pub zone1_min: Decimal, // Easy pace (slowest)
    pub zone2_min: Decimal, // Aerobic pace
    pub zone3_min: Decimal, // Tempo pace
    pub zone4_min: Decimal, // Threshold pace
    pub zone5_min: Decimal, // VO2 Max pace (fastest)
}

/// Athlete profile containing thresholds and personal data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AthleteProfile {
    /// Unique athlete identifier
    pub id: String,

    /// Athlete's display name
    pub name: String,

    /// Date of birth for age-based calculations
    pub date_of_birth: Option<NaiveDate>,

    /// Weight in kilograms
    pub weight: Option<Decimal>,

    /// Height in centimeters
    pub height: Option<u16>,

    /// Functional Threshold Power for cycling (watts)
    pub ftp: Option<u16>,

    /// Lactate Threshold Heart Rate
    pub lthr: Option<u16>,

    /// Threshold pace for running (minutes per mile or km, depending on units)
    pub threshold_pace: Option<Decimal>,

    /// Maximum Heart Rate
    pub max_hr: Option<u16>,

    /// Resting Heart Rate
    pub resting_hr: Option<u16>,

    /// Training zones for different sports
    pub training_zones: TrainingZones,

    /// Preferred units (metric/imperial)
    pub preferred_units: Units,

    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last update timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Unit preferences
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Units {
    Metric,
    Imperial,
}

impl Default for Units {
    fn default() -> Self {
        Units::Metric
    }
}

impl Default for TrainingZones {
    fn default() -> Self {
        TrainingZones {
            heart_rate_zones: None,
            power_zones: None,
            pace_zones: None,
        }
    }
}

impl Default for WorkoutSummary {
    fn default() -> Self {
        WorkoutSummary {
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, Utc};
    use rust_decimal_macros::dec;

    #[test]
    fn test_sport_enum_serialization() {
        let sport = Sport::Cycling;
        let json = serde_json::to_string(&sport).unwrap();
        assert_eq!(json, "\"Cycling\"");

        let deserialized: Sport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, Sport::Cycling);
    }

    #[test]
    fn test_workout_type_enum() {
        let workout_types = vec![
            WorkoutType::Interval,
            WorkoutType::Endurance,
            WorkoutType::Recovery,
            WorkoutType::Tempo,
            WorkoutType::Threshold,
            WorkoutType::VO2Max,
            WorkoutType::Strength,
            WorkoutType::Race,
            WorkoutType::Test,
        ];

        for workout_type in workout_types {
            let json = serde_json::to_string(&workout_type).unwrap();
            let deserialized: WorkoutType = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, workout_type);
        }
    }

    #[test]
    fn test_data_source_enum() {
        let sources = vec![
            DataSource::HeartRate,
            DataSource::Power,
            DataSource::Pace,
            DataSource::Rpe,
        ];

        for source in sources {
            let json = serde_json::to_string(&source).unwrap();
            let deserialized: DataSource = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, source);
        }
    }

    #[test]
    fn test_data_point_creation() {
        let data_point = DataPoint {
            timestamp: 60,
            heart_rate: Some(150),
            power: Some(250),
            pace: Some(dec!(6.5)),
            elevation: Some(100),
            cadence: Some(90),
            speed: Some(dec!(5.0)),
            distance: Some(dec!(1000.0)),
            left_power: Some(125),
            right_power: Some(125),
        };

        assert_eq!(data_point.timestamp, 60);
        assert_eq!(data_point.heart_rate, Some(150));
        assert_eq!(data_point.power, Some(250));
        assert_eq!(data_point.pace, Some(dec!(6.5)));
    }

    #[test]
    fn test_data_point_serialization() {
        let data_point = DataPoint {
            timestamp: 120,
            heart_rate: Some(140),
            power: None,
            pace: Some(dec!(7.0)),
            elevation: Some(50),
            cadence: Some(85),
            speed: Some(dec!(4.5)),
            distance: Some(dec!(500.0)),
            left_power: None,
            right_power: None,
        };

        let json = serde_json::to_string(&data_point).unwrap();
        let deserialized: DataPoint = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.timestamp, data_point.timestamp);
        assert_eq!(deserialized.heart_rate, data_point.heart_rate);
        assert_eq!(deserialized.power, data_point.power);
        assert_eq!(deserialized.pace, data_point.pace);
    }

    #[test]
    fn test_workout_summary_default() {
        let summary = WorkoutSummary::default();

        assert_eq!(summary.avg_heart_rate, None);
        assert_eq!(summary.max_heart_rate, None);
        assert_eq!(summary.avg_power, None);
        assert_eq!(summary.tss, None);
    }

    #[test]
    fn test_workout_summary_with_values() {
        let summary = WorkoutSummary {
            avg_heart_rate: Some(155),
            max_heart_rate: Some(180),
            avg_power: Some(220),
            normalized_power: Some(235),
            avg_pace: Some(dec!(6.8)),
            intensity_factor: Some(dec!(0.88)),
            tss: Some(dec!(85.5)),
            total_distance: Some(dec!(10000.0)),
            elevation_gain: Some(200),
            avg_cadence: Some(88),
            calories: Some(650),
        };

        assert_eq!(summary.avg_heart_rate, Some(155));
        assert_eq!(summary.tss, Some(dec!(85.5)));
        assert_eq!(summary.total_distance, Some(dec!(10000.0)));
    }

    #[test]
    fn test_workout_creation() {
        let workout = Workout {
            id: "workout_123".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 9, 23).unwrap(),
            sport: Sport::Running,
            duration_seconds: 3600,
            workout_type: WorkoutType::Endurance,
            data_source: DataSource::HeartRate,
            raw_data: None,
            summary: WorkoutSummary::default(),
            notes: Some("Great morning run".to_string()),
            athlete_id: Some("athlete_456".to_string()),
            source: Some("garmin_connect".to_string()),
        };

        assert_eq!(workout.id, "workout_123");
        assert_eq!(workout.sport, Sport::Running);
        assert_eq!(workout.duration_seconds, 3600);
        assert_eq!(workout.notes, Some("Great morning run".to_string()));
    }

    #[test]
    fn test_workout_with_raw_data() {
        let data_points = vec![
            DataPoint {
                timestamp: 0,
                heart_rate: Some(120),
                power: None,
                pace: Some(dec!(8.0)),
                elevation: Some(0),
                cadence: Some(80),
                speed: Some(dec!(3.35)),
                distance: Some(dec!(0.0)),
                left_power: None,
                right_power: None,
            },
            DataPoint {
                timestamp: 60,
                heart_rate: Some(150),
                power: None,
                pace: Some(dec!(7.0)),
                elevation: Some(10),
                cadence: Some(85),
                speed: Some(dec!(3.85)),
                distance: Some(dec!(230.0)),
                left_power: None,
                right_power: None,
            },
        ];

        let workout = Workout {
            id: "workout_with_data".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 9, 23).unwrap(),
            sport: Sport::Running,
            duration_seconds: 1800,
            workout_type: WorkoutType::Tempo,
            data_source: DataSource::HeartRate,
            raw_data: Some(data_points.clone()),
            summary: WorkoutSummary::default(),
            notes: None,
            athlete_id: Some("athlete_789".to_string()),
            source: None,
        };

        assert_eq!(workout.raw_data.as_ref().unwrap().len(), 2);
        assert_eq!(workout.raw_data.as_ref().unwrap()[0].heart_rate, Some(120));
        assert_eq!(workout.raw_data.as_ref().unwrap()[1].heart_rate, Some(150));
    }

    #[test]
    fn test_heart_rate_zones() {
        let hr_zones = HeartRateZones {
            zone1_max: 120,
            zone2_max: 140,
            zone3_max: 160,
            zone4_max: 175,
            zone5_max: 190,
        };

        assert_eq!(hr_zones.zone1_max, 120);
        assert_eq!(hr_zones.zone5_max, 190);
    }

    #[test]
    fn test_power_zones() {
        let power_zones = PowerZones {
            zone1_max: 150,
            zone2_max: 200,
            zone3_max: 250,
            zone4_max: 275,
            zone5_max: 315,
            zone6_max: 400,
            zone7_max: 800,
        };

        assert_eq!(power_zones.zone1_max, 150);
        assert_eq!(power_zones.zone7_max, 800);
    }

    #[test]
    fn test_pace_zones() {
        let pace_zones = PaceZones {
            zone1_min: dec!(9.0), // Easy pace (slowest)
            zone2_min: dec!(8.0), // Aerobic pace
            zone3_min: dec!(7.0), // Tempo pace
            zone4_min: dec!(6.5), // Threshold pace
            zone5_min: dec!(6.0), // VO2 Max pace (fastest)
        };

        assert_eq!(pace_zones.zone1_min, dec!(9.0));
        assert_eq!(pace_zones.zone5_min, dec!(6.0));
    }

    #[test]
    fn test_training_zones_default() {
        let zones = TrainingZones::default();

        assert!(zones.heart_rate_zones.is_none());
        assert!(zones.power_zones.is_none());
        assert!(zones.pace_zones.is_none());
    }

    #[test]
    fn test_athlete_profile_creation() {
        let now = Utc::now();
        let profile = AthleteProfile {
            id: "athlete_001".to_string(),
            name: "John Doe".to_string(),
            date_of_birth: Some(NaiveDate::from_ymd_opt(1985, 5, 15).unwrap()),
            weight: Some(dec!(75.0)),
            height: Some(180),
            ftp: Some(250),
            lthr: Some(165),
            threshold_pace: Some(dec!(6.0)),
            max_hr: Some(190),
            resting_hr: Some(50),
            training_zones: TrainingZones::default(),
            preferred_units: Units::Metric,
            created_at: now,
            updated_at: now,
        };

        assert_eq!(profile.id, "athlete_001");
        assert_eq!(profile.name, "John Doe");
        assert_eq!(profile.ftp, Some(250));
        assert_eq!(profile.preferred_units, Units::Metric);
    }

    #[test]
    fn test_athlete_profile_with_zones() {
        let hr_zones = HeartRateZones {
            zone1_max: 120,
            zone2_max: 140,
            zone3_max: 160,
            zone4_max: 175,
            zone5_max: 190,
        };

        let power_zones = PowerZones {
            zone1_max: 125,
            zone2_max: 175,
            zone3_max: 215,
            zone4_max: 250,
            zone5_max: 290,
            zone6_max: 350,
            zone7_max: 700,
        };

        let training_zones = TrainingZones {
            heart_rate_zones: Some(hr_zones),
            power_zones: Some(power_zones),
            pace_zones: None,
        };

        let now = Utc::now();
        let profile = AthleteProfile {
            id: "athlete_002".to_string(),
            name: "Jane Smith".to_string(),
            date_of_birth: Some(NaiveDate::from_ymd_opt(1990, 3, 22).unwrap()),
            weight: Some(dec!(65.0)),
            height: Some(165),
            ftp: Some(200),
            lthr: Some(155),
            threshold_pace: Some(dec!(6.5)),
            max_hr: Some(185),
            resting_hr: Some(45),
            training_zones,
            preferred_units: Units::Imperial,
            created_at: now,
            updated_at: now,
        };

        assert_eq!(profile.preferred_units, Units::Imperial);
        assert!(profile.training_zones.heart_rate_zones.is_some());
        assert!(profile.training_zones.power_zones.is_some());
        assert!(profile.training_zones.pace_zones.is_none());

        let hr_zones = profile.training_zones.heart_rate_zones.unwrap();
        assert_eq!(hr_zones.zone4_max, 175);
    }

    #[test]
    fn test_units_default() {
        let units = Units::default();
        assert_eq!(units, Units::Metric);
    }

    #[test]
    fn test_complete_workout_serialization() {
        let workout = Workout {
            id: "test_workout".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 9, 23).unwrap(),
            sport: Sport::Cycling,
            duration_seconds: 7200,
            workout_type: WorkoutType::Interval,
            data_source: DataSource::Power,
            raw_data: Some(vec![DataPoint {
                timestamp: 0,
                heart_rate: Some(110),
                power: Some(150),
                pace: None,
                elevation: Some(100),
                cadence: Some(85),
                speed: Some(dec!(8.5)),
                distance: Some(dec!(0.0)),
                left_power: Some(75),
                right_power: Some(75),
            }]),
            summary: WorkoutSummary {
                avg_heart_rate: Some(155),
                max_heart_rate: Some(180),
                avg_power: Some(220),
                normalized_power: Some(235),
                avg_pace: None,
                intensity_factor: Some(dec!(0.94)),
                tss: Some(dec!(95.2)),
                total_distance: Some(dec!(25000.0)),
                elevation_gain: Some(350),
                avg_cadence: Some(90),
                calories: Some(850),
            },
            notes: Some("Excellent interval session".to_string()),
            athlete_id: Some("athlete_123".to_string()),
            source: Some("wahoo_elemnt".to_string()),
        };

        // Test serialization
        let json = serde_json::to_string(&workout).unwrap();
        assert!(json.contains("\"sport\":\"Cycling\""));
        assert!(json.contains("\"workout_type\":\"Interval\""));

        // Test deserialization
        let deserialized: Workout = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, workout.id);
        assert_eq!(deserialized.sport, workout.sport);
        assert_eq!(deserialized.summary.tss, workout.summary.tss);
    }

    #[test]
    fn test_athlete_profile_serialization() {
        let now = Utc::now();
        let profile = AthleteProfile {
            id: "test_athlete".to_string(),
            name: "Test Athlete".to_string(),
            date_of_birth: Some(NaiveDate::from_ymd_opt(1988, 7, 10).unwrap()),
            weight: Some(dec!(70.5)),
            height: Some(175),
            ftp: Some(275),
            lthr: Some(170),
            threshold_pace: Some(dec!(5.5)),
            max_hr: Some(195),
            resting_hr: Some(48),
            training_zones: TrainingZones::default(),
            preferred_units: Units::Metric,
            created_at: now,
            updated_at: now,
        };

        // Test serialization
        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("\"preferred_units\":\"Metric\""));
        assert!(json.contains("\"ftp\":275"));

        // Test deserialization
        let deserialized: AthleteProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, profile.id);
        assert_eq!(deserialized.ftp, profile.ftp);
        assert_eq!(deserialized.preferred_units, profile.preferred_units);
    }
}

// Implementation methods for integration with zone calculations
#[allow(dead_code)]
impl AthleteProfile {
    /// Calculate and update training zones based on current thresholds
    pub fn calculate_zones(&mut self) -> Result<(), crate::zones::ZoneError> {
        self.training_zones = crate::zones::ZoneCalculator::calculate_all_zones(self);
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    /// Check if athlete has sufficient thresholds for zone calculations
    pub fn has_heart_rate_thresholds(&self) -> bool {
        self.lthr.is_some() || self.max_hr.is_some()
    }

    pub fn has_power_thresholds(&self) -> bool {
        self.ftp.is_some()
    }

    pub fn has_pace_thresholds(&self) -> bool {
        self.threshold_pace.is_some()
    }

    /// Get age from date of birth
    pub fn age(&self) -> Option<u8> {
        self.date_of_birth.map(|dob| {
            let today = chrono::Utc::now().date_naive();
            let age_duration = today.signed_duration_since(dob);
            let age_years = age_duration.num_days() / 365;
            (age_years as u8).min(255)
        })
    }

    /// Estimate missing thresholds based on available data
    pub fn estimate_missing_thresholds(&mut self) -> Result<(), crate::zones::ZoneError> {
        // Estimate max HR from age if missing
        if self.max_hr.is_none() && self.age().is_some() {
            let age = self.age().unwrap();
            match crate::zones::ThresholdEstimator::estimate_max_hr_from_age(age) {
                Ok(max_hr) => self.max_hr = Some(max_hr),
                Err(e) => return Err(crate::zones::ZoneError::InvalidThreshold(e.to_string())),
            }
        }

        self.updated_at = chrono::Utc::now();
        Ok(())
    }
}

#[allow(dead_code)]
impl TrainingZones {
    /// Check if any zones are configured
    pub fn has_any_zones(&self) -> bool {
        self.heart_rate_zones.is_some() || self.power_zones.is_some() || self.pace_zones.is_some()
    }

    /// Get zone for a heart rate value
    pub fn get_hr_zone(&self, heart_rate: u16) -> Option<u8> {
        self.heart_rate_zones
            .as_ref()
            .map(|zones| crate::zones::ZoneCalculator::get_heart_rate_zone(heart_rate, zones))
    }

    /// Get zone for a power value
    pub fn get_power_zone(&self, power: u16) -> Option<u8> {
        self.power_zones
            .as_ref()
            .map(|zones| crate::zones::ZoneCalculator::get_power_zone(power, zones))
    }

    /// Get zone for a pace value
    pub fn get_pace_zone(&self, pace: rust_decimal::Decimal) -> Option<u8> {
        self.pace_zones
            .as_ref()
            .map(|zones| crate::zones::ZoneCalculator::get_pace_zone(pace, zones))
    }
}
