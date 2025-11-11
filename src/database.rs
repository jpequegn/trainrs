use chrono::{DateTime, NaiveDate, Utc};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use rusqlite::{params, Connection, OptionalExtension, Row};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;
use uuid::Uuid;

use crate::models::{DataPoint, Sport, Workout, WorkoutSummary, WorkoutType, DataSource};
use crate::recovery::{
    HrvMeasurement, HrvStatus, SleepSession, SleepMetrics, SleepStageSegment, SleepStage,
    BodyBatteryData, PhysiologicalMetrics, RecoveryMetrics, RecoveryQuality,
};

/// Database error types
#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("SQLite error: {0}")]
    SqliteError(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Compression error: {0}")]
    CompressionError(#[from] std::io::Error),
    #[error("Data not found: {0}")]
    NotFound(String),
    #[error("Duplicate entry: {0}")]
    Duplicate(String),
    #[error("Integrity check failed: {0}")]
    IntegrityError(String),
}

/// Compressed time-series data for efficient storage
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedTimeSeriesData {
    pub compressed_data: Vec<u8>,
    pub original_size: usize,
    pub point_count: usize,
}

impl CompressedTimeSeriesData {
    /// Compress a vector of data points
    pub fn compress(data_points: &[DataPoint]) -> Result<Self, DatabaseError> {
        // Serialize the data points using bincode
        let serialized = bincode::serialize(data_points)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

        let original_size = serialized.len();

        // Compress using gzip
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&serialized)?;
        let compressed_data = encoder.finish()?;

        Ok(Self {
            compressed_data,
            original_size,
            point_count: data_points.len(),
        })
    }

    /// Decompress back to data points
    pub fn decompress(&self) -> Result<Vec<DataPoint>, DatabaseError> {
        // Decompress using gzip
        let mut decoder = GzDecoder::new(self.compressed_data.as_slice());
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        // Deserialize using bincode
        let data_points: Vec<DataPoint> = bincode::deserialize(&decompressed)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

        Ok(data_points)
    }

    /// Get compression ratio (original size / compressed size)
    pub fn compression_ratio(&self) -> f64 {
        self.original_size as f64 / self.compressed_data.len() as f64
    }
}

/// Database connection and management
#[allow(dead_code)]
pub struct Database {
    conn: Connection,
    cache: HashMap<String, Vec<u8>>, // Simple in-memory cache for frequently accessed data
}

#[allow(dead_code)]
impl Database {
    /// Create or open a database at the specified path
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, DatabaseError> {
        let conn = Connection::open(db_path)?;
        let mut db = Self {
            conn,
            cache: HashMap::new(),
        };

        // Initialize database schema
        db.init_schema()?;

        Ok(db)
    }

    /// Initialize database schema with tables and indexes
    fn init_schema(&mut self) -> Result<(), DatabaseError> {
        // Enable WAL mode for better concurrent access
        self.conn.execute("PRAGMA journal_mode=WAL", [])?;
        self.conn.execute("PRAGMA synchronous=NORMAL", [])?;
        self.conn.execute("PRAGMA cache_size=10000", [])?;

        // Athletes table
        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS athletes (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                birth_date DATE,
                max_heart_rate INTEGER,
                lactate_threshold_heart_rate INTEGER,
                functional_threshold_power INTEGER,
                threshold_pace_per_km REAL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            [],
        )?;

        // Workouts table (stores summary data and metadata)
        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS workouts (
                id TEXT PRIMARY KEY,
                athlete_id TEXT,
                date DATE NOT NULL,
                sport TEXT NOT NULL,
                duration_seconds INTEGER NOT NULL,
                workout_type TEXT NOT NULL,
                data_source TEXT NOT NULL,

                -- Summary statistics
                avg_heart_rate INTEGER,
                max_heart_rate INTEGER,
                avg_power INTEGER,
                normalized_power INTEGER,
                avg_pace REAL,
                intensity_factor REAL,
                tss REAL,
                total_distance REAL,
                elevation_gain INTEGER,
                avg_cadence INTEGER,
                calories INTEGER,

                -- Metadata
                notes TEXT,
                source TEXT,
                has_time_series BOOLEAN DEFAULT FALSE,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,

                FOREIGN KEY (athlete_id) REFERENCES athletes (id)
            )
            "#,
            [],
        )?;

        // Time series data table (stores compressed time-series data)
        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS time_series_data (
                id TEXT PRIMARY KEY,
                workout_id TEXT NOT NULL,
                compressed_data BLOB NOT NULL,
                original_size INTEGER NOT NULL,
                point_count INTEGER NOT NULL,
                compression_ratio REAL NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

                FOREIGN KEY (workout_id) REFERENCES workouts (id) ON DELETE CASCADE
            )
            "#,
            [],
        )?;

        // Create indexes for fast queries
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_workouts_date ON workouts (date)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_workouts_athlete_date ON workouts (athlete_id, date)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_workouts_sport ON workouts (sport)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_workouts_tss ON workouts (tss) WHERE tss IS NOT NULL",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_time_series_workout ON time_series_data (workout_id)",
            [],
        )?;

        // Recovery metrics table (daily aggregated recovery data)
        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS recovery_metrics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date DATE NOT NULL,
                athlete_id TEXT,

                -- HRV metrics
                hrv_rmssd REAL,
                hrv_status TEXT CHECK(hrv_status IN ('Poor', 'Unbalanced', 'Balanced', 'NoReading')),
                hrv_baseline REAL,
                hrv_score INTEGER CHECK(hrv_score BETWEEN 0 AND 100),

                -- Sleep metrics
                sleep_score INTEGER CHECK(sleep_score BETWEEN 0 AND 100),
                total_sleep_minutes INTEGER,
                deep_sleep_minutes INTEGER,
                light_sleep_minutes INTEGER,
                rem_sleep_minutes INTEGER,
                awake_minutes INTEGER,
                sleep_efficiency REAL CHECK(sleep_efficiency BETWEEN 0 AND 100),

                -- Body Battery
                body_battery_start INTEGER CHECK(body_battery_start BETWEEN 0 AND 100),
                body_battery_end INTEGER CHECK(body_battery_end BETWEEN 0 AND 100),
                body_battery_lowest INTEGER CHECK(body_battery_lowest BETWEEN 0 AND 100),
                body_battery_highest INTEGER CHECK(body_battery_highest BETWEEN 0 AND 100),

                -- Physiological metrics
                resting_hr INTEGER CHECK(resting_hr BETWEEN 30 AND 120),
                respiration_rate REAL CHECK(respiration_rate BETWEEN 5 AND 40),
                stress_score INTEGER CHECK(stress_score BETWEEN 0 AND 100),
                recovery_time_hours INTEGER,

                -- Composite scores
                training_readiness INTEGER CHECK(training_readiness BETWEEN 0 AND 100),
                recovery_quality TEXT CHECK(recovery_quality IN ('Excellent', 'Good', 'Fair', 'Poor')),

                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,

                FOREIGN KEY (athlete_id) REFERENCES athletes(id),
                UNIQUE(date, athlete_id)
            )
            "#,
            [],
        )?;

        // HRV measurements table (multiple daily readings)
        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS hrv_measurements (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                athlete_id TEXT,
                date DATE NOT NULL,
                measurement_time DATETIME NOT NULL,
                rmssd REAL NOT NULL CHECK(rmssd BETWEEN 10 AND 200),
                baseline REAL CHECK(baseline BETWEEN 10 AND 200),
                status TEXT CHECK(status IN ('Poor', 'Unbalanced', 'Balanced', 'NoReading')),
                score INTEGER CHECK(score BETWEEN 0 AND 100),
                context TEXT,
                source TEXT,
                metadata TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

                FOREIGN KEY (athlete_id) REFERENCES athletes(id),
                UNIQUE(athlete_id, measurement_time)
            )
            "#,
            [],
        )?;

        // Sleep sessions table (detailed sleep tracking)
        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS sleep_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                athlete_id TEXT,
                date DATE NOT NULL,
                start_time DATETIME NOT NULL,
                end_time DATETIME NOT NULL,

                -- Sleep stage durations (minutes)
                deep_sleep_minutes INTEGER,
                light_sleep_minutes INTEGER,
                rem_sleep_minutes INTEGER,
                awake_minutes INTEGER,

                -- Sleep metrics
                total_sleep_minutes INTEGER,
                sleep_score INTEGER CHECK(sleep_score BETWEEN 0 AND 100),
                sleep_efficiency REAL CHECK(sleep_efficiency BETWEEN 0 AND 100),
                sleep_onset_minutes INTEGER,
                interruptions INTEGER,

                source TEXT,
                metadata TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

                FOREIGN KEY (athlete_id) REFERENCES athletes(id),
                UNIQUE(athlete_id, start_time)
            )
            "#,
            [],
        )?;

        // Sleep stage segments table (detailed stage-by-stage data)
        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS sleep_stage_segments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL,
                stage TEXT NOT NULL CHECK(stage IN ('Awake', 'Light', 'Deep', 'REM')),
                start_time DATETIME NOT NULL,
                end_time DATETIME NOT NULL,
                duration_minutes INTEGER,

                FOREIGN KEY (session_id) REFERENCES sleep_sessions(id) ON DELETE CASCADE
            )
            "#,
            [],
        )?;

        // Body Battery events table (energy tracking timeline)
        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS body_battery_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                athlete_id TEXT,
                date DATE NOT NULL,
                timestamp DATETIME NOT NULL,
                battery_level INTEGER NOT NULL CHECK(battery_level BETWEEN 0 AND 100),
                drain_rate REAL,
                charge_rate REAL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

                FOREIGN KEY (athlete_id) REFERENCES athletes(id),
                UNIQUE(athlete_id, timestamp)
            )
            "#,
            [],
        )?;

        // Physiological measurements table (resting HR, respiration, stress)
        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS physiological_measurements (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                athlete_id TEXT,
                date DATE NOT NULL,
                measurement_time DATETIME NOT NULL,
                resting_hr INTEGER CHECK(resting_hr BETWEEN 30 AND 120),
                respiration_rate REAL CHECK(respiration_rate BETWEEN 5 AND 40),
                pulse_ox INTEGER CHECK(pulse_ox BETWEEN 70 AND 100),
                stress_score INTEGER CHECK(stress_score BETWEEN 0 AND 100),
                recovery_time_hours INTEGER,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

                FOREIGN KEY (athlete_id) REFERENCES athletes(id),
                UNIQUE(athlete_id, measurement_time)
            )
            "#,
            [],
        )?;

        // Create indexes for recovery tables
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_recovery_metrics_date ON recovery_metrics(date)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_recovery_metrics_athlete_date ON recovery_metrics(athlete_id, date)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_hrv_measurements_date ON hrv_measurements(date)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_hrv_measurements_athlete_time ON hrv_measurements(athlete_id, measurement_time)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sleep_sessions_date ON sleep_sessions(date)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sleep_sessions_athlete_date ON sleep_sessions(athlete_id, date)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_body_battery_date ON body_battery_events(date)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_body_battery_athlete_time ON body_battery_events(athlete_id, timestamp)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_physiological_date ON physiological_measurements(date)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_physiological_athlete_time ON physiological_measurements(athlete_id, measurement_time)",
            [],
        )?;

        // Create duplicate detection view
        self.conn.execute(
            r#"
            CREATE VIEW IF NOT EXISTS duplicate_workouts AS
            SELECT
                w1.id as workout_id,
                w1.athlete_id,
                w1.date,
                w1.duration_seconds,
                w1.sport,
                COUNT(*) as duplicate_count
            FROM workouts w1
            JOIN workouts w2 ON
                w1.athlete_id = w2.athlete_id AND
                w1.date = w2.date AND
                w1.duration_seconds = w2.duration_seconds AND
                w1.sport = w2.sport AND
                w1.id < w2.id  -- Ensure we only get pairs once
            GROUP BY w1.id, w1.athlete_id, w1.date, w1.duration_seconds, w1.sport
            "#,
            [],
        )?;

        Ok(())
    }

    /// Store a workout with optional time-series data
    pub fn store_workout(&mut self, workout: &Workout) -> Result<(), DatabaseError> {
        let tx = self.conn.transaction()?;

        // Check for duplicates
        if Self::is_duplicate_workout(&tx, workout)? {
            return Err(DatabaseError::Duplicate(format!(
                "Duplicate workout found: {} on {}",
                workout.sport.to_string(),
                workout.date
            )));
        }

        // Insert workout summary
        tx.execute(
            r#"
            INSERT OR REPLACE INTO workouts (
                id, athlete_id, date, sport, duration_seconds, workout_type, data_source,
                avg_heart_rate, max_heart_rate, avg_power, normalized_power, avg_pace,
                intensity_factor, tss, total_distance, elevation_gain, avg_cadence, calories,
                notes, source, has_time_series, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, CURRENT_TIMESTAMP
            )
            "#,
            params![
                workout.id,
                workout.athlete_id,
                workout.date.to_string(),
                workout.sport.to_string(),
                workout.duration_seconds,
                workout.workout_type.to_string(),
                workout.data_source.to_string(),
                workout.summary.avg_heart_rate,
                workout.summary.max_heart_rate,
                workout.summary.avg_power,
                workout.summary.normalized_power,
                workout.summary.avg_pace.map(|p| p.to_string()),
                workout.summary.intensity_factor.map(|if_val| if_val.to_string()),
                workout.summary.tss.map(|tss| tss.to_string()),
                workout.summary.total_distance.map(|d| d.to_string()),
                workout.summary.elevation_gain,
                workout.summary.avg_cadence,
                workout.summary.calories,
                workout.notes,
                workout.source,
                workout.raw_data.is_some(),
            ],
        )?;

        // Store compressed time-series data if available
        if let Some(ref raw_data) = workout.raw_data {
            let compressed = CompressedTimeSeriesData::compress(raw_data)?;
            let time_series_id = Uuid::new_v4().to_string();

            tx.execute(
                r#"
                INSERT INTO time_series_data (
                    id, workout_id, compressed_data, original_size, point_count, compression_ratio
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![
                    time_series_id,
                    workout.id,
                    compressed.compressed_data,
                    compressed.original_size,
                    compressed.point_count,
                    compressed.compression_ratio(),
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Check if a workout is a duplicate based on athlete, date, duration, and sport
    fn is_duplicate_workout(tx: &rusqlite::Transaction, workout: &Workout) -> Result<bool, DatabaseError> {
        let date_str = workout.date.to_string();
        let sport_str = workout.sport.to_string();

        let count: i64 = tx.query_row(
            r#"
            SELECT COUNT(*) FROM workouts
            WHERE athlete_id = ?1 AND date = ?2 AND duration_seconds = ?3 AND sport = ?4
            "#,
            params![
                workout.athlete_id,
                date_str,
                workout.duration_seconds,
                sport_str,
            ],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    /// Load a workout by ID (lazy loading - time series loaded separately)
    pub fn load_workout(&mut self, workout_id: &str) -> Result<Option<Workout>, DatabaseError> {
        let workout = self.conn.query_row(
            r#"
            SELECT
                id, athlete_id, date, sport, duration_seconds, workout_type, data_source,
                avg_heart_rate, max_heart_rate, avg_power, normalized_power, avg_pace,
                intensity_factor, tss, total_distance, elevation_gain, avg_cadence, calories,
                notes, source, has_time_series
            FROM workouts
            WHERE id = ?1
            "#,
            params![workout_id],
            |row| self.workout_from_row(row),
        ).optional()?;

        Ok(workout)
    }

    /// Load time-series data for a workout (lazy loading)
    pub fn load_time_series_data(&mut self, workout_id: &str) -> Result<Option<Vec<DataPoint>>, DatabaseError> {
        // Check cache first
        let cache_key = format!("time_series_{}", workout_id);
        if let Some(cached_data) = self.cache.get(&cache_key) {
            let data_points: Vec<DataPoint> = bincode::deserialize(cached_data)
                .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;
            return Ok(Some(data_points));
        }

        // Load from database
        let compressed_data = self.conn.query_row(
            "SELECT compressed_data FROM time_series_data WHERE workout_id = ?1",
            params![workout_id],
            |row| {
                let data: Vec<u8> = row.get(0)?;
                Ok(data)
            },
        ).optional()?;

        if let Some(compressed) = compressed_data {
            let compressed_ts = CompressedTimeSeriesData {
                compressed_data: compressed,
                original_size: 0, // Not needed for decompression
                point_count: 0,   // Not needed for decompression
            };

            let data_points = compressed_ts.decompress()?;

            // Cache the decompressed data
            if let Ok(serialized) = bincode::serialize(&data_points) {
                self.cache.insert(cache_key, serialized);
            }

            Ok(Some(data_points))
        } else {
            Ok(None)
        }
    }

    /// Query workouts with filters (returns without time-series data for performance)
    pub fn query_workouts(&self, filters: WorkoutFilters) -> Result<Vec<Workout>, DatabaseError> {
        // For now, implement a simple version that gets all workouts and filters in memory
        // This avoids complex lifetime issues while still providing functionality
        // TODO: Optimize with proper SQL filtering once lifetime issues are resolved

        let query = if let Some(limit) = filters.limit {
            format!("
                SELECT
                    id, athlete_id, date, sport, duration_seconds, workout_type, data_source,
                    avg_heart_rate, max_heart_rate, avg_power, normalized_power, avg_pace,
                    intensity_factor, tss, total_distance, elevation_gain, avg_cadence, calories,
                    notes, source, has_time_series
                FROM workouts
                ORDER BY date DESC
                LIMIT {}
            ", limit)
        } else {
            String::from("
                SELECT
                    id, athlete_id, date, sport, duration_seconds, workout_type, data_source,
                    avg_heart_rate, max_heart_rate, avg_power, normalized_power, avg_pace,
                    intensity_factor, tss, total_distance, elevation_gain, avg_cadence, calories,
                    notes, source, has_time_series
                FROM workouts
                ORDER BY date DESC
            ")
        };

        let mut stmt = self.conn.prepare(&query)?;
        let workout_iter = stmt.query_map([], |row| self.workout_from_row(row))?;

        let mut workouts = Vec::new();
        for workout_result in workout_iter {
            let workout = workout_result?;

            // Apply filters in memory (not optimal but works for now)
            let mut matches = true;

            if let Some(ref athlete_id) = filters.athlete_id {
                if workout.athlete_id.as_ref() != Some(athlete_id) {
                    matches = false;
                }
            }

            if let Some(start_date) = filters.start_date {
                if workout.date < start_date {
                    matches = false;
                }
            }

            if let Some(end_date) = filters.end_date {
                if workout.date > end_date {
                    matches = false;
                }
            }

            if let Some(ref sport) = filters.sport {
                if &workout.sport != sport {
                    matches = false;
                }
            }

            if matches {
                workouts.push(workout);
            }
        }

        Ok(workouts)
    }

    /// Helper to convert database row to Workout struct
    fn workout_from_row(&self, row: &Row) -> rusqlite::Result<Workout> {
        Ok(Workout {
            id: row.get("id")?,
            date: NaiveDate::parse_from_str(&row.get::<_, String>("date")?, "%Y-%m-%d").unwrap(),
            sport: Sport::from_str(&row.get::<_, String>("sport")?).unwrap(),
            duration_seconds: row.get("duration_seconds")?,
            workout_type: WorkoutType::from_str(&row.get::<_, String>("workout_type")?).unwrap(),
            data_source: DataSource::from_str(&row.get::<_, String>("data_source")?).unwrap(),
            raw_data: None, // Lazy loaded separately
            summary: WorkoutSummary {
                avg_heart_rate: row.get("avg_heart_rate")?,
                max_heart_rate: row.get("max_heart_rate")?,
                avg_power: row.get("avg_power")?,
                normalized_power: row.get("normalized_power")?,
                avg_pace: row.get::<_, Option<String>>("avg_pace")?
                    .and_then(|s| s.parse::<Decimal>().ok()),
                intensity_factor: row.get::<_, Option<String>>("intensity_factor")?
                    .and_then(|s| s.parse::<Decimal>().ok()),
                tss: row.get::<_, Option<String>>("tss")?
                    .and_then(|s| s.parse::<Decimal>().ok()),
                total_distance: row.get::<_, Option<String>>("total_distance")?
                    .and_then(|s| s.parse::<Decimal>().ok()),
                elevation_gain: row.get("elevation_gain")?,
                avg_cadence: row.get("avg_cadence")?,
                calories: row.get("calories")?,
            },
            notes: row.get("notes")?,
            athlete_id: row.get("athlete_id")?,
            source: row.get("source")?,
        })
    }

    /// Get database statistics
    pub fn get_stats(&self) -> Result<DatabaseStats, DatabaseError> {
        let workout_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM workouts",
            [],
            |row| row.get(0),
        )?;

        let athlete_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM athletes",
            [],
            |row| row.get(0),
        )?;

        let time_series_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM time_series_data",
            [],
            |row| row.get(0),
        )?;

        let (total_original_size, total_compressed_size): (i64, i64) = self.conn.query_row(
            "SELECT SUM(original_size), SUM(LENGTH(compressed_data)) FROM time_series_data",
            [],
            |row| Ok((row.get(0).unwrap_or(0), row.get(1).unwrap_or(0))),
        )?;

        let compression_ratio = if total_compressed_size > 0 {
            total_original_size as f64 / total_compressed_size as f64
        } else {
            0.0
        };

        Ok(DatabaseStats {
            workout_count: workout_count as usize,
            athlete_count: athlete_count as usize,
            time_series_count: time_series_count as usize,
            total_original_size: total_original_size as usize,
            total_compressed_size: total_compressed_size as usize,
            compression_ratio,
            cache_entries: self.cache.len(),
        })
    }

    /// Find and return duplicate workouts
    pub fn find_duplicates(&self) -> Result<Vec<DuplicateWorkout>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            "SELECT workout_id, athlete_id, date, duration_seconds, sport, duplicate_count FROM duplicate_workouts"
        )?;

        let duplicate_iter = stmt.query_map([], |row| {
            Ok(DuplicateWorkout {
                workout_id: row.get("workout_id")?,
                athlete_id: row.get("athlete_id")?,
                date: NaiveDate::parse_from_str(&row.get::<_, String>("date")?, "%Y-%m-%d").unwrap(),
                duration_seconds: row.get("duration_seconds")?,
                sport: Sport::from_str(&row.get::<_, String>("sport")?).unwrap(),
                duplicate_count: row.get("duplicate_count")?,
            })
        })?;

        let mut duplicates = Vec::new();
        for duplicate in duplicate_iter {
            duplicates.push(duplicate?);
        }

        Ok(duplicates)
    }

    /// Remove duplicate workouts (keeps the earliest one)
    pub fn remove_duplicates(&mut self) -> Result<usize, DatabaseError> {
        let tx = self.conn.transaction()?;

        // Find duplicate IDs to remove
        let ids_to_remove = {
            let mut stmt = tx.prepare(
                r#"
                SELECT w2.id
                FROM workouts w1
                JOIN workouts w2 ON
                    w1.athlete_id = w2.athlete_id AND
                    w1.date = w2.date AND
                    w1.duration_seconds = w2.duration_seconds AND
                    w1.sport = w2.sport AND
                    w1.created_at < w2.created_at  -- Keep the earlier one
                "#
            )?;

            let ids: Vec<String> = stmt.query_map([], |row| {
                row.get::<_, String>(0)
            })?.collect::<Result<Vec<_>, _>>()?;

            ids
        }; // Statement is dropped here

        let removed_count = ids_to_remove.len();

        // Remove duplicates
        for id in ids_to_remove {
            tx.execute("DELETE FROM workouts WHERE id = ?1", params![id])?;
        }

        tx.commit()?;
        Ok(removed_count)
    }

    /// Clear the in-memory cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    // ============================================================================
    // Recovery Data CRUD Operations
    // ============================================================================

    /// Store or update recovery metrics for a specific date
    pub fn store_recovery_metrics(&mut self, metrics: &RecoveryMetrics, athlete_id: Option<&str>) -> Result<(), DatabaseError> {
        let tx = self.conn.transaction()?;

        // Extract HRV metrics
        let (hrv_rmssd, hrv_status, hrv_baseline, hrv_score) = if let Some(hrv) = &metrics.hrv_metrics {
            (
                hrv.rmssd,
                hrv.status.as_ref().map(|s| format!("{:?}", s)),
                hrv.baseline,
                hrv.score,
            )
        } else {
            (None, None, None, None)
        };

        // Extract sleep metrics
        let (sleep_score, total_sleep, deep_sleep, light_sleep, rem_sleep, awake, sleep_efficiency) =
            if let Some(sleep) = &metrics.sleep_data {
                (
                    sleep.sleep_score.map(|s| s as i64),
                    Some(sleep.total_sleep as i64),
                    Some(sleep.deep_sleep as i64),
                    Some(sleep.light_sleep as i64),
                    Some(sleep.rem_sleep as i64),
                    Some(sleep.awake_time as i64),
                    sleep.sleep_efficiency,
                )
            } else {
                (None, None, None, None, None, None, None)
            };

        // Extract Body Battery metrics
        let (bb_start, bb_end, bb_lowest, bb_highest) = if let Some(bb) = &metrics.body_battery {
            (
                Some(bb.start_level as i64),
                Some(bb.end_level as i64),
                bb.lowest_level.map(|l| l as i64),
                bb.highest_level.map(|l| l as i64),
            )
        } else {
            (None, None, None, None)
        };

        // Extract physiological metrics
        let (resting_hr, resp_rate, stress, recovery_time) = if let Some(phys) = &metrics.physiological {
            (
                phys.resting_hr.map(|h| h as i64),
                phys.respiration_rate,
                phys.stress_score.map(|s| s as i64),
                phys.recovery_time.map(|t| t as i64),
            )
        } else {
            (None, None, None, None)
        };

        // Extract composite scores
        let training_readiness = metrics.training_readiness.map(|r| r as i64);
        let recovery_quality = metrics.recovery_quality.as_ref().map(|q| format!("{:?}", q));

        tx.execute(
            r#"
            INSERT OR REPLACE INTO recovery_metrics (
                date, athlete_id,
                hrv_rmssd, hrv_status, hrv_baseline, hrv_score,
                sleep_score, total_sleep_minutes, deep_sleep_minutes, light_sleep_minutes,
                rem_sleep_minutes, awake_minutes, sleep_efficiency,
                body_battery_start, body_battery_end, body_battery_lowest, body_battery_highest,
                resting_hr, respiration_rate, stress_score, recovery_time_hours,
                training_readiness, recovery_quality,
                updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                ?18, ?19, ?20, ?21, ?22, ?23, CURRENT_TIMESTAMP
            )
            "#,
            params![
                metrics.date.to_string(),
                athlete_id,
                hrv_rmssd,
                hrv_status,
                hrv_baseline,
                hrv_score,
                sleep_score,
                total_sleep,
                deep_sleep,
                light_sleep,
                rem_sleep,
                awake,
                sleep_efficiency,
                bb_start,
                bb_end,
                bb_lowest,
                bb_highest,
                resting_hr,
                resp_rate,
                stress,
                recovery_time,
                training_readiness,
                recovery_quality,
            ],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// Store an HRV measurement
    pub fn store_hrv_measurement(&mut self, measurement: &HrvMeasurement, athlete_id: Option<&str>) -> Result<i64, DatabaseError> {
        let date = measurement.timestamp.date_naive();
        let metadata = measurement.metadata.as_ref().map(|m| serde_json::to_string(m).ok()).flatten();

        self.conn.execute(
            r#"
            INSERT INTO hrv_measurements (
                athlete_id, date, measurement_time, rmssd, baseline, status, score, context, source, metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                athlete_id,
                date.to_string(),
                measurement.timestamp.to_rfc3339(),
                measurement.rmssd,
                measurement.baseline,
                format!("{:?}", measurement.status),
                measurement.score,
                measurement.context,
                measurement.source,
                metadata,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Store a sleep session with stage segments
    pub fn store_sleep_session(&mut self, session: &SleepSession, athlete_id: Option<&str>) -> Result<i64, DatabaseError> {
        let tx = self.conn.transaction()?;

        let date = session.start_time.date_naive();
        let metadata = session.metadata.as_ref().map(|m| serde_json::to_string(m).ok()).flatten();

        // Insert sleep session
        tx.execute(
            r#"
            INSERT INTO sleep_sessions (
                athlete_id, date, start_time, end_time,
                deep_sleep_minutes, light_sleep_minutes, rem_sleep_minutes, awake_minutes,
                total_sleep_minutes, sleep_score, sleep_efficiency, sleep_onset_minutes, interruptions,
                source, metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            "#,
            params![
                athlete_id,
                date.to_string(),
                session.start_time.to_rfc3339(),
                session.end_time.to_rfc3339(),
                session.metrics.deep_sleep as i64,
                session.metrics.light_sleep as i64,
                session.metrics.rem_sleep as i64,
                session.metrics.awake_time as i64,
                session.metrics.total_sleep as i64,
                session.metrics.sleep_score.map(|s| s as i64),
                session.metrics.sleep_efficiency,
                session.metrics.sleep_onset.map(|o| o as i64),
                session.metrics.interruptions.map(|i| i as i64),
                session.source,
                metadata,
            ],
        )?;

        let session_id = tx.last_insert_rowid();

        // Insert sleep stage segments
        for segment in &session.sleep_stages {
            let duration_minutes = (segment.end_time - segment.start_time).num_minutes();
            tx.execute(
                r#"
                INSERT INTO sleep_stage_segments (
                    session_id, stage, start_time, end_time, duration_minutes
                ) VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                params![
                    session_id,
                    format!("{:?}", segment.stage),
                    segment.start_time.to_rfc3339(),
                    segment.end_time.to_rfc3339(),
                    duration_minutes,
                ],
            )?;
        }

        tx.commit()?;
        Ok(session_id)
    }

    /// Store a Body Battery event
    pub fn store_body_battery_event(&mut self, event: &BodyBatteryData, athlete_id: Option<&str>) -> Result<i64, DatabaseError> {
        let date = event.timestamp.date_naive();

        self.conn.execute(
            r#"
            INSERT INTO body_battery_events (
                athlete_id, date, timestamp, battery_level, drain_rate, charge_rate
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                athlete_id,
                date.to_string(),
                event.timestamp.to_rfc3339(),
                event.end_level,
                event.drain_rate,
                event.charge_rate,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Store a physiological measurement
    pub fn store_physiological_measurement(&mut self, measurement: &PhysiologicalMetrics, athlete_id: Option<&str>) -> Result<i64, DatabaseError> {
        let date = measurement.timestamp.date_naive();

        self.conn.execute(
            r#"
            INSERT INTO physiological_measurements (
                athlete_id, date, measurement_time, resting_hr, respiration_rate,
                pulse_ox, stress_score, recovery_time_hours
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                athlete_id,
                date.to_string(),
                measurement.timestamp.to_rfc3339(),
                measurement.resting_hr,
                measurement.respiration_rate,
                measurement.pulse_ox,
                measurement.stress_score,
                measurement.recovery_time,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Get recovery metrics for a specific date range
    pub fn get_recovery_metrics(&self, athlete_id: Option<&str>, start_date: NaiveDate, end_date: NaiveDate) -> Result<Vec<RecoveryMetrics>, DatabaseError> {
        let query = if athlete_id.is_some() {
            "SELECT * FROM recovery_metrics WHERE athlete_id = ?1 AND date BETWEEN ?2 AND ?3 ORDER BY date"
        } else {
            "SELECT * FROM recovery_metrics WHERE date BETWEEN ?1 AND ?2 ORDER BY date"
        };

        let mut stmt = self.conn.prepare(query)?;

        let metrics_iter = if let Some(aid) = athlete_id {
            stmt.query_map(params![aid, start_date.to_string(), end_date.to_string()], Self::row_to_recovery_metrics)?
        } else {
            stmt.query_map(params![start_date.to_string(), end_date.to_string()], Self::row_to_recovery_metrics)?
        };

        let mut metrics = Vec::new();
        for m in metrics_iter {
            metrics.push(m?);
        }

        Ok(metrics)
    }

    /// Helper function to convert database row to RecoveryMetrics
    fn row_to_recovery_metrics(row: &Row) -> rusqlite::Result<RecoveryMetrics> {
        let date_str: String = row.get("date")?;
        let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?;

        Ok(RecoveryMetrics {
            date,
            hrv_metrics: None, // Simplified - would need full reconstruction
            sleep_data: None,
            body_battery: None,
            physiological: None,
            training_readiness: row.get::<_, Option<i64>>("training_readiness")?.map(|r| r as u8),
            recovery_quality: None,
        })
    }

    /// Get 7-day recovery trend
    pub fn get_recovery_trend_7day(&self, athlete_id: Option<&str>, end_date: NaiveDate) -> Result<Vec<RecoveryMetrics>, DatabaseError> {
        let start_date = end_date - chrono::Duration::days(6);
        self.get_recovery_metrics(athlete_id, start_date, end_date)
    }

    /// Get 30-day recovery trend
    pub fn get_recovery_trend_30day(&self, athlete_id: Option<&str>, end_date: NaiveDate) -> Result<Vec<RecoveryMetrics>, DatabaseError> {
        let start_date = end_date - chrono::Duration::days(29);
        self.get_recovery_metrics(athlete_id, start_date, end_date)
    }
}

/// Workout query filters
#[allow(dead_code)]
#[derive(Debug, Default, Clone)]
pub struct WorkoutFilters {
    pub athlete_id: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub sport: Option<Sport>,
    pub limit: Option<usize>,
}

/// Database statistics
#[allow(dead_code)]
#[derive(Debug)]
pub struct DatabaseStats {
    pub workout_count: usize,
    pub athlete_count: usize,
    pub time_series_count: usize,
    pub total_original_size: usize,
    pub total_compressed_size: usize,
    pub compression_ratio: f64,
    pub cache_entries: usize,
}

/// Duplicate workout information
#[allow(dead_code)]
#[derive(Debug)]
pub struct DuplicateWorkout {
    pub workout_id: String,
    pub athlete_id: Option<String>,
    pub date: NaiveDate,
    pub duration_seconds: u32,
    pub sport: Sport,
    pub duplicate_count: i64,
}

// Extend model enums with string conversion for database storage
impl Sport {
    pub fn from_str(s: &str) -> Result<Self, DatabaseError> {
        match s {
            "Running" => Ok(Sport::Running),
            "Cycling" => Ok(Sport::Cycling),
            "Swimming" => Ok(Sport::Swimming),
            "Triathlon" => Ok(Sport::Triathlon),
            "Rowing" => Ok(Sport::Rowing),
            "CrossTraining" => Ok(Sport::CrossTraining),
            _ => Err(DatabaseError::SerializationError(format!("Unknown sport: {}", s))),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Sport::Running => "Running".to_string(),
            Sport::Cycling => "Cycling".to_string(),
            Sport::Swimming => "Swimming".to_string(),
            Sport::Triathlon => "Triathlon".to_string(),
            Sport::Rowing => "Rowing".to_string(),
            Sport::CrossTraining => "CrossTraining".to_string(),
        }
    }
}

impl WorkoutType {
    pub fn from_str(s: &str) -> Result<Self, DatabaseError> {
        match s {
            "Interval" => Ok(WorkoutType::Interval),
            "Endurance" => Ok(WorkoutType::Endurance),
            "Recovery" => Ok(WorkoutType::Recovery),
            "Tempo" => Ok(WorkoutType::Tempo),
            "Threshold" => Ok(WorkoutType::Threshold),
            "VO2Max" => Ok(WorkoutType::VO2Max),
            "Strength" => Ok(WorkoutType::Strength),
            "Race" => Ok(WorkoutType::Race),
            "Test" => Ok(WorkoutType::Test),
            _ => Err(DatabaseError::SerializationError(format!("Unknown workout type: {}", s))),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            WorkoutType::Interval => "Interval".to_string(),
            WorkoutType::Endurance => "Endurance".to_string(),
            WorkoutType::Recovery => "Recovery".to_string(),
            WorkoutType::Tempo => "Tempo".to_string(),
            WorkoutType::Threshold => "Threshold".to_string(),
            WorkoutType::VO2Max => "VO2Max".to_string(),
            WorkoutType::Strength => "Strength".to_string(),
            WorkoutType::Race => "Race".to_string(),
            WorkoutType::Test => "Test".to_string(),
        }
    }
}

impl DataSource {
    pub fn from_str(s: &str) -> Result<Self, DatabaseError> {
        match s {
            "HeartRate" => Ok(DataSource::HeartRate),
            "Power" => Ok(DataSource::Power),
            "Pace" => Ok(DataSource::Pace),
            "Rpe" => Ok(DataSource::Rpe),
            _ => Err(DatabaseError::SerializationError(format!("Unknown data source: {}", s))),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            DataSource::HeartRate => "HeartRate".to_string(),
            DataSource::Power => "Power".to_string(),
            DataSource::Pace => "Pace".to_string(),
            DataSource::Rpe => "Rpe".to_string(),
        }
    }
}