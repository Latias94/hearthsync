use serde::{Deserialize, Serialize};

use crate::core::archive_path::validate_portable_path_segment;
use crate::core::bundle::{
    ApplyAction as DomainApplyAction, ApplyGroup as DomainApplyGroup,
    BundleApplyMappings as DomainBundleApplyMappings,
    CharacterMappingOverride as DomainCharacterMappingOverride, WtfScope as DomainWtfScope,
};
use crate::core::error::AppResult;
use crate::core::manifest::{
    ApplyDefaults as DomainApplyDefaults, BundleManifest as DomainBundleManifest,
    BundleResources as DomainBundleResources, CharacterMappingMode as DomainCharacterMappingMode,
    CharacterResource as DomainCharacterResource, MappingRules as DomainMappingRules,
    PackageMetadata as DomainPackageMetadata, ResourceApplyPolicy as DomainResourceApplyPolicy,
    SourceInstallation as DomainSourceInstallation,
};

use super::super::map_owned_vec;
use super::install::{HostPlatformValue, WowFlavorValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterMappingModeValue {
    KeepOriginal,
    Explicit,
    Prompt,
}

impl CharacterMappingModeValue {
    pub(crate) fn from_domain(value: DomainCharacterMappingMode) -> Self {
        match value {
            DomainCharacterMappingMode::KeepOriginal => Self::KeepOriginal,
            DomainCharacterMappingMode::Explicit => Self::Explicit,
            DomainCharacterMappingMode::Prompt => Self::Prompt,
        }
    }

    pub(crate) fn into_domain(self) -> DomainCharacterMappingMode {
        match self {
            Self::KeepOriginal => DomainCharacterMappingMode::KeepOriginal,
            Self::Explicit => DomainCharacterMappingMode::Explicit,
            Self::Prompt => DomainCharacterMappingMode::Prompt,
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

impl BundlePackageValue {
    pub(crate) fn from_domain(value: DomainPackageMetadata) -> Self {
        Self {
            id: value.id,
            name: value.name,
            created_by: value.created_by,
            description: value.description,
        }
    }

    pub(crate) fn into_domain(self) -> DomainPackageMetadata {
        DomainPackageMetadata {
            id: self.id,
            name: self.name,
            created_by: self.created_by,
            description: self.description,
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

impl BundleSourceValue {
    pub(crate) fn from_domain(value: DomainSourceInstallation) -> Self {
        Self {
            flavor: WowFlavorValue::from_domain(value.flavor),
            platform: value.platform.map(HostPlatformValue::from_domain),
            exported_at: value.exported_at,
            supported_targets: map_owned_vec(value.supported_targets, WowFlavorValue::from_domain),
        }
    }

    pub(crate) fn into_domain(self) -> DomainSourceInstallation {
        DomainSourceInstallation {
            flavor: self.flavor.into_domain(),
            platform: self.platform.map(HostPlatformValue::into_domain),
            exported_at: self.exported_at,
            supported_targets: map_owned_vec(self.supported_targets, WowFlavorValue::into_domain),
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

impl BundleCharacterResourceValue {
    pub(crate) fn from_domain(value: DomainCharacterResource) -> Self {
        Self {
            source_account: value.source_account,
            source_server: value.source_server,
            source_character: value.source_character,
            target_hint: value.target_hint,
        }
    }

    pub(crate) fn into_domain(self) -> DomainCharacterResource {
        DomainCharacterResource {
            source_account: self.source_account,
            source_server: self.source_server,
            source_character: self.source_character,
            target_hint: self.target_hint,
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

impl BundleResourcesValue {
    pub(crate) fn from_domain(value: DomainBundleResources) -> Self {
        Self {
            addons: value.addons,
            wtf_common: value.wtf_common,
            wtf_characters: map_owned_vec(
                value.wtf_characters,
                BundleCharacterResourceValue::from_domain,
            ),
            fonts: value.fonts,
            interface_assets: value.interface_assets,
            addon_lock: value.addon_lock,
            addon_indexes: value.addon_indexes,
        }
    }

    pub(crate) fn into_domain(self) -> DomainBundleResources {
        DomainBundleResources {
            addons: self.addons,
            wtf_common: self.wtf_common,
            wtf_characters: map_owned_vec(
                self.wtf_characters,
                BundleCharacterResourceValue::into_domain,
            ),
            fonts: self.fonts,
            interface_assets: self.interface_assets,
            addon_lock: self.addon_lock,
            addon_indexes: self.addon_indexes,
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

impl BundleMappingRulesValue {
    pub(crate) fn from_domain(value: DomainMappingRules) -> Self {
        Self {
            character_mode: CharacterMappingModeValue::from_domain(value.character_mode),
            rewrite_profile_keys: value.rewrite_profile_keys,
            rewrite_identity_strings: value.rewrite_identity_strings,
            allow_cross_platform: value.allow_cross_platform,
        }
    }

    pub(crate) fn into_domain(self) -> DomainMappingRules {
        DomainMappingRules {
            character_mode: self.character_mode.into_domain(),
            rewrite_profile_keys: self.rewrite_profile_keys,
            rewrite_identity_strings: self.rewrite_identity_strings,
            allow_cross_platform: self.allow_cross_platform,
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

impl BundleCharacterMappingOverrideValue {
    pub(crate) fn from_domain(value: DomainCharacterMappingOverride) -> Self {
        Self {
            source_account: value.source_account,
            source_server: value.source_server,
            source_character: value.source_character,
            target_account: value.target_account,
            target_server: value.target_server,
            target_character: value.target_character,
        }
    }

    pub(crate) fn into_domain(self) -> AppResult<DomainCharacterMappingOverride> {
        let Self {
            source_account,
            source_server,
            source_character,
            target_account,
            target_server,
            target_character,
        } = self;

        validate_optional_plain_name("source account", source_account.as_deref())?;
        validate_plain_name("source server", &source_server)?;
        validate_plain_name("source character", &source_character)?;
        validate_optional_plain_name("target account", target_account.as_deref())?;
        validate_plain_name("target server", &target_server)?;
        validate_plain_name("target character", &target_character)?;

        Ok(DomainCharacterMappingOverride {
            source_account,
            source_server,
            source_character,
            target_account,
            target_server,
            target_character,
        })
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

impl BundleApplyMappingsValue {
    pub(crate) fn from_domain(value: DomainBundleApplyMappings) -> Self {
        Self {
            target_account: value.target_account,
            target_server: value.target_server,
            target_character: value.target_character,
            selected_accounts: value.selected_accounts,
            all_accounts: value.all_accounts,
            characters: map_owned_vec(
                value.characters,
                BundleCharacterMappingOverrideValue::from_domain,
            ),
        }
    }

    pub(crate) fn into_domain(self) -> AppResult<DomainBundleApplyMappings> {
        let Self {
            target_account,
            target_server,
            target_character,
            selected_accounts,
            all_accounts,
            characters,
        } = self;

        validate_optional_plain_name("target account", target_account.as_deref())?;
        validate_optional_plain_name("target server", target_server.as_deref())?;
        validate_optional_plain_name("target character", target_character.as_deref())?;
        for selected_account in &selected_accounts {
            validate_plain_name("selected account", selected_account)?;
        }
        let characters = characters
            .into_iter()
            .map(BundleCharacterMappingOverrideValue::into_domain)
            .collect::<AppResult<Vec<_>>>()?;

        let mappings = DomainBundleApplyMappings {
            target_account,
            target_server,
            target_character,
            selected_accounts,
            all_accounts,
            characters,
        };
        mappings.validate()?;
        Ok(mappings)
    }
}

fn validate_optional_plain_name(kind: &str, value: Option<&str>) -> AppResult<()> {
    if let Some(value) = value {
        validate_plain_name(kind, value)?;
    }

    Ok(())
}

fn validate_plain_name(kind: &str, value: &str) -> AppResult<()> {
    validate_portable_path_segment(value, kind)
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

impl ApplyActionValue {
    pub(crate) fn from_domain(value: DomainApplyAction) -> Self {
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

impl ApplyGroupValue {
    pub(crate) fn from_domain(value: DomainApplyGroup) -> Self {
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

impl WtfScopeValue {
    pub(crate) fn from_domain(value: DomainWtfScope) -> Self {
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

impl ResourceApplyPolicyValue {
    pub(crate) fn from_domain(value: DomainResourceApplyPolicy) -> Self {
        match value {
            DomainResourceApplyPolicy::Merge => Self::Merge,
            DomainResourceApplyPolicy::Share => Self::Share,
            DomainResourceApplyPolicy::Sync => Self::Sync,
            DomainResourceApplyPolicy::Mirror => Self::Mirror,
            DomainResourceApplyPolicy::ReplaceSelected => Self::ReplaceSelected,
            DomainResourceApplyPolicy::Preserve => Self::Preserve,
        }
    }

    pub(crate) fn into_domain(self) -> DomainResourceApplyPolicy {
        match self {
            Self::Merge => DomainResourceApplyPolicy::Merge,
            Self::Share => DomainResourceApplyPolicy::Share,
            Self::Sync => DomainResourceApplyPolicy::Sync,
            Self::Mirror => DomainResourceApplyPolicy::Mirror,
            Self::ReplaceSelected => DomainResourceApplyPolicy::ReplaceSelected,
            Self::Preserve => DomainResourceApplyPolicy::Preserve,
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
        Self::from_domain(crate::core::bundle::author_package_apply_defaults())
    }

    pub(crate) fn from_domain(value: DomainApplyDefaults) -> Self {
        Self {
            create_backup: value.create_backup,
            addons: ResourceApplyPolicyValue::from_domain(value.addons),
            wtf_common: ResourceApplyPolicyValue::from_domain(value.wtf_common),
            wtf_characters: ResourceApplyPolicyValue::from_domain(value.wtf_characters),
            fonts: ResourceApplyPolicyValue::from_domain(value.fonts),
            interface_assets: ResourceApplyPolicyValue::from_domain(value.interface_assets),
        }
    }

    pub(crate) fn into_domain(self) -> DomainApplyDefaults {
        DomainApplyDefaults {
            create_backup: self.create_backup,
            addons: self.addons.into_domain(),
            wtf_common: self.wtf_common.into_domain(),
            wtf_characters: self.wtf_characters.into_domain(),
            fonts: self.fonts.into_domain(),
            interface_assets: self.interface_assets.into_domain(),
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

impl BundleManifestValue {
    pub(crate) fn from_domain(value: DomainBundleManifest) -> Self {
        Self {
            schema_version: value.schema_version,
            package: BundlePackageValue::from_domain(value.package),
            source: BundleSourceValue::from_domain(value.source),
            resources: BundleResourcesValue::from_domain(value.resources),
            mapping: BundleMappingRulesValue::from_domain(value.mapping),
            apply: BundleApplyDefaultsValue::from_domain(value.apply),
        }
    }

    pub(crate) fn into_domain(self) -> AppResult<DomainBundleManifest> {
        let manifest = DomainBundleManifest {
            schema_version: self.schema_version,
            package: self.package.into_domain(),
            source: self.source.into_domain(),
            resources: self.resources.into_domain(),
            mapping: self.mapping.into_domain(),
            apply: self.apply.into_domain(),
        };
        manifest.validate()?;
        Ok(manifest)
    }
}
