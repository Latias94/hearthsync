use serde::{Deserialize, Serialize};

use crate::core::bundle::{
    ApplyAction as DomainApplyAction, ApplyGroup as DomainApplyGroup,
    BundleApplyMappings as DomainBundleApplyMappings,
    CharacterMappingOverride as DomainCharacterMappingOverride, WtfScope as DomainWtfScope,
};
use crate::core::manifest::{
    ApplyDefaults as DomainApplyDefaults, BundleManifest as DomainBundleManifest,
    BundleResources as DomainBundleResources, CharacterMappingMode as DomainCharacterMappingMode,
    CharacterResource as DomainCharacterResource, MappingRules as DomainMappingRules,
    PackageMetadata as DomainPackageMetadata, ResourceApplyPolicy as DomainResourceApplyPolicy,
    SourceInstallation as DomainSourceInstallation,
};

use super::install::{HostPlatformValue, WowFlavorValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterMappingModeValue {
    KeepOriginal,
    Explicit,
    Prompt,
}

impl From<DomainCharacterMappingMode> for CharacterMappingModeValue {
    fn from(value: DomainCharacterMappingMode) -> Self {
        match value {
            DomainCharacterMappingMode::KeepOriginal => Self::KeepOriginal,
            DomainCharacterMappingMode::Explicit => Self::Explicit,
            DomainCharacterMappingMode::Prompt => Self::Prompt,
        }
    }
}

impl From<CharacterMappingModeValue> for DomainCharacterMappingMode {
    fn from(value: CharacterMappingModeValue) -> Self {
        match value {
            CharacterMappingModeValue::KeepOriginal => Self::KeepOriginal,
            CharacterMappingModeValue::Explicit => Self::Explicit,
            CharacterMappingModeValue::Prompt => Self::Prompt,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundlePackageValue {
    pub id: String,
    pub name: String,
    pub created_by: String,
    pub description: Option<String>,
}

impl From<DomainPackageMetadata> for BundlePackageValue {
    fn from(value: DomainPackageMetadata) -> Self {
        Self {
            id: value.id,
            name: value.name,
            created_by: value.created_by,
            description: value.description,
        }
    }
}

impl From<BundlePackageValue> for DomainPackageMetadata {
    fn from(value: BundlePackageValue) -> Self {
        Self {
            id: value.id,
            name: value.name,
            created_by: value.created_by,
            description: value.description,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleSourceValue {
    pub flavor: WowFlavorValue,
    pub platform: Option<HostPlatformValue>,
    pub exported_at: Option<String>,
    pub supported_targets: Vec<WowFlavorValue>,
}

impl From<DomainSourceInstallation> for BundleSourceValue {
    fn from(value: DomainSourceInstallation) -> Self {
        Self {
            flavor: value.flavor.into(),
            platform: value.platform.map(Into::into),
            exported_at: value.exported_at,
            supported_targets: value
                .supported_targets
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

impl From<BundleSourceValue> for DomainSourceInstallation {
    fn from(value: BundleSourceValue) -> Self {
        Self {
            flavor: value.flavor.into(),
            platform: value.platform.map(Into::into),
            exported_at: value.exported_at,
            supported_targets: value
                .supported_targets
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleCharacterResourceValue {
    pub source_account: Option<String>,
    pub source_server: String,
    pub source_character: String,
    pub target_hint: Option<String>,
}

impl From<DomainCharacterResource> for BundleCharacterResourceValue {
    fn from(value: DomainCharacterResource) -> Self {
        Self {
            source_account: value.source_account,
            source_server: value.source_server,
            source_character: value.source_character,
            target_hint: value.target_hint,
        }
    }
}

impl From<BundleCharacterResourceValue> for DomainCharacterResource {
    fn from(value: BundleCharacterResourceValue) -> Self {
        Self {
            source_account: value.source_account,
            source_server: value.source_server,
            source_character: value.source_character,
            target_hint: value.target_hint,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleResourcesValue {
    pub addons: Vec<String>,
    pub wtf_common: bool,
    pub wtf_characters: Vec<BundleCharacterResourceValue>,
    pub fonts: bool,
    pub interface_assets: Vec<String>,
    pub addon_lock: bool,
    pub addon_indexes: Vec<String>,
}

impl From<DomainBundleResources> for BundleResourcesValue {
    fn from(value: DomainBundleResources) -> Self {
        Self {
            addons: value.addons,
            wtf_common: value.wtf_common,
            wtf_characters: value
                .wtf_characters
                .into_iter()
                .map(BundleCharacterResourceValue::from)
                .collect(),
            fonts: value.fonts,
            interface_assets: value.interface_assets,
            addon_lock: value.addon_lock,
            addon_indexes: value.addon_indexes,
        }
    }
}

impl From<BundleResourcesValue> for DomainBundleResources {
    fn from(value: BundleResourcesValue) -> Self {
        Self {
            addons: value.addons,
            wtf_common: value.wtf_common,
            wtf_characters: value.wtf_characters.into_iter().map(Into::into).collect(),
            fonts: value.fonts,
            interface_assets: value.interface_assets,
            addon_lock: value.addon_lock,
            addon_indexes: value.addon_indexes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleMappingRulesValue {
    pub character_mode: CharacterMappingModeValue,
    pub rewrite_profile_keys: bool,
    pub rewrite_identity_strings: bool,
    pub allow_cross_platform: bool,
}

impl From<DomainMappingRules> for BundleMappingRulesValue {
    fn from(value: DomainMappingRules) -> Self {
        Self {
            character_mode: value.character_mode.into(),
            rewrite_profile_keys: value.rewrite_profile_keys,
            rewrite_identity_strings: value.rewrite_identity_strings,
            allow_cross_platform: value.allow_cross_platform,
        }
    }
}

impl From<BundleMappingRulesValue> for DomainMappingRules {
    fn from(value: BundleMappingRulesValue) -> Self {
        Self {
            character_mode: value.character_mode.into(),
            rewrite_profile_keys: value.rewrite_profile_keys,
            rewrite_identity_strings: value.rewrite_identity_strings,
            allow_cross_platform: value.allow_cross_platform,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleCharacterMappingOverrideValue {
    pub source_account: Option<String>,
    pub source_server: String,
    pub source_character: String,
    pub target_account: Option<String>,
    pub target_server: String,
    pub target_character: String,
}

impl From<DomainCharacterMappingOverride> for BundleCharacterMappingOverrideValue {
    fn from(value: DomainCharacterMappingOverride) -> Self {
        Self {
            source_account: value.source_account,
            source_server: value.source_server,
            source_character: value.source_character,
            target_account: value.target_account,
            target_server: value.target_server,
            target_character: value.target_character,
        }
    }
}

impl From<BundleCharacterMappingOverrideValue> for DomainCharacterMappingOverride {
    fn from(value: BundleCharacterMappingOverrideValue) -> Self {
        Self {
            source_account: value.source_account,
            source_server: value.source_server,
            source_character: value.source_character,
            target_account: value.target_account,
            target_server: value.target_server,
            target_character: value.target_character,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleApplyMappingsValue {
    pub target_account: Option<String>,
    pub target_server: Option<String>,
    pub target_character: Option<String>,
    pub selected_accounts: Vec<String>,
    pub all_accounts: bool,
    pub characters: Vec<BundleCharacterMappingOverrideValue>,
}

impl From<DomainBundleApplyMappings> for BundleApplyMappingsValue {
    fn from(value: DomainBundleApplyMappings) -> Self {
        Self {
            target_account: value.target_account,
            target_server: value.target_server,
            target_character: value.target_character,
            selected_accounts: value.selected_accounts,
            all_accounts: value.all_accounts,
            characters: value
                .characters
                .into_iter()
                .map(BundleCharacterMappingOverrideValue::from)
                .collect(),
        }
    }
}

impl From<BundleApplyMappingsValue> for DomainBundleApplyMappings {
    fn from(value: BundleApplyMappingsValue) -> Self {
        Self {
            target_account: value.target_account,
            target_server: value.target_server,
            target_character: value.target_character,
            selected_accounts: value.selected_accounts,
            all_accounts: value.all_accounts,
            characters: value.characters.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyActionValue {
    Remove,
    Add,
    Replace,
    Skip,
    Preserve,
}

impl From<DomainApplyAction> for ApplyActionValue {
    fn from(value: DomainApplyAction) -> Self {
        match value {
            DomainApplyAction::Remove => Self::Remove,
            DomainApplyAction::Add => Self::Add,
            DomainApplyAction::Replace => Self::Replace,
            DomainApplyAction::Skip => Self::Skip,
            DomainApplyAction::Preserve => Self::Preserve,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyGroupValue {
    Addons,
    WtfCommon,
    WtfCharacters,
    Fonts,
    InterfaceAssets,
    Metadata,
}

impl From<DomainApplyGroup> for ApplyGroupValue {
    fn from(value: DomainApplyGroup) -> Self {
        match value {
            DomainApplyGroup::Addons => Self::Addons,
            DomainApplyGroup::WtfCommon => Self::WtfCommon,
            DomainApplyGroup::WtfCharacters => Self::WtfCharacters,
            DomainApplyGroup::Fonts => Self::Fonts,
            DomainApplyGroup::InterfaceAssets => Self::InterfaceAssets,
            DomainApplyGroup::Metadata => Self::Metadata,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WtfScopeValue {
    GlobalConfig,
    RootSavedVariables,
    AccountRootFile,
    AccountSavedVariables,
    CharacterSavedVariables,
    CharacterState,
    CacheLike,
    Unknown,
}

impl From<DomainWtfScope> for WtfScopeValue {
    fn from(value: DomainWtfScope) -> Self {
        match value {
            DomainWtfScope::GlobalConfig => Self::GlobalConfig,
            DomainWtfScope::RootSavedVariables => Self::RootSavedVariables,
            DomainWtfScope::AccountRootFile => Self::AccountRootFile,
            DomainWtfScope::AccountSavedVariables => Self::AccountSavedVariables,
            DomainWtfScope::CharacterSavedVariables => Self::CharacterSavedVariables,
            DomainWtfScope::CharacterState => Self::CharacterState,
            DomainWtfScope::CacheLike => Self::CacheLike,
            DomainWtfScope::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceApplyPolicyValue {
    Merge,
    Share,
    Sync,
    Mirror,
    ReplaceSelected,
    Preserve,
}

impl From<DomainResourceApplyPolicy> for ResourceApplyPolicyValue {
    fn from(value: DomainResourceApplyPolicy) -> Self {
        match value {
            DomainResourceApplyPolicy::Merge => Self::Merge,
            DomainResourceApplyPolicy::Share => Self::Share,
            DomainResourceApplyPolicy::Sync => Self::Sync,
            DomainResourceApplyPolicy::Mirror => Self::Mirror,
            DomainResourceApplyPolicy::ReplaceSelected => Self::ReplaceSelected,
            DomainResourceApplyPolicy::Preserve => Self::Preserve,
        }
    }
}

impl From<ResourceApplyPolicyValue> for DomainResourceApplyPolicy {
    fn from(value: ResourceApplyPolicyValue) -> Self {
        match value {
            ResourceApplyPolicyValue::Merge => Self::Merge,
            ResourceApplyPolicyValue::Share => Self::Share,
            ResourceApplyPolicyValue::Sync => Self::Sync,
            ResourceApplyPolicyValue::Mirror => Self::Mirror,
            ResourceApplyPolicyValue::ReplaceSelected => Self::ReplaceSelected,
            ResourceApplyPolicyValue::Preserve => Self::Preserve,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleApplyDefaultsValue {
    pub create_backup: bool,
    pub addons: ResourceApplyPolicyValue,
    pub wtf_common: ResourceApplyPolicyValue,
    pub wtf_characters: ResourceApplyPolicyValue,
    pub fonts: ResourceApplyPolicyValue,
    pub interface_assets: ResourceApplyPolicyValue,
}

impl BundleApplyDefaultsValue {
    pub fn author_package_defaults() -> Self {
        crate::core::bundle::author_package_apply_defaults().into()
    }
}

impl From<DomainApplyDefaults> for BundleApplyDefaultsValue {
    fn from(value: DomainApplyDefaults) -> Self {
        Self {
            create_backup: value.create_backup,
            addons: ResourceApplyPolicyValue::from(value.addons),
            wtf_common: ResourceApplyPolicyValue::from(value.wtf_common),
            wtf_characters: ResourceApplyPolicyValue::from(value.wtf_characters),
            fonts: ResourceApplyPolicyValue::from(value.fonts),
            interface_assets: ResourceApplyPolicyValue::from(value.interface_assets),
        }
    }
}

impl From<BundleApplyDefaultsValue> for DomainApplyDefaults {
    fn from(value: BundleApplyDefaultsValue) -> Self {
        Self {
            create_backup: value.create_backup,
            addons: value.addons.into(),
            wtf_common: value.wtf_common.into(),
            wtf_characters: value.wtf_characters.into(),
            fonts: value.fonts.into(),
            interface_assets: value.interface_assets.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleManifestValue {
    pub schema_version: u32,
    pub package: BundlePackageValue,
    pub source: BundleSourceValue,
    pub resources: BundleResourcesValue,
    pub mapping: BundleMappingRulesValue,
    pub apply: BundleApplyDefaultsValue,
}

impl From<DomainBundleManifest> for BundleManifestValue {
    fn from(value: DomainBundleManifest) -> Self {
        Self {
            schema_version: value.schema_version,
            package: BundlePackageValue::from(value.package),
            source: BundleSourceValue::from(value.source),
            resources: BundleResourcesValue::from(value.resources),
            mapping: BundleMappingRulesValue::from(value.mapping),
            apply: BundleApplyDefaultsValue::from(value.apply),
        }
    }
}

impl From<BundleManifestValue> for DomainBundleManifest {
    fn from(value: BundleManifestValue) -> Self {
        Self {
            schema_version: value.schema_version,
            package: value.package.into(),
            source: value.source.into(),
            resources: value.resources.into(),
            mapping: value.mapping.into(),
            apply: value.apply.into(),
        }
    }
}
