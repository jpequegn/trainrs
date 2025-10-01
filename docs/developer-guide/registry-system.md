# Developer Field Registry System

The developer field registry is TrainRS's central system for recognizing and parsing custom FIT developer fields from various devices and applications.

## Overview

The registry system provides:

- **Automatic Recognition**: Identifies developer fields by UUID
- **Type-Safe Parsing**: Converts raw bytes to appropriate data types
- **Scaling & Units**: Applies scaling factors and unit conversions
- **Extensibility**: Easy addition of new devices and sensors

## Architecture

### Registry Components

```
developer_registry.json
├── applications (by UUID)
│   ├── uuid: Application identifier
│   ├── name: Human-readable name
│   ├── manufacturer: Device maker
│   └── fields: Array of field definitions
│       ├── field_number: FIT field ID (0-255)
│       ├── name: Field name
│       ├── data_type: FIT data type
│       ├── units: Optional units
│       ├── scale: Optional scaling factor
│       ├── offset: Optional offset value
│       └── description: Optional field description
```

### Data Flow

1. **FIT File Import**: Importer reads developer data ID messages
2. **UUID Lookup**: Registry matches UUID to application definition
3. **Field Parsing**: Registry extracts field definitions
4. **Data Extraction**: Field values parsed using registered types
5. **Scaling Applied**: Scale and offset transformations
6. **Output**: Typed, scaled values in workout data

## Registry File Format

### Application Entry

```json
{
  "applications": {
    "a42b5e01-d5e9-4eb6-9f42-91234567890a": {
      "uuid": "a42b5e01-d5e9-4eb6-9f42-91234567890a",
      "name": "Stryd Running Power",
      "manufacturer": "Stryd",
      "version": "1.0",
      "fields": [...]
    }
  }
}
```

### Field Definition

```json
{
  "field_number": 0,
  "name": "running_power",
  "data_type": "uint16",
  "units": "watts",
  "scale": 1.0,
  "offset": 0.0,
  "description": "Instantaneous running power output"
}
```

## Data Types

### Supported FIT Types

| FIT Type | Rust Type | Size | Range |
|----------|-----------|------|-------|
| `uint8` | `u8` | 1 byte | 0-255 |
| `uint16` | `u16` | 2 bytes | 0-65535 |
| `uint32` | `u32` | 4 bytes | 0-4,294,967,295 |
| `sint8` | `i8` | 1 byte | -128 to 127 |
| `sint16` | `i16` | 2 bytes | -32,768 to 32,767 |
| `sint32` | `i32` | 4 bytes | -2,147,483,648 to 2,147,483,647 |
| `float32` | `f32` | 4 bytes | IEEE 754 single |
| `float64` | `f64` | 8 bytes | IEEE 754 double |
| `string` | `String` | Variable | UTF-8 text |
| `byte` | `Vec<u8>` | Variable | Raw bytes |

### Type Selection Guidelines

- **Power/Force**: Use `uint16` (0-65535 W or N)
- **Percentages**: Use `uint16` with scale 10.0 or 100.0 (0-100%)
- **Temperature**: Use `sint16` with scale 10.0 or 100.0 (-273.15°C to 327.67°C)
- **Altitude**: Use `uint16` or `sint32` depending on range
- **Latitude/Longitude**: Use `sint32` (semicircles)
- **Timestamps**: Use `uint32` (seconds since UTC 00:00 Dec 31 1989)

## Scaling and Units

### Scaling Formula

```
actual_value = (raw_value / scale) + offset
```

### Common Scaling Patterns

**Percentage with 1 decimal place**:
```json
{
  "data_type": "uint16",
  "scale": 10.0,
  "units": "%"
}
```
- Raw: 567 → Actual: 56.7%

**Temperature with 2 decimal places**:
```json
{
  "data_type": "sint16",
  "scale": 100.0,
  "units": "°C"
}
```
- Raw: 2537 → Actual: 25.37°C

**Power (no scaling)**:
```json
{
  "data_type": "uint16",
  "scale": 1.0,
  "units": "watts"
}
```
- Raw: 250 → Actual: 250 watts

**Cadence with offset**:
```json
{
  "data_type": "uint8",
  "scale": 1.0,
  "offset": -128.0,
  "units": "rpm"
}
```
- Raw: 218 → Actual: 90 rpm

## Working with the Registry

### Loading the Registry

The registry is automatically loaded from the embedded JSON file:

```rust
use trainrs::import::fit::FitImporter;

let importer = FitImporter::new();
let registry = importer.registry();
```

### Checking Registration

```rust
// Check if UUID is registered
let uuid = "a42b5e01-d5e9-4eb6-9f42-91234567890a";
if registry.is_registered(uuid) {
    println!("Application registered");
}

// Get application info
if let Some(app) = registry.get_application(uuid) {
    println!("App: {} by {}", app.name, app.manufacturer);
}
```

### Retrieving Field Definitions

```rust
// Get specific field definition
let field = registry.get_field(uuid, 0)?;
println!("Field: {} ({})", field.name, field.units.unwrap_or(""));

// Get all fields for an application
let app = registry.get_application(uuid)?;
for field in &app.fields {
    println!("Field {}: {} ({})",
        field.field_number,
        field.name,
        field.data_type
    );
}
```

### Parsing Field Values

The registry automatically handles parsing based on data type:

```rust
use trainrs::import::fit::FieldValue;

// Registry handles type conversion
match registry.parse_field(uuid, field_number, raw_bytes) {
    Some(FieldValue::UInt16(value)) => {
        // Apply scaling if defined
        let actual = field.apply_scaling(value as f64);
        println!("{}: {}{}",
            field.name,
            actual,
            field.units.unwrap_or("")
        );
    }
    Some(FieldValue::Float32(value)) => {
        println!("{}: {}{}", field.name, value, field.units.unwrap_or(""));
    }
    None => println!("Failed to parse field"),
}
```

## Adding New Applications

### Step 1: Gather UUID and Field Information

Extract from FIT files using `fitdump`:

```bash
# Find UUID
fitdump workout.fit | grep -A 5 developer_data_id

# Find field definitions
fitdump workout.fit | grep -A 10 field_description
```

### Step 2: Add to Registry JSON

Edit `src/import/developer_registry.json`:

```json
{
  "applications": {
    "existing-apps": "...",
    "new-uuid-here": {
      "uuid": "12345678-1234-5678-1234-567890abcdef",
      "name": "New Device Name",
      "manufacturer": "Manufacturer Name",
      "version": "1.0",
      "fields": [
        {
          "field_number": 0,
          "name": "custom_metric",
          "data_type": "uint16",
          "units": "custom_unit",
          "scale": 1.0,
          "description": "Description of the metric"
        }
      ]
    }
  }
}
```

### Step 3: Validate JSON

```bash
# Check JSON syntax
cat src/import/developer_registry.json | jq .

# If valid, will pretty-print JSON
# If invalid, shows error location
```

### Step 4: Test Registration

```rust
#[test]
fn test_new_device_registration() {
    let importer = FitImporter::new();
    let registry = importer.registry();

    let uuid = "12345678-1234-5678-1234-567890abcdef";
    assert!(registry.is_registered(uuid));

    let app = registry.get_application(uuid).unwrap();
    assert_eq!(app.name, "New Device Name");

    let field = registry.get_field(uuid, 0).unwrap();
    assert_eq!(field.name, "custom_metric");
}
```

## Built-in Applications

TrainRS includes support for these applications out of the box:

### Running Power
- **Stryd**: Running power, form power, leg spring stiffness
- **Garmin Running Power**: Running power from compatible devices

### Muscle Oxygen
- **Moxy Monitor**: SmO2, tHb measurements
- **BSX Insight**: Muscle oxygen saturation

### Advanced Cycling
- **Garmin Vector**: Pedal-based power with dynamics
- **Wahoo KICKR**: Smart trainer metrics
- **Favero Assioma**: Power meter pedals

### Running Dynamics
- **Garmin Running Dynamics Pod**: Vertical oscillation, ground contact time
- **COROS POD**: Running power and dynamics
- **Stryd**: Advanced running metrics

### Other
- **Wahoo ELEMNT**: Bike computer metrics
- **Stages Power**: Cycling power meters
- **PowerTap**: Power meter systems

See full list in `src/import/developer_registry.json`.

## Registry Limitations

### Current Limitations

1. **Static Registry**: Requires rebuild to add new devices
2. **JSON Format**: Manual editing required
3. **No Runtime Registration**: Can't add fields at runtime
4. **Single File**: All definitions in one JSON file

### Future Enhancements

Potential improvements for future versions:

- **Plugin System**: Load definitions from separate files
- **Runtime Registration**: API for dynamic field addition
- **Auto-Discovery**: Attempt to parse unknown UUIDs
- **User Registry**: User-specific additions without rebuild
- **Field Validation**: Schema validation for definitions

## Troubleshooting

### UUID Not Found

**Symptom**: Registry doesn't recognize UUID from FIT file

**Solutions**:
1. Verify UUID format (lowercase with hyphens)
2. Check UUID matches exactly between FIT and JSON
3. Rebuild project after editing registry
4. Ensure JSON is valid

### Field Parsing Errors

**Symptom**: Field values are incorrect or missing

**Solutions**:
1. Verify `field_number` matches FIT definition
2. Check `data_type` is correct for field
3. Confirm scaling factor is appropriate
4. Ensure byte order matches expectation

### JSON Syntax Errors

**Symptom**: Build fails after editing registry

**Solutions**:
```bash
# Validate JSON
jq . src/import/developer_registry.json

# Common issues:
# - Missing commas between entries
# - Trailing commas in arrays/objects
# - Unquoted strings
# - Incorrect data types
```

### Scaling Issues

**Symptom**: Values are 10x, 100x off expected

**Solutions**:
1. Check if FIT already applies scaling
2. Verify scale factor from device documentation
3. Test with known reference values
4. Compare to manufacturer's software output

## Best Practices

### UUID Format
- Always use lowercase hexadecimal
- Include hyphens in standard UUID format
- Verify UUID uniqueness before adding

### Field Naming
- Use snake_case for field names
- Be descriptive but concise
- Follow existing naming conventions
- Avoid abbreviations unless standard

### Data Types
- Choose smallest type that fits data range
- Use signed types for values that can be negative
- Prefer integer types with scaling over floats
- Document unusual type choices

### Scaling
- Match manufacturer specifications exactly
- Test scaling with known values
- Document scaling rationale in description
- Use standard units where possible

### Documentation
- Always include field descriptions
- Document units clearly
- Note any special considerations
- Link to manufacturer documentation

## See Also

- [Getting Started](getting-started.md) - First integration walkthrough
- [Integration Guide](integration-guide.md) - Detailed integration process
- [Testing Guide](testing-guide.md) - Testing your additions
- [Troubleshooting](troubleshooting.md) - Common problems and solutions
