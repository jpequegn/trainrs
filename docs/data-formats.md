# Data Format Specifications

Comprehensive guide to all data formats supported by TrainRS for import and export operations.

## Table of Contents

- [Overview](#overview)
- [Import Formats](#import-formats)
- [Export Formats](#export-formats)
- [Data Structure Reference](#data-structure-reference)
- [Validation Requirements](#validation-requirements)
- [Examples](#examples)
- [Best Practices](#best-practices)

## Overview

TrainRS supports multiple data formats to ensure compatibility with various training platforms and devices. The system uses automatic format detection for imports and provides flexible export options for analysis and data sharing.

### Supported Import Formats
- **CSV** - Comma-separated values with flexible column mapping
- **GPX** - GPS Exchange Format for GPS track data
- **TCX** - Training Center XML format (Garmin)
- **FIT** - Flexible and Interoperable Data Transfer (ANT+/Garmin)

### Supported Export Formats
- **CSV** - For spreadsheet analysis and data exchange
- **JSON** - For programmatic use and API integration
- **Text** - Human-readable reports and terminal output
- **HTML** - Web-based reports and dashboards
- **PDF** - Printable reports and documentation

## Import Formats

### CSV Format

CSV files provide the most flexible import option with automatic column mapping and validation.

#### Supported Column Names

TrainRS automatically maps common column name variations to standard fields:

| Standard Field | Accepted Variations |
|---------------|-------------------|
| `timestamp` | timestamp, time, elapsed_time, elapsed, duration |
| `heart_rate` | heart_rate, hr, heartrate, bpm |
| `power` | power, watts, power_watts |
| `pace` | pace, min_per_km, pace_min_km |
| `speed` | speed, velocity, speed_ms, speed_kmh |
| `elevation` | elevation, altitude, alt, elev, height |
| `cadence` | cadence, rpm, steps_per_minute, spm |
| `distance` | distance, dist, total_distance, cumulative_distance |
| `latitude` | latitude, lat, position_lat |
| `longitude` | longitude, lng, lon, position_long |

#### Data Types and Formats

| Field | Data Type | Format | Example |
|-------|-----------|--------|---------|
| `timestamp` | DateTime/Seconds | Various formats supported | `2024-09-23 14:30:00`, `1695474600`, `3600` |
| `heart_rate` | Integer | Beats per minute | `150` |
| `power` | Integer | Watts | `250` |
| `pace` | Decimal | Minutes per unit | `6.5` (6:30 min/mile or min/km) |
| `speed` | Decimal | Meters per second | `5.0` |
| `elevation` | Integer | Meters above sea level | `1500` |
| `cadence` | Integer | RPM (cycling) or SPM (running) | `90` |
| `distance` | Decimal | Meters | `10000.0` |
| `latitude` | Decimal | Decimal degrees | `37.7749` |
| `longitude` | Decimal | Decimal degrees | `-122.4194` |

#### Timestamp Formats

TrainRS supports multiple timestamp formats:

1. **ISO 8601**: `2024-09-23T14:30:00Z`
2. **Standard DateTime**: `2024-09-23 14:30:00`
3. **Unix Timestamp**: `1695474600` (seconds since epoch)
4. **Elapsed Seconds**: `3600` (seconds from workout start)
5. **Various date formats**: `23/09/2024 14:30:00`, `09/23/2024 14:30:00`

#### CSV Example

```csv
timestamp,heart_rate,power,elevation,cadence,distance
2024-09-23 14:30:00,110,150,100,85,0.0
2024-09-23 14:30:01,112,155,101,86,8.5
2024-09-23 14:30:02,115,160,102,87,17.2
2024-09-23 14:30:03,118,165,103,88,26.1
```

#### Import Command
```bash
trainrs import --file workout.csv --format csv
```

### GPX Format

GPS Exchange Format for GPS-enabled devices and applications.

#### Supported Elements
- **Track points** with time, position, elevation
- **Heart rate data** (extensions)
- **Cadence data** (extensions)
- **Power data** (extensions)
- **Speed calculation** from position changes

#### GPX Structure
```xml
<?xml version="1.0"?>
<gpx version="1.1" creator="TrainRS">
  <trk>
    <name>Training Ride</name>
    <trkseg>
      <trkpt lat="37.7749" lon="-122.4194">
        <ele>100</ele>
        <time>2024-09-23T14:30:00Z</time>
        <extensions>
          <hr>150</hr>
          <cad>90</cad>
          <power>250</power>
        </extensions>
      </trkpt>
    </trkseg>
  </trk>
</gpx>
```

#### Import Command
```bash
trainrs import --file ride.gpx --format gpx --sport cycling
```

### TCX Format

Training Center XML format used by Garmin devices and Training Center software.

#### Supported Elements
- **Activities** with sport type and timing
- **Laps** for interval analysis
- **Track points** with full sensor data
- **Heart rate zones** and training data

#### TCX Structure
```xml
<?xml version="1.0"?>
<TrainingCenterDatabase>
  <Activities>
    <Activity Sport="Biking">
      <Id>2024-09-23T14:30:00Z</Id>
      <Lap StartTime="2024-09-23T14:30:00Z">
        <TotalTimeSeconds>3600</TotalTimeSeconds>
        <DistanceMeters>25000</DistanceMeters>
        <Track>
          <Trackpoint>
            <Time>2024-09-23T14:30:00Z</Time>
            <Position>
              <LatitudeDegrees>37.7749</LatitudeDegrees>
              <LongitudeDegrees>-122.4194</LongitudeDegrees>
            </Position>
            <AltitudeMeters>100</AltitudeMeters>
            <HeartRateBpm>
              <Value>150</Value>
            </HeartRateBpm>
            <Cadence>90</Cadence>
            <Extensions>
              <TPX>
                <Watts>250</Watts>
              </TPX>
            </Extensions>
          </Trackpoint>
        </Track>
      </Lap>
    </Activity>
  </Activities>
</TrainingCenterDatabase>
```

#### Import Command
```bash
trainrs import --file workout.tcx --format tcx
```

### FIT Format

Flexible and Interoperable Data Transfer format from ANT+ and Garmin.

#### Supported Message Types
- **File header** and session information
- **Record messages** with sensor data
- **Lap messages** for interval data
- **Event messages** for markers and annotations

#### Key Features
- **Binary format** for efficient storage
- **Comprehensive sensor support** including power, heart rate, GPS
- **Advanced metrics** like left/right power balance
- **Device information** and calibration data

#### Import Command
```bash
trainrs import --file activity.fit --format fit
```

### Batch Import

Import multiple files from a directory:

```bash
# Import all supported files from directory
trainrs import --directory ./workout_files/ --recursive

# Validate files without importing
trainrs import --file data.csv --validate-only
```

## Export Formats

### CSV Export

#### Workout Summaries Export

```bash
trainrs export --format csv --type workout-summaries --output workouts.csv
```

**Output Fields:**
```csv
date,sport,duration_hours,avg_heart_rate,max_heart_rate,avg_power,normalized_power,intensity_factor,tss,total_distance_km,elevation_gain_m,avg_cadence
2024-09-23,Cycling,2.0,155,180,220,235,0.94,95.2,25.0,350,90
2024-09-24,Running,1.5,165,185,,,,85.0,12.5,150,85
```

#### PMC Data Export

```bash
trainrs export --format csv --type pmc-data --output pmc.csv
```

**Output Fields:**
```csv
date,daily_tss,ctl,atl,tsb
2024-09-20,0,65.2,45.8,19.4
2024-09-21,75,65.5,47.2,18.3
2024-09-22,90,66.1,49.8,16.3
2024-09-23,95,66.8,53.1,13.7
```

### JSON Export

#### Training Report Export

```bash
trainrs export --format json --type training-report --output report.json
```

**Structure:**
```json
{
  "athlete_id": "athlete_123",
  "date_range": {
    "start": "2024-09-01",
    "end": "2024-09-30"
  },
  "generated_at": "2024-09-23T14:30:00Z",
  "summary_stats": {
    "total_workouts": 25,
    "total_tss": 2150.5,
    "total_duration_hours": 42.5,
    "avg_tss_per_workout": 86.02,
    "most_frequent_sport": "Cycling",
    "date_range_days": 30,
    "training_consistency": 83.33
  },
  "pmc_analysis": {
    "current_ctl": 68.5,
    "current_atl": 45.2,
    "current_tsb": 23.3,
    "fitness_trend": "Improving",
    "fatigue_trend": "Moderate",
    "form_trend": "Good",
    "recommendations": [
      "Current form is good for quality training",
      "Consider increasing weekly TSS by 10-15%"
    ]
  },
  "weekly_summaries": [...],
  "monthly_summaries": [...],
  "zone_analysis": [...]
}
```

#### Workout Data Export

```bash
trainrs export --format json --include-raw-data --output detailed_workouts.json
```

**Structure:**
```json
{
  "workouts": [
    {
      "id": "workout_123",
      "date": "2024-09-23",
      "sport": "Cycling",
      "duration_seconds": 7200,
      "workout_type": "Interval",
      "data_source": "Power",
      "summary": {
        "avg_heart_rate": 155,
        "max_heart_rate": 180,
        "avg_power": 220,
        "normalized_power": 235,
        "intensity_factor": 0.94,
        "tss": 95.2,
        "total_distance": 25000.0,
        "elevation_gain": 350,
        "avg_cadence": 90,
        "calories": 850
      },
      "raw_data": [
        {
          "timestamp": 0,
          "heart_rate": 110,
          "power": 150,
          "elevation": 100,
          "cadence": 85,
          "speed": 8.5,
          "distance": 0.0
        }
      ],
      "notes": "Excellent interval session",
      "source": "wahoo_elemnt"
    }
  ]
}
```

### Text Export

Human-readable training reports for terminal output or documentation.

```bash
trainrs export --format text --type training-report --output report.txt
```

**Example Output:**
```
==========================================
TrainRS Training Report
==========================================

Report Period: September 1-30, 2024
Athlete: John Doe (athlete_123)
Generated: September 23, 2024 14:30:00 UTC

SUMMARY STATISTICS
------------------------------------------
Total Workouts:        25
Total Training Time:   42.5 hours
Total TSS:            2,150.5
Average TSS/Workout:   86.0
Most Frequent Sport:   Cycling
Training Consistency:  83.3% (25/30 days)

PERFORMANCE MANAGEMENT CHART
------------------------------------------
Current Fitness (CTL):  68.5
Current Fatigue (ATL):  45.2
Current Form (TSB):     +23.3

Fitness Trend:  Improving ↗
Fatigue Trend:  Moderate →
Form Trend:     Good ↗

WEEKLY BREAKDOWN
------------------------------------------
Week 1 (Sep 1-7):   6 workouts, 495 TSS
Week 2 (Sep 8-14):  7 workouts, 580 TSS
Week 3 (Sep 15-21): 6 workouts, 535 TSS
Week 4 (Sep 22-28): 6 workouts, 540 TSS

TRAINING RECOMMENDATIONS
------------------------------------------
• Current form is excellent for quality training
• Consider increasing weekly TSS by 10-15%
• Maintain current training distribution
• Schedule recovery week in 2-3 weeks
```

### HTML Export

Web-based reports with charts and interactive elements.

```bash
trainrs export --format html --type training-report --output report.html
```

**Features:**
- Interactive charts and graphs
- Responsive design for all devices
- Detailed data tables
- Export-ready formatting
- Print-friendly layouts

### PDF Export

Printable reports for offline analysis and documentation.

```bash
trainrs export --format pdf --type training-report --output report.pdf
```

**Features:**
- Professional layout and typography
- High-quality charts and graphs
- Multi-page support
- Table of contents
- Header and footer information

## Data Structure Reference

### Core Data Types

#### DataPoint Structure
```rust
struct DataPoint {
    timestamp: u32,           // Seconds from workout start
    heart_rate: Option<u16>,  // Beats per minute
    power: Option<u16>,       // Watts
    pace: Option<Decimal>,    // Minutes per unit distance
    elevation: Option<i16>,   // Meters above sea level
    cadence: Option<u16>,     // RPM or SPM
    speed: Option<Decimal>,   // Meters per second
    distance: Option<Decimal>, // Cumulative meters
    left_power: Option<u16>,  // Left leg watts
    right_power: Option<u16>, // Right leg watts
}
```

#### WorkoutSummary Structure
```rust
struct WorkoutSummary {
    avg_heart_rate: Option<u16>,     // Average HR
    max_heart_rate: Option<u16>,     // Maximum HR
    avg_power: Option<u16>,          // Average power
    normalized_power: Option<u16>,   // Normalized power
    avg_pace: Option<Decimal>,       // Average pace
    intensity_factor: Option<Decimal>, // IF ratio
    tss: Option<Decimal>,            // Training stress score
    total_distance: Option<Decimal>,  // Total meters
    elevation_gain: Option<u16>,     // Total ascent
    avg_cadence: Option<u16>,        // Average cadence
    calories: Option<u16>,           // Estimated calories
}
```

#### Workout Structure
```rust
struct Workout {
    id: String,                      // Unique identifier
    date: NaiveDate,                 // Workout date
    sport: Sport,                    // Sport type
    duration_seconds: u32,           // Total duration
    workout_type: WorkoutType,       // Training type
    data_source: DataSource,         // Primary metric
    raw_data: Option<Vec<DataPoint>>, // Time series data
    summary: WorkoutSummary,         // Calculated metrics
    notes: Option<String>,           // User notes
    athlete_id: Option<String>,      // Athlete reference
    source: Option<String>,          // Import source
}
```

### Enumerated Types

#### Sport Types
```rust
enum Sport {
    Running,
    Cycling,
    Swimming,
    Triathlon,
    Rowing,
    CrossTraining,
}
```

#### Workout Types
```rust
enum WorkoutType {
    Interval,    // High-intensity intervals
    Endurance,   // Steady aerobic effort
    Recovery,    // Low-intensity recovery
    Tempo,       // Comfortably hard effort
    Threshold,   // Lactate threshold
    VO2Max,      // Maximum aerobic power
    Strength,    // Strength/resistance training
    Race,        // Competition effort
    Test,        // Performance testing
}
```

#### Data Sources
```rust
enum DataSource {
    HeartRate,   // HR-based calculations
    Power,       // Power-based calculations
    Pace,        // Pace-based calculations
    Rpe,         // Rate of perceived exertion
}
```

## Validation Requirements

### Data Quality Standards

#### Required Fields
- **Timestamp** or **Duration**: At least one timing reference
- **Primary Metric**: Heart rate, power, or pace data
- **Date**: Valid workout date

#### Optional Fields
- GPS coordinates (for outdoor activities)
- Elevation data (for terrain analysis)
- Cadence data (for efficiency analysis)
- Temperature and weather data

#### Data Range Validation

| Field | Minimum | Maximum | Notes |
|-------|---------|---------|-------|
| Heart Rate | 30 BPM | 220 BPM | Physiological limits |
| Power | 0 W | 2000 W | Equipment/human limits |
| Pace | 2:00 min/unit | 20:00 min/unit | Reasonable pace range |
| Elevation | -500m | 9000m | Sea level to high altitude |
| Cadence | 30 | 300 | Sport-dependent ranges |
| Speed | 0 m/s | 25 m/s | Human performance limits |

#### File Size Limits
- **CSV**: 50MB maximum
- **GPX**: 25MB maximum
- **TCX**: 25MB maximum
- **FIT**: 10MB maximum

### Error Handling

#### Common Import Errors
1. **Invalid date format**: Use ISO 8601 or standard formats
2. **Missing required columns**: Ensure timestamp and primary metric
3. **Out-of-range values**: Check data against validation limits
4. **Encoding issues**: Use UTF-8 encoding for text files
5. **Corrupted files**: Verify file integrity before import

#### Validation Commands
```bash
# Validate file before import
trainrs import --file data.csv --validate-only

# Import with error reporting
trainrs import --file data.csv --strict-validation

# Skip validation for trusted sources
trainrs import --file data.csv --skip-validation
```

## Examples

### Complete CSV Workout Example

```csv
timestamp,heart_rate,power,elevation,cadence,distance,speed
0,110,150,100,85,0.0,0.0
1,112,155,100,86,8.5,8.5
2,115,160,101,87,17.2,8.7
3,118,165,101,88,26.1,8.9
4,120,170,102,89,35.3,9.2
```

### Training Peaks Compatible Export

```bash
trainrs export --format trainingpeaks --output tp_data.csv
```

**Output:**
```csv
Date,Workout Name,Sport,Duration,TSS,IF,NP,Work,Distance
2024-09-23,Morning Ride,Bike,02:00:00,95.2,0.94,235,684000,25.0
2024-09-24,Recovery Run,Run,01:30:00,65.0,0.75,,468000,12.5
```

### Strava Bulk Upload Format

```bash
trainrs export --format strava --output strava_data.json
```

**Output:**
```json
{
  "activities": [
    {
      "name": "TrainRS Export - Cycling",
      "type": "Ride",
      "start_date_local": "2024-09-23T14:30:00Z",
      "elapsed_time": 7200,
      "description": "Exported from TrainRS - TSS: 95.2",
      "distance": 25000,
      "trainer": false,
      "commute": false
    }
  ]
}
```

### Multi-Sport Analysis Export

```bash
trainrs multi-sport --export --format json --output multisport.json
```

**Output:**
```json
{
  "analysis_period": {
    "start": "2024-09-01",
    "end": "2024-09-30"
  },
  "total_load": {
    "combined_tss": 2150.5,
    "sport_breakdown": {
      "Cycling": {
        "tss": 1420.0,
        "percentage": 66.0,
        "scaling_factor": 1.0
      },
      "Running": {
        "tss": 730.5,
        "percentage": 34.0,
        "scaling_factor": 1.3
      }
    }
  },
  "weekly_distribution": [...],
  "recommendations": [
    "Well-balanced multi-sport training",
    "Consider increasing swimming volume"
  ]
}
```

## Best Practices

### Import Guidelines

#### File Preparation
1. **Clean data**: Remove obviously erroneous data points
2. **Consistent formatting**: Use standard date/time formats
3. **Complete headers**: Include all relevant column names
4. **UTF-8 encoding**: Ensure proper text encoding
5. **Reasonable file sizes**: Keep under recommended limits

#### Column Naming
- Use descriptive, standard names when possible
- Avoid special characters and spaces
- Include units in column names if ambiguous
- Follow platform conventions for compatibility

#### Data Quality
- Validate data ranges before import
- Include calibration notes for power meters
- Document any data processing applied
- Maintain original files as backups

### Export Guidelines

#### Format Selection
- **CSV**: For spreadsheet analysis and data exchange
- **JSON**: For programmatic use and API integration
- **HTML**: For sharing and presentation
- **PDF**: For archival and documentation

#### Date Range Planning
- Export manageable date ranges (monthly/quarterly)
- Include sufficient history for trend analysis
- Consider file size limits for large exports
- Use incremental exports for ongoing analysis

#### Data Security
- Remove personal information for shared exports
- Use anonymized athlete IDs when appropriate
- Secure file transmission and storage
- Regular backup of exported data

### Performance Optimization

#### Large Dataset Handling
```bash
# Use streaming for large imports
trainrs import --file large_data.csv --streaming

# Batch process multiple files
trainrs import --directory ./data/ --batch-size 100

# Optimize database after large imports
trainrs maintenance --optimize-database
```

#### Export Optimization
```bash
# Export specific date ranges
trainrs export --from 2024-09-01 --to 2024-09-30

# Exclude raw data for smaller files
trainrs export --no-raw-data --format json

# Use compression for large exports
trainrs export --compress --output data.csv.gz
```

---

*For troubleshooting data format issues, see the [Troubleshooting Guide](troubleshooting.md). For sports science background on metrics calculations, see the [Sports Science Guide](sports-science.md).*