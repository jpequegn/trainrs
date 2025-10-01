# Advanced FIT Import Usage

This guide covers advanced scenarios for FIT file import.

## Batch Processing

### Processing Multiple Files

```rust
use trainrs::import::fit::FitImporter;
use std::path::Path;

fn import_directory<P: AsRef<Path>>(dir: P) -> Result<Vec<Workout>, Box<dyn std::error::Error>> {
    let importer = FitImporter::new();
    let mut all_workouts = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("fit") {
            match importer.import_file(&path) {
                Ok(mut workouts) => {
                    all_workouts.append(&mut workouts);
                }
                Err(e) => {
                    eprintln!("Failed to import {}: {}", path.display(), e);
                    // Continue processing other files
                }
            }
        }
    }

    Ok(all_workouts)
}
```

### Parallel Processing with Rayon

```rust
use rayon::prelude::*;
use trainrs::import::fit::FitImporter;

fn parallel_import(files: Vec<PathBuf>) -> Vec<Workout> {
    files
        .par_iter()
        .filter_map(|path| {
            let importer = FitImporter::new();
            match importer.import_file(path) {
                Ok(workouts) => Some(workouts),
                Err(e) => {
                    eprintln!("Error importing {}: {}", path.display(), e);
                    None
                }
            }
        })
        .flatten()
        .collect()
}
```

## Custom Developer Field Registry

### Creating a Custom Registry

```rust
use trainrs::import::fit::{FitImporter, DeveloperFieldRegistry, DeveloperFieldDefinition};

// Create custom registry
let mut registry = DeveloperFieldRegistry::new();

// Add your custom field definition
registry.register_field(DeveloperFieldDefinition {
    app_uuid: "your-app-uuid".to_string(),
    app_name: "Your App".to_string(),
    field_number: 0,
    field_name: "custom_metric".to_string(),
    field_type: "uint16".to_string(),
    units: Some("watts".to_string()),
    description: Some("Custom power metric".to_string()),
});

// Use custom registry
let importer = FitImporter::with_registry(registry);
let workouts = importer.import_file("workout.fit")?;
```

### Combining with Default Registry

```rust
// Start with default registry
let mut registry = DeveloperFieldRegistry::default();

// Add custom fields
registry.register_field(my_custom_field);

let importer = FitImporter::with_registry(registry);
```

## Data Validation

### Custom Validation Rules

```rust
use trainrs::import::validation::{ValidationConfig, ValidationRule};

let mut config = ValidationConfig::default();

// Require minimum data quality
config.min_data_points = 100;
config.require_power = true;
config.require_heart_rate = false;

// Add custom rules
config.add_rule(ValidationRule {
    name: "valid_power_range",
    check: Box::new(|workout| {
        workout.data_points.iter().all(|p| {
            p.power.map_or(true, |w| w > 0 && w < 2000)
        })
    }),
    message: "Power values must be between 0 and 2000W",
});
```

### Filtering Invalid Data

```rust
fn filter_valid_workouts(workouts: Vec<Workout>) -> Vec<Workout> {
    workouts
        .into_iter()
        .filter(|w| {
            // Has minimum duration
            w.duration_seconds >= 300 &&
            // Has sufficient data points
            w.data_points.len() >= 100 &&
            // Has key metrics
            w.summary.avg_power.is_some()
        })
        .collect()
}
```

## Handling Corrupted Files

### Partial File Recovery

```rust
use trainrs::import::fit::FitImporter;
use trainrs::error::FitError;

fn import_with_recovery(path: &Path) -> Result<Vec<Workout>, Box<dyn std::error::Error>> {
    let importer = FitImporter::new();

    match importer.import_file(path) {
        Ok(workouts) => Ok(workouts),
        Err(e) if e.is_recoverable() => {
            // Try recovery
            eprintln!("File partially corrupted, attempting recovery...");

            // Import with lenient mode
            importer.import_file_lenient(path)
        }
        Err(e) => Err(e.into())
    }
}
```

### Checksum Validation

```rust
// Verify file integrity before import
if let Err(FitError::ChecksumMismatch { expected, actual }) = importer.verify_checksum(path) {
    eprintln!("Checksum mismatch: expected {}, got {}", expected, actual);
    // Decide whether to proceed
}
```

## Memory-Efficient Streaming

For very large FIT files (>100MB), use streaming import:

```rust
use trainrs::import::streaming::StreamingFitImporter;

let importer = StreamingFitImporter::new();

// Process file in chunks
importer.import_streaming("large_workout.fit", |workout| {
    // Process each workout as it's parsed
    println!("Processed workout: {:?}", workout.sport);

    // Return true to continue, false to stop
    true
})?;
```

## Multi-Sport Workout Handling

### Separating Multi-Sport Sessions

```rust
fn separate_multisport(workouts: Vec<Workout>) -> HashMap<Sport, Vec<Workout>> {
    let mut by_sport = HashMap::new();

    for workout in workouts {
        by_sport
            .entry(workout.sport)
            .or_insert_with(Vec::new)
            .push(workout);
    }

    by_sport
}
```

### Triathlon Workflow

```rust
// Import triathlon file
let workouts = importer.import_file("triathlon.fit")?;

// Separate by sport
let swim = workouts.iter().find(|w| w.sport == Sport::Swimming);
let bike = workouts.iter().find(|w| w.sport == Sport::Cycling);
let run = workouts.iter().find(|w| w.sport == Sport::Running);

// Calculate sport-specific metrics
if let Some(bike_workout) = bike {
    let tss = calculate_power_tss(bike_workout, athlete_ftp)?;
    println!("Bike TSS: {}", tss);
}
```

## Performance Optimization

### Caching Parsed Data

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

struct CachedFitImporter {
    importer: FitImporter,
    cache: Arc<Mutex<HashMap<PathBuf, Vec<Workout>>>>,
}

impl CachedFitImporter {
    fn import_cached(&self, path: &Path) -> Result<Vec<Workout>, Box<dyn std::error::Error>> {
        let mut cache = self.cache.lock().unwrap();

        if let Some(workouts) = cache.get(path) {
            return Ok(workouts.clone());
        }

        let workouts = self.importer.import_file(path)?;
        cache.insert(path.to_path_buf(), workouts.clone());

        Ok(workouts)
    }
}
```

### Reducing Memory Usage

```rust
// Import only summary data, skip time-series
let config = ImportConfig {
    include_data_points: false,
    include_laps: true,
    include_sessions: true,
};

let importer = FitImporter::with_config(config);
let workouts = importer.import_file("workout.fit")?;

// Workouts will have summary but no data_points
assert!(workouts[0].data_points.is_empty());
```

## Integration with Database

### Batch Database Insert

```rust
use trainrs::database::Database;

fn import_and_store(db: &Database, files: Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let importer = FitImporter::new();

    for path in files {
        let workouts = importer.import_file(&path)?;

        // Store in transaction for atomic operation
        db.begin_transaction()?;

        for workout in workouts {
            db.insert_workout(&workout)?;
        }

        db.commit()?;
    }

    Ok(())
}
```

## Next Steps

- [Code Examples](examples/) - Complete working examples
- [Troubleshooting](troubleshooting.md) - Common issues and solutions
- [API Reference](https://docs.rs/trainrs) - Full API documentation
