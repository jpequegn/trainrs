# Developer Field Basics

Understanding FIT developer fields and how they work.

## What are Developer Fields?

Developer fields allow third-party developers to add custom data to FIT files without modifying the FIT specification. Each developer registers a unique UUID and defines fields within their namespace.

### Key Concepts

**UUID (Universally Unique Identifier)**
- 128-bit identifier for your application
- Must be registered with Garmin
- Format: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`

**Field Definition Number**
- 0-255 numeric identifier within your UUID namespace
- Each field has a unique number
- Example: Field 0 might be "power", Field 1 might be "cadence"

**Data Types**
- `uint8` - 8-bit unsigned integer (0-255)
- `uint16` - 16-bit unsigned integer (0-65535)
- `uint32` - 32-bit unsigned integer
- `sint8`, `sint16`, `sint32` - Signed integers
- `float32`, `float64` - Floating point numbers
- `string` - Variable length strings
- `byte` - Raw byte arrays

**Scaling and Offset**
- Used to store decimal values as integers
- Formula: `actual_value = (raw_value / scale) + offset`
- Example: Temperature stored as uint16 with scale=10 → `235 / 10 = 23.5°C`

## FIT File Structure

Developer fields appear in FIT files through three message types:

### 1. Developer Data ID Message

Defines the application UUID and metadata:

```
Message: developer_data_id
Fields:
  - application_id: 16-byte UUID
  - application_version: Optional version number
  - manufacturer_id: Optional manufacturer ID
  - developer_data_index: Local index for this session (0-255)
```

### 2. Field Description Message

Defines each field's properties:

```
Message: field_description
Fields:
  - developer_data_index: Links to developer_data_id
  - field_definition_number: 0-255
  - field_name: Human-readable name
  - fit_base_type_id: Data type identifier
  - units: Optional units string
  - scale: Optional scaling factor
  - offset: Optional offset value
```

### 3. Data Records with Developer Fields

Actual data values appear in record messages:

```
Message: record (or other message types)
Fields:
  - timestamp: 1234567890
  - power: 250 (standard field)
  - heart_rate: 145 (standard field)
  - [Developer field 0]: 180 (e.g., running power)
  - [Developer field 1]: 8.5 (e.g., leg spring stiffness)
```

## Example: Stryd Running Power

Let's examine how Stryd embeds running power data:

### Step 1: Developer Data ID

```
application_id: a42b5e01-d5e9-4eb6-9f42-91234567890a
application_version: 1
developer_data_index: 0
```

### Step 2: Field Descriptions

```
Field 0:
  field_definition_number: 0
  field_name: "running_power"
  fit_base_type_id: uint16
  units: "watts"
  scale: 1.0

Field 1:
  field_definition_number: 1
  field_name: "form_power"
  fit_base_type_id: uint16
  units: "watts"
  scale: 1.0
```

### Step 3: Data Records

```
Record at timestamp 0:
  running_power: 180 watts
  form_power: 45 watts

Record at timestamp 1:
  running_power: 185 watts
  form_power: 47 watts
```

## Data Type Mapping

Understanding how FIT types map to Rust:

| FIT Type | Rust Type | Size | Range |
|----------|-----------|------|-------|
| uint8 | u8 | 1 byte | 0 to 255 |
| uint16 | u16 | 2 bytes | 0 to 65,535 |
| uint32 | u32 | 4 bytes | 0 to 4,294,967,295 |
| sint8 | i8 | 1 byte | -128 to 127 |
| sint16 | i16 | 2 bytes | -32,768 to 32,767 |
| sint32 | i32 | 4 bytes | -2,147,483,648 to 2,147,483,647 |
| float32 | f32 | 4 bytes | ±3.4E+38 (7 digits) |
| float64 | f64 | 8 bytes | ±1.7E+308 (15 digits) |
| string | String | Variable | UTF-8 encoded |
| byte | Vec<u8> | Variable | Raw bytes |

### Scaling Examples

**Temperature (scale=10)**
```rust
// FIT file contains: 235 (uint16)
// Actual value: 235 / 10 = 23.5°C
let raw: u16 = 235;
let scale: f64 = 10.0;
let temperature = raw as f64 / scale;  // 23.5
```

**Heart Rate Variability (scale=1000, offset=0)**
```rust
// FIT file contains: 45 (uint16)
// Actual value: 45 / 1000 = 0.045 seconds (45ms)
let raw: u16 = 45;
let scale: f64 = 1000.0;
let hrv = raw as f64 / scale;  // 0.045
```

**Ground Contact Balance (scale=100)**
```rust
// FIT file contains: 5020 (uint16)
// Actual value: 5020 / 100 = 50.20%
let raw: u16 = 5020;
let scale: f64 = 100.0;
let balance = raw as f64 / scale;  // 50.20
```

## UUID Registration

To add developer fields to your application:

1. **Register with Garmin**
   - Visit [Garmin Developer Program](https://developer.garmin.com/fit/developer-data/)
   - Register your application
   - Receive unique UUID

2. **Document Your Fields**
   - Create field specifications
   - Define data types, units, scaling
   - Document the meaning of each field

3. **Embed in FIT Files**
   - Add developer_data_id message
   - Add field_description messages
   - Include field data in records

## Best Practices

**Choose Appropriate Data Types**
- Use smallest type that fits your range
- Consider future expansion
- uint16 is often sufficient for sensor data

**Use Proper Scaling**
- Enables decimal values as integers
- Preserves precision
- Common scales: 10, 100, 1000

**Name Fields Clearly**
- Use descriptive names: "muscle_oxygen_saturation" not "mo2"
- Follow snake_case convention
- Avoid abbreviations unless standard

**Document Units**
- Always specify units: "watts", "rpm", "%", "mm"
- Use metric units by default
- Document any special unit handling

**Version Your Format**
- Include version in developer_data_id
- Document breaking changes
- Maintain backward compatibility when possible

## Next Steps

- [Getting Started](getting-started.md) - Add your first custom field
- [Integration Guide](integration-guide.md) - Detailed integration process
- [Code Examples](examples/) - Working examples
