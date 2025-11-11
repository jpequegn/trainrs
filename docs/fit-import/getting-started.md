# Getting Started with FIT Import

This guide covers the basics of importing FIT files into TrainRS.

## Basic Usage

### Simple Import

The simplest way to import a FIT file:

```rust
use trainrs::import::fit::FitImporter;

let importer = FitImporter::new();
let workouts = importer.import_file("workout.fit")?;

for workout in workouts {
    println!("Sport: {:?}", workout.sport);
    println!("Duration: {}s", workout.duration_seconds);
    println!("TSS: {:?}", workout.summary.tss);
}
```

### Understanding the Output

Each FIT file produces a `Vec<Workout>` because:
- Multi-sport workouts (triathlons) create separate `Workout` entries per sport
- Some devices create multiple sessions in a single file

The `Workout` structure contains:
- **Basic info**: `date`, `sport`, `duration_seconds`
- **Summary metrics**: `WorkoutSummary` with TSS, distance, calories, etc.
- **Time-series data**: `Vec<DataPoint>` with power, heart rate, speed, etc.

### Accessing Workout Data

```rust
let workout = &workouts[0];

// Basic information
println!("Date: {}", workout.date);
println!("Sport: {:?}", workout.sport);
println!("Duration: {}s", workout.duration_seconds);

// Summary metrics
if let Some(avg_power) = workout.summary.avg_power {
    println!("Average Power: {}W", avg_power);
}

if let Some(tss) = workout.summary.tss {
    println!("Training Stress Score: {}", tss);
}

// Time-series data
for point in &workout.data_points {
    if let Some(power) = point.power {
        println!("Power at {}s: {}W", point.timestamp, power);
    }
}
```

## Developer Fields

TrainRS automatically extracts custom fields from popular cycling apps.

### Supported Applications

- **Xert**: Workout difficulty, focus, and recommendations
- **TrainerRoad**: Workout details and structure
- **Wahoo SYSTM**: Workout targets and metrics
- **Garmin Connect IQ**: Custom data fields
- **Zwift**: Virtual power and game data
- **And 7 more...**

### Accessing Developer Fields

```rust
let importer = FitImporter::new();
let workouts = importer.import_file("workout.fit")?;

// Access the registry to see what was found
let registry = importer.registry();

// Check for Xert fields
if let Some(xert_fields) = registry.get_fields_by_app_name("Xert") {
    println!("Found {} Xert fields", xert_fields.len());
}

// Access custom data in workout
for point in &workouts[0].data_points {
    // Custom fields are stored in the data point
    // Access through the developer field API
}
```

## Error Handling

FIT files can be corrupted or incomplete. Handle errors gracefully:

```rust
use trainrs::error::TrainRsError;

match importer.import_file("workout.fit") {
    Ok(workouts) => {
        println!("Successfully imported {} workouts", workouts.len());
    }
    Err(TrainRsError::FitParsing(fit_error)) => {
        eprintln!("FIT parsing error: {}", fit_error);
        // Try recovery or skip file
    }
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

### Common Error Types

- **FileNotFound**: File doesn't exist at specified path
- **Corrupted**: File is damaged or invalid
- **UnsupportedVersion**: FIT protocol version not supported
- **MissingMessage**: Required FIT messages missing
- **ChecksumMismatch**: File integrity check failed

## Next Steps

- [Advanced Usage](advanced-usage.md) - Batch processing, custom registries, validation
- [Code Examples](examples/) - Complete working examples
- [Troubleshooting](troubleshooting.md) - Common issues and solutions
- [API Reference](https://docs.rs/trainrs) - Full API documentation
