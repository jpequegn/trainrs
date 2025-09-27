use chrono::NaiveDate;
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use rusqlite::{params, Connection, OptionalExtension, Row};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;
use uuid::Uuid;

use crate::models::{DataPoint, Sport, Workout, WorkoutSummary, WorkoutType, DataSource};

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
        if Self::is_duplicate_workout(&tx, &workout)? {
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
            |row| Ok(row.get(0)?),
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
            |row| Ok(row.get(0)?),
        )?;

        let athlete_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM athletes",
            [],
            |row| Ok(row.get(0)?),
        )?;

        let time_series_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM time_series_data",
            [],
            |row| Ok(row.get(0)?),
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
                Ok(row.get::<_, String>(0)?)
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