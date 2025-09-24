use anyhow::Result;
use std::path::Path;

use crate::import::ImportFormat;
use crate::models::Workout;

/// GPX importer for GPS track data (stub implementation)
pub struct GpxImporter;

impl GpxImporter {
    pub fn new() -> Self {
        Self
    }
}

impl ImportFormat for GpxImporter {
    fn can_import(&self, file_path: &Path) -> bool {
        file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase() == "gpx")
            .unwrap_or(false)
    }

    fn import_file(&self, _file_path: &Path) -> Result<Vec<Workout>> {
        // TODO: Implement GPX file parsing
        // For now, return an error indicating GPX import is not yet implemented
        anyhow::bail!("GPX file import is not yet implemented. Please use CSV format for now.")
    }

    fn get_format_name(&self) -> &'static str {
        "GPX"
    }
}
