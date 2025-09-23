use anyhow::Result;
use std::path::Path;

use crate::import::ImportFormat;
use crate::models::Workout;

/// TCX (Training Center XML) importer (stub implementation)
pub struct TcxImporter;

impl TcxImporter {
    pub fn new() -> Self {
        Self
    }
}

impl ImportFormat for TcxImporter {
    fn can_import(&self, file_path: &Path) -> bool {
        file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase() == "tcx")
            .unwrap_or(false)
    }

    fn import_file(&self, _file_path: &Path) -> Result<Vec<Workout>> {
        // TODO: Implement TCX file parsing
        // For now, return an error indicating TCX import is not yet implemented
        anyhow::bail!("TCX file import is not yet implemented. Please use CSV format for now.")
    }

    fn get_format_name(&self) -> &'static str {
        "TCX"
    }
}
