use anyhow::Result;
use std::path::Path;

use crate::import::ImportFormat;
use crate::models::Workout;

/// FIT file importer for Garmin native format (stub implementation)
pub struct FitImporter;

impl FitImporter {
    pub fn new() -> Self {
        Self
    }
}

impl ImportFormat for FitImporter {
    fn can_import(&self, file_path: &Path) -> bool {
        file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase() == "fit")
            .unwrap_or(false)
    }

    fn import_file(&self, _file_path: &Path) -> Result<Vec<Workout>> {
        // TODO: Implement FIT file parsing
        // For now, return an error indicating FIT import is not yet implemented
        anyhow::bail!(
            "FIT file import is not yet implemented. Please use CSV, TCX, or GPX format for now."
        )
    }

    fn get_format_name(&self) -> &'static str {
        "FIT"
    }
}
