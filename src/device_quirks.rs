/// Device-specific quirks and workarounds for FIT file parsing
///
/// This module handles known issues and data format variations across different
/// device manufacturers and models.

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::models::{DataPoint, Workout};

/// Device information extracted from FIT files
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Manufacturer ID from FIT file
    pub manufacturer_id: u16,

    /// Product ID from FIT file
    pub product_id: u16,

    /// Firmware version (optional)
    pub firmware_version: Option<u16>,

    /// Manufacturer name (human-readable)
    pub manufacturer_name: Option<String>,

    /// Product name (human-readable)
    pub product_name: Option<String>,
}

impl DeviceInfo {
    /// Create a new DeviceInfo
    pub fn new(manufacturer_id: u16, product_id: u16) -> Self {
        Self {
            manufacturer_id,
            product_id,
            firmware_version: None,
            manufacturer_name: None,
            product_name: None,
        }
    }

    /// Create DeviceInfo with firmware version
    pub fn with_firmware(mut self, firmware_version: u16) -> Self {
        self.firmware_version = Some(firmware_version);
        self
    }

    /// Create DeviceInfo with names
    pub fn with_names(mut self, manufacturer_name: String, product_name: String) -> Self {
        self.manufacturer_name = Some(manufacturer_name);
        self.product_name = Some(product_name);
        self
    }
}

/// Types of quirks that can be applied
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QuirkType {
    /// Scale cadence by a factor
    CadenceScaling { factor: f64 },

    /// Remove power spikes at start of workout
    PowerSpikeStart { threshold: u16, window_seconds: u32 },

    /// Decompress timestamps
    TimestampCompression,

    /// Fix byte order for specific fields
    FieldByteOrder { field_name: String },

    /// Mark missing data as invalid
    MissingData { field_name: String },

    /// Fix left-only power meter doubling
    LeftOnlyPowerDoubling,

    /// Remove GPS drift markers
    GpsDriftTunnels,

    /// Fix running dynamics scaling
    RunningDynamicsScaling {
        gct_scale: Option<f64>,
        vo_scale: Option<f64>,
    },
}

/// Device quirk definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceQuirk {
    /// Manufacturer ID
    pub manufacturer_id: u16,

    /// Product ID
    pub product_id: u16,

    /// Firmware version range (min, max), None means all versions
    pub firmware_version_range: Option<(u16, u16)>,

    /// Human-readable description
    pub description: String,

    /// Type of quirk and its configuration
    pub quirk_type: QuirkType,

    /// Whether this quirk is enabled by default
    pub enabled_by_default: bool,
}

impl DeviceQuirk {
    /// Check if this quirk applies to a given device
    pub fn applies_to(&self, device: &DeviceInfo) -> bool {
        // Check manufacturer and product
        if self.manufacturer_id != device.manufacturer_id || self.product_id != device.product_id {
            return false;
        }

        // Check firmware version if specified
        if let Some((min_fw, max_fw)) = self.firmware_version_range {
            if let Some(fw) = device.firmware_version {
                return fw >= min_fw && fw <= max_fw;
            }
            // If quirk has firmware range but device doesn't provide firmware, don't apply
            return false;
        }

        true
    }

    /// Apply this quirk to a workout
    pub fn apply(&self, workout: &mut Workout) -> Result<String> {
        match &self.quirk_type {
            QuirkType::CadenceScaling { factor } => {
                self.apply_cadence_scaling(workout, *factor)
            }
            QuirkType::PowerSpikeStart { threshold, window_seconds } => {
                self.apply_power_spike_removal(workout, *threshold, *window_seconds)
            }
            QuirkType::LeftOnlyPowerDoubling => {
                self.apply_left_only_power_fix(workout)
            }
            QuirkType::RunningDynamicsScaling { gct_scale, vo_scale } => {
                self.apply_running_dynamics_scaling(workout, *gct_scale, *vo_scale)
            }
            _ => Ok(format!("Quirk type {:?} not yet implemented", self.quirk_type)),
        }
    }

    fn apply_cadence_scaling(&self, workout: &mut Workout, factor: f64) -> Result<String> {
        if let Some(ref mut data_points) = workout.raw_data {
            let mut fixed_count = 0;
            for point in data_points.iter_mut() {
                if let Some(cadence) = point.cadence {
                    let fixed_cadence = (cadence as f64 * factor).round() as u16;
                    point.cadence = Some(fixed_cadence);
                    fixed_count += 1;
                }
            }

            // Also fix summary
            if let Some(avg_cadence) = workout.summary.avg_cadence {
                workout.summary.avg_cadence = Some((avg_cadence as f64 * factor).round() as u16);
            }

            Ok(format!("Applied cadence scaling (factor: {}) to {} data points", factor, fixed_count))
        } else {
            Ok("No raw data to apply cadence scaling".to_string())
        }
    }

    fn apply_power_spike_removal(&self, workout: &mut Workout, threshold: u16, window_seconds: u32) -> Result<String> {
        if let Some(ref mut data_points) = workout.raw_data {
            let mut fixed_count = 0;

            for point in data_points.iter_mut() {
                // Only fix power spikes in the first window_seconds
                if point.timestamp <= window_seconds {
                    if let Some(power) = point.power {
                        if power > threshold {
                            // Remove the spike by setting to None
                            point.power = None;
                            point.left_power = None;
                            point.right_power = None;
                            fixed_count += 1;
                        }
                    }
                }
            }

            Ok(format!("Removed {} power spikes in first {} seconds (threshold: {} watts)",
                fixed_count, window_seconds, threshold))
        } else {
            Ok("No raw data to apply power spike removal".to_string())
        }
    }

    fn apply_left_only_power_fix(&self, workout: &mut Workout) -> Result<String> {
        if let Some(ref mut data_points) = workout.raw_data {
            let mut fixed_count = 0;

            for point in data_points.iter_mut() {
                // If we have left power but no right power, assume left-only
                if point.left_power.is_some() && point.right_power.is_none() {
                    if let Some(left_power) = point.left_power {
                        // The total power should be the left power, not doubled
                        point.power = Some(left_power);
                        fixed_count += 1;
                    }
                }
            }

            Ok(format!("Fixed {} left-only power readings (prevented incorrect doubling)", fixed_count))
        } else {
            Ok("No raw data to apply left-only power fix".to_string())
        }
    }

    fn apply_running_dynamics_scaling(&self, workout: &mut Workout, gct_scale: Option<f64>, vo_scale: Option<f64>) -> Result<String> {
        if let Some(ref mut data_points) = workout.raw_data {
            let mut gct_fixed = 0;
            let mut vo_fixed = 0;

            for point in data_points.iter_mut() {
                if let Some(scale) = gct_scale {
                    if let Some(gct) = point.ground_contact_time {
                        point.ground_contact_time = Some((gct as f64 * scale).round() as u16);
                        gct_fixed += 1;
                    }
                }

                if let Some(scale) = vo_scale {
                    if let Some(vo) = point.vertical_oscillation {
                        point.vertical_oscillation = Some((vo as f64 * scale).round() as u16);
                        vo_fixed += 1;
                    }
                }
            }

            let mut messages = Vec::new();
            if gct_fixed > 0 {
                messages.push(format!("Fixed {} ground contact time values", gct_fixed));
            }
            if vo_fixed > 0 {
                messages.push(format!("Fixed {} vertical oscillation values", vo_fixed));
            }

            if messages.is_empty() {
                Ok("No running dynamics data to fix".to_string())
            } else {
                Ok(messages.join(", "))
            }
        } else {
            Ok("No raw data to apply running dynamics scaling".to_string())
        }
    }
}

/// Registry of known device quirks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuirkRegistry {
    /// List of known quirks
    pub quirks: Vec<DeviceQuirk>,

    /// Known manufacturer mappings (ID -> name)
    pub manufacturers: HashMap<u16, String>,

    /// Known product mappings ((manufacturer_id, product_id) -> name)
    pub products: HashMap<(u16, u16), String>,
}

impl QuirkRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            quirks: Vec::new(),
            manufacturers: HashMap::new(),
            products: HashMap::new(),
        }
    }

    /// Create registry with default known devices
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.add_default_manufacturers();
        registry.add_default_products();
        registry.add_default_quirks();
        registry
    }

    /// Add default manufacturer mappings
    fn add_default_manufacturers(&mut self) {
        self.manufacturers.insert(1, "Garmin".to_string());
        self.manufacturers.insert(32, "Wahoo".to_string());
        self.manufacturers.insert(69, "Stages Cycling".to_string());
        self.manufacturers.insert(263, "4iiii".to_string());
    }

    /// Add default product mappings
    fn add_default_products(&mut self) {
        // Garmin products
        self.products.insert((1, 2697), "Edge 520".to_string());
        self.products.insert((1, 2713), "Edge 1030".to_string());
        self.products.insert((1, 3122), "Edge 130".to_string());
        self.products.insert((1, 2697), "Forerunner 945".to_string());
        self.products.insert((1, 3589), "Forerunner 255".to_string());

        // Wahoo products
        self.products.insert((32, 16), "ELEMNT BOLT".to_string());
        self.products.insert((32, 27), "ELEMNT ROAM".to_string());
    }

    /// Add default known quirks
    fn add_default_quirks(&mut self) {
        // Garmin Edge 520: Cadence doubled
        self.quirks.push(DeviceQuirk {
            manufacturer_id: 1,
            product_id: 2697,
            firmware_version_range: None,
            description: "Edge 520 reports cadence doubled".to_string(),
            quirk_type: QuirkType::CadenceScaling { factor: 0.5 },
            enabled_by_default: true,
        });

        // Wahoo ELEMNT BOLT: Power spikes at start
        self.quirks.push(DeviceQuirk {
            manufacturer_id: 32,
            product_id: 16,
            firmware_version_range: None,
            description: "BOLT has power spikes in first 5 seconds".to_string(),
            quirk_type: QuirkType::PowerSpikeStart {
                threshold: 1500,
                window_seconds: 5,
            },
            enabled_by_default: true,
        });

        // Stages: Left-only power should not be doubled
        self.quirks.push(DeviceQuirk {
            manufacturer_id: 69,
            product_id: 0, // Generic Stages quirk, applies to all products
            firmware_version_range: None,
            description: "Left-only power should not be doubled".to_string(),
            quirk_type: QuirkType::LeftOnlyPowerDoubling,
            enabled_by_default: true,
        });

        // Garmin FR 945: Running dynamics scaling
        self.quirks.push(DeviceQuirk {
            manufacturer_id: 1,
            product_id: 2697,
            firmware_version_range: Some((0, 1000)), // Older firmware versions
            description: "FR 945 running dynamics field scaling issues".to_string(),
            quirk_type: QuirkType::RunningDynamicsScaling {
                gct_scale: Some(0.1), // Scale ground contact time
                vo_scale: Some(0.1),  // Scale vertical oscillation
            },
            enabled_by_default: true,
        });
    }

    /// Load registry from TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read quirk registry file: {}", path.as_ref().display()))?;

        let registry: QuirkRegistry = toml::from_str(&content)
            .with_context(|| "Failed to parse TOML quirk registry")?;

        Ok(registry)
    }

    /// Save registry to TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let toml_content = toml::to_string_pretty(self)
            .with_context(|| "Failed to serialize quirk registry to TOML")?;

        fs::write(&path, toml_content)
            .with_context(|| format!("Failed to write quirk registry file: {}", path.as_ref().display()))?;

        Ok(())
    }

    /// Get applicable quirks for a device
    pub fn get_applicable_quirks(&self, device: &DeviceInfo) -> Vec<&DeviceQuirk> {
        self.quirks
            .iter()
            .filter(|quirk| quirk.enabled_by_default && quirk.applies_to(device))
            .collect()
    }

    /// Apply all applicable quirks to a workout
    pub fn apply_quirks(&self, workout: &mut Workout, device: &DeviceInfo, disabled: bool) -> Result<Vec<String>> {
        if disabled {
            return Ok(vec!["Device quirks disabled by user".to_string()]);
        }

        let applicable_quirks = self.get_applicable_quirks(device);
        let mut messages = Vec::new();

        for quirk in applicable_quirks {
            match quirk.apply(workout) {
                Ok(message) => messages.push(format!("{}: {}", quirk.description, message)),
                Err(e) => messages.push(format!("Failed to apply quirk '{}': {}", quirk.description, e)),
            }
        }

        if messages.is_empty() {
            messages.push("No quirks applied".to_string());
        }

        Ok(messages)
    }

    /// Get manufacturer name from ID
    pub fn get_manufacturer_name(&self, manufacturer_id: u16) -> Option<&str> {
        self.manufacturers.get(&manufacturer_id).map(|s| s.as_str())
    }

    /// Get product name from IDs
    pub fn get_product_name(&self, manufacturer_id: u16, product_id: u16) -> Option<&str> {
        self.products.get(&(manufacturer_id, product_id)).map(|s| s.as_str())
    }

    /// Add a custom quirk
    pub fn add_quirk(&mut self, quirk: DeviceQuirk) {
        self.quirks.push(quirk);
    }
}

impl Default for QuirkRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DataSource, Sport, Workout, WorkoutSummary, WorkoutType};
    use chrono::{NaiveDate, Utc};
    use rust_decimal_macros::dec;

    fn create_test_workout() -> Workout {
        let data_points = vec![
            DataPoint {
                timestamp: 0,
                heart_rate: Some(140),
                power: Some(200),
                cadence: Some(180), // This would be doubled on Edge 520
                pace: None,
                elevation: Some(100),
                speed: Some(dec!(10.0)),
                distance: Some(dec!(0.0)),
                left_power: Some(100),
                right_power: Some(100),
                ground_contact_time: Some(2500), // Scaled value
                vertical_oscillation: Some(800), // Scaled value
                stride_length: None,
                stroke_count: None,
                stroke_type: None,
                lap_number: Some(1),
                sport_transition: None,
            },
            DataPoint {
                timestamp: 1,
                heart_rate: Some(150),
                power: Some(250),
                cadence: Some(190),
                pace: None,
                elevation: Some(100),
                speed: Some(dec!(11.0)),
                distance: Some(dec!(11.0)),
                left_power: Some(125),
                right_power: Some(125),
                ground_contact_time: Some(2400),
                vertical_oscillation: Some(750),
                stride_length: None,
                stroke_count: None,
                stroke_type: None,
                lap_number: Some(1),
                sport_transition: None,
            },
        ];

        Workout {
            id: "test-workout".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            sport: Sport::Cycling,
            duration_seconds: 3600,
            workout_type: WorkoutType::Endurance,
            data_source: DataSource::Power,
            raw_data: Some(data_points),
            summary: WorkoutSummary {
                avg_cadence: Some(185),
                avg_power: Some(225),
                ..Default::default()
            },
            notes: None,
            athlete_id: None,
            source: None,
        }
    }

    #[test]
    fn test_device_info_creation() {
        let device = DeviceInfo::new(1, 2697)
            .with_firmware(500)
            .with_names("Garmin".to_string(), "Edge 520".to_string());

        assert_eq!(device.manufacturer_id, 1);
        assert_eq!(device.product_id, 2697);
        assert_eq!(device.firmware_version, Some(500));
        assert_eq!(device.manufacturer_name, Some("Garmin".to_string()));
        assert_eq!(device.product_name, Some("Edge 520".to_string()));
    }

    #[test]
    fn test_quirk_applies_to_device() {
        let quirk = DeviceQuirk {
            manufacturer_id: 1,
            product_id: 2697,
            firmware_version_range: Some((100, 600)),
            description: "Test quirk".to_string(),
            quirk_type: QuirkType::CadenceScaling { factor: 0.5 },
            enabled_by_default: true,
        };

        // Device within firmware range
        let device1 = DeviceInfo::new(1, 2697).with_firmware(500);
        assert!(quirk.applies_to(&device1));

        // Device outside firmware range
        let device2 = DeviceInfo::new(1, 2697).with_firmware(700);
        assert!(!quirk.applies_to(&device2));

        // Different product
        let device3 = DeviceInfo::new(1, 2713).with_firmware(500);
        assert!(!quirk.applies_to(&device3));
    }

    #[test]
    fn test_cadence_scaling_quirk() {
        let mut workout = create_test_workout();
        let original_cadence = workout.raw_data.as_ref().unwrap()[0].cadence.unwrap();

        let quirk = DeviceQuirk {
            manufacturer_id: 1,
            product_id: 2697,
            firmware_version_range: None,
            description: "Edge 520 cadence doubling".to_string(),
            quirk_type: QuirkType::CadenceScaling { factor: 0.5 },
            enabled_by_default: true,
        };

        let result = quirk.apply(&mut workout);
        assert!(result.is_ok());

        let scaled_cadence = workout.raw_data.as_ref().unwrap()[0].cadence.unwrap();
        assert_eq!(scaled_cadence, (original_cadence as f64 * 0.5).round() as u16);
    }

    #[test]
    fn test_power_spike_removal() {
        let mut workout = create_test_workout();

        // Add a power spike at start
        if let Some(ref mut data_points) = workout.raw_data {
            data_points[0].power = Some(2000); // Spike
        }

        let quirk = DeviceQuirk {
            manufacturer_id: 32,
            product_id: 16,
            firmware_version_range: None,
            description: "BOLT power spikes".to_string(),
            quirk_type: QuirkType::PowerSpikeStart {
                threshold: 1500,
                window_seconds: 5,
            },
            enabled_by_default: true,
        };

        let result = quirk.apply(&mut workout);
        assert!(result.is_ok());

        // Power spike should be removed
        assert_eq!(workout.raw_data.as_ref().unwrap()[0].power, None);
        // Second data point should still have power
        assert_eq!(workout.raw_data.as_ref().unwrap()[1].power, Some(250));
    }

    #[test]
    fn test_left_only_power_fix() {
        let mut workout = create_test_workout();

        // Simulate left-only power meter
        if let Some(ref mut data_points) = workout.raw_data {
            data_points[0].left_power = Some(200);
            data_points[0].right_power = None;
            data_points[0].power = Some(400); // Incorrectly doubled
        }

        let quirk = DeviceQuirk {
            manufacturer_id: 69,
            product_id: 0,
            firmware_version_range: None,
            description: "Stages left-only power".to_string(),
            quirk_type: QuirkType::LeftOnlyPowerDoubling,
            enabled_by_default: true,
        };

        let result = quirk.apply(&mut workout);
        assert!(result.is_ok());

        // Power should be corrected to just left power
        assert_eq!(workout.raw_data.as_ref().unwrap()[0].power, Some(200));
    }

    #[test]
    fn test_running_dynamics_scaling() {
        let mut workout = create_test_workout();
        let original_gct = workout.raw_data.as_ref().unwrap()[0].ground_contact_time.unwrap();

        let quirk = DeviceQuirk {
            manufacturer_id: 1,
            product_id: 2697,
            firmware_version_range: Some((0, 1000)),
            description: "FR 945 running dynamics scaling".to_string(),
            quirk_type: QuirkType::RunningDynamicsScaling {
                gct_scale: Some(0.1),
                vo_scale: Some(0.1),
            },
            enabled_by_default: true,
        };

        let result = quirk.apply(&mut workout);
        assert!(result.is_ok());

        let scaled_gct = workout.raw_data.as_ref().unwrap()[0].ground_contact_time.unwrap();
        assert_eq!(scaled_gct, (original_gct as f64 * 0.1).round() as u16);
    }

    #[test]
    fn test_registry_with_defaults() {
        let registry = QuirkRegistry::with_defaults();

        assert!(!registry.quirks.is_empty());
        assert!(!registry.manufacturers.is_empty());
        assert!(!registry.products.is_empty());

        // Check for known manufacturers
        assert_eq!(registry.get_manufacturer_name(1), Some("Garmin"));
        assert_eq!(registry.get_manufacturer_name(32), Some("Wahoo"));
    }

    #[test]
    fn test_get_applicable_quirks() {
        let registry = QuirkRegistry::with_defaults();

        // Edge 520 should have cadence scaling quirk
        let device = DeviceInfo::new(1, 2697);
        let quirks = registry.get_applicable_quirks(&device);

        assert!(!quirks.is_empty());
        assert!(quirks.iter().any(|q| matches!(q.quirk_type, QuirkType::CadenceScaling { .. })));
    }

    #[test]
    fn test_apply_quirks_integration() {
        let registry = QuirkRegistry::with_defaults();
        let mut workout = create_test_workout();

        let device = DeviceInfo::new(1, 2697); // Edge 520
        let messages = registry.apply_quirks(&mut workout, &device, false).unwrap();

        assert!(!messages.is_empty());
        // Should have applied cadence scaling
        assert!(messages.iter().any(|m| m.contains("cadence")));
    }

    #[test]
    fn test_quirks_disabled() {
        let registry = QuirkRegistry::with_defaults();
        let mut workout = create_test_workout();

        let device = DeviceInfo::new(1, 2697);
        let messages = registry.apply_quirks(&mut workout, &device, true).unwrap();

        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("disabled"));
    }
}
