use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::models::{Sport, Units};
use crate::pmc::PmcConfig;

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Application metadata
    pub metadata: ConfigMetadata,

    /// General application settings
    pub settings: AppSettings,

    /// Performance Management Chart settings
    pub pmc: PmcConfig,

    /// Training zone calculation methods
    pub zones: ZoneSettings,

    /// Data import preferences
    pub import: ImportSettings,

    /// Athletes configuration
    pub athletes: HashMap<String, AthleteConfig>,

    /// Default athlete ID (currently active)
    pub default_athlete_id: Option<String>,
}

/// Configuration metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMetadata {
    /// Configuration format version
    pub version: String,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,
}

/// General application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Data directory path
    pub data_dir: PathBuf,

    /// Default units (metric/imperial)
    pub default_units: Units,

    /// Auto-backup settings
    pub auto_backup: BackupSettings,

    /// Default sport for analysis when not specified
    pub default_sport: Option<Sport>,
}

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSettings {
    /// Enable automatic backups
    pub enabled: bool,

    /// Backup directory path
    pub backup_dir: PathBuf,

    /// Days to retain backups
    pub retention_days: u16,

    /// Backup before major operations
    pub backup_before_cleanup: bool,
}

/// Training zone calculation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneSettings {
    /// Heart rate zone calculation method
    pub hr_zone_method: HRZoneMethod,

    /// Power zone calculation method
    pub power_zone_method: PowerZoneMethod,

    /// Pace zone calculation method
    pub pace_zone_method: PaceZoneMethod,

    /// Custom zone percentages (if using custom method)
    pub custom_hr_zones: Option<Vec<Decimal>>,
    pub custom_power_zones: Option<Vec<Decimal>>,
    pub custom_pace_zones: Option<Vec<Decimal>>,
}

/// Heart rate zone calculation methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HRZoneMethod {
    /// Based on lactate threshold heart rate
    LTHR,
    /// Based on maximum heart rate
    MaxHR,
    /// Custom zone boundaries
    Custom,
}

/// Power zone calculation methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PowerZoneMethod {
    /// Coggan 7-zone model based on FTP
    CogganFTP,
    /// Custom zone boundaries
    Custom,
}

/// Pace zone calculation methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaceZoneMethod {
    /// Based on threshold pace
    ThresholdPace,
    /// Based on race performance
    RacePerformance,
    /// Custom zone boundaries
    Custom,
}

/// Data import preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSettings {
    /// Automatically calculate missing TSS values
    pub auto_calculate_tss: bool,

    /// Default file formats to scan for
    pub supported_formats: Vec<String>,

    /// Import data processing chunk size
    pub chunk_size: usize,

    /// Skip duplicate detection during import
    pub skip_duplicate_detection: bool,

    /// Default timezone for workout data without timezone
    pub default_timezone: String,
}

/// Athlete-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AthleteConfig {
    /// Unique athlete identifier
    pub id: String,

    /// Basic athlete profile information
    pub profile: AthleteProfile,

    /// Primary sport for this athlete
    pub primary_sport: Sport,

    /// Sport-specific thresholds and settings
    pub sport_profiles: HashMap<Sport, SportProfile>,

    /// Historical threshold changes
    pub threshold_history: Vec<ThresholdChange>,

    /// Creation date
    pub created_date: DateTime<Utc>,

    /// Last updated date
    pub last_updated: DateTime<Utc>,

    /// Per-athlete data directory
    pub data_directory: PathBuf,

    /// Legacy data_dir field for compatibility
    pub data_dir: PathBuf,
}

/// Enhanced athlete profile with multi-sport support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AthleteProfile {
    /// Unique athlete identifier
    pub id: String,

    /// Athlete's display name
    pub name: String,

    /// Date of birth for age-based calculations
    pub date_of_birth: Option<NaiveDate>,

    /// Weight in kilograms
    pub weight: Option<Decimal>,

    /// Height in centimeters
    pub height: Option<u16>,

    /// Preferred units (metric/imperial)
    pub preferred_units: Units,

    /// Maximum heart rate (applies across all sports)
    pub max_hr: Option<u16>,

    /// Resting heart rate (applies across all sports)
    pub resting_hr: Option<u16>,

    /// Functional Threshold Power for cycling (watts) - default sport
    pub ftp: Option<u16>,

    /// Lactate Threshold Heart Rate - default sport
    pub lthr: Option<u16>,

    /// Threshold pace for running (minutes per km) - default sport
    pub threshold_pace: Option<Decimal>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Active/inactive status
    pub active: bool,
}

/// Sport-specific athlete profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SportProfile {
    /// Sport this profile applies to
    pub sport: Sport,

    /// Functional Threshold Power for cycling (watts)
    pub ftp: Option<u16>,

    /// Lactate Threshold Heart Rate for this sport
    pub lthr: Option<u16>,

    /// Threshold pace for running/swimming (min/mile or min/km)
    pub threshold_pace: Option<Decimal>,

    /// Threshold swim pace (min/100m or min/100y)
    pub threshold_swim_pace: Option<Decimal>,

    /// Critical Power/Velocity for this sport
    pub critical_power: Option<u16>,

    /// Anaerobic Work Capacity (kJ for cycling, time for running)
    pub awc: Option<Decimal>,

    /// Sport-specific training zones
    pub zones: Option<SportZones>,

    /// Last threshold test date
    pub last_test_date: Option<NaiveDate>,

    /// Maximum heart rate for this sport
    pub max_hr: Option<u16>,

    /// Zone calculation method for this sport
    pub zone_method: Option<String>,

    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,

    /// Notes about this sport profile
    pub notes: Option<String>,
}

/// Sport-specific training zones
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SportZones {
    /// Heart rate zones for this sport
    pub heart_rate: Option<Vec<ZoneBoundary>>,

    /// Power zones (cycling)
    pub power: Option<Vec<ZoneBoundary>>,

    /// Pace zones (running/swimming)
    pub pace: Option<Vec<ZoneBoundary>>,
}

/// Training zone boundary definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneBoundary {
    /// Zone number (1-based)
    pub zone: u8,

    /// Zone name/description
    pub name: String,

    /// Lower bound (inclusive)
    pub min: Decimal,

    /// Upper bound (exclusive, None means no upper limit)
    pub max: Option<Decimal>,

    /// Zone color for visualization
    pub color: Option<String>,
}

/// Historical threshold change record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdChange {
    /// Date of the change
    pub date: NaiveDate,

    /// Sport this change applies to
    pub sport: Sport,

    /// Type of threshold that changed
    pub threshold_type: ThresholdType,

    /// Previous value
    pub old_value: Option<Decimal>,

    /// New value
    pub new_value: Decimal,

    /// Source of the change (test, manual, estimated)
    pub source: ThresholdSource,

    /// Additional notes about the change
    pub notes: Option<String>,
}

/// Types of thresholds that can be tracked
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThresholdType {
    Ftp,
    Lthr,
    ThresholdPace,
    ThresholdSwimPace,
    CriticalPower,
    Awc,
    MaxHr,
}

impl std::fmt::Display for ThresholdType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThresholdType::Ftp => write!(f, "FTP"),
            ThresholdType::Lthr => write!(f, "LTHR"),
            ThresholdType::ThresholdPace => write!(f, "Threshold Pace"),
            ThresholdType::ThresholdSwimPace => write!(f, "Threshold Swim Pace"),
            ThresholdType::CriticalPower => write!(f, "Critical Power"),
            ThresholdType::Awc => write!(f, "AWC"),
            ThresholdType::MaxHr => write!(f, "Max HR"),
        }
    }
}

/// Source of threshold change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThresholdSource {
    /// From a structured threshold test
    Test,
    /// Manually entered by user
    Manual,
    /// Estimated from recent performance
    Estimated,
    /// Imported from external source
    Import,
}

impl Default for AppConfig {
    fn default() -> Self {
        let now = Utc::now();

        AppConfig {
            metadata: ConfigMetadata {
                version: "1.0".to_string(),
                created_at: now,
                updated_at: now,
            },
            settings: AppSettings::default(),
            pmc: PmcConfig::default(),
            zones: ZoneSettings::default(),
            import: ImportSettings::default(),
            athletes: HashMap::new(),
            default_athlete_id: None,
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            data_dir: PathBuf::from("./data"),
            default_units: Units::Metric,
            auto_backup: BackupSettings::default(),
            default_sport: None,
        }
    }
}

impl Default for BackupSettings {
    fn default() -> Self {
        BackupSettings {
            enabled: true,
            backup_dir: PathBuf::from("./backups"),
            retention_days: 30,
            backup_before_cleanup: true,
        }
    }
}

impl Default for ZoneSettings {
    fn default() -> Self {
        ZoneSettings {
            hr_zone_method: HRZoneMethod::LTHR,
            power_zone_method: PowerZoneMethod::CogganFTP,
            pace_zone_method: PaceZoneMethod::ThresholdPace,
            custom_hr_zones: None,
            custom_power_zones: None,
            custom_pace_zones: None,
        }
    }
}

impl Default for ImportSettings {
    fn default() -> Self {
        ImportSettings {
            auto_calculate_tss: true,
            supported_formats: vec![
                "fit".to_string(),
                "tcx".to_string(),
                "gpx".to_string(),
                "json".to_string(),
            ],
            chunk_size: 100,
            skip_duplicate_detection: false,
            default_timezone: "UTC".to_string(),
        }
    }
}

/// Configuration management implementation
impl AppConfig {
    /// Load configuration from TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;

        let config: AppConfig = toml::from_str(&content)
            .with_context(|| "Failed to parse TOML configuration")?;

        Ok(config)
    }

    /// Save configuration to TOML file
    pub fn save_to_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        // Update modification timestamp
        self.metadata.updated_at = Utc::now();

        // Create directory if it doesn't exist
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }

        let toml_content = toml::to_string_pretty(self)
            .with_context(|| "Failed to serialize configuration to TOML")?;

        fs::write(&path, toml_content)
            .with_context(|| format!("Failed to write config file: {}", path.as_ref().display()))?;

        Ok(())
    }

    /// Get default configuration file path
    pub fn default_config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".trainrs")
            .join("config.toml")
    }

    /// Load configuration with fallback to defaults
    pub fn load_or_default() -> Self {
        let config_path = Self::default_config_path();

        match Self::load_from_file(&config_path) {
            Ok(config) => config,
            Err(_) => {
                eprintln!("Config file not found, using defaults: {}", config_path.display());
                Self::default()
            }
        }
    }

    /// Save configuration to default location
    pub fn save_default(&mut self) -> Result<()> {
        let config_path = Self::default_config_path();
        self.save_to_file(config_path)
    }

    /// Save configuration (alias for save_default)
    pub fn save(&mut self) -> Result<()> {
        self.save_default()
    }

    /// Add a new athlete to the configuration
    pub fn add_athlete(&mut self, athlete_config: AthleteConfig) -> Result<()> {
        let athlete_id = athlete_config.id.clone();

        // Set as default if this is the first athlete
        if self.athletes.is_empty() {
            self.default_athlete_id = Some(athlete_id.clone());
        }

        self.athletes.insert(athlete_id, athlete_config);
        self.metadata.updated_at = Utc::now();

        Ok(())
    }

    /// Remove an athlete from the configuration
    pub fn remove_athlete(&mut self, athlete_id: &str) -> Result<()> {
        if !self.athletes.contains_key(athlete_id) {
            return Err(anyhow::anyhow!("Athlete not found: {}", athlete_id));
        }

        self.athletes.remove(athlete_id);

        // Clear default if it was the removed athlete
        if self.default_athlete_id.as_deref() == Some(athlete_id) {
            self.default_athlete_id = self.athletes.keys().next().cloned();
        }

        self.metadata.updated_at = Utc::now();
        Ok(())
    }

    /// Get athlete configuration by ID
    pub fn get_athlete(&self, athlete_id: &str) -> Option<&AthleteConfig> {
        self.athletes.get(athlete_id)
    }

    /// Get mutable athlete configuration by ID
    pub fn get_athlete_mut(&mut self, athlete_id: &str) -> Option<&mut AthleteConfig> {
        self.athletes.get_mut(athlete_id)
    }

    /// Get the default (currently active) athlete
    pub fn get_default_athlete(&self) -> Option<&AthleteConfig> {
        self.default_athlete_id
            .as_ref()
            .and_then(|id| self.athletes.get(id))
    }

    /// Set the default athlete
    pub fn set_default_athlete(&mut self, athlete_id: &str) -> Result<()> {
        if !self.athletes.contains_key(athlete_id) {
            return Err(anyhow::anyhow!("Athlete not found: {}", athlete_id));
        }

        self.default_athlete_id = Some(athlete_id.to_string());
        self.metadata.updated_at = Utc::now();
        Ok(())
    }

    /// List all athletes
    pub fn list_athletes(&self) -> Vec<&AthleteConfig> {
        self.athletes.values().collect()
    }

    /// Get data directory for a specific athlete
    pub fn get_athlete_data_dir(&self, athlete_id: &str) -> Option<PathBuf> {
        self.get_athlete(athlete_id)
            .map(|athlete| athlete.data_dir.clone())
    }
}

impl AthleteConfig {
    /// Create a new athlete configuration
    pub fn new(name: String, athlete_id: Option<String>) -> Self {
        let id = athlete_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let now = Utc::now();

        AthleteConfig {
            id: id.clone(),
            profile: AthleteProfile {
                id: id.clone(),
                name,
                date_of_birth: None,
                weight: None,
                height: None,
                preferred_units: Units::Metric,
                max_hr: None,
                resting_hr: None,
                ftp: None,
                lthr: None,
                threshold_pace: None,
                created_at: now,
                updated_at: now,
                active: true,
            },
            primary_sport: Sport::Cycling, // Default to cycling
            sport_profiles: HashMap::new(),
            threshold_history: Vec::new(),
            created_date: now,
            last_updated: now,
            data_directory: PathBuf::from("data").join(&id),
            data_dir: PathBuf::from("data").join(&id),
        }
    }

    /// Add or update sport-specific profile
    pub fn set_sport_profile(&mut self, sport: Sport, profile: SportProfile) {
        self.sport_profiles.insert(sport, profile);
        self.profile.updated_at = Utc::now();
    }

    /// Get sport-specific profile
    pub fn get_sport_profile(&self, sport: &Sport) -> Option<&SportProfile> {
        self.sport_profiles.get(sport)
    }

    /// Add threshold change to history
    pub fn add_threshold_change(&mut self, change: ThresholdChange) {
        self.threshold_history.push(change);
        // Keep history sorted by date (newest first)
        self.threshold_history.sort_by(|a, b| b.date.cmp(&a.date));
        self.profile.updated_at = Utc::now();
    }

    /// Get threshold history for a specific sport and threshold type
    pub fn get_threshold_history(&self, sport: &Sport, threshold_type: &ThresholdType) -> Vec<&ThresholdChange> {
        self.threshold_history
            .iter()
            .filter(|change| change.sport == *sport && change.threshold_type == *threshold_type)
            .collect()
    }
}

// Add the dirs dependency for getting home directory
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_serialization() {
        let config = AppConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: AppConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(config.metadata.version, deserialized.metadata.version);
        assert_eq!(config.settings.default_units, deserialized.settings.default_units);
    }

    #[test]
    fn test_athlete_management() {
        let mut config = AppConfig::default();
        let athlete = AthleteConfig::new("Test Athlete".to_string(), Some("test-id".to_string()));

        config.add_athlete(athlete).unwrap();

        assert_eq!(config.athletes.len(), 1);
        assert_eq!(config.default_athlete_id, Some("test-id".to_string()));

        let retrieved = config.get_athlete("test-id").unwrap();
        assert_eq!(retrieved.profile.name, "Test Athlete");
    }

    #[test]
    fn test_config_file_io() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        let mut original_config = AppConfig::default();
        let athlete = AthleteConfig::new("Test Athlete".to_string(), None);
        original_config.add_athlete(athlete).unwrap();

        // Save and reload
        original_config.save_to_file(&config_path).unwrap();
        let loaded_config = AppConfig::load_from_file(&config_path).unwrap();

        assert_eq!(loaded_config.athletes.len(), 1);
        assert!(loaded_config.default_athlete_id.is_some());
    }
}