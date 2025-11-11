# Device Quirks System

TrainRS includes a comprehensive device quirks system to handle known issues and data format variations across different manufacturer devices and models.

## Overview

The device quirks system automatically detects your device from FIT files and applies necessary corrections to ensure accurate training data analysis. This is especially important for devices with known data reporting issues like doubled cadence values, power spikes, or left-only power meter doubling.

## Supported Devices

### Garmin

**Edge 520**
- **Issue**: Cadence values reported doubled
- **Fix**: Automatically scales cadence by 0.5
- **Manufacturer ID**: 1
- **Product ID**: 2697

**Forerunner 945** (Older firmware)
- **Issue**: Running dynamics field scaling inconsistencies
- **Fix**: Scales ground contact time and vertical oscillation by 0.1
- **Manufacturer ID**: 1
- **Product ID**: 2697
- **Firmware Range**: 0-1000

### Wahoo

**ELEMNT BOLT**
- **Issue**: Power spikes in first 5 seconds of workout
- **Fix**: Removes power values >1500W in first 5 seconds
- **Manufacturer ID**: 32
- **Product ID**: 16

### Stages Cycling

**All Stages Power Meters**
- **Issue**: Left-only power should not be doubled
- **Fix**: Prevents incorrect doubling of left-only power readings
- **Manufacturer ID**: 69
- **Product ID**: All (0 = wildcard)

## CLI Commands

### View Device Information

Extract device information and applicable quirks from a FIT file:

```bash
trainrs device info --file workout.fit
```

**Example Output:**
```
📱 Analyzing FIT file: workout.fit

DEVICE INFORMATION
─────────────────────────────────
Manufacturer ID:   1 (Garmin)
Product ID:        2697 (Edge 520)
Firmware Version:  500

APPLICABLE QUIRKS
─────────────────────────────────
• Edge 520 reports cadence doubled
  Type: CadenceScaling { factor: 0.5 }
  Enabled by default: Yes
```

### List Known Quirks

View all known device quirks in the registry:

```bash
# List all quirks
trainrs device list

# Filter by manufacturer
trainrs device list --manufacturer-id 1

# Show only enabled quirks
trainrs device list --enabled-only
```

**Example Output:**
```
KNOWN DEVICE QUIRKS
═══════════════════════════════════════

Found 4 quirk(s)

═══ QUIRK #1 ═══
Device:       Garmin Edge 520 (1:2697)
Description:  Edge 520 reports cadence doubled
Type:         CadenceScaling { factor: 0.5 }
Enabled:      Yes

═══ QUIRK #2 ═══
Device:       Wahoo ELEMNT BOLT (32:16)
Description:  BOLT has power spikes in first 5 seconds
Type:         PowerSpikeStart { threshold: 1500, window_seconds: 5 }
Enabled:      Yes

KNOWN MANUFACTURERS
─────────────────────────────────
  1: Garmin (2 quirks)
 32: Wahoo (1 quirk)
 69: Stages Cycling (1 quirk)
263: 4iiii (0 quirks)
```

### Export Quirk Registry

Export the quirk registry to a TOML file for customization:

```bash
trainrs device export --output custom-quirks.toml
```

This allows you to:
- Add custom quirks for your specific devices
- Modify existing quirk parameters
- Disable specific quirks if needed
- Share quirk configurations across multiple installations

## Automatic Application

Device quirks are **automatically applied** during FIT file import. You'll see confirmation in the import notes:

```bash
trainrs import --file workout.fit
```

The import will include device quirk information in the workout notes:
```
Imported from FIT file: workout.fit
Device Quirks: Edge 520 reports cadence doubled: Applied cadence scaling (factor: 0.5) to 3600 data points; No other quirks applied
```

## Disabling Quirks

To disable automatic quirk application during development or testing, you can modify the FIT importer:

```rust
let importer = FitImporter::new().with_quirks_disabled();
```

## Supported Quirk Types

### Cadence Scaling
Multiplies cadence values by a scaling factor to correct doubled or scaled readings.

```rust
QuirkType::CadenceScaling { factor: 0.5 }
```

### Power Spike Removal
Removes unrealistic power spikes at the start of workouts.

```rust
QuirkType::PowerSpikeStart {
    threshold: 1500,      // Watts
    window_seconds: 5     // First N seconds
}
```

### Left-Only Power Doubling
Prevents incorrect doubling of left-only power meter readings.

```rust
QuirkType::LeftOnlyPowerDoubling
```

### Running Dynamics Scaling
Scales running dynamics values (ground contact time, vertical oscillation) to correct units.

```rust
QuirkType::RunningDynamicsScaling {
    gct_scale: Some(0.1),
    vo_scale: Some(0.1)
}
```

## Adding Custom Quirks

### Option 1: Export and Edit TOML

1. Export the current registry:
```bash
trainrs device export --output my-quirks.toml
```

2. Edit the TOML file to add your quirk:
```toml
[[quirks]]
manufacturer_id = 1
product_id = 3122
firmware_version_range = [0, 500]
description = "Edge 130 has GPS drift in tunnels"
enabled_by_default = true

[quirks.quirk_type]
type = "gps_drift_tunnels"
```

3. Load the custom registry (programmatically):
```rust
let registry = QuirkRegistry::load_from_file("my-quirks.toml")?;
let importer = FitImporter::new().with_quirk_registry(registry);
```

### Option 2: Programmatic Addition

```rust
use trainrs::device_quirks::{DeviceQuirk, QuirkType, QuirkRegistry};

let mut registry = QuirkRegistry::with_defaults();

registry.add_quirk(DeviceQuirk {
    manufacturer_id: 1,
    product_id: 3122,
    firmware_version_range: Some((0, 500)),
    description: "Custom quirk for Edge 130".to_string(),
    quirk_type: QuirkType::CadenceScaling { factor: 0.5 },
    enabled_by_default: true,
});
```

## Device ID Reference

### Common Manufacturer IDs
- **1**: Garmin
- **32**: Wahoo
- **69**: Stages Cycling
- **263**: 4iiii

### Common Garmin Product IDs
- **2697**: Edge 520, Forerunner 945
- **2713**: Edge 1030
- **3122**: Edge 130
- **3589**: Forerunner 255

### Common Wahoo Product IDs
- **16**: ELEMNT BOLT
- **27**: ELEMNT ROAM

## Technical Details

### Device Detection

The system extracts device information from FIT file `FileId` and `DeviceInfo` messages:

1. **FileId Message**: Contains manufacturer and product IDs
2. **DeviceInfo Message**: Contains firmware version
3. **Registry Lookup**: Matches device against known quirks
4. **Automatic Application**: Applies all matching enabled quirks

### Quirk Application Order

1. Quirks are filtered by device match (manufacturer, product, firmware)
2. Only quirks with `enabled_by_default = true` are applied
3. Multiple quirks can apply to the same device
4. Quirks are applied in registry order
5. Results are logged in workout notes

### Data Point Modification

Quirks operate directly on workout data points:

```rust
pub struct DataPoint {
    timestamp: u32,
    power: Option<u16>,
    cadence: Option<u16>,
    heart_rate: Option<u16>,
    // ... other fields
}
```

Modifications preserve data integrity and maintain summary statistics consistency.

## Testing

The quirk system includes comprehensive unit tests:

```bash
cargo test device_quirks
```

Test coverage includes:
- Device info creation and matching
- Quirk application logic for each type
- Registry loading/saving
- Firmware version range matching
- Integration with FIT import pipeline

## Contributing New Quirks

To contribute a new device quirk:

1. Identify the device manufacturer and product ID
2. Document the issue and expected behavior
3. Implement the quirk fix
4. Add comprehensive tests
5. Update this documentation
6. Submit a pull request

### Quirk Contribution Template

```rust
DeviceQuirk {
    manufacturer_id: X,
    product_id: Y,
    firmware_version_range: Some((min, max)), // or None
    description: "Clear description of the issue".to_string(),
    quirk_type: QuirkType::YourQuirkType { /* config */ },
    enabled_by_default: true, // or false if experimental
}
```

## Troubleshooting

### Quirk Not Being Applied

1. Check device info:
```bash
trainrs device info --file workout.fit
```

2. Verify device is in registry:
```bash
trainrs device list --manufacturer-id X --product-id Y
```

3. Check quirk is enabled by default
4. Ensure firmware version matches (if specified)

### Incorrect Data After Quirk

1. Export current registry to review quirk configuration
2. Test with quirks disabled to verify issue is quirk-related
3. File an issue with sample FIT file and expected behavior

### Adding Support for New Device

1. Use `device info` command to identify manufacturer/product IDs
2. Export registry and add new manufacturer/product mapping
3. Test quirk application with your device's FIT files
4. Consider contributing back to the project

## Future Enhancements

Planned improvements to the device quirks system:

- [ ] Web-based quirk registry for community contributions
- [ ] Automatic quirk suggestion based on data anomaly detection
- [ ] Per-user quirk override configuration
- [ ] Quirk effectiveness metrics and reporting
- [ ] Support for TCX and GPX device-specific issues
- [ ] Automatic firmware version detection improvements
- [ ] Machine learning-based anomaly detection

## References

- [FIT SDK Documentation](https://developer.garmin.com/fit/protocol/)
- [Manufacturer ID List](https://developer.garmin.com/fit/file-types/manufacturers/)
- [Device-Specific Issues Discussion](https://github.com/jpequegn/trainrs/discussions)

## License

The device quirks system is part of TrainRS and is licensed under the same terms as the main project.
