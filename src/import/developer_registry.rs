use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Registry of known developer field UUIDs and their field mappings
/// Enables automatic field detection and parsing for popular third-party applications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeveloperFieldRegistry {
    /// Map of application UUIDs to their metadata and field definitions
    applications: HashMap<String, ApplicationInfo>,
}

/// Information about a registered developer application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationInfo {
    /// Application UUID as hex string
    pub uuid: String,
    /// Human-readable application name
    pub name: String,
    /// Manufacturer/developer name
    pub manufacturer: String,
    /// Optional version information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Known field definitions for this application
    pub fields: Vec<KnownField>,
}

/// Definition of a known developer field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownField {
    /// Field definition number within developer namespace
    pub field_number: u8,
    /// Field name
    pub name: String,
    /// Data type identifier
    pub data_type: String,
    /// Units of measurement
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units: Option<String>,
    /// Scale factor (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    /// Offset value (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<f64>,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl DeveloperFieldRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            applications: HashMap::new(),
        }
    }

    /// Load registry from embedded JSON data
    pub fn from_embedded() -> Result<Self, serde_json::Error> {
        const REGISTRY_JSON: &str = include_str!("developer_registry.json");
        serde_json::from_str(REGISTRY_JSON)
    }

    /// Load registry from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Register a new application
    pub fn register_application(&mut self, app: ApplicationInfo) {
        self.applications.insert(app.uuid.clone(), app);
    }

    /// Look up application info by UUID
    pub fn get_application(&self, uuid: &str) -> Option<&ApplicationInfo> {
        self.applications.get(uuid)
    }

    /// Look up application info by UUID bytes
    pub fn get_application_by_bytes(&self, uuid_bytes: &[u8; 16]) -> Option<&ApplicationInfo> {
        let uuid_str = uuid::Uuid::from_bytes(*uuid_bytes).to_string();
        self.get_application(&uuid_str)
    }

    /// Look up a specific field by application UUID and field number
    pub fn get_field(&self, uuid: &str, field_number: u8) -> Option<&KnownField> {
        self.applications
            .get(uuid)?
            .fields
            .iter()
            .find(|f| f.field_number == field_number)
    }

    /// Look up a specific field by UUID bytes and field number
    pub fn get_field_by_bytes(
        &self,
        uuid_bytes: &[u8; 16],
        field_number: u8,
    ) -> Option<&KnownField> {
        let uuid_str = uuid::Uuid::from_bytes(*uuid_bytes).to_string();
        self.get_field(&uuid_str, field_number)
    }

    /// Get all registered application UUIDs
    pub fn registered_uuids(&self) -> Vec<String> {
        self.applications.keys().cloned().collect()
    }

    /// Get total number of registered applications
    pub fn application_count(&self) -> usize {
        self.applications.len()
    }

    /// Get total number of registered fields across all applications
    pub fn field_count(&self) -> usize {
        self.applications
            .values()
            .map(|app| app.fields.len())
            .sum()
    }

    /// Check if a UUID is registered
    pub fn is_registered(&self, uuid: &str) -> bool {
        self.applications.contains_key(uuid)
    }

    /// Check if UUID bytes are registered
    pub fn is_registered_by_bytes(&self, uuid_bytes: &[u8; 16]) -> bool {
        let uuid_str = uuid::Uuid::from_bytes(*uuid_bytes).to_string();
        self.is_registered(&uuid_str)
    }
}

impl Default for DeveloperFieldRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_registry() {
        let registry = DeveloperFieldRegistry::new();
        assert_eq!(registry.application_count(), 0);
        assert_eq!(registry.field_count(), 0);
    }

    #[test]
    fn test_register_application() {
        let mut registry = DeveloperFieldRegistry::new();

        let app = ApplicationInfo {
            uuid: "12345678-1234-5678-1234-567812345678".to_string(),
            name: "Test App".to_string(),
            manufacturer: "Test Manufacturer".to_string(),
            version: Some("1.0".to_string()),
            fields: vec![KnownField {
                field_number: 0,
                name: "test_field".to_string(),
                data_type: "uint16".to_string(),
                units: Some("watts".to_string()),
                scale: Some(1.0),
                offset: None,
                description: Some("Test field".to_string()),
            }],
        };

        registry.register_application(app);
        assert_eq!(registry.application_count(), 1);
        assert_eq!(registry.field_count(), 1);
    }

    #[test]
    fn test_lookup_application() {
        let mut registry = DeveloperFieldRegistry::new();
        let uuid = "12345678-1234-5678-1234-567812345678";

        let app = ApplicationInfo {
            uuid: uuid.to_string(),
            name: "Test App".to_string(),
            manufacturer: "Test Manufacturer".to_string(),
            version: None,
            fields: vec![],
        };

        registry.register_application(app);

        let found = registry.get_application(uuid);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test App");
    }

    #[test]
    fn test_lookup_field() {
        let mut registry = DeveloperFieldRegistry::new();
        let uuid = "12345678-1234-5678-1234-567812345678";

        let app = ApplicationInfo {
            uuid: uuid.to_string(),
            name: "Test App".to_string(),
            manufacturer: "Test Manufacturer".to_string(),
            version: None,
            fields: vec![
                KnownField {
                    field_number: 0,
                    name: "power".to_string(),
                    data_type: "uint16".to_string(),
                    units: Some("watts".to_string()),
                    scale: Some(1.0),
                    offset: None,
                    description: None,
                },
                KnownField {
                    field_number: 1,
                    name: "cadence".to_string(),
                    data_type: "uint8".to_string(),
                    units: Some("rpm".to_string()),
                    scale: Some(1.0),
                    offset: None,
                    description: None,
                },
            ],
        };

        registry.register_application(app);

        let field = registry.get_field(uuid, 1);
        assert!(field.is_some());
        assert_eq!(field.unwrap().name, "cadence");

        let missing = registry.get_field(uuid, 99);
        assert!(missing.is_none());
    }

    #[test]
    fn test_uuid_bytes_lookup() {
        let mut registry = DeveloperFieldRegistry::new();

        // Use a valid UUID
        let uuid_str = "12345678-1234-5678-1234-567812345678";
        let uuid = uuid::Uuid::parse_str(uuid_str).unwrap();
        let uuid_bytes = uuid.as_bytes();

        let app = ApplicationInfo {
            uuid: uuid_str.to_string(),
            name: "Test App".to_string(),
            manufacturer: "Test Manufacturer".to_string(),
            version: None,
            fields: vec![],
        };

        registry.register_application(app);

        assert!(registry.is_registered_by_bytes(uuid_bytes));
        let found = registry.get_application_by_bytes(uuid_bytes);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test App");
    }

    #[test]
    fn test_json_serialization() {
        let mut registry = DeveloperFieldRegistry::new();

        let app = ApplicationInfo {
            uuid: "12345678-1234-5678-1234-567812345678".to_string(),
            name: "Test App".to_string(),
            manufacturer: "Test Manufacturer".to_string(),
            version: Some("1.0".to_string()),
            fields: vec![KnownField {
                field_number: 0,
                name: "power".to_string(),
                data_type: "uint16".to_string(),
                units: Some("watts".to_string()),
                scale: Some(1.0),
                offset: None,
                description: Some("Power output".to_string()),
            }],
        };

        registry.register_application(app);

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&registry).unwrap();
        assert!(json.contains("Test App"));
        assert!(json.contains("power"));

        // Deserialize back
        let deserialized: DeveloperFieldRegistry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.application_count(), 1);
        assert_eq!(deserialized.field_count(), 1);
    }

    #[test]
    fn test_embedded_registry_loads() {
        // Test that the embedded JSON registry loads successfully
        let registry = DeveloperFieldRegistry::from_embedded();
        assert!(registry.is_ok(), "Embedded registry should load successfully");

        let registry = registry.unwrap();

        // Verify we have the expected number of applications
        assert!(registry.application_count() >= 12, "Should have at least 12 registered applications");

        // Verify some known applications are present
        assert!(registry.is_registered("a42b5e01-d5e9-4eb6-9f42-91234567890a"), "Stryd should be registered");
        assert!(registry.is_registered("c4d5e6f7-8901-4bcd-ef01-234567890123"), "Moxy should be registered");
        assert!(registry.is_registered("f7890123-4567-4ef0-1234-567890123456"), "Garmin Vector should be registered");

        // Verify field lookups work
        let stryd = registry.get_application("a42b5e01-d5e9-4eb6-9f42-91234567890a");
        assert!(stryd.is_some());
        assert_eq!(stryd.unwrap().name, "Stryd Running Power");

        let power_field = registry.get_field("a42b5e01-d5e9-4eb6-9f42-91234567890a", 0);
        assert!(power_field.is_some());
        assert_eq!(power_field.unwrap().name, "running_power");
    }

    #[test]
    fn test_registry_statistics() {
        let registry = DeveloperFieldRegistry::from_embedded().unwrap();

        assert!(registry.application_count() >= 12);
        assert!(registry.field_count() >= 30); // At least 30 total fields across all apps

        let uuids = registry.registered_uuids();
        assert!(uuids.len() >= 12);
        assert!(uuids.contains(&"a42b5e01-d5e9-4eb6-9f42-91234567890a".to_string()));
    }
}