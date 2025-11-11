# Getting Started

Quick start guide to integrating your first custom developer field.

## Overview

This guide walks you through adding support for a custom developer field in TrainRS. We'll use Stryd running power as an example.

## Prerequisites

- TrainRS source code checked out
- Rust toolchain installed
- Basic understanding of FIT developer fields (see [Developer Field Basics](developer-field-basics.md))
- Sample FIT file with developer fields

## Step 1: Identify the Developer Field

First, you need to identify the UUID and field definitions from your device or application.

### Finding UUID in FIT Files

You can use `fitdump` or inspect the FIT file to find developer field information:

```bash
# Using fitdump (from FIT SDK)
fitdump workout.fit | grep -A 10 developer_data_id

# Output will show:
# developer_data_id:
#   application_id: a42b5e01d5e94eb69f4291234567890a
#   developer_data_index: 0
```

### Getting Field Definitions

Look for `field_description` messages:

```bash
fitdump workout.fit | grep -A 5 field_description

# Output:
# field_description:
#   developer_data_index: 0
#   field_definition_number: 0
#   field_name: running_power
#   fit_base_type_id: uint16
#   units: watts
```

## Step 2: Add to Developer Registry

Edit `src/import/developer_registry.json` to add your application:

```json
{
  "applications": {
    "a42b5e01-d5e9-4eb6-9f42-91234567890a": {
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
        }
      ]
    }
  }
}
```

### Field Properties

- **uuid**: Application UUID (must match FIT file)
- **name**: Human-readable application name
- **manufacturer**: Device/software manufacturer
- **version**: Optional version identifier
- **fields**: Array of field definitions

### Field Definition Properties

- **field_number**: 0-255, must match FIT field definition
- **name**: Field name (snake_case recommended)
- **data_type**: FIT data type (uint8, uint16, sint16, etc.)
- **units**: Optional units string
- **scale**: Optional scaling factor (default: 1.0)
- **offset**: Optional offset value (default: 0.0)
- **description**: Optional field description

## Step 3: Verify Registry Loads

The registry is automatically loaded from the embedded JSON. Verify it loads correctly:

```rust
use trainrs::import::fit::FitImporter;

let importer = FitImporter::new();
let registry = importer.registry();

// Check if your UUID is registered
assert!(registry.is_registered("a42b5e01-d5e9-4eb6-9f42-91234567890a"));

// Get application info
let app = registry.get_application("a42b5e01-d5e9-4eb6-9f42-91234567890a");
assert_eq!(app.unwrap().name, "Stryd Running Power");

// Get specific field
let field = registry.get_field("a42b5e01-d5e9-4eb6-9f42-91234567890a", 0);
assert_eq!(field.unwrap().name, "running_power");
```

## Step 4: Test with Real Data

Create a test to verify your field is properly extracted:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use trainrs::import::fit::FitImporter;

    #[test]
    fn test_stryd_power_import() {
        // Use a real FIT file with Stryd data
        let importer = FitImporter::new();
        let workouts = importer
            .import_file("tests/fixtures/stryd_workout.fit")
            .expect("Failed to import FIT file");

        assert_eq!(workouts.len(), 1);
        let workout = &workouts[0];

        // Verify basic workout data
        assert!(workout.duration_seconds > 0);
        assert!(!workout.data_points.is_empty());

        // Developer fields are automatically extracted
        // Check that the registry recognized the UUID
        let registry = importer.registry();
        assert!(registry.is_registered("a42b5e01-d5e9-4eb6-9f42-91234567890a"));
    }
}
```

## Step 5: Run Tests

Build and test your integration:

```bash
# Build with your changes
cargo build

# Run tests
cargo test test_stryd_power_import

# Run all developer field tests
cargo test developer_registry
```

## Complete Example

Here's a complete example adding support for a muscle oxygen monitor:

### 1. Add to registry JSON

```json
"c4d5e6f7-8901-4bcd-ef01-234567890123": {
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

### 2. Create test

```rust
#[test]
fn test_moxy_import() {
    let importer = FitImporter::new();
    let registry = importer.registry();

    // Verify Moxy is registered
    let uuid = "c4d5e6f7-8901-4bcd-ef01-234567890123";
    assert!(registry.is_registered(uuid));

    // Check field definitions
    let smo2 = registry.get_field(uuid, 0).unwrap();
    assert_eq!(smo2.name, "smo2");
    assert_eq!(smo2.units.as_deref(), Some("%"));
    assert_eq!(smo2.scale.unwrap(), 10.0);

    let thb = registry.get_field(uuid, 1).unwrap();
    assert_eq!(thb.name, "thb");
    assert_eq!(thb.scale.unwrap(), 100.0);

    // Import a FIT file with Moxy data
    let workouts = importer
        .import_file("tests/fixtures/moxy_workout.fit")
        .expect("Failed to import");

    assert!(!workouts.is_empty());
}
```

### 3. Build and verify

```bash
cargo build
cargo test test_moxy_import
```

## Troubleshooting

### UUID Not Found

If the registry doesn't find your UUID:

1. Verify UUID format is correct (with hyphens)
2. Check the JSON syntax is valid
3. Ensure the file is saved
4. Rebuild the project

### Field Not Extracted

If fields aren't being extracted:

1. Verify `field_number` matches the FIT file
2. Check `data_type` is correct
3. Ensure UUID registration succeeded
4. Check FIT file actually contains the developer fields

### Build Errors

If you get build errors after editing JSON:

```bash
# Validate JSON syntax
cat src/import/developer_registry.json | jq .

# If invalid, fix syntax errors
# Common issues: missing commas, unquoted strings, trailing commas
```

## Next Steps

- [Integration Guide](integration-guide.md) - Detailed integration patterns
- [Sensor Integration](sensor-integration.md) - Advanced sensor protocols
- [Code Examples](examples/) - More working examples
- [Testing Guide](testing-guide.md) - Comprehensive testing strategies
