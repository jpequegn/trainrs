# Integration Guide

Comprehensive guide to integrating custom developer fields and sensors into TrainRS.

## Integration Overview

Integrating custom developer fields involves:

1. Field identification and documentation
2. Registry configuration
3. Data extraction (automatic via registry)
4. Validation and testing
5. Documentation

## Architecture

TrainRS uses a registry-based system for developer field parsing:

```
FIT File → FitImporter → DeveloperFieldRegistry → Automatic Parsing
                                ↓
                         developer_registry.json
                                ↓
                         Field Definitions
```

### Components

**FitImporter** (`src/import/fit.rs`)
- Parses FIT files
- Detects developer data ID messages
- Extracts field descriptions
- Looks up fields in registry

**DeveloperFieldRegistry** (`src/import/developer_registry.rs`)
- Stores field definitions
- Provides lookup methods
- Loaded from embedded JSON

**developer_registry.json** (`src/import/developer_registry.json`)
- JSON configuration file
- Contains all field definitions
- Compiled into binary

## Step-by-Step Integration

### 1. Gather Field Information

Before adding a field, collect:

- **UUID**: Application identifier
- **Field numbers**: 0-255 for each field
- **Data types**: uint8, uint16, etc.
- **Units**: watts, rpm, %, etc.
- **Scaling**: How to convert raw values
- **Documentation**: What each field means

#### From FIT Files

```bash
# Install FIT SDK tools
# Download from: https://developer.garmin.com/fit/download/

# Dump developer field info
fitdump workout.fit | grep -A 20 developer_data_id
fitdump workout.fit | grep -A 10 field_description
```

#### From Device Documentation

Check manufacturer documentation:
- API documentation
- Developer guides
- FIT field specifications
- Example files

### 2. Design Field Schema

Plan your field structure:

```rust
// Example: Cycling power meter with pedal dynamics

UUID: f7890123-4567-4ef0-1234-567890123456
Application: Garmin Vector
Manufacturer: Garmin

Fields:
  0: left_power_phase_start (uint8, degrees, scale=1)
  1: left_power_phase_end (uint8, degrees, scale=1)
  2: right_power_phase_start (uint8, degrees, scale=1)
  3: right_power_phase_end (uint8, degrees, scale=1)
  4: left_peak_power_phase_start (uint8, degrees, scale=1)
  5: left_peak_power_phase_end (uint8, degrees, scale=1)
  6: left_pedal_smoothness (uint8, %, scale=2)
  7: right_pedal_smoothness (uint8, %, scale=2)
```

### 3. Add to Registry JSON

Edit `src/import/developer_registry.json`:

```json
{
  "applications": {
    "f7890123-4567-4ef0-1234-567890123456": {
      "uuid": "f7890123-4567-4ef0-1234-567890123456",
      "name": "Garmin Vector",
      "manufacturer": "Garmin",
      "version": "3.0",
      "fields": [
        {
          "field_number": 0,
          "name": "left_power_phase_start",
          "data_type": "uint8",
          "units": "degrees",
          "scale": 1.0,
          "description": "Left pedal power phase start angle"
        },
        {
          "field_number": 1,
          "name": "left_power_phase_end",
          "data_type": "uint8",
          "units": "degrees",
          "scale": 1.0,
          "description": "Left pedal power phase end angle"
        },
        {
          "field_number": 6,
          "name": "left_pedal_smoothness",
          "data_type": "uint8",
          "units": "%",
          "scale": 2.0,
          "description": "Left pedal smoothness percentage"
        },
        {
          "field_number": 7,
          "name": "right_pedal_smoothness",
          "data_type": "uint8",
          "units": "%",
          "scale": 2.0,
          "description": "Right pedal smoothness percentage"
        }
      ]
    }
  }
}
```

### 4. Validate JSON Schema

Ensure JSON is valid:

```bash
# Use jq to validate and format
cat src/import/developer_registry.json | jq . > /dev/null
echo $?  # Should output 0 for success

# Pretty-print to check structure
cat src/import/developer_registry.json | jq '.applications | keys'
```

### 5. Write Tests

Create comprehensive tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use trainrs::import::fit::{FitImporter, DeveloperFieldRegistry};

    #[test]
    fn test_vector_registry() {
        let registry = DeveloperFieldRegistry::from_embedded()
            .expect("Failed to load registry");

        let uuid = "f7890123-4567-4ef0-1234-567890123456";

        // Test UUID registration
        assert!(
            registry.is_registered(uuid),
            "Garmin Vector should be registered"
        );

        // Test application info
        let app = registry.get_application(uuid).unwrap();
        assert_eq!(app.name, "Garmin Vector");
        assert_eq!(app.manufacturer, "Garmin");
        assert_eq!(app.version.as_deref(), Some("3.0"));

        // Test field lookups
        let field0 = registry.get_field(uuid, 0).unwrap();
        assert_eq!(field0.name, "left_power_phase_start");
        assert_eq!(field0.data_type, "uint8");
        assert_eq!(field0.units.as_deref(), Some("degrees"));

        let field6 = registry.get_field(uuid, 6).unwrap();
        assert_eq!(field6.name, "left_pedal_smoothness");
        assert_eq!(field6.scale.unwrap(), 2.0);
    }

    #[test]
    fn test_vector_import() {
        let importer = FitImporter::new();

        // Import FIT file with Vector data
        let workouts = importer
            .import_file("tests/fixtures/vector_workout.fit")
            .expect("Failed to import Vector workout");

        assert!(!workouts.is_empty());
        let workout = &workouts[0];

        // Verify workout has data
        assert!(workout.duration_seconds > 0);
        assert!(!workout.data_points.is_empty());

        // Verify registry recognized the UUID
        let registry = importer.registry();
        assert!(registry.is_registered("f7890123-4567-4ef0-1234-567890123456"));
    }

    #[test]
    fn test_uuid_bytes_conversion() {
        let registry = DeveloperFieldRegistry::from_embedded().unwrap();

        // Test UUID lookup by bytes
        let uuid_str = "f7890123-4567-4ef0-1234-567890123456";
        let uuid = uuid::Uuid::parse_str(uuid_str).unwrap();
        let uuid_bytes = uuid.as_bytes();

        assert!(registry.is_registered_by_bytes(uuid_bytes));

        let app = registry.get_application_by_bytes(uuid_bytes).unwrap();
        assert_eq!(app.name, "Garmin Vector");

        let field = registry.get_field_by_bytes(uuid_bytes, 0).unwrap();
        assert_eq!(field.name, "left_power_phase_start");
    }
}
```

### 6. Add Test Fixtures

Create test FIT files:

```bash
# Create test fixtures directory
mkdir -p tests/fixtures

# Add your FIT file
cp ~/Downloads/vector_workout.fit tests/fixtures/

# Document the file
cat > tests/fixtures/README.md << EOF
# Test Fixtures

## vector_workout.fit
- Source: Garmin Vector 3 power meter
- Duration: 60 minutes
- Contains: Pedal dynamics data
- Developer fields: Power phase angles, pedal smoothness
EOF
```

### 7. Build and Test

```bash
# Build with new registry
cargo build

# Run specific tests
cargo test test_vector_registry
cargo test test_vector_import

# Run all developer field tests
cargo test developer_registry

# Check test coverage
cargo test --all-features
```

## Advanced Integration Patterns

### Multi-Sensor Applications

For applications with multiple sensor types:

```json
{
  "uuid": "multi-sensor-uuid",
  "name": "Multi-Sensor Platform",
  "manufacturer": "Vendor",
  "version": "2.0",
  "fields": [
    {
      "field_number": 0,
      "name": "sensor_type",
      "data_type": "uint8",
      "description": "Sensor type identifier (0=HR, 1=Power, 2=Cadence)"
    },
    {
      "field_number": 1,
      "name": "sensor_location",
      "data_type": "uint8",
      "description": "Sensor location (0=chest, 1=left_arm, 2=right_arm)"
    },
    {
      "field_number": 2,
      "name": "sensor_value",
      "data_type": "uint16",
      "description": "Sensor reading (interpretation depends on type)"
    }
  ]
}
```

### Complex Scaling

For fields with complex transformations:

```json
{
  "field_number": 10,
  "name": "leg_spring_stiffness",
  "data_type": "uint16",
  "units": "kN/m",
  "scale": 10.0,
  "offset": 0.0,
  "description": "Leg spring stiffness: (raw / 10) kN/m"
}
```

Usage:
```rust
// Raw value from FIT: 85
// Calculation: 85 / 10.0 = 8.5 kN/m
let raw: u16 = 85;
let scale = 10.0;
let stiffness = raw as f64 / scale;  // 8.5
```

### Temperature with Offset

```json
{
  "field_number": 15,
  "name": "core_temperature",
  "data_type": "uint16",
  "units": "°C",
  "scale": 100.0,
  "offset": -273.15,
  "description": "Core body temperature: (raw / 100) + offset"
}
```

Usage:
```rust
// Raw value: 31015 (representing 310.15K)
// Calculation: (31015 / 100) - 273.15 = 37.0°C
let raw: u16 = 31015;
let scale = 100.0;
let offset = -273.15;
let temp = (raw as f64 / scale) + offset;  // 37.0
```

## Custom Data Validation

While TrainRS automatically parses registered fields, you may want custom validation:

```rust
use trainrs::import::fit::{FitImporter, DeveloperFieldRegistry};

fn validate_stryd_data(workout: &Workout) -> Result<(), String> {
    // Check that running power is reasonable
    for point in &workout.data_points {
        if let Some(power) = point.power {
            if power > 500 {
                return Err(format!(
                    "Unrealistic running power: {} watts at timestamp {}",
                    power, point.timestamp
                ));
            }
        }
    }

    Ok(())
}

#[test]
fn test_stryd_data_validation() {
    let importer = FitImporter::new();
    let workouts = importer
        .import_file("tests/fixtures/stryd_workout.fit")
        .unwrap();

    for workout in &workouts {
        validate_stryd_data(workout).expect("Invalid Stryd data");
    }
}
```

## Registry Management

### Loading Custom Registry

Create and use a custom registry:

```rust
use trainrs::import::fit::{FitImporter, DeveloperFieldRegistry};

// Load from custom JSON file
let json = std::fs::read_to_string("custom_registry.json")?;
let registry = DeveloperFieldRegistry::from_json(&json)?;

// Create importer with custom registry
let importer = FitImporter::with_registry(registry);

// Import with custom field definitions
let workouts = importer.import_file("workout.fit")?;
```

### Extending Default Registry

Add fields to the default registry:

```rust
use trainrs::import::fit::{DeveloperFieldRegistry, ApplicationInfo, KnownField};

// Start with default registry
let mut registry = DeveloperFieldRegistry::from_embedded()?;

// Add custom application
let custom_app = ApplicationInfo {
    uuid: "custom-uuid".to_string(),
    name: "Custom App".to_string(),
    manufacturer: "Me".to_string(),
    version: Some("1.0".to_string()),
    fields: vec![
        KnownField {
            field_number: 0,
            name: "custom_metric".to_string(),
            data_type: "uint16".to_string(),
            units: Some("custom_units".to_string()),
            scale: Some(1.0),
            offset: None,
            description: Some("My custom metric".to_string()),
        },
    ],
};

registry.register_application(custom_app);

// Use extended registry
let importer = FitImporter::with_registry(registry);
```

## Best Practices

### 1. Documentation

Always document:
- Field meanings and interpretations
- Units and scaling formulas
- Valid ranges
- Special values (e.g., 0xFF = invalid)
- Dependencies between fields

### 2. Testing

Test with:
- Real device data
- Edge cases (min/max values)
- Invalid data
- Missing fields
- Multiple sensors

### 3. Versioning

- Include version in registry
- Document breaking changes
- Maintain backward compatibility
- Support multiple versions if needed

### 4. Error Handling

- Validate field values
- Handle missing fields gracefully
- Check for out-of-range values
- Log warnings for suspicious data

### 5. Performance

- Use appropriate data types
- Minimize conversions
- Cache registry lookups
- Avoid unnecessary allocations

## Next Steps

- [Sensor Integration](sensor-integration.md) - Advanced sensor protocols
- [Code Examples](examples/) - Working examples
- [Testing Guide](testing-guide.md) - Comprehensive testing
- [Troubleshooting](troubleshooting.md) - Common issues
