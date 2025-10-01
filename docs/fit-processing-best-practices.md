# FIT Data Processing: Best Practices Guide

Comprehensive guide to processing FIT files with optimal performance, data quality, and reliability.

## Table of Contents

1. [Performance Best Practices](#performance-best-practices)
2. [Data Quality Best Practices](#data-quality-best-practices)
3. [Error Handling Best Practices](#error-handling-best-practices)
4. [Anti-Patterns to Avoid](#anti-patterns-to-avoid)
5. [Real-World Examples](#real-world-examples)
6. [Troubleshooting Guide](#troubleshooting-guide)

---

## Performance Best Practices

### 1. Use Streaming for Large Files

Processing large FIT files efficiently requires streaming to avoid memory issues.

❌ **Don't** load entire file into memory:
```rust
// BAD: Loads entire file at once
let data = std::fs::read("large_workout.fit")?;
let workout = parse_fit_bytes(&data)?;
```

✅ **Do** use streaming parser:
```rust
// GOOD: Processes file in chunks
use trainrs::import::streaming::StreamingFitParser;

let mut parser = StreamingFitParser::new("large_workout.fit", 8192)?;
for data_point in parser.iter_data_points() {
    process_point(data_point?);
}
```

**Benefits:**
- Constant memory usage regardless of file size
- Start processing immediately (no upfront load time)
- Handle files larger than available RAM

### 2. Batch Import with Parallel Processing

When importing multiple files, leverage parallelism for significant speedups.

❌ **Don't** process files sequentially:
```rust
// BAD: Sequential processing
for file in files {
    import_file(&file)?;
}
```

✅ **Do** use parallel processing:
```rust
// GOOD: Parallel processing with rayon
use rayon::prelude::*;

let results: Vec<_> = files
    .par_iter()
    .map(|file| import_file(file))
    .collect();

// Handle errors after parallel execution
for result in results {
    result?;
}
```

**Performance gain:** 4-8x faster on multi-core systems

### 3. Cache Parsed Results

Avoid re-parsing the same file multiple times.

✅ **Do** implement caching:
```rust
use std::collections::HashMap;
use std::path::PathBuf;

pub struct FitCache {
    cache: HashMap<PathBuf, Workout>,
}

impl FitCache {
    pub fn get_or_parse(&mut self, path: &Path) -> Result<&Workout> {
        if !self.cache.contains_key(path) {
            let workout = parse_fit(path)?;
            self.cache.insert(path.to_path_buf(), workout);
        }
        Ok(self.cache.get(path).unwrap())
    }
}
```

**Cache invalidation strategies:**
- File modification time (mtime)
- File size changes
- Explicit cache clear on demand

### 4. Database Batch Operations

Minimize database round-trips by batching operations.

❌ **Don't** insert one at a time:
```rust
// BAD: Individual inserts
for data_point in data_points {
    db.insert_data_point(&data_point)?;
}
```

✅ **Do** batch inserts:
```rust
// GOOD: Batch insert
db.begin_transaction()?;
db.batch_insert_data_points(&data_points)?;
db.commit()?;
```

**Performance gain:** 10-100x faster for large datasets

### 5. Optimize TSS Calculations

TSS calculation can be CPU-intensive for long workouts.

✅ **Do** use optimized algorithms:
```rust
use rust_decimal::Decimal;

// Pre-compute normalized power efficiently
pub fn calculate_normalized_power_optimized(
    power_data: &[u16],
) -> Decimal {
    // Use rolling window average
    let window_size = 30; // 30-second window
    let mut rolling_sum = 0u64;
    let mut np_values = Vec::with_capacity(power_data.len());

    for (i, &power) in power_data.iter().enumerate() {
        rolling_sum += power as u64;

        if i >= window_size {
            rolling_sum -= power_data[i - window_size] as u64;
        }

        let window_len = (i + 1).min(window_size) as u64;
        let avg = rolling_sum / window_len;
        np_values.push(avg);
    }

    // Fourth root calculation
    let sum_fourth_power: f64 = np_values
        .iter()
        .map(|&v| (v as f64).powi(4))
        .sum();

    let np = (sum_fourth_power / np_values.len() as f64).powf(0.25);
    Decimal::from_f64_retain(np).unwrap()
}
```

### 6. Memory-Efficient Data Structures

Choose appropriate data structures for your use case.

✅ **Do** use efficient representations:
```rust
// For sparse data (many missing values)
pub struct SparseDataPoint {
    timestamp: u32,
    values: HashMap<DataField, f64>, // Only store present values
}

// For dense data (most values present)
pub struct DenseDataPoint {
    timestamp: u32,
    power: Option<u16>,
    heart_rate: Option<u8>,
    cadence: Option<u8>,
    // ... fixed fields
}
```

**Memory savings:** Up to 50% for sparse workout data

---

## Data Quality Best Practices

### 1. Always Validate Input Data

Never trust FIT file data without validation.

✅ **Do** validate before processing:
```rust
use trainrs::validation::{DataValidator, ValidationConfig};

pub fn safe_import_workout(path: &Path) -> Result<Workout> {
    let workout = import_fit(path)?;

    let validator = DataValidator::new(ValidationConfig::strict());
    let report = validator.validate_workout(&workout);

    if !report.passed {
        for error in &report.errors {
            eprintln!("Validation error: {}", error);
        }
        return Err(ValidationError::FailedValidation(report));
    }

    Ok(workout)
}
```

**Validation checks:**
- Timestamp monotonicity
- Value range checks (heart rate 20-250 bpm)
- Sport-specific constraints
- Data consistency (duration matches data points)

### 2. Handle Missing Data Gracefully

FIT files often have missing or incomplete data.

❌ **Don't** panic on missing data:
```rust
// BAD: Will panic
let power = data_point.power.unwrap();
let tss = calculate_tss(power, ftp);
```

✅ **Do** use proper error handling:
```rust
// GOOD: Graceful fallback
match data_point.power {
    Some(power) => {
        let tss = calculate_tss(power, ftp);
        Some(tss)
    }
    None => {
        log::warn!("Missing power data at {}", data_point.timestamp);
        // Use heart rate-based estimate or skip
        estimate_tss_from_hr(&data_point, athlete_profile)
    }
}
```

**Fallback strategies:**
- Power → Pace → Heart Rate → Estimated TSS
- Interpolate short gaps (<5 seconds)
- Mark data quality in output

### 3. Detect and Handle Outliers

GPS errors and sensor glitches create outliers.

✅ **Do** implement outlier detection:
```rust
pub fn detect_power_outliers(
    data_points: &[DataPoint],
    sport: Sport,
) -> Vec<usize> {
    let max_power = match sport {
        Sport::Cycling => 2000,  // Elite max ~1800W
        Sport::Running => 500,   // Elite max ~400W
        _ => u16::MAX,
    };

    data_points
        .iter()
        .enumerate()
        .filter_map(|(i, point)| {
            if let Some(power) = point.power {
                if power > max_power {
                    log::warn!(
                        "Power outlier at index {}: {}W (max: {}W)",
                        i, power, max_power
                    );
                    return Some(i);
                }
            }
            None
        })
        .collect()
}

// Remove or clip outliers
pub fn remove_outliers(
    mut data_points: Vec<DataPoint>,
    outlier_indices: &[usize],
) -> Vec<DataPoint> {
    for &idx in outlier_indices.iter().rev() {
        data_points.remove(idx);
    }
    data_points
}
```

**Common outliers:**
- GPS spikes (speed >100 km/h for running)
- Heart rate spikes (>220 bpm)
- Power spikes (>2000W for cycling)
- Cadence errors (>200 rpm for cycling)

### 4. Sport-Specific Validation

Different sports have different data characteristics.

✅ **Do** apply sport-specific validation:
```rust
pub fn validate_sport_specific(
    workout: &Workout,
) -> ValidationResult {
    match workout.sport {
        Sport::Cycling => {
            // Cycling-specific checks
            if let Some(avg_power) = workout.avg_power {
                if avg_power < 50 || avg_power > 500 {
                    return ValidationResult::Warning(
                        "Unusual average power for cycling"
                    );
                }
            }
        }
        Sport::Running => {
            // Running-specific checks
            if let Some(avg_pace) = workout.avg_pace {
                let pace_min_km = avg_pace.as_secs() / 60;
                if pace_min_km < 2 || pace_min_km > 10 {
                    return ValidationResult::Warning(
                        "Unusual pace for running"
                    );
                }
            }
        }
        Sport::Swimming => {
            // Swimming-specific checks
            if workout.distance.is_some() && workout.pool_length.is_none() {
                return ValidationResult::Error(
                    "Swimming workout missing pool length"
                );
            }
        }
        _ => {}
    }

    ValidationResult::Ok
}
```

### 5. Privacy and Data Sanitization

Remove sensitive information when sharing or exporting data.

✅ **Do** sanitize personal data:
```rust
pub fn sanitize_workout(workout: &mut Workout) {
    // Remove precise GPS coordinates (keep general area)
    if let Some(ref mut points) = workout.data_points {
        for point in points {
            if let Some(ref mut pos) = point.position {
                // Round to ~1km precision
                pos.latitude = (pos.latitude * 100.0).round() / 100.0;
                pos.longitude = (pos.longitude * 100.0).round() / 100.0;
            }
        }
    }

    // Remove home/work locations
    workout.start_position = None;
    workout.end_position = None;

    // Keep timestamps but remove dates
    workout.start_time = None;
}
```

**Sensitive data to consider:**
- GPS coordinates (especially start/end)
- Timestamps (can reveal schedule)
- Device serial numbers
- User profile information

---

## Error Handling Best Practices

### 1. Graceful Degradation

Continue processing even when some data is corrupted.

✅ **Do** implement partial recovery:
```rust
pub fn import_with_recovery(path: &Path) -> Result<Workout> {
    let mut parser = FitParser::new(path)?;
    let mut data_points = Vec::new();
    let mut error_count = 0;
    const MAX_ERRORS: usize = 10;

    for record in parser.records() {
        match record {
            Ok(data_point) => data_points.push(data_point),
            Err(e) => {
                log::warn!("Skipping corrupted record: {}", e);
                error_count += 1;

                if error_count > MAX_ERRORS {
                    return Err(ImportError::TooManyErrors(error_count));
                }
            }
        }
    }

    if error_count > 0 {
        log::info!(
            "Imported with {} errors, recovered {} data points",
            error_count,
            data_points.len()
        );
    }

    Ok(Workout {
        data_points,
        quality: if error_count > 0 {
            DataQuality::Degraded
        } else {
            DataQuality::Good
        },
        ..Default::default()
    })
}
```

### 2. User-Friendly Error Messages

Provide actionable error messages for users.

❌ **Don't** expose internal errors:
```rust
// BAD: Technical error message
return Err("CRC validation failed at offset 0x1a4b".into());
```

✅ **Do** provide helpful context:
```rust
// GOOD: User-friendly error message
pub enum ImportError {
    CorruptedFile {
        path: PathBuf,
        reason: String,
        suggestion: String,
    },
    // ...
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ImportError::CorruptedFile { path, reason, suggestion } => {
                write!(
                    f,
                    "Failed to import '{}': {}\n\nSuggestion: {}",
                    path.display(),
                    reason,
                    suggestion
                )
            }
        }
    }
}

// Usage
if crc_failed {
    return Err(ImportError::CorruptedFile {
        path: path.to_path_buf(),
        reason: "File appears to be corrupted or incomplete".into(),
        suggestion: "Try re-downloading the file from your device".into(),
    });
}
```

### 3. Structured Logging

Use structured logging for better debugging.

✅ **Do** use structured logging:
```rust
use tracing::{info, warn, error, debug};
use tracing::instrument;

#[instrument(skip(data), fields(data_points = data.len()))]
pub fn process_workout_data(data: &[DataPoint]) -> Result<WorkoutSummary> {
    debug!("Starting workout data processing");

    let summary = calculate_summary(data)?;

    info!(
        duration_seconds = summary.duration_seconds,
        avg_power = summary.avg_power,
        "Workout processing complete"
    );

    Ok(summary)
}
```

**Logging best practices:**
- Use appropriate log levels (error, warn, info, debug, trace)
- Include context (file path, workout ID)
- Log structured data (JSON-friendly)
- Don't log sensitive information

### 4. Error Recovery Strategies

Implement smart recovery for common issues.

✅ **Do** attempt automatic recovery:
```rust
pub fn parse_with_recovery(path: &Path) -> Result<Workout> {
    // Try normal parsing first
    match parse_fit_strict(path) {
        Ok(workout) => return Ok(workout),
        Err(e) => {
            log::warn!("Strict parsing failed: {}, trying recovery", e);
        }
    }

    // Try CRC-tolerant parsing
    match parse_fit_skip_crc(path) {
        Ok(workout) => {
            log::info!("Recovered workout using CRC-tolerant mode");
            return Ok(workout);
        }
        Err(e) => {
            log::warn!("CRC-tolerant parsing failed: {}", e);
        }
    }

    // Try header-only import (metadata without data points)
    match parse_fit_metadata_only(path) {
        Ok(workout) => {
            log::info!("Recovered workout metadata only");
            return Ok(workout);
        }
        Err(e) => {
            log::error!("All recovery attempts failed: {}", e);
            return Err(e);
        }
    }
}
```

---

## Anti-Patterns to Avoid

### 1. ❌ Ignoring CRC Errors

**Problem:** Silently accepting corrupted data leads to incorrect calculations.

```rust
// BAD: Skip CRC validation
let workout = parse_fit_unchecked(file)?;
```

**Solution:** Always validate CRC, use recovery mode if needed.

```rust
// GOOD: Validate CRC with fallback
let workout = match parse_fit_with_crc(file) {
    Ok(w) => w,
    Err(CrcError) => {
        log::warn!("CRC failed, attempting recovery");
        parse_fit_with_recovery(file)?
    }
    Err(e) => return Err(e),
};
```

### 2. ❌ Blocking Operations in Hot Paths

**Problem:** Synchronous I/O in tight loops kills performance.

```rust
// BAD: Database write per data point
for point in data_points {
    db.insert_data_point(&point)?; // Slow!
}
```

**Solution:** Batch operations or use async I/O.

```rust
// GOOD: Batch database operations
const BATCH_SIZE: usize = 1000;

for chunk in data_points.chunks(BATCH_SIZE) {
    db.batch_insert(chunk)?;
}
```

### 3. ❌ Assuming Single Sport

**Problem:** Multi-sport workouts (triathlons) break single-sport assumptions.

```rust
// BAD: Assumes single sport
assert_eq!(workout.sport, Sport::Cycling);
let ftp = athlete_profile.cycling_ftp;
```

**Solution:** Handle multi-sport sessions.

```rust
// GOOD: Support multi-sport
for session in workout.sessions {
    let threshold = match session.sport {
        Sport::Cycling => athlete_profile.cycling_ftp,
        Sport::Running => athlete_profile.running_threshold_pace,
        Sport::Swimming => athlete_profile.swimming_css,
        _ => continue,
    };

    calculate_session_tss(&session, threshold)?;
}
```

### 4. ❌ Ignoring Time Zones

**Problem:** Naive datetime handling loses timezone information.

```rust
// BAD: Loses timezone info
let naive_time = NaiveDateTime::from_timestamp(workout.timestamp, 0);
```

**Solution:** Preserve timezone throughout processing.

```rust
// GOOD: Timezone-aware timestamps
use chrono::{DateTime, Utc};

let utc_time: DateTime<Utc> = DateTime::from_timestamp(
    workout.timestamp as i64,
    0
).unwrap();

// Convert to local if needed
let local_time = utc_time.with_timezone(&workout.timezone);
```

### 5. ❌ Precision Loss in Calculations

**Problem:** Using floating point for financial-grade precision requirements.

```rust
// BAD: Floating point precision issues
let tss = (duration_hours * intensity_factor.powi(2)) * 100.0; // f64
```

**Solution:** Use Decimal for precise calculations.

```rust
// GOOD: Decimal precision
use rust_decimal::Decimal;

let tss = (duration_hours * intensity_factor.powi(2)) * dec!(100);
```

### 6. ❌ Unbounded Memory Growth

**Problem:** Accumulating data without limits.

```rust
// BAD: Unbounded cache
let mut cache: HashMap<PathBuf, Workout> = HashMap::new();
for file in all_files {
    cache.insert(file.clone(), parse_fit(&file)?); // Memory leak!
}
```

**Solution:** Implement LRU cache with size limits.

```rust
// GOOD: Bounded LRU cache
use lru::LruCache;

let mut cache = LruCache::new(100); // Max 100 entries
for file in all_files {
    if !cache.contains(&file) {
        cache.put(file.clone(), parse_fit(&file)?);
    }
}
```

### 7. ❌ Tight Coupling to FIT Format

**Problem:** Business logic mixed with FIT parsing.

```rust
// BAD: TSS calculation in FIT parser
impl FitParser {
    fn parse_and_calculate_tss(&self) -> Result<(Workout, Decimal)> {
        // Mixing concerns
    }
}
```

**Solution:** Separate parsing from domain logic.

```rust
// GOOD: Separation of concerns
let workout = fit_parser.parse(file)?;  // Pure parsing
let tss = tss_calculator.calculate(&workout)?;  // Business logic
```

---

## Real-World Examples

### Example 1: High-Performance Batch Import

Complete example of importing multiple FIT files efficiently.

```rust
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use tracing::info;

pub struct BatchImporter {
    cache: FitCache,
    validator: DataValidator,
}

impl BatchImporter {
    pub fn import_directory(&mut self, dir: &Path) -> Result<Vec<Workout>> {
        // Find all FIT files
        let files: Vec<PathBuf> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension() == Some("fit".as_ref()))
            .collect();

        info!("Found {} FIT files to import", files.len());

        // Parallel processing
        let results: Vec<_> = files
            .par_iter()
            .map(|file| self.import_single_with_recovery(file))
            .collect();

        // Separate successes and failures
        let mut workouts = Vec::new();
        let mut errors = Vec::new();

        for (file, result) in files.iter().zip(results) {
            match result {
                Ok(workout) => workouts.push(workout),
                Err(e) => {
                    errors.push((file.clone(), e));
                }
            }
        }

        if !errors.is_empty() {
            log::warn!("Failed to import {} files", errors.len());
            for (file, err) in errors {
                log::error!("  {}: {}", file.display(), err);
            }
        }

        info!("Successfully imported {} workouts", workouts.len());
        Ok(workouts)
    }

    fn import_single_with_recovery(&self, file: &Path) -> Result<Workout> {
        // Check cache first
        if let Some(cached) = self.cache.get(file) {
            return Ok(cached.clone());
        }

        // Import with recovery
        let workout = parse_with_recovery(file)?;

        // Validate
        let validation = self.validator.validate_workout(&workout);
        if !validation.passed {
            return Err(ImportError::ValidationFailed(validation));
        }

        // Cache result
        self.cache.insert(file, workout.clone());

        Ok(workout)
    }
}
```

### Example 2: Robust TSS Calculation

TSS calculation with all edge cases handled.

```rust
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub struct TssCalculator {
    config: TssConfig,
}

impl TssCalculator {
    pub fn calculate_workout_tss(
        &self,
        workout: &Workout,
        athlete: &AthleteProfile,
    ) -> Result<Decimal> {
        // Determine TSS method based on available data
        if let Some(tss) = self.try_power_based_tss(workout, athlete)? {
            return Ok(tss);
        }

        if let Some(tss) = self.try_pace_based_tss(workout, athlete)? {
            return Ok(tss);
        }

        if let Some(tss) = self.try_hr_based_tss(workout, athlete)? {
            return Ok(tss);
        }

        // Fallback to duration-based estimate
        self.estimate_tss_from_duration(workout)
    }

    fn try_power_based_tss(
        &self,
        workout: &Workout,
        athlete: &AthleteProfile,
    ) -> Result<Option<Decimal>> {
        // Validate power data availability
        let power_data: Vec<u16> = workout
            .data_points
            .iter()
            .filter_map(|p| p.power)
            .collect();

        if power_data.is_empty() {
            return Ok(None);
        }

        // Check data quality
        let valid_ratio = power_data.len() as f64
            / workout.data_points.len() as f64;

        if valid_ratio < 0.8 {
            log::warn!(
                "Insufficient power data: {:.1}% coverage",
                valid_ratio * 100.0
            );
            return Ok(None);
        }

        // Get appropriate FTP
        let ftp = match workout.sport {
            Sport::Cycling => athlete.cycling_ftp,
            Sport::Running => {
                // Convert running power threshold
                athlete.running_power_threshold
            }
            _ => return Ok(None),
        };

        if ftp == 0 {
            return Err(TssError::MissingThreshold);
        }

        // Calculate normalized power
        let np = calculate_normalized_power(&power_data);

        // Calculate intensity factor
        let if_value = np / Decimal::from(ftp);

        // Calculate TSS
        let duration_hours = Decimal::from(workout.duration_seconds)
            / dec!(3600);
        let tss = (duration_hours * if_value.powi(2)) * dec!(100);

        Ok(Some(tss))
    }

    fn estimate_tss_from_duration(
        &self,
        workout: &Workout,
    ) -> Result<Decimal> {
        // Conservative estimate: 50 TSS per hour at moderate intensity
        let hours = Decimal::from(workout.duration_seconds) / dec!(3600);
        let estimated_tss = hours * dec!(50);

        log::info!(
            "Using duration-based TSS estimate: {} (low confidence)",
            estimated_tss
        );

        Ok(estimated_tss)
    }
}
```

### Example 3: Data Quality Reporting

Comprehensive data quality analysis.

```rust
pub struct QualityReport {
    pub overall_score: f64,  // 0.0-1.0
    pub issues: Vec<QualityIssue>,
    pub metrics: QualityMetrics,
}

pub struct QualityAnalyzer;

impl QualityAnalyzer {
    pub fn analyze(&self, workout: &Workout) -> QualityReport {
        let mut issues = Vec::new();
        let mut scores = Vec::new();

        // Check data completeness
        let completeness = self.check_completeness(workout);
        scores.push(completeness.score);
        if completeness.score < 0.8 {
            issues.push(QualityIssue::LowCompleteness(completeness));
        }

        // Check for outliers
        let outliers = self.detect_outliers(workout);
        scores.push(1.0 - (outliers.count as f64 / workout.data_points.len() as f64));
        if outliers.count > 0 {
            issues.push(QualityIssue::OutliersDetected(outliers));
        }

        // Check temporal consistency
        let temporal = self.check_temporal_consistency(workout);
        scores.push(temporal.score);
        if temporal.gaps > 5 {
            issues.push(QualityIssue::TemporalGaps(temporal));
        }

        // Calculate overall score
        let overall_score = scores.iter().sum::<f64>() / scores.len() as f64;

        QualityReport {
            overall_score,
            issues,
            metrics: QualityMetrics {
                completeness: completeness.score,
                outlier_ratio: outliers.count as f64 / workout.data_points.len() as f64,
                temporal_consistency: temporal.score,
            },
        }
    }
}
```

---

## Troubleshooting Guide

### Issue: Import Fails with CRC Error

**Symptom:**
```
Error: CRC validation failed for FIT file
```

**Causes:**
1. File corrupted during transfer
2. Incomplete file download
3. Device wrote bad CRC

**Solutions:**
```rust
// Try recovery mode
let workout = parse_fit_with_recovery(path)?;

// Or skip CRC (use with caution)
let workout = parse_fit_skip_crc(path)?;
```

### Issue: Memory Usage Too High

**Symptom:** Process consumes excessive memory during import

**Causes:**
1. Loading entire file into memory
2. Unbounded cache
3. Memory leak in data structures

**Solutions:**
```rust
// Use streaming parser
let parser = StreamingFitParser::new(path, 8192)?;

// Implement bounded cache
let mut cache = LruCache::new(100);

// Process in chunks
for chunk in data_points.chunks(1000) {
    process_chunk(chunk)?;
}
```

### Issue: TSS Calculation Inaccurate

**Symptom:** TSS values seem too high or too low

**Causes:**
1. Incorrect FTP/threshold values
2. Missing power data
3. Wrong sport type
4. Precision loss

**Solutions:**
```rust
// Verify athlete thresholds
assert!(athlete.cycling_ftp > 0, "FTP not set");

// Check data quality
let power_coverage = power_data.len() as f64
    / total_points as f64;
if power_coverage < 0.8 {
    log::warn!("Low power data coverage: {:.1}%", power_coverage * 100.0);
}

// Use Decimal for precision
use rust_decimal::Decimal;
let tss = calculate_tss_decimal(workout, athlete)?;
```

### Issue: Parallel Import Crashes

**Symptom:** Program crashes during parallel file import

**Causes:**
1. Shared mutable state
2. Resource exhaustion
3. Thread panic propagation

**Solutions:**
```rust
// Use thread-safe structures
use std::sync::{Arc, Mutex};

let cache = Arc::new(Mutex::new(FitCache::new()));

// Limit parallelism
use rayon::ThreadPoolBuilder;

let pool = ThreadPoolBuilder::new()
    .num_threads(4)
    .build()?;

pool.install(|| {
    files.par_iter()
        .map(|f| import_file(f))
        .collect()
})
```

### Issue: Time Zone Problems

**Symptom:** Workout times displayed incorrectly

**Causes:**
1. Naive datetime handling
2. Missing timezone information
3. DST transitions

**Solutions:**
```rust
use chrono::{DateTime, Utc, Local};

// Always use timezone-aware timestamps
let utc_time: DateTime<Utc> = DateTime::from_timestamp(
    workout.timestamp as i64,
    0
).unwrap();

// Convert to local for display
let local_time = utc_time.with_timezone(&Local);
println!("Workout time: {}", local_time.format("%Y-%m-%d %H:%M:%S %Z"));
```

---

## Summary Checklist

Before deploying FIT processing code, verify:

**Performance:**
- [ ] Using streaming for large files
- [ ] Parallel processing enabled for batch imports
- [ ] Database operations batched
- [ ] Caching implemented with size limits
- [ ] Memory usage monitored and bounded

**Data Quality:**
- [ ] Input validation enabled
- [ ] Outlier detection implemented
- [ ] Missing data handled gracefully
- [ ] Sport-specific validation applied
- [ ] Privacy/sanitization for exports

**Error Handling:**
- [ ] Graceful degradation on errors
- [ ] User-friendly error messages
- [ ] Structured logging in place
- [ ] Recovery strategies implemented
- [ ] Error metrics tracked

**Code Quality:**
- [ ] No anti-patterns present
- [ ] Separation of concerns maintained
- [ ] Type safety enforced
- [ ] Tests cover edge cases
- [ ] Documentation complete

---

## Additional Resources

- [FIT SDK Documentation](https://developer.garmin.com/fit/overview/)
- [TrainRS API Documentation](https://docs.rs/trainrs)
- [Sports Science Guide](./sports-science.md)
- [Data Formats Reference](./data-formats.md)
- [Troubleshooting Guide](./troubleshooting.md)

## Contributing

Found a pattern we missed? Have a better approach? Please contribute:

1. Open an issue describing the pattern
2. Provide code examples
3. Explain the benefits
4. Submit a pull request

---

*Last updated: 2024-10-01*
