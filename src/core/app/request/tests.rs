use std::path::PathBuf;

use super::RuntimeDefaultableRequest;
use crate::core::addon::index::{
    AddonIndexAttachRequest as DomainAddonIndexAttachRequest,
    AddonIndexRelinkRequest as DomainAddonIndexRelinkRequest,
};
use crate::core::addon::policy::{
    AddonPolicyPin as DomainAddonPolicyPin,
    RemoveAddonPolicyRequest as DomainRemoveAddonPolicyRequest,
    SetAddonPolicyRequest as DomainSetAddonPolicyRequest,
};
use crate::core::addon::{
    InstallAddonRequest as DomainInstallAddonRequest,
    RelinkAddonRequest as DomainRelinkAddonRequest,
};
use crate::core::app::{
    AddonLockSourceOverrideRequest, AddonPackageMetadataValue, AddonPolicyPinValue,
    AddonReleaseChannelValue, AdoptAddonsAppRequest, AppRuntime, ApplyAddonLockAppRequest,
    ApplyBundleAddonLockAppRequest, ApplyBundleAppRequest, ApplyConfigAppRequest,
    ApplyExternalPackageAppRequest, AttachAddonIndexAppRequest, BackupGroupValue,
    BundleApplyDefaultsValue, BundleApplyMappingsValue, BundleCharacterMappingOverrideValue,
    BundleCharacterResourceValue, BundleManifestValue, BundleMappingRulesValue, BundlePackageValue,
    BundleResourcesValue, BundleSourceValue, CharacterMappingModeValue, ConfigPackageAppRequest,
    CreateBackupAppRequest, CreateExternalPackageBundleAppRequest, ExternalPackageLayoutValue,
    ExternalPackageSharingModeValue, HostPlatformValue, InspectAddonPolicyRequest,
    InspectConfigAppRequest, InstallAddonAppRequest, InstallAddonIndexAppRequest,
    ListAddonsRequest, ListBackupsRequest, PackBundleAppRequest, PlanAddonLockSyncRequest,
    PlanBundleApplyRequest, PlanConfigApplyAppRequest, PlanExternalPackageApplyAppRequest,
    RelinkAddonAppRequest, RelinkAddonIndexAppRequest, RemoveAddonAppRequest,
    RemoveAddonPolicyAppRequest, ResolvedInstallationValue, ResourceApplyPolicyValue,
    RestoreBackupAppRequest, SetAddonPolicyAppRequest, UpdateAddonAppRequest,
    UpdateAddonIndexAppRequest, WowFlavorValue, WtfScopeValue,
};
use crate::core::bundle::{
    CreateExternalPackageBundleRequest as DomainCreateExternalPackageBundleRequest,
    PackBundleRequest as DomainPackBundleRequest, UnpackBundleRequest as DomainUnpackBundleRequest,
};
use crate::core::manifest::{CharacterMappingMode, ResourceApplyPolicy};

mod addon;
mod addon_index;
mod addon_lock;
mod addon_policy;
mod bundle;
mod defaults;
mod external_package;
mod installation;

fn runtime_with_relative_path_base(base: PathBuf) -> AppRuntime {
    AppRuntime::builder()
        .with_relative_path_base(Some(base))
        .build()
        .expect("runtime")
}

fn runtime_with_default_backup_dir(default_backup_dir: PathBuf) -> AppRuntime {
    AppRuntime::builder()
        .with_default_backup_dir(Some(default_backup_dir))
        .build()
        .expect("runtime")
}

fn runtime_with_default_dirs(
    default_backup_dir: PathBuf,
    default_bundle_output_dir: PathBuf,
) -> AppRuntime {
    AppRuntime::builder()
        .with_default_backup_dir(Some(default_backup_dir))
        .with_default_bundle_output_dir(Some(default_bundle_output_dir))
        .build()
        .expect("runtime")
}

fn sample_installation() -> ResolvedInstallationValue {
    let product_root = std::env::current_dir()
        .expect("cwd")
        .join("World of Warcraft");
    let flavor_root = product_root.join("_retail_");
    let interface_dir = flavor_root.join("Interface");

    ResolvedInstallationValue {
        platform: HostPlatformValue::Windows,
        flavor: WowFlavorValue::Retail,
        product_root,
        flavor_root: flavor_root.clone(),
        interface_dir: interface_dir.clone(),
        addon_dir: interface_dir.join("AddOns"),
        wtf_dir: flavor_root.join("WTF"),
        fonts_dir: flavor_root.join("Fonts"),
    }
}

fn sample_manifest() -> BundleManifestValue {
    BundleManifestValue {
        schema_version: 1,
        package: BundlePackageValue {
            id: "author-ui".to_string(),
            name: "Author UI".to_string(),
            created_by: "tester".to_string(),
            description: Some("fixture".to_string()),
        },
        source: BundleSourceValue {
            flavor: WowFlavorValue::Retail,
            platform: Some(HostPlatformValue::Windows),
            exported_at: None,
            supported_targets: vec![WowFlavorValue::Retail],
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
            interface_assets: vec!["Buttons".to_string()],
            addon_lock: false,
            addon_indexes: Vec::new(),
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
    }
}

fn sample_external_package_bundle_request() -> CreateExternalPackageBundleAppRequest {
    CreateExternalPackageBundleAppRequest {
        source_path: PathBuf::from("author-ui.zip"),
        layout: ExternalPackageLayoutValue::Auto,
        source_account: None,
        source_server: None,
        source_character: None,
        source_flavor: WowFlavorValue::Retail,
        source_platform: None,
        supported_targets: vec![WowFlavorValue::Retail],
        output_path: None,
        package_id: Some("author-ui".to_string()),
        package_name: Some("Author UI".to_string()),
        created_by: Some("tester".to_string()),
        description: Some("fixture".to_string()),
        apply_defaults: None,
        sharing_mode: ExternalPackageSharingModeValue::Private,
        allow_public_sharing_risks: false,
        excluded_wtf_scopes: Vec::new(),
    }
}

fn sample_config_package_request() -> ConfigPackageAppRequest {
    ConfigPackageAppRequest {
        source_path: PathBuf::from("author-ui.zip"),
        source_flavor: WowFlavorValue::Retail,
        source_platform: None,
        supported_targets: vec![WowFlavorValue::Retail],
        output_path: None,
        package_id: Some("author-ui".to_string()),
        package_name: Some("Author UI".to_string()),
        created_by: Some("tester".to_string()),
        description: Some("fixture".to_string()),
        apply_defaults: None,
    }
}
