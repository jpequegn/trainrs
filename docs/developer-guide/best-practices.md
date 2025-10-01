# Best Practices: Developer Field Integration

Guidelines and recommendations for integrating custom developer fields in TrainRS.

## General Principles

### 1. Understand Before Implementing

- **Study the Device Documentation**: Read manufacturer specs thoroughly
- **Analyze Sample FIT Files**: Use `fitdump` to inspect actual data
- **Validate Against Reference**: Compare with manufacturer's software output
- **Document Your Findings**: Keep notes on data formats and quirks

### 2. Follow Existing Patterns

- **Match Naming Conventions**: Use snake_case for field names
- **Follow Data Type Patterns**: Choose types consistent with similar fields
- **Maintain Registry Structure**: Keep JSON organized and consistent
- **Use Standard Units**: Prefer SI units where applicable

### 3. Test Thoroughly

- **Use Real Device Data**: Test with actual FIT files from the device
- **Cover Edge Cases**: Test boundary conditions and error scenarios
- **Validate Scaling**: Verify values match manufacturer's software
- **Performance Test**: Ensure good performance with large files

## UUID Management

### UUID Format

**Always use lowercase with hyphens**:

✅ Good:
```json
"uuid": "a42b5e01-d5e9-4eb6-9f42-91234567890a"
```

❌ Bad:
```json
"uuid": "A42B5E01D5E94EB69F4291234567890A"  // Wrong: uppercase, no hyphens
```

### UUID Discovery

**Preferred methods**:

1. **Device Documentation**: Check manufacturer's developer docs
2. **FIT SDK Tools**: Use `fitdump` to extract from FIT files
3. **Community Resources**: Check FIT developer forums
4. **Reverse Engineering**: Last resort, analyze FIT files carefully

**Example extraction**:
```bash
# Extract developer data ID
fitdump workout.fit | grep -A 5 developer_data_id

# Find associated field definitions
fitdump workout.fit | grep -A 10 field_description
```

### UUID Uniqueness

- **Check Registry First**: Ensure UUID not already registered
- **Verify with Manufacturer**: Confirm UUID is official
- **Document Source**: Note where UUID was obtained
- **Report Duplicates**: File issue if duplicate found

## Field Definitions

### Naming Conventions

**Use descriptive, snake_case names**:

✅ Good:
```json
{
  "name": "muscle_oxygen_saturation",
  "name": "ground_contact_time",
  "name": "vertical_oscillation"
}
```

❌ Bad:
```json
{
  "name": "smo2",          // Too abbreviated
  "name": "GCT",           // Uppercase acronym
  "name": "VertOsc",       // CamelCase
  "name": "val_1"          // Non-descriptive
}
```

### Data Type Selection

**Choose the smallest appropriate type**:

| Use Case | Type | Rationale |
|----------|------|-----------|
| Power (0-2000W) | `uint16` | Fits range, standard for power |
| Percentage (0-100%) | `uint16` with scale | Precision with decimals |
| Temperature (-40°C to 80°C) | `sint16` with scale | Needs negative values |
| Altitude (-500m to 9000m) | `sint32` | Wide range, needs negatives |
| Cadence (0-255) | `uint8` | Small range, saves space |
| Latitude/Longitude | `sint32` | FIT standard (semicircles) |

**Type decision flowchart**:
```
Does value need decimals?
├─ Yes → Use integer with scaling factor
│  ├─ Range 0-255 → uint8 with scale
│  ├─ Range 0-65535 → uint16 with scale
│  └─ Larger range → uint32 with scale
└─ No → Use smallest integer that fits
   ├─ Needs negative? → Use signed (sint)
   └─ Always positive? → Use unsigned (uint)
```

### Scaling Factors

**Choose appropriate scale for precision**:

✅ Good:
```json
{
  "name": "smo2",
  "data_type": "uint16",
  "scale": 10.0,          // 1 decimal place (56.7%)
  "units": "%"
}

{
  "name": "temperature",
  "data_type": "sint16",
  "scale": 100.0,         // 2 decimal places (25.37°C)
  "units": "°C"
}

{
  "name": "power",
  "data_type": "uint16",
  "scale": 1.0,           // No scaling needed
  "units": "watts"
}
```

❌ Bad:
```json
{
  "name": "percentage",
  "scale": 3.7,           // Arbitrary scale
  "scale": 1000000.0      // Excessive precision
}
```

**Scaling guidelines**:
- **1 decimal**: Use scale 10.0
- **2 decimals**: Use scale 100.0
- **3 decimals**: Use scale 1000.0
- **No decimals**: Use scale 1.0
- **Match device**: Use manufacturer's scale

### Units

**Use standard, clear units**:

✅ Good:
```json
{
  "units": "watts",       // Power
  "units": "%",           // Percentage
  "units": "°C",          // Temperature
  "units": "bpm",         // Heart rate
  "units": "rpm",         // Cadence
  "units": "ms",          // Time duration
  "units": "mm",          // Distance (small)
  "units": "m/s"          // Speed
}
```

❌ Bad:
```json
{
  "units": "w",           // Too abbreviated
  "units": "percent",     // Use %
  "units": "degrees",     // Use °C or °F
  "units": "beats/min"    // Use bpm
}
```

## Registry Organization

### JSON Structure

**Keep registry well-organized**:

```json
{
  "applications": {
    "uuid-1": {
      "uuid": "uuid-1",
      "name": "Application Name",
      "manufacturer": "Manufacturer",
      "version": "1.0",
      "fields": [
        {
          "field_number": 0,
          "name": "field_name",
          "data_type": "uint16",
          "units": "watts",
          "scale": 1.0,
          "description": "Detailed description"
        }
      ]
    }
  }
}
```

### Alphabetical Ordering

**Order applications alphabetically by manufacturer**:

```json
{
  "applications": {
    // BSX devices
    "bsx-uuid": { "manufacturer": "BSX Insight" },

    // COROS devices
    "coros-uuid": { "manufacturer": "COROS" },

    // Garmin devices
    "garmin-uuid-1": { "manufacturer": "Garmin" },
    "garmin-uuid-2": { "manufacturer": "Garmin" },

    // Moxy devices
    "moxy-uuid": { "manufacturer": "Moxy Monitor" },

    // Stryd devices
    "stryd-uuid": { "manufacturer": "Stryd" }
  }
}
```

### Field Ordering

**Order fields by number, document logically**:

```json
{
  "fields": [
    {
      "field_number": 0,
      "name": "primary_metric",
      "description": "Main measurement from device"
    },
    {
      "field_number": 1,
      "name": "secondary_metric",
      "description": "Supporting measurement"
    },
    {
      "field_number": 2,
      "name": "derived_metric",
      "description": "Calculated from primary/secondary"
    }
  ]
}
```

## Documentation

### Field Descriptions

**Write clear, informative descriptions**:

✅ Good:
```json
{
  "field_number": 0,
  "name": "running_power",
  "description": "Instantaneous running power output measured at the foot"
}

{
  "field_number": 1,
  "name": "form_power",
  "description": "Power required to overcome vertical and horizontal oscillation"
}

{
  "field_number": 2,
  "name": "leg_spring_stiffness",
  "description": "Measure of leg stiffness during ground contact (kN/m)"
}
```

❌ Bad:
```json
{
  "description": "Power"                    // Too vague
  "description": "The power value"          // Not informative
  "description": ""                         // Missing
}
```

### Application Metadata

**Provide complete application information**:

✅ Good:
```json
{
  "uuid": "a42b5e01-d5e9-4eb6-9f42-91234567890a",
  "name": "Stryd Running Power",
  "manufacturer": "Stryd",
  "version": "1.0",
  "description": "Running power and dynamics from Stryd foot pod"
}
```

### Code Comments

**Document integration details**:

```rust
/// Parse Stryd running power from developer field
///
/// Stryd stores power as uint16 with no scaling.
/// Valid range: 0-999 watts
/// Reference: Stryd Developer Guide v1.0
fn parse_stryd_power(value: &FitValue) -> Option<u16> {
    match value {
        FitValue::UInt16(power) => {
            // Sanity check: running power rarely exceeds 500W
            if *power < 1000 {
                Some(*power)
            } else {
                log::warn!("Unusually high running power: {}W", power);
                Some(*power) // Return anyway, might be valid
            }
        }
        _ => None,
    }
}
```

## Error Handling

### Graceful Degradation

**Handle missing data gracefully**:

✅ Good:
```rust
// Handle missing developer fields
let power = if let Some(dev_fields) = &data_point.developer_fields {
    dev_fields.get_field(stryd_uuid, 0)
        .unwrap_or_else(|| {
            log::debug!("Stryd power not found, using estimated power");
            estimate_running_power(&data_point)
        })
} else {
    estimate_running_power(&data_point)
};
```

❌ Bad:
```rust
// Panics if field missing
let power = data_point
    .developer_fields
    .unwrap()
    .get_field(stryd_uuid, 0)
    .unwrap();
```

### Validation

**Validate field values**:

```rust
fn validate_smo2(value: f64) -> Result<f64, ValidationError> {
    match value {
        v if v < 0.0 => Err(ValidationError::BelowRange {
            field: "smo2",
            value: v,
            min: 0.0,
        }),
        v if v > 100.0 => Err(ValidationError::AboveRange {
            field: "smo2",
            value: v,
            max: 100.0,
        }),
        v => Ok(v),
    }
}

// Use validation
match registry.get_field(moxy_uuid, 0) {
    Some(raw_value) => {
        let scaled = field.apply_scaling(raw_value);
        match validate_smo2(scaled) {
            Ok(valid_value) => data_point.smo2 = Some(valid_value),
            Err(e) => log::warn!("Invalid SmO2 value: {}", e),
        }
    }
    None => { /* Field not present */ }
}
```

### Logging

**Log important events**:

```rust
use log::{debug, info, warn, error};

// Information logging
info!("Registered developer application: {} ({})",
    app.name, app.manufacturer);

// Debug details
debug!("Parsing field {}: {} ({})",
    field.field_number, field.name, field.data_type);

// Warnings
warn!("Unknown UUID in FIT file: {}", uuid);

// Errors
error!("Failed to parse developer field: {} - {}",
    field.name, e);
```

## Testing Best Practices

### Test Coverage

**Aim for comprehensive coverage**:

1. **Registry Loading** (100% coverage)
   - Registry loads successfully
   - All UUIDs registered
   - Field definitions correct

2. **Field Parsing** (95%+ coverage)
   - Correct data type conversion
   - Proper scaling applied
   - Edge cases handled

3. **Error Handling** (90%+ coverage)
   - Missing fields handled
   - Invalid data rejected
   - Corrupt files handled

4. **Integration** (85%+ coverage)
   - End-to-end import works
   - Multiple devices supported
   - Real-world files import correctly

### Test Data

**Use representative test files**:

✅ Good:
```rust
#[test]
fn test_stryd_workout() {
    // Real Stryd FIT file with known characteristics:
    // - 30 minute run
    // - Average power: 245W
    // - Form power present
    let workouts = import("tests/fixtures/stryd/30min_run.fit")?;
    // ...
}
```

❌ Bad:
```rust
#[test]
fn test_import() {
    // Unclear what file contains or tests
    let workouts = import("test.fit")?;
    assert!(!workouts.is_empty());
}
```

### Test Documentation

**Document test purpose**:

```rust
/// Verifies Stryd running power is correctly extracted and scaled
///
/// Test file: 30-minute easy run with Stryd pod
/// Expected: Power values between 180-280W
/// Validates: UUID recognition, field parsing, scaling
#[test]
fn test_stryd_power_extraction() {
    // Test implementation
}
```

## Performance Optimization

### Registry Caching

**Cache registry lookups**:

```rust
// ✅ Good: Cache registry reference
let registry = importer.registry();
for workout_file in files {
    let workouts = importer.import_with_registry(
        workout_file,
        &registry  // Reuse cached registry
    )?;
}

// ❌ Bad: Reload registry each time
for workout_file in files {
    let importer = FitImporter::new();  // Reloads registry
    let workouts = importer.import(workout_file)?;
}
```

### Batch Processing

**Process multiple files efficiently**:

```rust
// ✅ Good: Batch import with shared resources
let importer = FitImporter::new();
let results: Vec<_> = files
    .par_iter()  // Parallel processing
    .map(|file| importer.import_file(file))
    .collect();

// ❌ Bad: Sequential with overhead
for file in files {
    let importer = FitImporter::new();
    let result = importer.import_file(file);
}
```

### Memory Management

**Handle large files efficiently**:

```rust
// ✅ Good: Stream large files
let reader = BufReader::new(File::open(path)?);
for record in FitReader::new(reader) {
    process_record(record?)?;
}

// ❌ Bad: Load entire file to memory
let data = std::fs::read(path)?;
let records = parse_all_records(&data)?;
```

## Maintenance

### Version Compatibility

**Document version requirements**:

```json
{
  "uuid": "device-uuid",
  "name": "Device Name",
  "manufacturer": "Manufacturer",
  "version": "2.5",
  "min_firmware": "1.0",
  "notes": "Fields 3-5 added in firmware 2.0"
}
```

### Deprecation

**Handle deprecated fields**:

```json
{
  "field_number": 10,
  "name": "legacy_metric",
  "data_type": "uint16",
  "deprecated": true,
  "deprecated_since": "2.0",
  "replacement": "new_metric",
  "description": "Deprecated: Use new_metric (field 15) instead"
}
```

### Change Management

**Track changes systematically**:

1. **Update Registry**: Add/modify field definitions
2. **Update Tests**: Add tests for new fields
3. **Update Docs**: Document changes
4. **Version Bump**: Increment version number
5. **Changelog**: Record in CHANGELOG.md

## Security Considerations

### Input Validation

**Never trust FIT file input**:

```rust
// ✅ Good: Validate before use
fn parse_uuid(raw: &[u8]) -> Option<String> {
    if raw.len() != 16 {
        log::warn!("Invalid UUID length: {}", raw.len());
        return None;
    }

    // Additional validation
    let uuid = format_uuid(raw);
    if !is_valid_uuid_format(&uuid) {
        log::warn!("Invalid UUID format: {}", uuid);
        return None;
    }

    Some(uuid)
}
```

### Resource Limits

**Prevent resource exhaustion**:

```rust
const MAX_DEVELOPER_FIELDS: usize = 100;
const MAX_FIELD_SIZE: usize = 1024 * 1024; // 1MB

if dev_fields.len() > MAX_DEVELOPER_FIELDS {
    return Err(ImportError::TooManyDeveloperFields);
}

if field_data.len() > MAX_FIELD_SIZE {
    return Err(ImportError::FieldTooLarge);
}
```

## Collaboration

### Pull Request Guidelines

**When submitting new device support**:

1. **Title**: "Add support for [Device Name] developer fields"
2. **Description**: Include device details, UUID source, tested firmware versions
3. **Files**: Registry JSON, tests, documentation updates
4. **Testing**: Provide test FIT file or describe testing
5. **References**: Link to device documentation

### Code Review Checklist

**For reviewers**:

- [ ] UUID format correct (lowercase with hyphens)
- [ ] Data types appropriate for values
- [ ] Scaling factors validated
- [ ] Units specified and standard
- [ ] Descriptions clear and informative
- [ ] Tests included with real device data
- [ ] Documentation updated
- [ ] No breaking changes
- [ ] Performance impact acceptable

## Common Pitfalls

### UUID Issues

❌ **Uppercase UUID**
```json
"uuid": "A42B5E01-D5E9-4EB6-9F42-91234567890A"
```
✅ **Lowercase UUID**
```json
"uuid": "a42b5e01-d5e9-4eb6-9f42-91234567890a"
```

### Scaling Errors

❌ **Wrong scale factor**
```json
{
  "name": "temperature",
  "scale": 1.0,  // Results in whole degrees only
  "units": "°C"
}
```
✅ **Appropriate scale**
```json
{
  "name": "temperature",
  "scale": 100.0,  // Preserves 2 decimal places
  "units": "°C"
}
```

### Type Mismatches

❌ **Type too small**
```json
{
  "name": "altitude",
  "data_type": "uint8",  // Max 255m, too small
  "units": "m"
}
```
✅ **Appropriate type**
```json
{
  "name": "altitude",
  "data_type": "sint32",  // Handles full range including negative
  "units": "m"
}
```

### Missing Validation

❌ **No bounds checking**
```rust
let smo2 = field.apply_scaling(raw_value);
data_point.smo2 = Some(smo2);  // Could be >100% or negative
```
✅ **Validated input**
```rust
let smo2 = field.apply_scaling(raw_value);
if smo2 >= 0.0 && smo2 <= 100.0 {
    data_point.smo2 = Some(smo2);
} else {
    log::warn!("SmO2 out of range: {}", smo2);
}
```

## Summary Checklist

Before submitting developer field integration:

- [ ] UUID obtained from official source
- [ ] UUID formatted correctly (lowercase, hyphens)
- [ ] Data types appropriate for value ranges
- [ ] Scaling factors validated against device output
- [ ] Units specified using standard abbreviations
- [ ] Field descriptions clear and informative
- [ ] Tests written using real device data
- [ ] Edge cases and errors handled
- [ ] Documentation updated
- [ ] Performance impact minimal
- [ ] Code reviewed and validated

## See Also

- [Getting Started](getting-started.md) - First integration tutorial
- [Integration Guide](integration-guide.md) - Detailed integration steps
- [Registry System](registry-system.md) - Registry architecture
- [Testing Guide](testing-guide.md) - Comprehensive testing
- [Troubleshooting](troubleshooting.md) - Common issues and solutions
