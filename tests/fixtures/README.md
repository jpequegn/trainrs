# Real-World FIT File Test Fixtures

This directory contains real-world FIT files from various devices, sports, and scenarios for comprehensive integration testing.

## ⚠️ Important: Anonymization Required

**Before adding any FIT file to this repository:**

1. **Remove PII (Personally Identifiable Information)**
   - Strip GPS coordinates
   - Remove athlete names
   - Anonymize user IDs
   - Remove sensitive metadata

2. **Use FIT SDK tools or scripts to sanitize files**
   ```bash
   # Example using FIT SDK (not included)
   fit-sanitize input.fit output.fit --remove-gps --anonymize
   ```

3. **Document source and purpose**
   - Add entry to this README
   - Note device model, sport, duration
   - Describe what the file tests

## Directory Structure

```
fixtures/
├── garmin/
│   ├── edge_520/          # Garmin Edge 520 cycling computer
│   ├── edge_1030/         # Garmin Edge 1030 cycling computer
│   ├── forerunner_945/    # Garmin Forerunner 945 multisport watch
│   └── fenix_6/           # Garmin Fenix 6 multisport watch
├── wahoo/
│   ├── elemnt_bolt/       # Wahoo ELEMNT BOLT cycling computer
│   └── elemnt_roam/       # Wahoo ELEMNT ROAM cycling computer
├── zwift/                 # Zwift virtual cycling platform
├── corrupted/             # Intentionally corrupted files for error handling tests
├── developer_fields/      # Files with custom developer fields
└── edge_cases/            # Edge cases (empty, minimal, unusual data)
```

## Test File Inventory

### Garmin Edge 520
*Currently empty - add anonymized files here*

**What to test:**
- Cadence doubling quirk detection
- Power data accuracy
- Basic cycling metrics

### Garmin Edge 1030
*Currently empty - add anonymized files here*

**What to test:**
- Advanced cycling metrics
- Navigation data handling
- High-resolution data streams

### Garmin Forerunner 945
*Currently empty - add anonymized files here*

**What to test:**
- Running dynamics
- Multisport activities
- Triathlon mode
- Running-specific metrics

### Garmin Fenix 6
*Currently empty - add anonymized files here*

**What to test:**
- Multi-sport support
- Hiking/trail running
- Swimming metrics
- Battery-optimized recordings

### Wahoo ELEMNT BOLT
*Currently empty - add anonymized files here*

**What to test:**
- Power spike quirk (first 5 seconds)
- Wahoo-specific data fields
- Integration with power meters

### Wahoo ELEMNT ROAM
*Currently empty - add anonymized files here*

**What to test:**
- Similar to BOLT
- Larger screen/different firmware

### Zwift
*Currently empty - add anonymized files here*

**What to test:**
- Virtual power data
- Structured workout detection
- ERG mode recordings
- Smart trainer data

### Corrupted Files
*Currently empty - add intentionally broken files here*

**What to test:**
- Graceful error handling
- Partial file recovery
- Descriptive error messages
- No panics/crashes

### Developer Fields
*Currently empty - add files with custom fields here*

**What to test:**
- Stryd running power
- Garmin Connect IQ apps
- Custom sensor data
- Third-party field extraction

### Edge Cases
*Currently empty - add unusual files here*

**What to test:**
- Very short activities (<1 min)
- Very long activities (>12 hours)
- Paused/resumed activities
- Files with missing data fields
- Single data point files
- Files with only metadata

## Adding New Test Files

### 1. Obtain File
Get a FIT file from your device or a trusted source.

### 2. Anonymize
```bash
# Recommended: Use FIT SDK or similar tool
# Remove: GPS coordinates, athlete name, user ID, timestamps (optional)
```

### 3. Categorize
Place in appropriate directory based on:
- Device manufacturer and model
- Sport type
- Special characteristics (corrupted, developer fields, etc.)

### 4. Document
Update this README with:
- File name
- Source device/app
- Sport/activity type
- Duration
- Special features (power, HR, GPS, developer fields)
- What it tests

### 5. Verify
Run the test suite to ensure it parses correctly:
```bash
cargo test real_world_fit_files
```

## Test Coverage Goals

- [ ] **100+ total FIT files** across all categories
- [ ] **10+ device manufacturers**
  - [ ] Garmin (10+ models)
  - [ ] Wahoo (3+ models)
  - [ ] Stages
  - [ ] 4iiii
  - [ ] SRM
  - [ ] Others
- [ ] **5+ sports**
  - [ ] Cycling (road, mountain, indoor)
  - [ ] Running (road, trail, track)
  - [ ] Swimming (pool, open water)
  - [ ] Triathlon
  - [ ] Other (rowing, hiking, etc.)
- [ ] **Duration coverage**
  - [ ] Short (<30 min): 10+ files
  - [ ] Medium (1-3 hours): 30+ files
  - [ ] Long (3-8 hours): 10+ files
  - [ ] Ultra (>12 hours): 5+ files
- [ ] **Data richness**
  - [ ] Power-only: 10+ files
  - [ ] HR-only: 10+ files
  - [ ] GPS-only: 10+ files
  - [ ] Full telemetry: 50+ files
  - [ ] Developer fields: 10+ files

## Running Tests

### All Tests
```bash
cargo test real_world_fit_files
```

### Specific Device
```bash
cargo test test_garmin_edge_520_files
cargo test test_wahoo_elemnt_bolt_files
```

### Specific Category
```bash
cargo test test_corrupted_files
cargo test test_developer_field_files
```

### Coverage Report
```bash
cargo test test_file_coverage -- --nocapture
```

This will show:
- Number of files per category
- Total file count
- Success/failure statistics

## Continuous Integration

These tests run automatically on:
- Every commit to main
- Every pull request
- Daily scheduled runs

### CI Expectations
- **95%+ success rate** for non-corrupted files
- **No panics or crashes** on any file
- **Graceful error handling** for corrupted files
- **Reasonable performance** (< 1s per file average)

## Test File Examples

### Good Test File Naming
```
garmin/edge_520/cycling_1hour_power_hr_20240115.fit
wahoo/elemnt_bolt/indoor_zwift_60min_20240120.fit
corrupted/truncated_garmin_edge.fit
developer_fields/stryd_running_power.fit
```

### File Metadata Document
For each file, consider documenting:
- **Source**: Where it came from (anonymized)
- **Device**: Manufacturer and model
- **Sport**: Activity type
- **Duration**: Length in minutes
- **Data fields**: Power, HR, GPS, cadence, etc.
- **Special features**: Developer fields, quirks tested, edge cases
- **Anonymization**: What was removed/modified

## Privacy & Legal

- **Never commit files with PII**
- **Respect copyright**: Only add files you have permission to use
- **Test data only**: These files are for testing purposes only
- **No distribution**: Do not redistribute original files outside this repo

## Contributing

When adding test files:

1. Create a branch: `git checkout -b test/add-garmin-edge-files`
2. Add anonymized files to appropriate directory
3. Update this README
4. Run tests: `cargo test real_world_fit_files`
5. Commit with message: `test: add <device> FIT files for <purpose>`
6. Create PR with description of what files test

## Future Enhancements

- [ ] Automated anonymization script
- [ ] FIT file validator/sanitizer tool
- [ ] Performance regression tests for large files
- [ ] Comparative testing (before/after quirk fixes)
- [ ] Synthetic FIT file generator for missing scenarios
- [ ] Test coverage dashboard

## References

- [FIT SDK Documentation](https://developer.garmin.com/fit/overview/)
- [FIT File Specification](https://developer.garmin.com/fit/protocol/)
- [TrainRS Device Quirks](../../docs/DEVICE_QUIRKS.md)

## License

Test files in this directory are used solely for testing purposes. Original FIT files remain property of their creators and are used with permission for quality assurance only.
