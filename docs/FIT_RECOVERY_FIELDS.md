# FIT Recovery & HRV Field Mappings

**Research Date:** 2024-12-19
**FIT SDK Version:** 21.67.00 (from fitparser 0.4.3)
**Purpose:** Technical specification for implementing recovery metrics extraction from FIT files

## 📋 Overview

This document provides detailed field mappings for extracting recovery, HRV, and physiological monitoring data from FIT files using the fitparser crate.

## 🔍 Key Message Types for Recovery Data

### 1. HRV Message (`MesgNum::Hrv = 78`)

**Purpose:** Heart Rate Variability measurements
**Usage:** Individual RR interval data for HRV calculation

```rust
pub struct HrvRecord {
    pub time: Option<f64>,  // Field 0: Time between beats (scale: 1000.0, units: "s")
}
```

**Field Mapping:**
- `time` (field 0): Time between consecutive heartbeats in seconds
  - Type: `UInt16`
  - Scale: `1000.0` (stored as milliseconds, divided by 1000 for seconds)
  - Used for: RMSSD calculation, HRV analysis

### 2. Stress Level Message (`MesgNum::StressLevel = 227`)

**Purpose:** Calculated stress levels from HRV analysis
**Usage:** FirstBeat stress score (1-100 scale)

```rust
pub struct StressLevelRecord {
    pub stress_level_value: Option<i16>,    // Field 0: Stress score (1-100)
    pub stress_level_time: Option<DateTime<Local>>,  // Field 1: Calculation timestamp
}
```

**Field Mapping:**
- `stress_level_value` (field 0): Stress level (1-100, calculated by FirstBeat)
  - Type: `SInt16`
  - Range: 1-100 (higher = more stress)
- `stress_level_time` (field 1): Timestamp when stress was calculated
  - Type: `DateTime`

### 3. Monitoring Message (`MesgNum::Monitoring = 55`)

**Purpose:** Daily activity and physiological monitoring
**Usage:** Steps, calories, distance, activity intensity, and potentially recovery metrics

```rust
pub struct MonitoringRecord {
    pub device_index: Option<u8>,           // Field 0: Device identifier
    pub calories: Option<u16>,              // Field 1: Accumulated calories (kcal)
    pub distance: Option<f64>,              // Field 2: Accumulated distance (m, scale: 100.0)
    pub cycles: Option<f64>,                // Field 3: Steps/strokes (scale: 2.0)
    pub active_time: Option<u32>,           // Field 4: Active time (s)
    pub activity_type: Option<u8>,          // Field 5: Activity type
    pub activity_subtype: Option<u8>,       // Field 6: Activity subtype
    pub activity_level: Option<u8>,         // Field 7: Activity intensity level
    pub distance_16: Option<u16>,           // Field 8: Distance (16-bit)
    pub cycles_16: Option<u16>,             // Field 9: Cycles (16-bit)
    pub active_time_16: Option<u16>,        // Field 10: Active time (16-bit)
    pub local_timestamp: Option<u32>,       // Field 11: Local timestamp
    pub temperature: Option<i8>,            // Field 12: Temperature (°C)
    pub temperature_min: Option<i8>,        // Field 13: Min temperature (°C)
    pub temperature_max: Option<i8>,        // Field 14: Max temperature (°C)
    // ... more fields potentially including recovery metrics
}
```

### 4. Monitoring Info Message (`MesgNum::MonitoringInfo = 103`)

**Purpose:** Configuration and metadata for monitoring data
**Usage:** Time zone correction, monitoring settings

```rust
pub struct MonitoringInfoRecord {
    pub local_timestamp: Option<DateTime<Local>>, // Field 0: Local time reference
    pub activity_type: Option<Vec<u8>>,           // Field 1: Supported activity types
    pub rest_mode: Option<bool>,                  // Field 2: Rest/sleep mode
    // Potentially more fields for HRV and recovery settings
}
```

## 🔋 Garmin-Specific Recovery Fields

### Body Battery (Developer Fields)

**Expected Location:** Developer fields in Monitoring or Session messages
**UUID Pattern:** Garmin Connect IQ app UUIDs

```rust
// Expected developer field structure
pub struct BodyBatteryData {
    pub level: u8,              // 0-100 energy level
    pub drain_rate: Option<f32>, // Energy drain per hour
    pub charge_rate: Option<f32>, // Energy charge per hour during rest
}
```

### Sleep Data (Developer Fields / Monitoring)

**Expected Sources:**
- Monitoring messages with sleep activity types
- Developer fields from Garmin sleep analysis
- Session messages for sleep sessions

```rust
pub struct SleepData {
    pub total_sleep: u16,       // Total sleep time (minutes)
    pub deep_sleep: u16,        // Deep sleep duration (minutes)
    pub light_sleep: u16,       // Light sleep duration (minutes)
    pub rem_sleep: u16,         // REM sleep duration (minutes)
    pub awake_time: u16,        // Time awake during sleep (minutes)
    pub sleep_score: Option<u8>, // Sleep quality score (0-100)
}
```

### Resting Heart Rate

**Location:** Monitoring message or User Profile
**Field Pattern:** Daily minimum heart rate during rest periods

```rust
pub struct RestingHrData {
    pub resting_hr: u8,         // Beats per minute
    pub measurement_time: DateTime<Local>, // When measured
    pub confidence: Option<u8>,  // Measurement confidence (0-100)
}
```

## 📊 Implementation Strategy

### 1. HRV Calculation Pipeline

```rust
impl FitImporter {
    fn extract_hrv_data(&self, records: &[FitDataRecord]) -> Result<Vec<HrvMeasurement>> {
        let mut hrv_data = Vec::new();

        for record in records {
            match record.kind() {
                MesgNum::Hrv => {
                    // Parse individual RR intervals
                    for field in record.fields() {
                        if field.name() == "time" {
                            if let Value::UInt16(interval_ms) = field.value() {
                                let interval_s = *interval_ms as f64 / 1000.0;
                                hrv_data.push(HrvMeasurement {
                                    rr_interval: interval_s,
                                    timestamp: /* derive from context */
                                });
                            }
                        }
                    }
                }
                MesgNum::StressLevel => {
                    // Parse computed stress scores
                    let mut stress_level = None;
                    let mut stress_time = None;

                    for field in record.fields() {
                        match field.name() {
                            "stress_level_value" => {
                                if let Value::SInt16(level) = field.value() {
                                    stress_level = Some(*level);
                                }
                            }
                            "stress_level_time" => {
                                if let Value::Timestamp(time) = field.value() {
                                    stress_time = Some(time.with_timezone(&Utc));
                                }
                            }
                            _ => {}
                        }
                    }

                    if let (Some(level), Some(time)) = (stress_level, stress_time) {
                        // Store stress measurement
                    }
                }
                _ => {}
            }
        }

        Ok(hrv_data)
    }
}
```

### 2. Recovery Metrics Extraction

```rust
impl FitImporter {
    fn extract_recovery_metrics(&self, records: &[FitDataRecord]) -> Result<RecoveryData> {
        let mut recovery = RecoveryData::default();

        for record in records {
            match record.kind() {
                MesgNum::Monitoring => {
                    // Extract daily monitoring data
                    self.parse_monitoring_fields(record, &mut recovery)?;
                }
                MesgNum::MonitoringInfo => {
                    // Extract monitoring configuration
                    self.parse_monitoring_info(record, &mut recovery)?;
                }
                _ => {}
            }
        }

        Ok(recovery)
    }
}
```

## 🛠️ Developer Field Extraction

### Common Garmin Developer Field UUIDs

**Body Battery:**
- App UUID: `adbd5f10-7f65-4d92-aa59-f87a25e2b9c3` (estimated)
- Field: Body Battery level, drain rate, charge rate

**Advanced Sleep Metrics:**
- App UUID: `sleep-analysis-uuid` (varies by device)
- Fields: Sleep stages, efficiency, interruptions

**HRV Status:**
- App UUID: `hrv-status-uuid` (varies by device)
- Fields: HRV baseline, status (balanced/unbalanced/poor)

```rust
fn parse_developer_fields(&self, record: &FitDataRecord) -> Result<HashMap<String, Value>> {
    let mut dev_fields = HashMap::new();

    for field in record.fields() {
        // Check if field name matches known developer field patterns
        match field.name() {
            name if name.contains("body_battery") => {
                dev_fields.insert("body_battery".to_string(), field.value().clone());
            }
            name if name.contains("hrv_status") => {
                dev_fields.insert("hrv_status".to_string(), field.value().clone());
            }
            name if name.contains("sleep_score") => {
                dev_fields.insert("sleep_score".to_string(), field.value().clone());
            }
            _ => {}
        }
    }

    Ok(dev_fields)
}
```

## 📈 Expected Data Patterns

### File Types for Recovery Data

1. **Activity Files** (`File::Activity`):
   - HRV data during workouts
   - Recovery heart rate measurements
   - Real-time stress levels

2. **Monitoring Files** (`File::MonitoringA`, `File::MonitoringDaily`):
   - Daily HRV measurements
   - Sleep data
   - Body Battery trends
   - Resting heart rate

3. **Settings Files** (`File::Settings`):
   - HRV baselines
   - Recovery preferences
   - Personal thresholds

### Typical Data Frequency

- **HRV**: 1-5 measurements per day (morning, pre/post workout)
- **Stress**: Continuous throughout day (every 5-15 minutes)
- **Body Battery**: Continuous (every minute during wear)
- **Sleep**: Once per sleep session
- **Resting HR**: Once per day (typically morning)

## 🔍 Field Discovery Strategy

Since FIT format evolves and Garmin adds new recovery fields, implement dynamic field discovery:

```rust
fn discover_recovery_fields(&self, records: &[FitDataRecord]) -> HashMap<String, FieldInfo> {
    let mut discovered_fields = HashMap::new();

    for record in records {
        if matches!(record.kind(), MesgNum::Monitoring | MesgNum::StressLevel | MesgNum::Hrv) {
            for field in record.fields() {
                let field_name = field.name().to_lowercase();

                // Pattern matching for recovery-related fields
                if field_name.contains("stress") ||
                   field_name.contains("hrv") ||
                   field_name.contains("recovery") ||
                   field_name.contains("sleep") ||
                   field_name.contains("battery") ||
                   field_name.contains("resting") {

                    discovered_fields.insert(
                        field.name().to_string(),
                        FieldInfo {
                            field_type: field.value().type_name(),
                            units: field.units().to_string(),
                            scale: /* extract from field definition */,
                        }
                    );
                }
            }
        }
    }

    discovered_fields
}
```

## 🎯 Implementation Priority

### Phase 1: Basic HRV & Stress
1. ✅ HRV message parsing (`MesgNum::Hrv`)
2. ✅ Stress level extraction (`MesgNum::StressLevel`)
3. ✅ Basic RMSSD calculation from RR intervals

### Phase 2: Monitoring Integration
1. ✅ Monitoring message parsing (`MesgNum::Monitoring`)
2. ✅ Daily metric aggregation
3. ✅ Activity type correlation

### Phase 3: Advanced Recovery
1. ✅ Developer field extraction for Body Battery
2. ✅ Sleep data parsing and analysis
3. ✅ Resting heart rate trend analysis

### Phase 4: Analytics & Insights
1. ✅ HRV baseline calculation
2. ✅ Recovery recommendation algorithms
3. ✅ Training readiness scoring

## 📚 References

- **FIT SDK Documentation**: Official Garmin FIT SDK v21.67.00
- **FirstBeat Algorithm**: Stress and recovery calculation methods
- **HRV Standards**: Task Force of European Society of Cardiology
- **fitparser Crate**: Rust FIT file parsing library v0.4.3

---

**Note:** Field availability depends on device model and firmware version. Always implement graceful fallbacks for missing data.