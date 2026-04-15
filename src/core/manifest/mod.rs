use std::path::Path;

use serde::{Deserialize, Serialize};

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

        for addon_index in &self.resources.addon_indexes {
            if addon_index.trim().is_empty() {
                return Err(AppError::Validation(
                    "resources.addon_indexes must not contain empty paths".to_string(),
                ));
            }
        }

        if self.mapping.character_mode == CharacterMappingMode::Explicit
            && self.resources.wtf_characters.is_empty()
        {
            return Err(AppError::Validation(
                "explicit character mapping requires at least one wtf_characters entry".to_string(),
            ));
        }

        Ok(())
    }
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

pub fn example_manifest() -> AppResult<String> {
    toml::to_string_pretty(&BundleManifest {
        schema_version: 1,
        package: PackageMetadata {
            id: "starter-ui-retail".to_string(),
            name: "Starter UI for Retail".to_string(),
            created_by: "hearthsync".to_string(),
            description: Some("Example bundle manifest for addon and config sync.".to_string()),
        },
        source: SourceInstallation {
            flavor: WowFlavor::Retail,
            platform: Some(HostPlatform::Windows),
            exported_at: Some("2026-04-15T00:00:00Z".to_string()),
            supported_targets: vec![WowFlavor::Retail],
        },
        resources: BundleResources {
            addons: vec![
                "WeakAuras".to_string(),
                "Plater".to_string(),
                "Details".to_string(),
            ],
            wtf_common: true,
            wtf_characters: vec![CharacterResource {
                source_account: Some("ACCOUNT_NAME".to_string()),
                source_server: "Illidan".to_string(),
                source_character: "Examplemage".to_string(),
                target_hint: Some("Map to your main character".to_string()),
            }],
            fonts: false,
            interface_assets: vec!["SharedXML".to_string()],
            addon_lock: false,
            addon_indexes: Vec::new(),
        },
        mapping: MappingRules {
            character_mode: CharacterMappingMode::Explicit,
            rewrite_profile_keys: true,
            rewrite_identity_strings: true,
            allow_cross_platform: true,
        },
        apply: ApplyDefaults {
            create_backup: true,
            addons: ResourceApplyPolicy::Merge,
            wtf_common: ResourceApplyPolicy::Merge,
            wtf_characters: ResourceApplyPolicy::Merge,
            fonts: ResourceApplyPolicy::Merge,
            interface_assets: ResourceApplyPolicy::Merge,
        },
    })
    .map_err(Into::into)
}

pub fn load_manifest(path: &Path) -> AppResult<BundleManifest> {
    let content = std::fs::read_to_string(path)?;
    let manifest: BundleManifest = toml::from_str(&content)?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::example_manifest;
    use crate::core::manifest::BundleManifest;

    #[test]
    fn example_manifest_is_valid() {
        let manifest = example_manifest().expect("example");
        let parsed: BundleManifest = toml::from_str(&manifest).expect("parse");
        parsed.validate().expect("validate");
    }
}
