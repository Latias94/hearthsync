use serde::{Deserialize, Serialize};

use crate::core::archive_path::validate_portable_path_segment;
use crate::core::error::{AppError, AppResult};
use crate::core::install::{HostPlatform, WowFlavor};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    pub schema_version: u32,
    pub package: PackageMetadata,
    pub source: SourceInstallation,
    pub resources: BundleResources,
    pub mapping: MappingRules,
    pub apply: ApplyDefaults,
}

impl BundleManifest {
    pub fn validate(&self) -> AppResult<()> {
        if self.schema_version == 0 {
            return Err(AppError::Validation(
                "schema_version must be greater than zero".to_string(),
            ));
        }

        if self.package.id.trim().is_empty() {
            return Err(AppError::Validation(
                "package.id must not be empty".to_string(),
            ));
        }

        if self.package.name.trim().is_empty() {
            return Err(AppError::Validation(
                "package.name must not be empty".to_string(),
            ));
        }
        if self.package.created_by.trim().is_empty() {
            return Err(AppError::Validation(
                "package.created_by must not be empty".to_string(),
            ));
        }

        if self.resources.addons.is_empty()
            && !self.resources.wtf_common
            && self.resources.wtf_characters.is_empty()
            && !self.resources.fonts
            && self.resources.interface_assets.is_empty()
            && !self.resources.addon_lock
            && self.resources.addon_indexes.is_empty()
        {
            return Err(AppError::Validation(
                "resources must include at least one addon, config group, font, or interface asset"
                    .to_string(),
            ));
        }

        for addon in &self.resources.addons {
            validate_manifest_plain_name("resources.addons", addon)?;
        }

        for character in &self.resources.wtf_characters {
            if let Some(source_account) = character.source_account.as_deref() {
                validate_manifest_plain_name(
                    "resources.wtf_characters.source_account",
                    source_account,
                )?;
            }
            validate_manifest_plain_name(
                "resources.wtf_characters.source_server",
                &character.source_server,
            )?;
            validate_manifest_plain_name(
                "resources.wtf_characters.source_character",
                &character.source_character,
            )?;
        }

        for interface_asset in &self.resources.interface_assets {
            validate_manifest_plain_name("resources.interface_assets", interface_asset)?;
        }

        for addon_index in &self.resources.addon_indexes {
            if addon_index.trim().is_empty() {
                return Err(AppError::Validation(
                    "resources.addon_indexes must not contain empty paths".to_string(),
                ));
            }
        }

        if matches!(
            self.mapping.character_mode,
            CharacterMappingMode::Explicit | CharacterMappingMode::Prompt
        ) && self.resources.wtf_characters.is_empty()
        {
            let mode = match self.mapping.character_mode {
                CharacterMappingMode::Explicit => "explicit",
                CharacterMappingMode::Prompt => "prompt",
                CharacterMappingMode::KeepOriginal => "keep_original",
            };
            return Err(AppError::Validation(format!(
                "{mode} character mapping requires at least one wtf_characters entry"
            )));
        }

        Ok(())
    }
}

fn validate_manifest_plain_name(field: &str, value: &str) -> AppResult<()> {
    validate_portable_path_segment(value, field)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    pub id: String,
    pub name: String,
    pub created_by: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInstallation {
    pub flavor: WowFlavor,
    pub platform: Option<HostPlatform>,
    pub exported_at: Option<String>,
    pub supported_targets: Vec<WowFlavor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleResources {
    pub addons: Vec<String>,
    pub wtf_common: bool,
    pub wtf_characters: Vec<CharacterResource>,
    pub fonts: bool,
    pub interface_assets: Vec<String>,
    #[serde(default)]
    pub addon_lock: bool,
    #[serde(default)]
    pub addon_indexes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterResource {
    pub source_account: Option<String>,
    pub source_server: String,
    pub source_character: String,
    pub target_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingRules {
    pub character_mode: CharacterMappingMode,
    pub rewrite_profile_keys: bool,
    pub rewrite_identity_strings: bool,
    pub allow_cross_platform: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterMappingMode {
    KeepOriginal,
    Explicit,
    Prompt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyDefaults {
    pub create_backup: bool,
    pub addons: ResourceApplyPolicy,
    pub wtf_common: ResourceApplyPolicy,
    pub wtf_characters: ResourceApplyPolicy,
    pub fonts: ResourceApplyPolicy,
    pub interface_assets: ResourceApplyPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceApplyPolicy {
    Merge,
    Share,
    Sync,
    Mirror,
    ReplaceSelected,
    Preserve,
}
