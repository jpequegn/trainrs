# Sensor Integration Guide

Advanced guide for integrating custom sensor data and complex device protocols.

## Overview

This guide covers advanced scenarios for sensor integration including:

- Multiple sensor types
- Complex data protocols
- Real-time sensor streaming
- Sensor calibration data
- Data fusion from multiple sensors

## Sensor Types

### Physiological Sensors

**Heart Rate Monitors**
- Chest strap (ECG-based)
- Optical (PPG-based)
- Advanced HRV metrics

**Muscle Oxygen Monitors**
- SmO2 (muscle oxygen saturation)
- tHb (total hemoglobin)
- Multiple sensor sites

**Core Temperature**
- Ingestible pills
- Skin patches
- Integrated with HR monitors

### Biomechanical Sensors

**Running Dynamics**
- Ground contact time
- Vertical oscillation
- Stride length/cadence
- Leg spring stiffness

**Pedal Dynamics (Cycling)**
- Power phase angles
- Pedal smoothness
- Torque effectiveness
- Left/right balance

**Swimming Metrics**
- Stroke rate
- Stroke type detection
- SWOLF score
- Underwater time

## Integration Patterns

### Pattern 1: Single-Value Sensors

Simple sensors with one primary metric:

```json
{
  "uuid": "simple-sensor-uuid",
  "name": "Simple Heart Rate Sensor",
  "manufacturer": "Vendor",
  "version": "1.0",
  "fields": [
    {
      "field_number": 0,
      "name": "heart_rate",
      "data_type": "uint8",
      "units": "bpm",
      "scale": 1.0,
      "description": "Heart rate in beats per minute"
    }
  ]
}
```

### Pattern 2: Multi-Channel Sensors

Sensors with multiple simultaneous measurements:

```json
{
  "uuid": "multi-channel-uuid",
  "name": "Dual-Site Muscle Oxygen Monitor",
  "manufacturer": "Vendor",
  "version": "2.0",
  "fields": [
    {
      "field_number": 0,
      "name": "left_quad_smo2",
      "data_type": "uint16",
      "units": "%",
      "scale": 10.0,
      "description": "Left quadriceps SmO2"
    },
    {
      "field_number": 1,
      "name": "left_quad_thb",
      "data_type": "uint16",
      "units": "g/dL",
      "scale": 100.0,
      "description": "Left quadriceps tHb"
    },
    {
      "field_number": 2,
      "name": "right_quad_smo2",
      "data_type": "uint16",
      "units": "%",
      "scale": 10.0,
      "description": "Right quadriceps SmO2"
    },
    {
      "field_number": 3,
      "name": "right_quad_thb",
      "data_type": "uint16",
      "units": "g/dL",
      "scale": 100.0,
      "description": "Right quadriceps tHb"
    }
  ]
}
```

### Pattern 3: Complex State Machines

Sensors with state-dependent interpretation:

```json
{
  "uuid": "complex-sensor-uuid",
  "name": "Advanced Power Meter",
  "manufacturer": "Vendor",
  "version": "3.0",
  "fields": [
    {
      "field_number": 0,
      "name": "measurement_mode",
      "data_type": "uint8",
      "description": "0=single, 1=dual, 2=left_only, 3=right_only"
    },
    {
      "field_number": 1,
      "name": "power_value",
      "data_type": "uint16",
      "units": "watts",
      "scale": 1.0,
      "description": "Power (interpretation depends on mode)"
    },
    {
      "field_number": 2,
      "name": "balance",
      "data_type": "uint8",
      "units": "%",
      "scale": 1.0,
      "description": "Left/right balance (only in dual mode)"
    }
  ]
}
```

## Real-World Examples

### Example 1: Stryd Running Power

Complete integration for running power pod:

```json
{
  "uuid": "a42b5e01-d5e9-4eb6-9f42-91234567890a",
  "name": "Stryd Running Power",
  "manufacturer": "Stryd",
  "version": "1.0",
  "fields": [
    {
      "field_number": 0,
      "name": "running_power",
      "data_type": "uint16",
      "units": "watts",
      "scale": 1.0,
      "description": "Instantaneous running power output"
    },
    {
      "field_number": 1,
      "name": "form_power",
      "data_type": "uint16",
      "units": "watts",
      "scale": 1.0,
      "description": "Power required to overcome oscillation"
    },
    {
      "field_number": 2,
      "name": "leg_spring_stiffness",
      "data_type": "uint16",
      "units": "kN/m",
      "scale": 10.0,
      "description": "Leg spring stiffness coefficient"
    },
    {
      "field_number": 3,
      "name": "air_power",
      "data_type": "uint16",
      "units": "watts",
      "scale": 1.0,
      "description": "Power required to overcome air resistance"
    }
  ]
}
```

**Processing Example:**

```rust
fn process_stryd_data(workout: &Workout) -> StrydMetrics {
    let mut total_power = 0;
    let mut total_form_power = 0;
    let mut samples = 0;

    for point in &workout.data_points {
        // Running power is stored in standard power field
        if let Some(power) = point.power {
            total_power += power;
            samples += 1;
        }

        // Form power from developer fields
        // (Would need additional DataPoint fields to access)
    }

    StrydMetrics {
        avg_power: if samples > 0 {
            Some(total_power / samples)
        } else {
            None
        },
        avg_form_power: None, // Calculate from developer fields
    }
}

struct StrydMetrics {
    avg_power: Option<u16>,
    avg_form_power: Option<u16>,
}
```

### Example 2: Moxy Muscle Oxygen

Dual-channel physiological monitoring:

```json
{
  "uuid": "c4d5e6f7-8901-4bcd-ef01-234567890123",
  "name": "Moxy Muscle Oxygen Monitor",
  "manufacturer": "Moxy Monitor",
  "version": "1.5",
  "fields": [
    {
      "field_number": 0,
      "name": "smo2",
      "data_type": "uint16",
      "units": "%",
      "scale": 10.0,
      "description": "Muscle oxygen saturation (SmO2)"
    },
    {
      "field_number": 1,
      "name": "thb",
      "data_type": "uint16",
      "units": "g/dL",
      "scale": 100.0,
      "description": "Total hemoglobin concentration (tHb)"
    }
  ]
}
```

**Analysis Example:**

```rust
fn analyze_moxy_data(workout: &Workout) -> MoxyAnalysis {
    // Track SmO2 trends during workout
    let mut smo2_samples = Vec::new();
    let mut thb_samples = Vec::new();

    // Developer field data would be extracted here
    // For this example, assume we have it available

    // Detect desaturation events (SmO2 < 50%)
    let desaturation_events = detect_desaturation(&smo2_samples);

    // Calculate recovery rate
    let recovery_rate = calculate_recovery_rate(&smo2_samples);

    MoxyAnalysis {
        avg_smo2: average(&smo2_samples),
        min_smo2: min(&smo2_samples),
        desaturation_events,
        recovery_rate,
    }
}

struct MoxyAnalysis {
    avg_smo2: Option<f64>,
    min_smo2: Option<f64>,
    desaturation_events: usize,
    recovery_rate: Option<f64>,
}

fn detect_desaturation(samples: &[f64]) -> usize {
    samples.iter().filter(|&&s| s < 50.0).count()
}

fn calculate_recovery_rate(samples: &[f64]) -> Option<f64> {
    // Implementation for calculating recovery rate
    // Find valleys and measure rate of increase
    None
}
```

### Example 3: Core Temperature

Ingestible temperature sensor:

```json
{
  "uuid": "e5f6a7b8-9012-4cde-f012-345678901234",
  "name": "Core Temperature Sensor",
  "manufacturer": "Temp Systems",
  "version": "2.0",
  "fields": [
    {
      "field_number": 0,
      "name": "core_temp",
      "data_type": "uint16",
      "units": "°C",
      "scale": 100.0,
      "description": "Core body temperature"
    },
    {
      "field_number": 1,
      "name": "sensor_status",
      "data_type": "uint8",
      "description": "0=OK, 1=low_battery, 2=signal_lost"
    },
    {
      "field_number": 2,
      "name": "signal_strength",
      "data_type": "uint8",
      "units": "%",
      "scale": 1.0,
      "description": "Signal strength percentage"
    }
  ]
}
```

## Data Validation

### Physiological Limits

Validate sensor data against known physiological limits:

```rust
fn validate_heart_rate(hr: u16) -> Result<u16, String> {
    match hr {
        0 => Err("Heart rate cannot be zero".to_string()),
        1..=220 => Ok(hr),
        _ => Err(format!("Heart rate {} exceeds maximum", hr)),
    }
}

fn validate_smo2(smo2: f64) -> Result<f64, String> {
    match smo2 {
        s if s < 0.0 => Err("SmO2 cannot be negative".to_string()),
        s if s > 100.0 => Err("SmO2 cannot exceed 100%".to_string()),
        s => Ok(s),
    }
}

fn validate_core_temp(temp: f64) -> Result<f64, String> {
    match temp {
        t if t < 35.0 => Err("Core temp too low (hypothermia)".to_string()),
        t if t > 42.0 => Err("Core temp too high (hyperthermia)".to_string()),
        t => Ok(t),
    }
}
```

### Sensor Quality Checks

Detect sensor issues:

```rust
fn check_sensor_quality(data: &[f64]) -> Vec<String> {
    let mut warnings = Vec::new();

    // Check for constant values (sensor stuck)
    if data.windows(10).all(|w| w.iter().all(|&x| x == w[0])) {
        warnings.push("Sensor may be stuck - constant values".to_string());
    }

    // Check for unrealistic changes
    for window in data.windows(2) {
        let change = (window[1] - window[0]).abs();
        if change > 20.0 {
            warnings.push(format!(
                "Unrealistic change: {} to {}",
                window[0], window[1]
            ));
        }
    }

    // Check for missing data
    let missing = data.iter().filter(|&&x| x == 0.0).count();
    if missing > data.len() / 10 {
        warnings.push(format!(
            "High missing data rate: {} of {} samples",
            missing,
            data.len()
        ));
    }

    warnings
}
```

## Calibration Data

Many sensors require calibration. Store calibration in registry:

```json
{
  "uuid": "power-meter-uuid",
  "name": "Power Meter",
  "manufacturer": "Vendor",
  "version": "1.0",
  "fields": [
    {
      "field_number": 0,
      "name": "raw_power",
      "data_type": "uint16",
      "units": "counts",
      "scale": 1.0,
      "description": "Raw power sensor counts"
    },
    {
      "field_number": 1,
      "name": "calibration_factor",
      "data_type": "uint16",
      "units": "N/count",
      "scale": 1000.0,
      "description": "Calibration factor to convert counts to watts"
    },
    {
      "field_number": 2,
      "name": "zero_offset",
      "data_type": "sint16",
      "units": "counts",
      "scale": 1.0,
      "description": "Zero offset from calibration"
    }
  ]
}
```

**Processing:**

```rust
fn apply_calibration(
    raw_value: u16,
    calibration_factor: f64,
    zero_offset: i16,
) -> f64 {
    let corrected = raw_value as i32 - zero_offset as i32;
    corrected as f64 * calibration_factor
}
```

## Multi-Sensor Fusion

Combine data from multiple sensors:

```rust
struct SensorFusion {
    heart_rate: Option<u16>,
    power: Option<u16>,
    smo2: Option<f64>,
    core_temp: Option<f64>,
}

impl SensorFusion {
    fn calculate_strain_index(&self) -> Option<f64> {
        // Combine multiple sensors for overall strain assessment
        let hr = self.heart_rate? as f64;
        let power = self.power? as f64;
        let smo2 = self.smo2?;

        // Example formula (not validated)
        let hr_factor = hr / 180.0;  // Normalize to max HR
        let power_factor = power / 300.0;  // Normalize to FTP
        let smo2_factor = (100.0 - smo2) / 100.0;  // Invert SmO2

        Some((hr_factor + power_factor + smo2_factor) / 3.0)
    }

    fn detect_overheating(&self) -> bool {
        if let Some(temp) = self.core_temp {
            if let Some(hr) = self.heart_rate {
                // High temp + elevated HR = potential overheating
                return temp > 39.0 && hr > 160;
            }
        }
        false
    }
}
```

## Testing Sensor Integration

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smo2_validation() {
        assert!(validate_smo2(50.0).is_ok());
        assert!(validate_smo2(0.0).is_ok());
        assert!(validate_smo2(100.0).is_ok());
        assert!(validate_smo2(-1.0).is_err());
        assert!(validate_smo2(101.0).is_err());
    }

    #[test]
    fn test_calibration() {
        let raw = 1000;
        let cal_factor = 0.5;  // 0.5 watts per count
        let zero = 50;

        let power = apply_calibration(raw, cal_factor, zero);
        assert_eq!(power, 475.0);  // (1000 - 50) * 0.5
    }

    #[test]
    fn test_sensor_quality() {
        // Test constant values detection
        let stuck = vec![75.0; 20];
        let warnings = check_sensor_quality(&stuck);
        assert!(!warnings.is_empty());

        // Test normal variation
        let normal = vec![75.0, 76.0, 75.5, 77.0, 76.5];
        let warnings = check_sensor_quality(&normal);
        assert!(warnings.is_empty());
    }
}
```

### Integration Tests

```rust
#[test]
fn test_moxy_sensor_import() {
    let importer = FitImporter::new();
    let workouts = importer
        .import_file("tests/fixtures/moxy_workout.fit")
        .unwrap();

    let workout = &workouts[0];

    // Verify Moxy data was extracted
    let registry = importer.registry();
    let uuid = "c4d5e6f7-8901-4bcd-ef01-234567890123";

    assert!(registry.is_registered(uuid));

    // Verify fields are defined
    let smo2 = registry.get_field(uuid, 0).unwrap();
    assert_eq!(smo2.name, "smo2");
    assert_eq!(smo2.scale.unwrap(), 10.0);
}
```

## Best Practices

1. **Validation**: Always validate sensor data against physiological limits
2. **Error Handling**: Handle missing/invalid data gracefully
3. **Calibration**: Store and apply calibration data when available
4. **Quality Checks**: Implement sensor quality detection
5. **Documentation**: Document sensor-specific protocols and requirements
6. **Testing**: Test with real sensor data, including edge cases
7. **Performance**: Consider sampling rates and data volumes

## Next Steps

- [Code Examples](examples/) - Working sensor examples
- [Testing Guide](testing-guide.md) - Comprehensive testing
- [Troubleshooting](troubleshooting.md) - Common sensor issues
