# Testing Guide: Developer Field Integration

Comprehensive testing strategies for custom developer field integrations.

## Overview

Testing ensures your developer field integration:

- **Correctly parses** FIT files with custom fields
- **Accurately scales** and converts data values
- **Handles errors** gracefully (missing fields, corrupt data)
- **Maintains compatibility** across FIT SDK versions
- **Performs well** with large datasets

## Testing Levels

### 1. Unit Tests (Registry)

Test individual components of the registry system.

#### Registry Loading

```rust
#[cfg(test)]
mod registry_tests {
    use trainrs::import::fit::FitImporter;

    #[test]
    fn test_registry_loads() {
        let importer = FitImporter::new();
        let registry = importer.registry();

        // Verify registry is not empty
        assert!(registry.application_count() > 0);
    }

    #[test]
    fn test_stryd_registration() {
        let importer = FitImporter::new();
        let registry = importer.registry();

        let uuid = "a42b5e01-d5e9-4eb6-9f42-91234567890a";
        assert!(registry.is_registered(uuid));

        let app = registry.get_application(uuid).unwrap();
        assert_eq!(app.name, "Stryd Running Power");
        assert_eq!(app.manufacturer, "Stryd");
    }

    #[test]
    fn test_field_definitions() {
        let importer = FitImporter::new();
        let registry = importer.registry();

        let uuid = "a42b5e01-d5e9-4eb6-9f42-91234567890a";

        // Test field 0 (running power)
        let field = registry.get_field(uuid, 0).unwrap();
        assert_eq!(field.name, "running_power");
        assert_eq!(field.data_type, "uint16");
        assert_eq!(field.units.as_deref(), Some("watts"));

        // Test field 1 (form power)
        let field = registry.get_field(uuid, 1).unwrap();
        assert_eq!(field.name, "form_power");
    }
}
```

#### Scaling Tests

```rust
#[test]
fn test_field_scaling() {
    let importer = FitImporter::new();
    let registry = importer.registry();

    // Test percentage scaling (scale: 10.0)
    let uuid = "moxy-uuid-here";
    let field = registry.get_field(uuid, 0).unwrap(); // SmO2

    let raw_value = 567; // Raw FIT value
    let actual_value = field.apply_scaling(raw_value as f64);
    assert_eq!(actual_value, 56.7); // 567 / 10.0
}

#[test]
fn test_temperature_scaling() {
    let importer = FitImporter::new();
    let registry = importer.registry();

    // Test temperature with scale 100.0
    let uuid = "temp-sensor-uuid";
    let field = registry.get_field(uuid, 0).unwrap();

    let raw_value = 2537;
    let actual_value = field.apply_scaling(raw_value as f64);
    assert_eq!(actual_value, 25.37); // 2537 / 100.0
}

#[test]
fn test_offset_scaling() {
    let importer = FitImporter::new();
    let registry = importer.registry();

    // Test offset transformation
    let uuid = "device-with-offset";
    let field = registry.get_field(uuid, 0).unwrap();

    // Formula: (raw / scale) + offset
    let raw_value = 218;
    let actual_value = field.apply_scaling(raw_value as f64);
    // Expected: (218 / 1.0) + (-128) = 90
    assert_eq!(actual_value, 90.0);
}
```

### 2. Integration Tests (FIT Import)

Test complete FIT file import with developer fields.

#### Basic Import Test

```rust
#[cfg(test)]
mod integration_tests {
    use trainrs::import::fit::FitImporter;
    use std::path::Path;

    #[test]
    fn test_stryd_fit_import() {
        let fit_path = "tests/fixtures/stryd_workout.fit";
        assert!(Path::new(fit_path).exists(), "Test file not found");

        let importer = FitImporter::new();
        let workouts = importer
            .import_file(fit_path)
            .expect("Failed to import FIT file");

        assert_eq!(workouts.len(), 1);
        let workout = &workouts[0];

        // Basic workout properties
        assert!(workout.duration_seconds > 0);
        assert!(workout.sport.is_some());
        assert!(!workout.data_points.is_empty());

        // Verify Stryd UUID was recognized
        let registry = importer.registry();
        let stryd_uuid = "a42b5e01-d5e9-4eb6-9f42-91234567890a";
        assert!(registry.is_registered(stryd_uuid));
    }
}
```

#### Data Validation Test

```rust
#[test]
fn test_developer_field_values() {
    let importer = FitImporter::new();
    let workouts = importer
        .import_file("tests/fixtures/stryd_workout.fit")
        .expect("Failed to import");

    let workout = &workouts[0];

    // Check that developer fields were extracted
    assert!(workout.has_developer_fields());

    // Validate data point with developer fields
    if let Some(point) = workout.data_points.first() {
        // Check for Stryd running power
        if let Some(dev_fields) = &point.developer_fields {
            let stryd_uuid = "a42b5e01-d5e9-4eb6-9f42-91234567890a";

            // Check running power field (field 0)
            if let Some(power) = dev_fields.get_field(stryd_uuid, 0) {
                assert!(power > 0.0 && power < 1000.0,
                    "Power value out of range: {}", power);
            }
        }
    }
}
```

#### Multi-Device Test

```rust
#[test]
fn test_multiple_developer_apps() {
    let importer = FitImporter::new();
    let workouts = importer
        .import_file("tests/fixtures/multi_device_workout.fit")
        .expect("Failed to import");

    let workout = &workouts[0];
    let dev_fields = workout.data_points[0]
        .developer_fields
        .as_ref()
        .unwrap();

    // Verify both Stryd and Moxy data present
    let stryd_uuid = "a42b5e01-d5e9-4eb6-9f42-91234567890a";
    let moxy_uuid = "c4d5e6f7-8901-4bcd-ef01-234567890123";

    assert!(dev_fields.has_application(stryd_uuid));
    assert!(dev_fields.has_application(moxy_uuid));

    // Verify specific fields
    assert!(dev_fields.get_field(stryd_uuid, 0).is_some()); // Running power
    assert!(dev_fields.get_field(moxy_uuid, 0).is_some()); // SmO2
}
```

### 3. Property-Based Tests

Test with automatically generated inputs to catch edge cases.

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_scaling_never_panics(
        raw_value in 0u16..=65535,
        scale in 1.0f64..=1000.0
    ) {
        let importer = FitImporter::new();
        let registry = importer.registry();

        // Create a field with the given scale
        let mut field = FieldDefinition::default();
        field.scale = Some(scale);

        // Should never panic, regardless of inputs
        let result = field.apply_scaling(raw_value as f64);
        assert!(result.is_finite());
    }

    #[test]
    fn test_uuid_parsing(uuid_str: String) {
        let importer = FitImporter::new();
        let registry = importer.registry();

        // Should handle any string without panicking
        let result = registry.is_registered(&uuid_str);
        // Result can be true or false, but shouldn't crash
        assert!(result == true || result == false);
    }
}
```

### 4. Error Handling Tests

Test robustness with invalid or missing data.

```rust
#[test]
fn test_missing_uuid() {
    let importer = FitImporter::new();
    let registry = importer.registry();

    let fake_uuid = "00000000-0000-0000-0000-000000000000";
    assert!(!registry.is_registered(fake_uuid));
    assert!(registry.get_application(fake_uuid).is_none());
}

#[test]
fn test_missing_field() {
    let importer = FitImporter::new();
    let registry = importer.registry();

    let stryd_uuid = "a42b5e01-d5e9-4eb6-9f42-91234567890a";

    // Field 99 doesn't exist for Stryd
    assert!(registry.get_field(stryd_uuid, 99).is_none());
}

#[test]
fn test_corrupt_fit_file() {
    let importer = FitImporter::new();

    // Should return error, not panic
    let result = importer.import_file("tests/fixtures/corrupt.fit");
    assert!(result.is_err());
}

#[test]
fn test_fit_without_developer_fields() {
    let importer = FitImporter::new();
    let workouts = importer
        .import_file("tests/fixtures/basic_workout.fit")
        .expect("Should import successfully");

    let workout = &workouts[0];

    // Should handle absence of developer fields gracefully
    assert!(!workout.has_developer_fields());
}
```

### 5. Performance Tests

Ensure good performance with large datasets.

```rust
#[test]
fn test_large_file_performance() {
    use std::time::Instant;

    let importer = FitImporter::new();
    let start = Instant::now();

    let workouts = importer
        .import_file("tests/fixtures/large_workout.fit")
        .expect("Failed to import");

    let duration = start.elapsed();

    // Should import within reasonable time (e.g., < 5 seconds)
    assert!(duration.as_secs() < 5, "Import took too long: {:?}", duration);

    // Verify data was imported
    assert!(!workouts.is_empty());
    assert!(workouts[0].data_points.len() > 1000);
}

#[test]
fn test_batch_import() {
    let importer = FitImporter::new();
    let files = vec![
        "tests/fixtures/workout1.fit",
        "tests/fixtures/workout2.fit",
        "tests/fixtures/workout3.fit",
    ];

    let start = std::time::Instant::now();

    for file in files {
        importer.import_file(file).expect("Import failed");
    }

    let duration = start.elapsed();
    assert!(duration.as_secs() < 10, "Batch import too slow");
}
```

## Test Fixtures

### Creating Test FIT Files

#### Option 1: Use Real Device Data

Best approach: Use actual FIT files from your device.

```bash
# Copy workout from device
cp /Volumes/GARMIN/GARMIN/Activity/2024-10-01-123456.fit \
   tests/fixtures/stryd_workout.fit

# Verify it has developer fields
fitdump tests/fixtures/stryd_workout.fit | grep developer
```

#### Option 2: Create Synthetic FIT Files

For testing specific scenarios:

```rust
// Example: Create minimal FIT with developer fields
use fitparse::{FitFile, FitMessage};

fn create_test_fit() -> Vec<u8> {
    let mut fit = FitFile::new();

    // Add developer data ID
    fit.add_developer_data_id(
        0, // index
        "a42b5e01-d5e9-4eb6-9f42-91234567890a" // Stryd UUID
    );

    // Add field description
    fit.add_field_description(
        0, // dev data index
        0, // field number
        "running_power",
        "uint16",
        "watts"
    );

    // Add record with developer field
    fit.add_record_with_developer_field(
        0, // dev data index
        0, // field number
        250u16 // running power value
    );

    fit.to_bytes()
}

#[test]
fn test_synthetic_fit() {
    let fit_data = create_test_fit();
    std::fs::write("tests/fixtures/synthetic.fit", fit_data).unwrap();

    let importer = FitImporter::new();
    let workouts = importer
        .import_bytes(&fit_data)
        .expect("Import failed");

    assert_eq!(workouts.len(), 1);
}
```

### Test Fixture Organization

```
tests/
├── fixtures/
│   ├── stryd/
│   │   ├── run_with_power.fit
│   │   ├── run_without_power.fit
│   │   └── corrupt_power.fit
│   ├── moxy/
│   │   ├── smo2_workout.fit
│   │   └── thb_workout.fit
│   ├── multi_device/
│   │   ├── stryd_plus_moxy.fit
│   │   └── garmin_dynamics.fit
│   └── edge_cases/
│       ├── empty_fields.fit
│       ├── unknown_uuid.fit
│       └── large_workout.fit
└── integration_tests.rs
```

## Running Tests

### Run All Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_stryd_registration
```

### Run Category Tests

```bash
# Registry tests only
cargo test registry_tests

# Integration tests only
cargo test integration_tests

# Property tests (slower)
cargo test proptest
```

### Run Performance Tests

```bash
# Performance tests with release optimizations
cargo test --release test_large_file_performance

# With timing information
cargo test -- --nocapture test_performance
```

## Test Coverage

### Measure Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin --out Html --output-dir coverage

# View report
open coverage/index.html
```

### Coverage Goals

- **Registry Loading**: 100% (critical path)
- **Field Parsing**: 95%+ (core functionality)
- **Error Handling**: 90%+ (graceful degradation)
- **Integration**: 85%+ (end-to-end workflows)

## Continuous Integration

### GitHub Actions Example

```yaml
name: Developer Fields Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run Registry Tests
        run: cargo test registry_tests

      - name: Run Integration Tests
        run: cargo test integration_tests

      - name: Check Coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --ignore-tests --out Xml

      - name: Upload Coverage
        uses: codecov/codecov-action@v3
```

## Debugging Tests

### Enable Logging

```rust
#[test]
fn test_with_logging() {
    env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .init();

    // Test code here
    let importer = FitImporter::new();
    // ...
}
```

### Inspect FIT Files

```bash
# Dump FIT file contents
fitdump -v tests/fixtures/stryd_workout.fit > stryd_dump.txt

# Search for specific fields
fitdump tests/fixtures/stryd_workout.fit | grep -A 10 "field_description"

# Check developer data
fitdump tests/fixtures/stryd_workout.fit | grep -A 5 "developer_data"
```

### Debug Registry Loading

```rust
#[test]
fn debug_registry() {
    let importer = FitImporter::new();
    let registry = importer.registry();

    // Print all registered UUIDs
    for uuid in registry.application_uuids() {
        println!("UUID: {}", uuid);
        if let Some(app) = registry.get_application(uuid) {
            println!("  Name: {}", app.name);
            println!("  Fields: {}", app.fields.len());
            for field in &app.fields {
                println!("    {}: {} ({})",
                    field.field_number,
                    field.name,
                    field.data_type
                );
            }
        }
    }
}
```

## Best Practices

### Test Organization

1. **Group Related Tests**: Use modules to organize tests by feature
2. **Descriptive Names**: Use clear, specific test names
3. **Document Expected Behavior**: Add comments explaining what you're testing
4. **Test One Thing**: Each test should verify a single behavior

### Test Data

1. **Use Real Data**: Prefer actual device FIT files when possible
2. **Version Control**: Check in test fixtures (if size permits)
3. **Document Source**: Note where test files came from
4. **Sanitize Sensitive Data**: Remove personal information from test files

### Assertions

1. **Specific Assertions**: Test exact values, not just presence
2. **Error Messages**: Include helpful messages in assertions
3. **Range Checks**: Validate values are in expected ranges
4. **Type Safety**: Verify correct types are returned

### Maintenance

1. **Update Tests**: When adding fields, add corresponding tests
2. **Review Failures**: Investigate and fix failing tests promptly
3. **Refactor Together**: Update tests when refactoring code
4. **Document Quirks**: Note unusual device behaviors in test comments

## Troubleshooting Test Failures

### Registry Not Loading

**Symptom**: Tests fail with "registry not found" errors

**Solutions**:
```bash
# Verify JSON is valid
jq . src/import/developer_registry.json

# Rebuild project
cargo clean && cargo build

# Check file is included in binary
cargo build --verbose
```

### Field Values Incorrect

**Symptom**: Assertions fail on field values

**Solutions**:
1. Verify scaling factor is correct
2. Check data type matches FIT file
3. Inspect raw bytes in FIT file
4. Compare with manufacturer's software

### Import Failures

**Symptom**: FIT import returns errors

**Solutions**:
1. Verify FIT file is not corrupt
2. Check FIT SDK version compatibility
3. Ensure file has developer fields
4. Test with minimal FIT file

## See Also

- [Integration Guide](integration-guide.md) - Step-by-step integration
- [Registry System](registry-system.md) - Registry architecture
- [Troubleshooting](troubleshooting.md) - Common problems
- [Best Practices](best-practices.md) - Development guidelines
