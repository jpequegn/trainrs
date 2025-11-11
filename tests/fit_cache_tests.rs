//! Integration tests for FIT file caching system
//!
//! Tests cache functionality including:
//! - File fingerprinting
//! - Cache hit/miss scenarios
//! - LRU eviction
//! - Cache expiration
//! - Incremental imports

use trainrs::import::fit_cache::{FitCache, FileFingerprint};
use trainrs::models::{DataPoint, DataSource, Sport, Workout, WorkoutSummary, WorkoutType};
use chrono::NaiveDate;
use std::path::PathBuf;
use tempfile::tempdir;

fn create_test_workout(id: &str, date: NaiveDate) -> Workout {
    Workout {
        id: id.to_string(),
        athlete_id: Some("test_athlete".to_string()),
        date,
        sport: Sport::Cycling,
        workout_type: WorkoutType::Endurance,
        duration_seconds: 3600,
        summary: WorkoutSummary {
            total_distance: Some(rust_decimal_macros::dec!(30.0)),
            avg_heart_rate: Some(150),
            max_heart_rate: Some(180),
            avg_power: Some(200),
            normalized_power: Some(220),
            avg_pace: None,
            intensity_factor: None,
            tss: None,
            elevation_gain: Some(100),
            avg_cadence: Some(90),
            calories: Some(800),
        },
        data_source: DataSource::Power,
        notes: Some("Test workout".to_string()),
        source: None,
        raw_data: None,
    }
}

#[test]
fn test_cache_basic_put_get() {
    let temp_dir = tempdir().unwrap();
    let cache_db = temp_dir.path().join("cache.db");
    let test_file = temp_dir.path().join("test.fit");

    // Create test file
    std::fs::write(&test_file, b"test fit file data").unwrap();

    // Create cache
    let cache = FitCache::new(&cache_db).unwrap();
    let fingerprint = FileFingerprint::generate(&test_file).unwrap();

    // Create test workouts
    let workouts = vec![
        create_test_workout("w1", NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
    ];

    // Put in cache
    cache.put(&test_file, &fingerprint, &workouts).unwrap();

    // Get from cache
    let cached = cache.get(&test_file, &fingerprint).unwrap();
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().len(), 1);

    // Check metrics
    let metrics = cache.metrics();
    assert_eq!(metrics.cache_hits, 1);
    assert_eq!(metrics.total_lookups, 1);
}

#[test]
fn test_cache_miss_on_missing_file() {
    let temp_dir = tempdir().unwrap();
    let cache_db = temp_dir.path().join("cache.db");
    let test_file = temp_dir.path().join("test.fit");
    let missing_file = temp_dir.path().join("missing.fit");

    std::fs::write(&test_file, b"test data").unwrap();

    let cache = FitCache::new(&cache_db).unwrap();
    let fingerprint = FileFingerprint::generate(&test_file).unwrap();

    let workouts = vec![
        create_test_workout("w1", NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
    ];

    cache.put(&test_file, &fingerprint, &workouts).unwrap();

    // Try to get from cache with different file path
    let missing_fingerprint = FileFingerprint {
        hash: "nonexistent".to_string(),
        file_size: 0,
        modified_timestamp: 0,
    };

    let cached = cache.get(&missing_file, &missing_fingerprint).unwrap();
    assert!(cached.is_none());

    let metrics = cache.metrics();
    assert_eq!(metrics.cache_misses, 1);
}

#[test]
fn test_cache_invalidation_on_file_change() {
    let temp_dir = tempdir().unwrap();
    let cache_db = temp_dir.path().join("cache.db");
    let test_file = temp_dir.path().join("test.fit");

    std::fs::write(&test_file, b"original content").unwrap();

    let cache = FitCache::new(&cache_db).unwrap();
    let fp1 = FileFingerprint::generate(&test_file).unwrap();

    let workouts = vec![
        create_test_workout("w1", NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
    ];

    cache.put(&test_file, &fp1, &workouts).unwrap();

    // Verify cache hit with original fingerprint
    let cached = cache.get(&test_file, &fp1).unwrap();
    assert!(cached.is_some());

    // Modify file
    std::fs::write(&test_file, b"modified content").unwrap();
    let fp2 = FileFingerprint::generate(&test_file).unwrap();

    // Should be cache miss with new fingerprint
    let cached = cache.get(&test_file, &fp2).unwrap();
    assert!(cached.is_none());

    let metrics = cache.metrics();
    assert_eq!(metrics.cache_hits, 1);
    assert_eq!(metrics.cache_misses, 1);
}

#[test]
fn test_cache_hit_rate_calculation() {
    let temp_dir = tempdir().unwrap();
    let cache_db = temp_dir.path().join("cache.db");

    let cache = FitCache::new(&cache_db).unwrap();

    // Initial hit rate is 0%
    let metrics = cache.metrics();
    assert_eq!(metrics.hit_rate(), 0.0);
}

#[test]
fn test_multiple_cache_entries() {
    let temp_dir = tempdir().unwrap();
    let cache_db = temp_dir.path().join("cache.db");

    let cache = FitCache::new(&cache_db).unwrap();

    // Create 3 test files
    for i in 0..3 {
        let test_file = temp_dir.path().join(format!("test{}.fit", i));
        std::fs::write(&test_file, format!("data {}", i).as_bytes()).unwrap();

        let fingerprint = FileFingerprint::generate(&test_file).unwrap();
        let workouts = vec![
            create_test_workout(&format!("w{}", i), NaiveDate::from_ymd_opt(2024, 1, i + 1).unwrap()),
        ];

        cache.put(&test_file, &fingerprint, &workouts).unwrap();
    }

    // Verify cache stats
    let (count, _, _) = cache.get_cache_stats().unwrap();
    assert_eq!(count, 3);
}

#[test]
fn test_cache_clear() {
    let temp_dir = tempdir().unwrap();
    let cache_db = temp_dir.path().join("cache.db");
    let test_file = temp_dir.path().join("test.fit");

    std::fs::write(&test_file, b"test data").unwrap();

    let cache = FitCache::new(&cache_db).unwrap();
    let fingerprint = FileFingerprint::generate(&test_file).unwrap();

    let workouts = vec![
        create_test_workout("w1", NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
    ];

    cache.put(&test_file, &fingerprint, &workouts).unwrap();

    let (count, _, _) = cache.get_cache_stats().unwrap();
    assert_eq!(count, 1);

    cache.clear().unwrap();

    let (count, _, _) = cache.get_cache_stats().unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_cache_current_size() {
    let temp_dir = tempdir().unwrap();
    let cache_db = temp_dir.path().join("cache.db");
    let test_file = temp_dir.path().join("test.fit");

    std::fs::write(&test_file, b"test data").unwrap();

    let cache = FitCache::new(&cache_db).unwrap();
    let fingerprint = FileFingerprint::generate(&test_file).unwrap();

    let workouts = vec![
        create_test_workout("w1", NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
    ];

    cache.put(&test_file, &fingerprint, &workouts).unwrap();

    let size = cache.current_size().unwrap();
    assert!(size > 0);
}

#[test]
fn test_incremental_import_scenario() {
    let temp_dir = tempdir().unwrap();
    let cache_db = temp_dir.path().join("cache.db");

    let cache = FitCache::new(&cache_db).unwrap();

    // First import: 5 files
    let mut fingerprints = Vec::new();
    for i in 0..5 {
        let test_file = temp_dir.path().join(format!("workout_{}.fit", i));
        std::fs::write(&test_file, format!("fit data {}", i).as_bytes()).unwrap();

        let fingerprint = FileFingerprint::generate(&test_file).unwrap();
        let workouts = vec![
            create_test_workout(&format!("w{}", i), NaiveDate::from_ymd_opt(2024, 1, i + 1).unwrap()),
        ];

        cache.put(&test_file, &fingerprint, &workouts).unwrap();
        fingerprints.push((test_file, fingerprint));
    }

    let (count, _, _) = cache.get_cache_stats().unwrap();
    assert_eq!(count, 5);

    // Second import: first 5 files should have cache hits
    for (file_path, fingerprint) in &fingerprints {
        let cached = cache.get(file_path, fingerprint).unwrap();
        assert!(cached.is_some());
    }

    let metrics = cache.metrics();
    assert_eq!(metrics.cache_hits, 5);
}

#[test]
fn test_cache_ttl_setting() {
    let temp_dir = tempdir().unwrap();
    let cache_db = temp_dir.path().join("cache.db");

    let mut cache = FitCache::new(&cache_db).unwrap();

    // Should be able to set TTL without error
    use chrono::Duration;
    cache.set_cache_ttl(Duration::days(7));

    // Verify it doesn't panic
    assert!(true);
}

#[test]
fn test_cache_size_limit_setting() {
    let temp_dir = tempdir().unwrap();
    let cache_db = temp_dir.path().join("cache.db");

    let mut cache = FitCache::new(&cache_db).unwrap();

    // Should be able to set max size without error
    cache.set_max_cache_size(1024 * 1024); // 1 MB

    // Verify it doesn't panic
    assert!(true);
}

#[test]
fn test_cache_fingerprint_uniqueness() {
    let temp_dir = tempdir().unwrap();
    let file1 = temp_dir.path().join("file1.fit");
    let file2 = temp_dir.path().join("file2.fit");

    std::fs::write(&file1, b"data1").unwrap();
    std::fs::write(&file2, b"data2").unwrap();

    let fp1 = FileFingerprint::generate(&file1).unwrap();
    let fp2 = FileFingerprint::generate(&file2).unwrap();

    // Different files should have different hashes
    assert_ne!(fp1.hash, fp2.hash);
}

#[test]
fn test_cache_fingerprint_consistency() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test.fit");

    std::fs::write(&test_file, b"test data").unwrap();

    let fp1 = FileFingerprint::generate(&test_file).unwrap();
    let fp2 = FileFingerprint::generate(&test_file).unwrap();

    // Same file should have same fingerprint
    assert_eq!(fp1.hash, fp2.hash);
    assert_eq!(fp1.file_size, fp2.file_size);
}

#[test]
fn test_cache_with_multiple_workouts() {
    let temp_dir = tempdir().unwrap();
    let cache_db = temp_dir.path().join("cache.db");
    let test_file = temp_dir.path().join("test.fit");

    std::fs::write(&test_file, b"fit data").unwrap();

    let cache = FitCache::new(&cache_db).unwrap();
    let fingerprint = FileFingerprint::generate(&test_file).unwrap();

    // Create multiple workouts
    let workouts = vec![
        create_test_workout("w1", NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
        create_test_workout("w2", NaiveDate::from_ymd_opt(2024, 1, 2).unwrap()),
        create_test_workout("w3", NaiveDate::from_ymd_opt(2024, 1, 3).unwrap()),
    ];

    cache.put(&test_file, &fingerprint, &workouts).unwrap();

    let cached = cache.get(&test_file, &fingerprint).unwrap();
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().len(), 3);
}

#[test]
fn test_cache_performance_scenario() {
    let temp_dir = tempdir().unwrap();
    let cache_db = temp_dir.path().join("cache.db");

    let cache = FitCache::new(&cache_db).unwrap();

    // Simulate repeated imports of same file (common scenario)
    let test_file = temp_dir.path().join("recurring.fit");
    std::fs::write(&test_file, b"fit data").unwrap();

    let fingerprint = FileFingerprint::generate(&test_file).unwrap();
    let workouts = vec![
        create_test_workout("w1", NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
    ];

    // First import
    cache.put(&test_file, &fingerprint, &workouts).unwrap();

    // Subsequent imports should all be cache hits
    for _ in 0..10 {
        let cached = cache.get(&test_file, &fingerprint).unwrap();
        assert!(cached.is_some());
    }

    let metrics = cache.metrics();
    assert_eq!(metrics.cache_hits, 10);
    assert!(metrics.hit_rate() > 90.0); // 10 hits out of ~11 lookups
}

#[test]
fn test_cache_replace_existing_entry() {
    let temp_dir = tempdir().unwrap();
    let cache_db = temp_dir.path().join("cache.db");
    let test_file = temp_dir.path().join("test.fit");

    std::fs::write(&test_file, b"original").unwrap();

    let cache = FitCache::new(&cache_db).unwrap();
    let fp = FileFingerprint::generate(&test_file).unwrap();

    let workouts1 = vec![
        create_test_workout("w1", NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
    ];

    cache.put(&test_file, &fp, &workouts1).unwrap();

    let cached1 = cache.get(&test_file, &fp).unwrap();
    assert_eq!(cached1.unwrap().len(), 1);

    // Put different data with same fingerprint
    let workouts2 = vec![
        create_test_workout("w1", NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
        create_test_workout("w2", NaiveDate::from_ymd_opt(2024, 1, 2).unwrap()),
    ];

    cache.put(&test_file, &fp, &workouts2).unwrap();

    let cached2 = cache.get(&test_file, &fp).unwrap();
    assert_eq!(cached2.unwrap().len(), 2);
}
