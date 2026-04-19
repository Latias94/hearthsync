use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::addon::AddonPackageMetadata as DomainAddonPackageMetadata;
use crate::core::backup::BackupGroup as DomainBackupGroup;
use crate::core::bundle::{
    ApplyAction as DomainApplyAction, ApplyGroup as DomainApplyGroup,
    BundleApplyMappings as DomainBundleApplyMappings,
    CharacterMappingOverride as DomainCharacterMappingOverride,
    ExternalPackageWarningCategory as DomainExternalPackageWarningCategory,
    ExternalPackageWarningCode as DomainExternalPackageWarningCode,
    HelperStrategy as DomainHelperStrategy, WtfScope as DomainWtfScope,
};
use crate::core::install::{
    DetectedFlavorInstallation, HealthStatus as DomainHealthStatus,
    HostPlatform as DomainHostPlatform, WowFlavor as DomainWowFlavor,
};
use crate::core::manifest::{
    ApplyDefaults as DomainApplyDefaults, BundleManifest as DomainBundleManifest,
    BundleResources as DomainBundleResources, CharacterMappingMode as DomainCharacterMappingMode,
    CharacterResource as DomainCharacterResource, MappingRules as DomainMappingRules,
    PackageMetadata as DomainPackageMetadata, ResourceApplyPolicy as DomainResourceApplyPolicy,
    SourceInstallation as DomainSourceInstallation,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostPlatformValue {
    Windows,
    MacOs,
    Linux,
    Unknown,
}

impl HostPlatformValue {
    pub fn current() -> Self {
        DomainHostPlatform::current().into()
    }
}

impl From<DomainHostPlatform> for HostPlatformValue {
    fn from(value: DomainHostPlatform) -> Self {
        match value {
            DomainHostPlatform::Windows => Self::Windows,
            DomainHostPlatform::MacOs => Self::MacOs,
            DomainHostPlatform::Linux => Self::Linux,
            DomainHostPlatform::Unknown => Self::Unknown,
        }
    }
}

impl From<HostPlatformValue> for DomainHostPlatform {
    fn from(value: HostPlatformValue) -> Self {
        match value {
            HostPlatformValue::Windows => Self::Windows,
            HostPlatformValue::MacOs => Self::MacOs,
            HostPlatformValue::Linux => Self::Linux,
            HostPlatformValue::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WowFlavorValue {
    Retail,
    Classic,
    ClassicEra,
    Ptr,
    Beta,
    Xptr,
}

impl WowFlavorValue {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Retail => "retail",
            Self::Classic => "classic",
            Self::ClassicEra => "classic_era",
            Self::Ptr => "ptr",
            Self::Beta => "beta",
            Self::Xptr => "xptr",
        }
    }

    pub fn folder_name(&self) -> &'static str {
        match self {
            Self::Retail => "_retail_",
            Self::Classic => "_classic_",
            Self::ClassicEra => "_classic_era_",
            Self::Ptr => "_ptr_",
            Self::Beta => "_beta_",
            Self::Xptr => "_xptr_",
        }
    }
}

impl From<DomainWowFlavor> for WowFlavorValue {
    fn from(value: DomainWowFlavor) -> Self {
        match value {
            DomainWowFlavor::Retail => Self::Retail,
            DomainWowFlavor::Classic => Self::Classic,
            DomainWowFlavor::ClassicEra => Self::ClassicEra,
            DomainWowFlavor::Ptr => Self::Ptr,
            DomainWowFlavor::Beta => Self::Beta,
            DomainWowFlavor::Xptr => Self::Xptr,
        }
    }
}

impl From<WowFlavorValue> for DomainWowFlavor {
    fn from(value: WowFlavorValue) -> Self {
        match value {
            WowFlavorValue::Retail => Self::Retail,
            WowFlavorValue::Classic => Self::Classic,
            WowFlavorValue::ClassicEra => Self::ClassicEra,
            WowFlavorValue::Ptr => Self::Ptr,
            WowFlavorValue::Beta => Self::Beta,
            WowFlavorValue::Xptr => Self::Xptr,
        }
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatusValue {
    Healthy,
    Warning,
    Broken,
}

impl From<DomainHealthStatus> for HealthStatusValue {
    fn from(value: DomainHealthStatus) -> Self {
        match value {
            DomainHealthStatus::Healthy => Self::Healthy,
            DomainHealthStatus::Warning => Self::Warning,
            DomainHealthStatus::Broken => Self::Broken,
        }
    }
}

impl From<HealthStatusValue> for DomainHealthStatus {
    fn from(value: HealthStatusValue) -> Self {
        match value {
            HealthStatusValue::Healthy => Self::Healthy,
            HealthStatusValue::Warning => Self::Warning,
            HealthStatusValue::Broken => Self::Broken,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedInstallationValue {
    pub platform: HostPlatformValue,
    pub flavor: WowFlavorValue,
    pub product_root: PathBuf,
    pub flavor_root: PathBuf,
    pub interface_dir: PathBuf,
    pub addon_dir: PathBuf,
    pub wtf_dir: PathBuf,
    pub fonts_dir: PathBuf,
}

impl From<DetectedFlavorInstallation> for ResolvedInstallationValue {
    fn from(value: DetectedFlavorInstallation) -> Self {
        Self {
            platform: value.platform.into(),
            flavor: value.flavor.into(),
            product_root: value.product_root,
            flavor_root: value.flavor_root,
            interface_dir: value.interface_dir,
            addon_dir: value.addon_dir,
            wtf_dir: value.wtf_dir,
            fonts_dir: value.fonts_dir,
        }
    }
}

impl From<ResolvedInstallationValue> for DetectedFlavorInstallation {
    fn from(value: ResolvedInstallationValue) -> Self {
        Self {
            platform: value.platform.into(),
            flavor: value.flavor.into(),
            product_root: value.product_root,
            flavor_root: value.flavor_root,
            interface_dir: value.interface_dir,
            addon_dir: value.addon_dir,
            wtf_dir: value.wtf_dir,
            fonts_dir: value.fonts_dir,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddonPackageMetadataValue {
    #[serde(default)]
    pub index_name: Option<String>,
    #[serde(default)]
    pub index_package_id: Option<String>,
    #[serde(default)]
    pub package_name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub website_url: Option<String>,
    #[serde(default)]
    pub source_sha256: Option<String>,
    #[serde(default)]
    pub supported_flavors: Vec<String>,
}

impl From<DomainAddonPackageMetadata> for AddonPackageMetadataValue {
    fn from(value: DomainAddonPackageMetadata) -> Self {
        Self {
            index_name: value.index_name,
            index_package_id: value.index_package_id,
            package_name: value.package_name,
            version: value.version,
            source_url: value.source_url,
            website_url: value.website_url,
            source_sha256: value.source_sha256,
            supported_flavors: value.supported_flavors,
        }
    }
}

impl From<AddonPackageMetadataValue> for DomainAddonPackageMetadata {
    fn from(value: AddonPackageMetadataValue) -> Self {
        Self {
            index_name: value.index_name,
            index_package_id: value.index_package_id,
            package_name: value.package_name,
            version: value.version,
            source_url: value.source_url,
            website_url: value.website_url,
            source_sha256: value.source_sha256,
            supported_flavors: value.supported_flavors,
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
    #[serde(default)]
    pub addon_lock: bool,
    #[serde(default)]
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
    #[serde(default)]
    pub selected_accounts: Vec<String>,
    #[serde(default)]
    pub all_accounts: bool,
    #[serde(default)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupGroupValue {
    Addons,
    Wtf,
    Fonts,
    InterfaceAssets,
}

impl BackupGroupValue {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Addons => "addons",
            Self::Wtf => "wtf",
            Self::Fonts => "fonts",
            Self::InterfaceAssets => "interface_assets",
        }
    }
}

impl From<DomainBackupGroup> for BackupGroupValue {
    fn from(value: DomainBackupGroup) -> Self {
        match value {
            DomainBackupGroup::Addons => Self::Addons,
            DomainBackupGroup::Wtf => Self::Wtf,
            DomainBackupGroup::Fonts => Self::Fonts,
            DomainBackupGroup::InterfaceAssets => Self::InterfaceAssets,
        }
    }
}

impl From<BackupGroupValue> for DomainBackupGroup {
    fn from(value: BackupGroupValue) -> Self {
        match value {
            BackupGroupValue::Addons => Self::Addons,
            BackupGroupValue::Wtf => Self::Wtf,
            BackupGroupValue::Fonts => Self::Fonts,
            BackupGroupValue::InterfaceAssets => Self::InterfaceAssets,
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
pub enum HelperStrategyValue {
    NativeRust,
}

impl From<DomainHelperStrategy> for HelperStrategyValue {
    fn from(value: DomainHelperStrategy) -> Self {
        match value {
            DomainHelperStrategy::NativeRust => Self::NativeRust,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalPackageWarningCategoryValue {
    Addon,
    Wtf,
}

impl ExternalPackageWarningCategoryValue {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Addon => "addon",
            Self::Wtf => "wtf",
        }
    }
}

impl From<DomainExternalPackageWarningCategory> for ExternalPackageWarningCategoryValue {
    fn from(value: DomainExternalPackageWarningCategory) -> Self {
        match value {
            DomainExternalPackageWarningCategory::Addon => Self::Addon,
            DomainExternalPackageWarningCategory::Wtf => Self::Wtf,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalPackageWarningCodeValue {
    AddonRootNotDetected,
    UnsupportedWtfLayout,
    UnsupportedWtfRootSavedVariables,
    WtfAccountPathWithoutFile,
    WtfSavedVariablesPathWithoutFile,
    UnsupportedWtfNestedAccountLayout,
}

impl ExternalPackageWarningCodeValue {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AddonRootNotDetected => "addon_root_not_detected",
            Self::UnsupportedWtfLayout => "unsupported_wtf_layout",
            Self::UnsupportedWtfRootSavedVariables => "unsupported_wtf_root_savedvariables",
            Self::WtfAccountPathWithoutFile => "wtf_account_path_without_file",
            Self::WtfSavedVariablesPathWithoutFile => "wtf_savedvariables_path_without_file",
            Self::UnsupportedWtfNestedAccountLayout => "unsupported_wtf_nested_account_layout",
        }
    }
}

impl From<DomainExternalPackageWarningCode> for ExternalPackageWarningCodeValue {
    fn from(value: DomainExternalPackageWarningCode) -> Self {
        match value {
            DomainExternalPackageWarningCode::AddonRootNotDetected => Self::AddonRootNotDetected,
            DomainExternalPackageWarningCode::UnsupportedWtfLayout => Self::UnsupportedWtfLayout,
            DomainExternalPackageWarningCode::UnsupportedWtfRootSavedVariables => {
                Self::UnsupportedWtfRootSavedVariables
            }
            DomainExternalPackageWarningCode::WtfAccountPathWithoutFile => {
                Self::WtfAccountPathWithoutFile
            }
            DomainExternalPackageWarningCode::WtfSavedVariablesPathWithoutFile => {
                Self::WtfSavedVariablesPathWithoutFile
            }
            DomainExternalPackageWarningCode::UnsupportedWtfNestedAccountLayout => {
                Self::UnsupportedWtfNestedAccountLayout
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::manifest::ResourceApplyPolicy;

    #[test]
    fn host_platform_value_roundtrips_domain_shape() {
        let value = HostPlatformValue::MacOs;

        let domain: DomainHostPlatform = value.into();

        assert_eq!(HostPlatformValue::from(domain), value);
    }

    #[test]
    fn wow_flavor_value_roundtrips_domain_shape() {
        let value = WowFlavorValue::ClassicEra;

        let domain: DomainWowFlavor = value.into();

        assert_eq!(WowFlavorValue::from(domain), value);
    }

    #[test]
    fn wow_flavor_value_helpers_return_stable_strings() {
        assert_eq!(WowFlavorValue::Retail.as_str(), "retail");
        assert_eq!(WowFlavorValue::ClassicEra.as_str(), "classic_era");
        assert_eq!(WowFlavorValue::ClassicEra.folder_name(), "_classic_era_");
    }

    #[test]
    fn character_mapping_mode_value_roundtrips_domain_shape() {
        let value = CharacterMappingModeValue::Prompt;

        let domain: DomainCharacterMappingMode = value.into();

        assert_eq!(CharacterMappingModeValue::from(domain), value);
    }

    #[test]
    fn health_status_value_roundtrips_domain_shape() {
        let value = HealthStatusValue::Warning;

        let domain: DomainHealthStatus = value.into();

        assert_eq!(HealthStatusValue::from(domain), value);
    }

    #[test]
    fn addon_package_metadata_value_roundtrips_domain_shape() {
        let value = AddonPackageMetadataValue {
            index_name: Some("curated".to_string()),
            index_package_id: Some("weakauras".to_string()),
            package_name: Some("WeakAuras".to_string()),
            version: Some("1.2.3".to_string()),
            source_url: Some("https://example.invalid/weakauras.zip".to_string()),
            website_url: Some("https://example.invalid/weakauras".to_string()),
            source_sha256: Some("abc123".to_string()),
            supported_flavors: vec!["retail".to_string(), "classic".to_string()],
        };

        let domain: DomainAddonPackageMetadata = value.clone().into();

        assert_eq!(AddonPackageMetadataValue::from(domain), value);
    }

    #[test]
    fn bundle_apply_mappings_value_roundtrips_domain_shape() {
        let value = BundleApplyMappingsValue {
            target_account: Some("AccountA".to_string()),
            target_server: Some("Illidan".to_string()),
            target_character: Some("Main".to_string()),
            selected_accounts: vec!["AccountA".to_string(), "AccountB".to_string()],
            all_accounts: true,
            characters: vec![BundleCharacterMappingOverrideValue {
                source_account: Some("SourceAccount".to_string()),
                source_server: "Stormrage".to_string(),
                source_character: "SourceMain".to_string(),
                target_account: Some("TargetAccount".to_string()),
                target_server: "Illidan".to_string(),
                target_character: "TargetMain".to_string(),
            }],
        };

        let domain: DomainBundleApplyMappings = value.clone().into();

        assert_eq!(BundleApplyMappingsValue::from(domain), value);
    }

    #[test]
    fn bundle_apply_defaults_value_author_package_defaults_match_shared_profile() {
        let defaults = BundleApplyDefaultsValue::author_package_defaults();

        assert!(defaults.create_backup);
        assert_eq!(defaults.addons, ResourceApplyPolicyValue::Mirror);
        assert_eq!(defaults.wtf_common, ResourceApplyPolicyValue::Share);
        assert_eq!(
            defaults.wtf_characters,
            ResourceApplyPolicyValue::ReplaceSelected
        );
        assert_eq!(defaults.fonts, ResourceApplyPolicyValue::Mirror);
        assert_eq!(defaults.interface_assets, ResourceApplyPolicyValue::Mirror);
    }

    #[test]
    fn bundle_apply_defaults_value_converts_back_to_domain_defaults() {
        let value = BundleApplyDefaultsValue {
            create_backup: false,
            addons: ResourceApplyPolicyValue::Merge,
            wtf_common: ResourceApplyPolicyValue::Share,
            wtf_characters: ResourceApplyPolicyValue::Sync,
            fonts: ResourceApplyPolicyValue::Preserve,
            interface_assets: ResourceApplyPolicyValue::ReplaceSelected,
        };

        let domain: DomainApplyDefaults = value.clone().into();

        assert!(!domain.create_backup);
        assert_eq!(domain.addons, ResourceApplyPolicy::Merge);
        assert_eq!(domain.wtf_common, ResourceApplyPolicy::Share);
        assert_eq!(domain.wtf_characters, ResourceApplyPolicy::Sync);
        assert_eq!(domain.fonts, ResourceApplyPolicy::Preserve);
        assert_eq!(
            domain.interface_assets,
            ResourceApplyPolicy::ReplaceSelected
        );
        assert_eq!(BundleApplyDefaultsValue::from(domain), value);
    }

    #[test]
    fn bundle_manifest_value_roundtrips_domain_shape() {
        let value = BundleManifestValue {
            schema_version: 1,
            package: BundlePackageValue {
                id: "author-ui".to_string(),
                name: "Author UI".to_string(),
                created_by: "tester".to_string(),
                description: Some("fixture manifest".to_string()),
            },
            source: BundleSourceValue {
                flavor: WowFlavorValue::Retail,
                platform: Some(HostPlatformValue::Windows),
                exported_at: Some("2026-04-18T10:00:00Z".to_string()),
                supported_targets: vec![WowFlavorValue::Retail, WowFlavorValue::Classic],
            },
            resources: BundleResourcesValue {
                addons: vec!["WeakAuras".to_string()],
                wtf_common: true,
                wtf_characters: vec![BundleCharacterResourceValue {
                    source_account: Some("AccountA".to_string()),
                    source_server: "Illidan".to_string(),
                    source_character: "Main".to_string(),
                    target_hint: Some("Main".to_string()),
                }],
                fonts: true,
                interface_assets: vec!["Interface/Buttons".to_string()],
                addon_lock: true,
                addon_indexes: vec!["metadata/addons/index.toml".to_string()],
            },
            mapping: BundleMappingRulesValue {
                character_mode: CharacterMappingModeValue::Explicit,
                rewrite_profile_keys: true,
                rewrite_identity_strings: true,
                allow_cross_platform: true,
            },
            apply: BundleApplyDefaultsValue {
                create_backup: true,
                addons: ResourceApplyPolicyValue::Mirror,
                wtf_common: ResourceApplyPolicyValue::Share,
                wtf_characters: ResourceApplyPolicyValue::ReplaceSelected,
                fonts: ResourceApplyPolicyValue::Mirror,
                interface_assets: ResourceApplyPolicyValue::Mirror,
            },
        };

        let domain: DomainBundleManifest = value.clone().into();

        assert_eq!(BundleManifestValue::from(domain), value);
    }
}
