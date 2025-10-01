# Troubleshooting Guide

Common issues and solutions when integrating custom developer fields.

## Registry Issues

### UUID Not Registered

**Symptom**: Registry lookup returns `None` for your UUID.

**Causes**:
1. UUID format incorrect
2. JSON syntax error
3. Registry not rebuilt
4. UUID mismatch

**Solutions**:

```bash
# 1. Validate JSON syntax
cat src/import/developer_registry.json | jq .

# 2. Check UUID format (must include hyphens)
# Correct:   "a42b5e01-d5e9-4eb6-9f42-91234567890a"
# Incorrect: "a42b5e01d5e94eb69f4291234567890a"

# 3. Rebuild project
cargo clean
cargo build

# 4. Verify UUID in FIT file matches registry
fitdump workout.fit | grep application_id
```

**Debug Code**:

```rust
#[test]
fn debug_registry_lookup() {
    let registry = DeveloperFieldRegistry::from_embedded().unwrap();

    let uuid = "a42b5e01-d5e9-4eb6-9f42-91234567890a";

    // Check registration
    println!("Is registered: {}", registry.is_registered(uuid));

    // List all registered UUIDs
    println!("Registered UUIDs:");
    for reg_uuid in registry.registered_uuids() {
        println!("  {}", reg_uuid);
    }

    // Try getting application
    match registry.get_application(uuid) {
        Some(app) => println!("Found: {}", app.name),
        None => println!("Not found"),
    }
}
```

### Field Not Found

**Symptom**: `get_field()` returns `None` for a field number.

**Causes**:
1. Field number mismatch
2. Field not in JSON
3. Wrong UUID

**Solutions**:

```rust
#[test]
fn debug_field_lookup() {
    let registry = DeveloperFieldRegistry::from_embedded().unwrap();
    let uuid = "a42b5e01-d5e9-4eb6-9f42-91234567890a";

    // Get application
    let app = registry.get_application(uuid).unwrap();

    // List all fields
    println!("Fields for {}:", app.name);
    for field in &app.fields {
        println!("  {}: {} ({})",
            field.field_number,
            field.name,
            field.data_type
        );
    }

    // Try specific field
    let field_num = 0;
    match registry.get_field(uuid, field_num) {
        Some(f) => println!("Field {}: {}", field_num, f.name),
        None => println!("Field {} not found", field_num),
    }
}
```

## JSON Issues

### Syntax Errors

**Symptom**: Build fails with JSON parsing error.

**Common Errors**:

1. **Missing commas**:
```json
{
  "uuid": "...",
  "name": "App"  // Missing comma!
  "manufacturer": "Vendor"
}
```

Fix:
```json
{
  "uuid": "...",
  "name": "App",  // Added comma
  "manufacturer": "Vendor"
}
```

2. **Trailing commas**:
```json
{
  "fields": [
    { "field_number": 0 },
    { "field_number": 1 },  // Trailing comma!
  ]
}
```

Fix:
```json
{
  "fields": [
    { "field_number": 0 },
    { "field_number": 1 }  // Removed trailing comma
  ]
}
```

3. **Unquoted strings**:
```json
{
  "name": Stryd  // Missing quotes!
}
```

Fix:
```json
{
  "name": "Stryd"  // Added quotes
}
```

**Validation**:

```bash
# Validate JSON
jq . src/import/developer_registry.json

# Pretty-print to check structure
jq . src/import/developer_registry.json > temp.json
mv temp.json src/import/developer_registry.json

# Check for common issues
grep -n ",$]" src/import/developer_registry.json  # Trailing commas
grep -n "[^\"]\{" src/import/developer_registry.json  # Unquoted keys
```

## Data Extraction Issues

### Fields Not Extracted

**Symptom**: FIT file imports but developer fields are empty/missing.

**Causes**:
1. FIT file doesn't contain developer fields
2. UUID doesn't match
3. Field numbers don't match
4. Parser not accessing fields correctly

**Debugging**:

```bash
# 1. Check FIT file for developer fields
fitdump workout.fit | grep -A 5 developer_data_id
fitdump workout.fit | grep -A 5 field_description

# 2. Check for developer field data
fitdump workout.fit | grep "developer field"

# 3. Extract UUID from file
fitdump workout.fit | grep application_id | \
  sed 's/.*: //' | \
  xxd -r -p | \
  od -An -tx1 | \
  tr -d ' \n'
```

**Test Code**:

```rust
#[test]
fn debug_field_extraction() {
    let importer = FitImporter::new();
    let workouts = importer.import_file("workout.fit").unwrap();

    // Check if workout has data
    let workout = &workouts[0];
    println!("Duration: {}", workout.duration_seconds);
    println!("Data points: {}", workout.data_points.len());

    // Check registry
    let registry = importer.registry();
    println!("Applications: {}", registry.application_count());
    println!("Total fields: {}", registry.field_count());

    // List registered UUIDs
    for uuid in registry.registered_uuids() {
        if let Some(app) = registry.get_application(&uuid) {
            println!("{}: {} fields", app.name, app.fields.len());
        }
    }
}
```

### Incorrect Values

**Symptom**: Fields extracted but values are wrong.

**Causes**:
1. Wrong data type
2. Wrong scale/offset
3. Endianness issues
4. Missing conversions

**Solutions**:

Check scaling:
```rust
// If you expect 23.5 but get 235, check scale
let raw: u16 = 235;
let scale = 10.0;  // Should be 10.0, not 1.0
let actual = raw as f64 / scale;  // 23.5
```

Check data type:
```rust
// If values are negative when they shouldn't be
// Check if using sint instead of uint
"data_type": "uint16"  // Not "sint16"
```

Verify with `fitdump`:
```bash
# Compare your values with fitdump output
fitdump workout.fit | grep -A 2 "developer field 0"
```

## Build Issues

### Registry Not Updating

**Symptom**: Changes to JSON don't appear in build.

**Solutions**:

```bash
# Force rebuild
cargo clean
cargo build

# Verify JSON is included
strings target/debug/trainrs | grep "Stryd"

# Check file timestamp
ls -la src/import/developer_registry.json
```

### Compilation Errors

**Symptom**: Build fails after adding registry entries.

**Common Causes**:

1. **Invalid JSON embedded**:
```rust
// Error: failed to parse embedded JSON
let registry = DeveloperFieldRegistry::from_embedded();
```

Fix: Validate JSON syntax (see JSON Issues section)

2. **Missing dependencies**:
```
error: cannot find value `uuid` in this scope
```

Fix: Ensure `uuid` crate is in `Cargo.toml`:
```toml
[dependencies]
uuid = "1.0"
```

## Test Failures

### Test File Not Found

**Symptom**: `No such file or directory` error in tests.

**Solutions**:

```bash
# 1. Check file exists
ls -la tests/fixtures/workout.fit

# 2. Use correct path in test
#[test]
fn test_import() {
    // Use path relative to project root
    let path = "tests/fixtures/workout.fit";
    assert!(std::path::Path::new(path).exists());
}

# 3. Create fixtures directory
mkdir -p tests/fixtures
```

### Test Data Invalid

**Symptom**: Tests pass but data doesn't make sense.

**Solutions**:

```rust
#[test]
fn validate_test_data() {
    let importer = FitImporter::new();
    let workouts = importer.import_file("tests/fixtures/workout.fit").unwrap();

    let workout = &workouts[0];

    // Sanity checks
    assert!(workout.duration_seconds > 0, "Duration should be positive");
    assert!(workout.duration_seconds < 86400, "Duration < 24 hours");
    assert!(!workout.data_points.is_empty(), "Should have data points");

    // Check for unrealistic values
    for point in &workout.data_points {
        if let Some(power) = point.power {
            assert!(power < 2000, "Power should be < 2000W");
        }
        if let Some(hr) = point.heart_rate {
            assert!(hr < 220, "HR should be < 220 bpm");
        }
    }
}
```

## Performance Issues

### Slow Import

**Symptom**: FIT file import takes too long.

**Causes**:
1. Large file
2. Many developer fields
3. Inefficient parsing

**Solutions**:

```rust
// Time your imports
use std::time::Instant;

#[test]
fn benchmark_import() {
    let importer = FitImporter::new();

    let start = Instant::now();
    let workouts = importer.import_file("workout.fit").unwrap();
    let duration = start.elapsed();

    println!("Import took: {:?}", duration);
    println!("Workouts: {}", workouts.len());
    println!("Data points: {}", workouts[0].data_points.len());

    // Should be fast for typical files
    assert!(duration.as_secs() < 5, "Import should take < 5 seconds");
}
```

### High Memory Usage

**Symptom**: Large memory consumption during import.

**Solutions**:

```rust
// Use streaming import for large files
// (Not yet implemented in TrainRS, but good practice)

// Or limit data points
fn import_with_limits(path: &str, max_points: usize) -> Result<Workout> {
    let mut workout = import_file(path)?;

    // Downsample if too many points
    if workout.data_points.len() > max_points {
        workout.data_points = downsample(workout.data_points, max_points);
    }

    Ok(workout)
}

fn downsample(points: Vec<DataPoint>, target: usize) -> Vec<DataPoint> {
    let ratio = points.len() / target;
    points.into_iter()
        .step_by(ratio.max(1))
        .take(target)
        .collect()
}
```

## Common Error Messages

### "Failed to load embedded registry"

**Cause**: JSON syntax error in `developer_registry.json`

**Fix**:
```bash
jq . src/import/developer_registry.json
# Fix any syntax errors reported
cargo clean && cargo build
```

### "Field number out of range"

**Cause**: Field number > 255

**Fix**:
```json
{
  "field_number": 256  // ERROR: Max is 255
}
```

Change to:
```json
{
  "field_number": 0  // Valid: 0-255
}
```

### "Invalid UUID format"

**Cause**: UUID missing hyphens or wrong format

**Fix**:
```json
// Wrong
"uuid": "a42b5e01d5e94eb69f4291234567890a"

// Correct
"uuid": "a42b5e01-d5e9-4eb6-9f42-91234567890a"
```

## Getting Help

If you're still having issues:

1. **Check Examples**: Review [Code Examples](examples/)
2. **Read Guides**: See [Integration Guide](integration-guide.md)
3. **Inspect Test Data**: Use `fitdump` to examine FIT files
4. **Enable Logging**: Add debug output to understand what's happening
5. **Ask for Help**: Open an issue on GitHub with:
   - Error message
   - FIT file (if possible)
   - JSON configuration
   - Steps to reproduce

## Debug Checklist

When things don't work, check:

- [ ] JSON syntax is valid (`jq` validates)
- [ ] UUID format includes hyphens
- [ ] UUID matches FIT file
- [ ] Field numbers match FIT file
- [ ] Data types are correct
- [ ] Scale/offset values are appropriate
- [ ] Project is rebuilt (`cargo clean && cargo build`)
- [ ] Test files exist and are accessible
- [ ] Registry loads successfully
- [ ] UUID is registered
- [ ] Fields are defined

## Next Steps

- [Code Examples](examples/) - Working examples
- [Integration Guide](integration-guide.md) - Detailed integration
- [Testing Guide](testing-guide.md) - Testing strategies
