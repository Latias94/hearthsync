use std::path::PathBuf;

use serde::Serialize;

use crate::core::app::RuntimeSettingsValue;

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeSettingsInspectionResult {
    pub settings_path: PathBuf,
    pub settings_file_exists: bool,
    pub settings: RuntimeSettingsValue,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeSettingsMutationResult {
    pub settings_path: PathBuf,
    pub settings_file_exists: bool,
    pub file_removed: bool,
    pub settings: RuntimeSettingsValue,
}
