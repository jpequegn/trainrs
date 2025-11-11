//! Intelligent caching system for parsed FIT files
//!
//! Provides file fingerprinting, parsed data caching, and LRU eviction to avoid
//! re-processing unchanged files. Uses SHA256 hashing for file integrity and
//! SQLite for persistent cache storage.
//!
//! Features:
//! - SHA256-based file fingerprinting
//! - SQLite-backed persistent cache
//! - LRU cache eviction policy
//! - Timestamp-based cache validation
//! - Cache hit/miss metrics
//! - Incremental import support

use crate::models::Workout;
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::fs::File;
use std::io::Read;
use tracing::{debug, info, warn};

/// File fingerprint using SHA256 hash
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFingerprint {
    pub hash: String,
    pub file_size: u64,
    pub modified_timestamp: i64,
}

impl FileFingerprint {
    /// Generate fingerprint for a file
    pub fn generate(file_path: &Path) -> Result<Self> {
        let metadata = std::fs::metadata(file_path)
            .with_context(|| format!("Failed to read file metadata: {}", file_path.display()))?;

        let file_size = metadata.len();
        let modified_timestamp = metadata
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        // Read file and compute SHA256
        let mut file = File::open(file_path)
            .with_context(|| format!("Failed to open file for hashing: {}", file_path.display()))?;

        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let hash = format!("{:x}", hasher.finalize());

        Ok(Self {
            hash,
            file_size,
            modified_timestamp,
        })
    }
}

/// Cache entry metadata
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub file_path: String,
    pub file_fingerprint: String,
    pub cached_at: DateTime<Utc>,
    pub accessed_at: DateTime<Utc>,
    pub workout_count: usize,
    pub serialized_data: Vec<u8>,
}

/// Cache statistics and metrics
#[derive(Debug, Clone, Default)]
pub struct CacheMetrics {
    pub total_lookups: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub evictions: u64,
    pub total_bytes_cached: u64,
}

impl CacheMetrics {
    /// Get hit rate as percentage
    pub fn hit_rate(&self) -> f64 {
        if self.total_lookups == 0 {
            return 0.0;
        }
        (self.cache_hits as f64 / self.total_lookups as f64) * 100.0
    }

    /// Get average cache size
    pub fn avg_cached_size(&self) -> f64 {
        if self.cache_hits + self.evictions == 0 {
            return 0.0;
        }
        self.total_bytes_cached as f64 / (self.cache_hits + self.evictions) as f64
    }
}

/// FIT file cache manager with LRU eviction
pub struct FitCache {
    db_connection: Connection,
    max_cache_size: usize,
    cache_ttl: Duration,
    metrics: std::sync::Mutex<CacheMetrics>,
}

impl FitCache {
    /// Create or open a cache database
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open cache database at {:?}", db_path))?;

        // Initialize schema
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS fit_cache (
                id INTEGER PRIMARY KEY,
                file_path TEXT NOT NULL UNIQUE,
                file_fingerprint TEXT NOT NULL,
                cached_at TIMESTAMP NOT NULL,
                accessed_at TIMESTAMP NOT NULL,
                workout_count INTEGER NOT NULL,
                serialized_data BLOB NOT NULL,
                data_size INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_file_path ON fit_cache(file_path);
            CREATE INDEX IF NOT EXISTS idx_accessed_at ON fit_cache(accessed_at);
            CREATE INDEX IF NOT EXISTS idx_fingerprint ON fit_cache(file_fingerprint);

            CREATE TABLE IF NOT EXISTS cache_stats (
                id INTEGER PRIMARY KEY,
                total_lookups INTEGER NOT NULL DEFAULT 0,
                cache_hits INTEGER NOT NULL DEFAULT 0,
                cache_misses INTEGER NOT NULL DEFAULT 0,
                evictions INTEGER NOT NULL DEFAULT 0,
                total_bytes_cached INTEGER NOT NULL DEFAULT 0
            );"
        )?;

        // Initialize stats row if not exists
        conn.execute(
            "INSERT OR IGNORE INTO cache_stats (id, total_lookups, cache_hits, cache_misses, evictions, total_bytes_cached)
             VALUES (1, 0, 0, 0, 0, 0)",
            [],
        )?;

        Ok(Self {
            db_connection: conn,
            max_cache_size: 500 * 1024 * 1024, // 500 MB default
            cache_ttl: Duration::days(30),
            metrics: std::sync::Mutex::new(CacheMetrics::default()),
        })
    }

    /// Set maximum cache size in bytes
    pub fn set_max_cache_size(&mut self, size: usize) {
        self.max_cache_size = size;
    }

    /// Set cache TTL
    pub fn set_cache_ttl(&mut self, ttl: Duration) {
        self.cache_ttl = ttl;
    }

    /// Try to retrieve cached workout data
    pub fn get(&self, file_path: &Path, current_fingerprint: &FileFingerprint) -> Result<Option<Vec<Workout>>> {
        let file_path_str = file_path.to_string_lossy().to_string();

        // Update lookup metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.total_lookups += 1;
        }

        // Query cache
        let result = self.db_connection.query_row(
            "SELECT serialized_data, file_fingerprint, accessed_at FROM fit_cache WHERE file_path = ?1",
            params![&file_path_str],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        ).optional()?;

        match result {
            Some((serialized_data, cached_fingerprint, last_accessed)) => {
                // Check if fingerprint matches (file hasn't changed)
                if cached_fingerprint == current_fingerprint.hash {
                    // Check if cache is still valid (not expired)
                    let accessed_at = DateTime::<Utc>::from_timestamp(last_accessed, 0)
                        .ok_or_else(|| anyhow::anyhow!("Invalid timestamp"))?;

                    if Utc::now().signed_duration_since(accessed_at) <= self.cache_ttl {
                        // Update access time
                        self.db_connection.execute(
                            "UPDATE fit_cache SET accessed_at = ?1 WHERE file_path = ?2",
                            params![Utc::now().timestamp(), &file_path_str],
                        )?;

                        // Deserialize data using serde_json
                        let workouts: Vec<Workout> = serde_json::from_slice(&serialized_data)
                            .context("Failed to deserialize cached workout data")?;

                        debug!("Cache hit for {}", file_path.display());
                        let mut metrics = self.metrics.lock().unwrap();
                        metrics.cache_hits += 1;

                        return Ok(Some(workouts));
                    } else {
                        debug!("Cache expired for {}", file_path.display());
                        // Remove expired entry
                        self.db_connection.execute(
                            "DELETE FROM fit_cache WHERE file_path = ?1",
                            params![&file_path_str],
                        )?;
                    }
                } else {
                    debug!("Fingerprint mismatch for {} - file has changed", file_path.display());
                    // Remove stale entry
                    self.db_connection.execute(
                        "DELETE FROM fit_cache WHERE file_path = ?1",
                        params![&file_path_str],
                    )?;
                }
            }
            None => {
                debug!("Cache miss for {}", file_path.display());
            }
        }

        let mut metrics = self.metrics.lock().unwrap();
        metrics.cache_misses += 1;
        Ok(None)
    }

    /// Store workout data in cache
    pub fn put(&self, file_path: &Path, fingerprint: &FileFingerprint, workouts: &[Workout]) -> Result<()> {
        let file_path_str = file_path.to_string_lossy().to_string();

        // Serialize workouts using serde_json
        let serialized = serde_json::to_vec(workouts)
            .context("Failed to serialize workouts for caching")?;
        let data_size = serialized.len();

        // Check if we need to evict entries
        self.evict_if_needed(data_size)?;

        // Insert or replace cache entry
        let now = Utc::now();
        self.db_connection.execute(
            "INSERT OR REPLACE INTO fit_cache (file_path, file_fingerprint, cached_at, accessed_at, workout_count, serialized_data, data_size)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &file_path_str,
                &fingerprint.hash,
                now.timestamp(),
                now.timestamp(),
                workouts.len(),
                serialized,
                data_size
            ],
        )?;

        // Update metrics
        let mut metrics = self.metrics.lock().unwrap();
        metrics.total_bytes_cached = metrics.total_bytes_cached.saturating_add(data_size as u64);

        debug!("Cached {} workouts from {}", workouts.len(), file_path.display());
        Ok(())
    }

    /// Evict entries using LRU policy if cache size exceeds limit
    fn evict_if_needed(&self, new_entry_size: usize) -> Result<()> {
        let current_size: u64 = self.db_connection.query_row(
            "SELECT COALESCE(SUM(data_size), 0) FROM fit_cache",
            [],
            |row| row.get(0),
        )?;

        let new_total = current_size as usize + new_entry_size;

        if new_total > self.max_cache_size {
            let bytes_to_free = new_total - self.max_cache_size;
            info!("Cache size exceeded ({} bytes), evicting entries", bytes_to_free);

            // Get least recently accessed entries
            let mut stmt = self.db_connection.prepare(
                "SELECT file_path, data_size FROM fit_cache ORDER BY accessed_at ASC"
            )?;

            let entries_to_remove: Vec<(String, usize)> = stmt.query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            let mut freed = 0;
            for (file_path, size) in entries_to_remove {
                if freed >= bytes_to_free {
                    break;
                }

                self.db_connection.execute(
                    "DELETE FROM fit_cache WHERE file_path = ?1",
                    params![&file_path],
                )?;

                freed += size;
                let mut metrics = self.metrics.lock().unwrap();
                metrics.evictions += 1;

                debug!("Evicted cache entry: {}", file_path);
            }
        }

        Ok(())
    }

    /// Clear entire cache
    pub fn clear(&self) -> Result<()> {
        self.db_connection.execute("DELETE FROM fit_cache", [])?;
        info!("Cache cleared");
        Ok(())
    }

    /// Get cache metrics
    pub fn metrics(&self) -> CacheMetrics {
        self.metrics.lock().unwrap().clone()
    }

    /// Get current cache size in bytes
    pub fn current_size(&self) -> Result<u64> {
        let size: u64 = self.db_connection.query_row(
            "SELECT COALESCE(SUM(data_size), 0) FROM fit_cache",
            [],
            |row| row.get(0),
        )?;
        Ok(size)
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> Result<(usize, u64, f64)> {
        let count: usize = self.db_connection.query_row(
            "SELECT COUNT(*) FROM fit_cache",
            [],
            |row| row.get(0),
        )?;

        let size = self.current_size()?;
        let metrics = self.metrics();

        Ok((count, size, metrics.hit_rate()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_fingerprint_generation() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.fit");
        std::fs::write(&test_file, b"test data").unwrap();

        let fp = FileFingerprint::generate(&test_file).unwrap();
        assert!(!fp.hash.is_empty());
        assert_eq!(fp.file_size, 9);
    }

    #[test]
    fn test_cache_creation() {
        let temp_dir = tempdir().unwrap();
        let cache_db = temp_dir.path().join("cache.db");

        let cache = FitCache::new(&cache_db).unwrap();
        assert!(cache_db.exists());
    }

    #[test]
    fn test_cache_put_and_get() {
        let temp_dir = tempdir().unwrap();
        let cache_db = temp_dir.path().join("cache.db");
        let test_file = temp_dir.path().join("test.fit");
        std::fs::write(&test_file, b"test data").unwrap();

        let cache = FitCache::new(&cache_db).unwrap();
        let fp = FileFingerprint::generate(&test_file).unwrap();

        // Create test workouts
        use crate::models::{Sport, WorkoutType, WorkoutSummary};
        use chrono::NaiveDate;

        let workouts = vec![
            Workout {
                id: "test1".to_string(),
                athlete_id: None,
                date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                sport: Sport::Cycling,
                workout_type: WorkoutType::Endurance,
                duration_seconds: 3600,
                summary: WorkoutSummary::default(),
                data_source: crate::models::DataSource::Power,
                notes: None,
                source: None,
                raw_data: None,
            }
        ];

        // Put in cache
        cache.put(&test_file, &fp, &workouts).unwrap();

        // Get from cache
        let cached = cache.get(&test_file, &fp).unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 1);

        // Check metrics
        let metrics = cache.metrics();
        assert_eq!(metrics.cache_hits, 1);
    }

    #[test]
    fn test_cache_miss_on_fingerprint_change() {
        let temp_dir = tempdir().unwrap();
        let cache_db = temp_dir.path().join("cache.db");
        let test_file = temp_dir.path().join("test.fit");
        std::fs::write(&test_file, b"test data").unwrap();

        let cache = FitCache::new(&cache_db).unwrap();
        let fp1 = FileFingerprint::generate(&test_file).unwrap();

        use crate::models::{Sport, WorkoutType, WorkoutSummary};
        use chrono::NaiveDate;

        let workouts = vec![
            Workout {
                id: "test1".to_string(),
                athlete_id: None,
                date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                sport: Sport::Cycling,
                workout_type: WorkoutType::Endurance,
                duration_seconds: 3600,
                summary: WorkoutSummary::default(),
                data_source: crate::models::DataSource::Power,
                notes: None,
                source: None,
                raw_data: None,
            }
        ];

        cache.put(&test_file, &fp1, &workouts).unwrap();

        // Modify file
        std::fs::write(&test_file, b"modified data").unwrap();
        let fp2 = FileFingerprint::generate(&test_file).unwrap();

        // Should get cache miss due to fingerprint change
        let cached = cache.get(&test_file, &fp2).unwrap();
        assert!(cached.is_none());

        let metrics = cache.metrics();
        assert_eq!(metrics.cache_misses, 1);
    }

    #[test]
    fn test_cache_metrics() {
        let temp_dir = tempdir().unwrap();
        let cache_db = temp_dir.path().join("cache.db");

        let cache = FitCache::new(&cache_db).unwrap();
        let metrics = cache.metrics();

        assert_eq!(metrics.hit_rate(), 0.0);
        assert_eq!(metrics.cache_hits, 0);
        assert_eq!(metrics.cache_misses, 0);
    }

    #[test]
    fn test_cache_clear() {
        let temp_dir = tempdir().unwrap();
        let cache_db = temp_dir.path().join("cache.db");
        let test_file = temp_dir.path().join("test.fit");
        std::fs::write(&test_file, b"test data").unwrap();

        let cache = FitCache::new(&cache_db).unwrap();
        let fp = FileFingerprint::generate(&test_file).unwrap();

        use crate::models::{Sport, WorkoutType, WorkoutSummary};
        use chrono::NaiveDate;

        let workouts = vec![
            Workout {
                id: "test1".to_string(),
                athlete_id: None,
                date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                sport: Sport::Cycling,
                workout_type: WorkoutType::Endurance,
                duration_seconds: 3600,
                summary: WorkoutSummary::default(),
                data_source: crate::models::DataSource::Power,
                notes: None,
                source: None,
                raw_data: None,
            }
        ];

        cache.put(&test_file, &fp, &workouts).unwrap();
        cache.clear().unwrap();

        let (count, _, _) = cache.get_cache_stats().unwrap();
        assert_eq!(count, 0);
    }
}
