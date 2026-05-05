use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::archive_path::validate_portable_path_segment;
use crate::core::error::{AppError, AppResult};
use crate::core::install::{DetectedFlavorInstallation, LocalWowAccount};
use crate::core::lua_patch::CharacterMapping;
use crate::core::manifest::{BundleManifest, ResourceApplyPolicy};

#[derive(Debug, Clone, Serialize)]
pub struct BundleApplyPlan {
    pub bundle_path: PathBuf,
    pub target_flavor_root: PathBuf,
    pub discovered_accounts: Vec<LocalWowAccount>,
    pub selected_target_accounts: Vec<String>,
    pub character_mappings: Vec<CharacterMapping>,
    pub operations: Vec<ApplyOperation>,
    pub summary: ApplyPlanSummary,
    pub group_policies: ApplyGroupPolicies,
    pub manifest: BundleManifest,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyOperation {
    pub group: ApplyGroup,
    pub wtf_scope: Option<WtfScope>,
    pub action: ApplyAction,
    pub archive_name: String,
    pub destination: PathBuf,
    pub target_account: Option<String>,
    pub target_server: Option<String>,
    pub target_character: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ApplyPlanSummary {
    pub files_to_add: usize,
    pub files_to_replace: usize,
    pub files_to_skip: usize,
    pub paths_to_remove: usize,
    pub files_to_preserve: usize,
}

impl ApplyPlanSummary {
    pub(crate) fn from_operations(operations: &[ApplyOperation]) -> Self {
        let mut summary = Self::default();

        for operation in operations {
            match operation.action {
                ApplyAction::Remove => summary.paths_to_remove += 1,
                ApplyAction::Add => summary.files_to_add += 1,
                ApplyAction::Replace => summary.files_to_replace += 1,
                ApplyAction::Skip => summary.files_to_skip += 1,
                ApplyAction::Preserve => summary.files_to_preserve += 1,
            }
        }

        summary
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyAction {
    Remove,
    Add,
    Replace,
    Skip,
    Preserve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyGroup {
    Addons,
    WtfCommon,
    WtfCharacters,
    Fonts,
    InterfaceAssets,
    Metadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WtfScope {
    GlobalConfig,
    RootSavedVariables,
    AccountRootFile,
    AccountSavedVariables,
    CharacterSavedVariables,
    CharacterState,
    CacheLike,
    Unknown,
}

impl WtfScope {
    pub fn risk(self) -> WtfScopeRisk {
        match self {
            Self::GlobalConfig
            | Self::AccountRootFile
            | Self::CharacterSavedVariables
            | Self::CharacterState => WtfScopeRisk::Medium,
            Self::RootSavedVariables | Self::AccountSavedVariables => WtfScopeRisk::High,
            Self::CacheLike => WtfScopeRisk::Low,
            Self::Unknown => WtfScopeRisk::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WtfScopeRisk {
    Low,
    Medium,
    High,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyGroupPolicies {
    pub addons: GroupPolicy,
    pub wtf_common: GroupPolicy,
    pub wtf_characters: GroupPolicy,
    pub fonts: GroupPolicy,
    pub interface_assets: GroupPolicy,
    pub metadata: GroupPolicy,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupPolicy {
    pub policy: ResourceApplyPolicy,
}

#[derive(Debug, Clone)]
pub struct UnpackBundleRequest {
    pub bundle_path: PathBuf,
    pub installation: DetectedFlavorInstallation,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub apply_mappings: BundleApplyMappings,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnpackedBundle {
    pub bundle_path: PathBuf,
    pub target_flavor_root: PathBuf,
    pub dry_run: bool,
    pub planned_files: usize,
    pub written_files: usize,
    pub rewritten_files: usize,
    pub backup_path: Option<PathBuf>,
    pub selected_target_accounts: Vec<String>,
    pub plan_summary: ApplyPlanSummary,
    pub character_mappings: Vec<CharacterMapping>,
    pub manifest: BundleManifest,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BundleApplyMappings {
    pub target_account: Option<String>,
    pub target_server: Option<String>,
    pub target_character: Option<String>,
    #[serde(default)]
    pub selected_accounts: Vec<String>,
    #[serde(default)]
    pub all_accounts: bool,
    #[serde(default)]
    pub characters: Vec<CharacterMappingOverride>,
}

impl BundleApplyMappings {
    pub(crate) fn validate(&self) -> AppResult<()> {
        validate_optional_mapping_segment("target account", self.target_account.as_deref())?;
        validate_optional_mapping_segment("target server", self.target_server.as_deref())?;
        validate_optional_mapping_segment("target character", self.target_character.as_deref())?;

        let mut selected_accounts = std::collections::BTreeMap::new();
        for selected_account in &self.selected_accounts {
            validate_mapping_segment("selected account", selected_account)?;
            let key = normalized_mapping_key(selected_account);
            if let Some(existing) = selected_accounts.insert(key, selected_account) {
                return Err(AppError::Validation(format!(
                    "duplicate selected account mapping: `{selected_account}` conflicts with `{existing}`"
                )));
            }
        }

        validate_character_mapping_overrides(&self.characters)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterMappingOverride {
    pub source_account: Option<String>,
    pub source_server: String,
    pub source_character: String,
    pub target_account: Option<String>,
    pub target_server: String,
    pub target_character: String,
}

impl CharacterMappingOverride {
    fn validate(&self) -> AppResult<()> {
        validate_optional_mapping_segment("source account", self.source_account.as_deref())?;
        validate_mapping_segment("source server", &self.source_server)?;
        validate_mapping_segment("source character", &self.source_character)?;
        validate_optional_mapping_segment("target account", self.target_account.as_deref())?;
        validate_mapping_segment("target server", &self.target_server)?;
        validate_mapping_segment("target character", &self.target_character)?;

        Ok(())
    }
}

fn validate_character_mapping_overrides(overrides: &[CharacterMappingOverride]) -> AppResult<()> {
    let mut seen = Vec::new();
    for mapping in overrides {
        mapping.validate()?;
        let current = CharacterMappingOverrideKey::from(mapping);
        for previous in &seen {
            if current.overlaps(previous) {
                return Err(AppError::Validation(format!(
                    "overlapping character mapping override for `{}/{}`",
                    mapping.source_server, mapping.source_character
                )));
            }
        }
        seen.push(current);
    }

    Ok(())
}

#[derive(Debug)]
struct CharacterMappingOverrideKey {
    source_account: Option<String>,
    source_server: String,
    source_character: String,
}

impl CharacterMappingOverrideKey {
    fn from(mapping: &CharacterMappingOverride) -> Self {
        Self {
            source_account: mapping
                .source_account
                .as_deref()
                .map(normalized_mapping_key),
            source_server: normalized_mapping_key(&mapping.source_server),
            source_character: normalized_mapping_key(&mapping.source_character),
        }
    }

    fn overlaps(&self, other: &Self) -> bool {
        self.source_server == other.source_server
            && self.source_character == other.source_character
            && (self.source_account.is_none()
                || other.source_account.is_none()
                || self.source_account == other.source_account)
    }
}

fn validate_optional_mapping_segment(kind: &str, value: Option<&str>) -> AppResult<()> {
    if let Some(value) = value {
        validate_mapping_segment(kind, value)?;
    }

    Ok(())
}

fn validate_mapping_segment(kind: &str, value: &str) -> AppResult<()> {
    validate_portable_path_segment(value, kind)
}

fn normalized_mapping_key(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{BundleApplyMappings, CharacterMappingOverride};

    #[test]
    fn bundle_apply_mappings_rejects_duplicate_selected_accounts() {
        let error = BundleApplyMappings {
            selected_accounts: vec!["AccountA".to_string(), "accounta".to_string()],
            ..BundleApplyMappings::default()
        }
        .validate()
        .expect_err("duplicate selected accounts should fail");

        assert!(
            error
                .to_string()
                .contains("duplicate selected account mapping")
        );
    }

    #[test]
    fn bundle_apply_mappings_rejects_overlapping_character_overrides() {
        let error = BundleApplyMappings {
            characters: vec![
                character_override(None, "Illidan", "Mage"),
                character_override(Some("AccountA"), "illidan", "Mage"),
            ],
            ..BundleApplyMappings::default()
        }
        .validate()
        .expect_err("overlapping character overrides should fail");

        assert!(
            error
                .to_string()
                .contains("overlapping character mapping override")
        );
    }

    #[test]
    fn bundle_apply_mappings_allows_distinct_account_specific_character_overrides() {
        BundleApplyMappings {
            characters: vec![
                character_override(Some("AccountA"), "Illidan", "Mage"),
                character_override(Some("AccountB"), "illidan", "Mage"),
            ],
            ..BundleApplyMappings::default()
        }
        .validate()
        .expect("account-specific overrides do not overlap");
    }

    fn character_override(
        source_account: Option<&str>,
        source_server: &str,
        source_character: &str,
    ) -> CharacterMappingOverride {
        CharacterMappingOverride {
            source_account: source_account.map(str::to_string),
            source_server: source_server.to_string(),
            source_character: source_character.to_string(),
            target_account: Some("TargetAccount".to_string()),
            target_server: "Stormrage".to_string(),
            target_character: "TargetMage".to_string(),
        }
    }
}
