use std::path::PathBuf;

use super::*;
use crate::core::addon::InstallAddonRequest as DomainInstallAddonRequest;
use crate::core::app::{
    AddonPackageMetadataValue, AppRuntime, BackupGroupValue, BundleApplyDefaultsValue,
    BundleApplyMappingsValue, BundleCharacterMappingOverrideValue, BundleCharacterResourceValue,
    BundleManifestValue, BundleMappingRulesValue, BundlePackageValue, BundleResourcesValue,
    BundleSourceValue, CharacterMappingModeValue, HostPlatformValue, ResolvedInstallationValue,
    ResourceApplyPolicyValue, WowFlavorValue,
};
use crate::core::bundle::{
    CreateExternalPackageBundleRequest as DomainCreateExternalPackageBundleRequest,
    PackBundleRequest as DomainPackBundleRequest, UnpackBundleRequest as DomainUnpackBundleRequest,
};
use crate::core::manifest::{CharacterMappingMode, ResourceApplyPolicy};

#[test]
fn addon_family_requests_apply_runtime_backup_defaults() {
    let runtime = AppRuntime::new().with_default_backup_dir(Some(PathBuf::from("runtime-backups")));

    let install = InstallAddonAppRequest {
        installation: sample_installation(),
        source: "https://example.invalid/weakauras.zip".to_string(),
        dry_run: false,
        backup_output_path: None,
        replace_existing: true,
        metadata: None,
    }
    .apply_runtime_defaults(&runtime);
    let update = UpdateAddonAppRequest {
        installation: sample_installation(),
        name: Some("WeakAuras".to_string()),
        dry_run: false,
        backup_output_path: None,
    }
    .apply_runtime_defaults(&runtime);
    let remove = RemoveAddonAppRequest {
        installation: sample_installation(),
        name: "WeakAuras".to_string(),
        dry_run: false,
        backup_output_path: None,
    }
    .apply_runtime_defaults(&runtime);
    let index_install = InstallAddonIndexAppRequest {
        installation: sample_installation(),
        index_path: PathBuf::from("addon-index.toml"),
        name: "WeakAuras".to_string(),
        dry_run: false,
        backup_output_path: None,
        replace_existing: true,
    }
    .apply_runtime_defaults(&runtime);
    let index_update = UpdateAddonIndexAppRequest {
        installation: sample_installation(),
        index_path: PathBuf::from("addon-index.toml"),
        name: None,
        dry_run: false,
        backup_output_path: None,
    }
    .apply_runtime_defaults(&runtime);
    let lock_apply = ApplyAddonLockAppRequest {
        installation: sample_installation(),
        lock_path: None,
        backup_output_path: None,
        replace_existing: true,
        source_overrides: Vec::new(),
    }
    .apply_runtime_defaults(&runtime);

    assert_eq!(
        install.backup_output_path,
        Some(PathBuf::from("runtime-backups"))
    );
    assert_eq!(
        update.backup_output_path,
        Some(PathBuf::from("runtime-backups"))
    );
    assert_eq!(
        remove.backup_output_path,
        Some(PathBuf::from("runtime-backups"))
    );
    assert_eq!(
        index_install.backup_output_path,
        Some(PathBuf::from("runtime-backups"))
    );
    assert_eq!(
        index_update.backup_output_path,
        Some(PathBuf::from("runtime-backups"))
    );
    assert_eq!(
        lock_apply.backup_output_path,
        Some(PathBuf::from("runtime-backups"))
    );
}

#[test]
fn backup_requests_apply_runtime_defaults() {
    let runtime = AppRuntime::new().with_default_backup_dir(Some(PathBuf::from("runtime-backups")));

    let list = ListBackupsRequest { backup_dir: None }.apply_runtime_defaults(&runtime);
    let create = CreateBackupAppRequest {
        installation: sample_installation(),
        output_path: None,
        groups: vec![BackupGroupValue::Addons],
        label: Some("nightly".to_string()),
    }
    .apply_runtime_defaults(&runtime);
    let restore = RestoreBackupAppRequest {
        installation: sample_installation(),
        archive_path: None,
        backup_id: Some("backup-001".to_string()),
        backup_dir: None,
    }
    .apply_runtime_defaults(&runtime);

    assert_eq!(list.backup_dir, Some(PathBuf::from("runtime-backups")));
    assert_eq!(create.output_path, Some(PathBuf::from("runtime-backups")));
    assert_eq!(restore.backup_dir, Some(PathBuf::from("runtime-backups")));
}

#[test]
fn bundle_requests_apply_runtime_defaults() {
    let runtime = AppRuntime::new()
        .with_default_backup_dir(Some(PathBuf::from("runtime-backups")))
        .with_default_bundle_output_dir(Some(PathBuf::from("runtime-bundles")));

    let pack = PackBundleAppRequest {
        installation: sample_installation(),
        manifest: sample_manifest(),
        output_path: None,
        manifest_base_dir: None,
    }
    .apply_runtime_defaults(&runtime);
    let apply = ApplyBundleAppRequest {
        bundle_path: PathBuf::from("bundle.zip"),
        installation: sample_installation(),
        dry_run: false,
        backup_output_path: None,
        apply_mappings: BundleApplyMappingsValue::default(),
    }
    .apply_runtime_defaults(&runtime);
    let addon_lock = ApplyBundleAddonLockAppRequest {
        bundle_path: PathBuf::from("bundle.zip"),
        installation: sample_installation(),
        backup_output_path: None,
        replace_existing: true,
    }
    .apply_runtime_defaults(&runtime);

    assert_eq!(pack.output_path, Some(PathBuf::from("runtime-bundles")));
    assert_eq!(
        apply.backup_output_path,
        Some(PathBuf::from("runtime-backups"))
    );
    assert_eq!(
        addon_lock.backup_output_path,
        Some(PathBuf::from("runtime-backups"))
    );
}

#[test]
fn external_package_requests_apply_runtime_defaults() {
    let runtime = AppRuntime::new()
        .with_host_platform(HostPlatformValue::MacOs)
        .with_default_backup_dir(Some(PathBuf::from("runtime-backups")))
        .with_default_bundle_output_dir(Some(PathBuf::from("runtime-bundles")));

    let bundle_request = sample_external_package_bundle_request().apply_runtime_defaults(&runtime);
    let plan_request = PlanExternalPackageApplyAppRequest {
        external_package: sample_external_package_bundle_request(),
        installation: sample_installation(),
        apply_mappings: BundleApplyMappingsValue::default(),
    }
    .apply_runtime_defaults(&runtime);
    let apply_request = ApplyExternalPackageAppRequest {
        external_package: sample_external_package_bundle_request(),
        installation: sample_installation(),
        dry_run: false,
        backup_output_path: None,
        apply_mappings: BundleApplyMappingsValue::default(),
    }
    .apply_runtime_defaults(&runtime);

    assert_eq!(
        bundle_request.source_platform,
        Some(HostPlatformValue::MacOs)
    );
    assert_eq!(
        bundle_request.output_path,
        Some(PathBuf::from("runtime-bundles"))
    );
    assert_eq!(
        plan_request.external_package.source_platform,
        Some(HostPlatformValue::MacOs)
    );
    assert_eq!(
        apply_request.external_package.output_path,
        Some(PathBuf::from("runtime-bundles"))
    );
    assert_eq!(
        apply_request.backup_output_path,
        Some(PathBuf::from("runtime-backups"))
    );
}

#[test]
fn runtime_backed_request_helpers_compose_defaults_and_domain_projection() {
    let runtime = AppRuntime::new()
        .with_host_platform(HostPlatformValue::MacOs)
        .with_default_backup_dir(Some(PathBuf::from("runtime-backups")))
        .with_default_bundle_output_dir(Some(PathBuf::from("runtime-bundles")));

    let install = InstallAddonAppRequest {
        installation: sample_installation(),
        source: "https://example.invalid/weakauras.zip".to_string(),
        dry_run: false,
        backup_output_path: None,
        replace_existing: true,
        metadata: None,
    }
    .into_domain_request(&runtime);
    let backup_dir = ListBackupsRequest { backup_dir: None }.into_backup_dir(&runtime);
    let external_bundle = sample_external_package_bundle_request().into_domain_request(&runtime);

    assert_eq!(
        install.backup_output_path,
        Some(PathBuf::from("runtime-backups"))
    );
    assert_eq!(backup_dir, Some(PathBuf::from("runtime-backups")));
    assert_eq!(
        external_bundle.source_platform,
        Some(crate::core::install::HostPlatform::MacOs)
    );
    assert_eq!(
        external_bundle.output_path,
        Some(PathBuf::from("runtime-bundles"))
    );
}

#[test]
fn thin_installation_requests_project_domain_inputs() {
    let installation = sample_installation();
    let domain_installation = ListAddonsRequest {
        installation: installation.clone(),
    }
    .into_domain_installation();
    let (lock_installation, lock_path) = PlanAddonLockSyncRequest {
        installation: installation.clone(),
        lock_path: Some(PathBuf::from("lock.toml")),
    }
    .into_domain_inputs();
    let (bundle_path, bundle_installation, apply_mappings) = PlanBundleApplyRequest {
        bundle_path: PathBuf::from("bundle.zip"),
        installation,
        apply_mappings: BundleApplyMappingsValue {
            target_account: Some("AccountA".to_string()),
            target_server: Some("Illidan".to_string()),
            target_character: Some("Main".to_string()),
            selected_accounts: vec!["AccountA".to_string()],
            all_accounts: false,
            characters: Vec::new(),
        },
    }
    .into_domain_inputs();

    assert_eq!(
        domain_installation.product_root,
        PathBuf::from("World of Warcraft")
    );
    assert_eq!(
        lock_installation.flavor_root,
        PathBuf::from("World of Warcraft/_retail_")
    );
    assert_eq!(lock_path, Some(PathBuf::from("lock.toml")));
    assert_eq!(bundle_path, PathBuf::from("bundle.zip"));
    assert_eq!(
        bundle_installation.addon_dir,
        PathBuf::from("World of Warcraft/_retail_/Interface/AddOns")
    );
    assert_eq!(apply_mappings.target_account.as_deref(), Some("AccountA"));
    assert_eq!(apply_mappings.target_server.as_deref(), Some("Illidan"));
    assert_eq!(apply_mappings.target_character.as_deref(), Some("Main"));
}

#[test]
fn apply_bundle_request_converts_app_owned_apply_mappings() {
    let runtime = AppRuntime::new();

    let domain: DomainUnpackBundleRequest = ApplyBundleAppRequest {
        bundle_path: PathBuf::from("bundle.zip"),
        installation: sample_installation(),
        dry_run: true,
        backup_output_path: Some(PathBuf::from("backup")),
        apply_mappings: BundleApplyMappingsValue {
            target_account: Some("AccountA".to_string()),
            target_server: Some("Illidan".to_string()),
            target_character: Some("Main".to_string()),
            selected_accounts: vec!["AccountA".to_string()],
            all_accounts: true,
            characters: vec![BundleCharacterMappingOverrideValue {
                source_account: Some("SourceAccount".to_string()),
                source_server: "Stormrage".to_string(),
                source_character: "SourceMain".to_string(),
                target_account: Some("TargetAccount".to_string()),
                target_server: "Illidan".to_string(),
                target_character: "TargetMain".to_string(),
            }],
        },
    }
    .into_domain_request(&runtime);

    assert_eq!(domain.bundle_path, PathBuf::from("bundle.zip"));
    assert!(domain.dry_run);
    assert_eq!(
        domain.apply_mappings.target_account.as_deref(),
        Some("AccountA")
    );
    assert_eq!(
        domain.apply_mappings.target_server.as_deref(),
        Some("Illidan")
    );
    assert_eq!(
        domain.apply_mappings.target_character.as_deref(),
        Some("Main")
    );
    assert_eq!(domain.apply_mappings.selected_accounts, vec!["AccountA"]);
    assert!(domain.apply_mappings.all_accounts);
    assert_eq!(domain.apply_mappings.characters.len(), 1);
    assert_eq!(
        domain.apply_mappings.characters[0]
            .source_account
            .as_deref(),
        Some("SourceAccount")
    );
}

#[test]
fn create_external_package_request_converts_app_owned_apply_defaults() {
    let runtime = AppRuntime::new();

    let domain: DomainCreateExternalPackageBundleRequest = CreateExternalPackageBundleAppRequest {
        source_path: PathBuf::from("author-ui.zip"),
        source_flavor: WowFlavorValue::Retail,
        source_platform: Some(HostPlatformValue::Windows),
        supported_targets: vec![WowFlavorValue::Retail, WowFlavorValue::Classic],
        output_path: Some(PathBuf::from("out")),
        package_id: Some("author-ui".to_string()),
        package_name: Some("Author UI".to_string()),
        created_by: Some("tester".to_string()),
        description: Some("normalized".to_string()),
        apply_defaults: Some(BundleApplyDefaultsValue {
            create_backup: false,
            addons: ResourceApplyPolicyValue::Mirror,
            wtf_common: ResourceApplyPolicyValue::Share,
            wtf_characters: ResourceApplyPolicyValue::ReplaceSelected,
            fonts: ResourceApplyPolicyValue::Preserve,
            interface_assets: ResourceApplyPolicyValue::Sync,
        }),
    }
    .into_domain_request(&runtime);

    let apply_defaults = domain.apply_defaults.expect("apply defaults");
    assert!(!apply_defaults.create_backup);
    assert_eq!(apply_defaults.addons, ResourceApplyPolicy::Mirror);
    assert_eq!(apply_defaults.wtf_common, ResourceApplyPolicy::Share);
    assert_eq!(
        apply_defaults.wtf_characters,
        ResourceApplyPolicy::ReplaceSelected
    );
    assert_eq!(apply_defaults.fonts, ResourceApplyPolicy::Preserve);
    assert_eq!(apply_defaults.interface_assets, ResourceApplyPolicy::Sync);
}

#[test]
fn pack_bundle_request_converts_app_owned_manifest() {
    let runtime = AppRuntime::new();

    let domain: DomainPackBundleRequest = PackBundleAppRequest {
        installation: sample_installation(),
        manifest: sample_manifest(),
        output_path: Some(PathBuf::from("bundle.zip")),
        manifest_base_dir: Some(PathBuf::from("manifest-dir")),
    }
    .into_domain_request(&runtime);

    assert_eq!(domain.manifest.schema_version, 1);
    assert_eq!(domain.manifest.package.id, "author-ui");
    assert_eq!(
        domain.manifest.source.flavor,
        crate::core::install::WowFlavor::Retail
    );
    assert_eq!(domain.manifest.resources.addons, vec!["WeakAuras"]);
    assert_eq!(domain.manifest.resources.wtf_characters.len(), 1);
    assert_eq!(
        domain.manifest.mapping.character_mode,
        CharacterMappingMode::Explicit
    );
    assert_eq!(domain.manifest.apply.addons, ResourceApplyPolicy::Mirror);
}

#[test]
fn install_addon_request_converts_app_owned_metadata() {
    let runtime = AppRuntime::new();

    let domain: DomainInstallAddonRequest = InstallAddonAppRequest {
        installation: sample_installation(),
        source: "https://example.invalid/weakauras.zip".to_string(),
        dry_run: false,
        backup_output_path: Some(PathBuf::from("backup")),
        replace_existing: true,
        metadata: Some(AddonPackageMetadataValue {
            index_name: Some("curated".to_string()),
            index_package_id: Some("weakauras".to_string()),
            package_name: Some("WeakAuras".to_string()),
            version: Some("1.2.3".to_string()),
            source_url: Some("https://example.invalid/weakauras.zip".to_string()),
            website_url: Some("https://example.invalid/weakauras".to_string()),
            source_sha256: Some("abc123".to_string()),
            supported_flavors: vec!["retail".to_string()],
        }),
    }
    .into_domain_request(&runtime);

    let metadata = domain.metadata.expect("metadata");
    assert_eq!(metadata.index_name.as_deref(), Some("curated"));
    assert_eq!(metadata.index_package_id.as_deref(), Some("weakauras"));
    assert_eq!(metadata.package_name.as_deref(), Some("WeakAuras"));
    assert_eq!(metadata.version.as_deref(), Some("1.2.3"));
    assert_eq!(
        metadata.source_url.as_deref(),
        Some("https://example.invalid/weakauras.zip")
    );
    assert_eq!(metadata.supported_flavors, vec!["retail"]);
}

fn sample_installation() -> ResolvedInstallationValue {
    ResolvedInstallationValue {
        platform: HostPlatformValue::Windows,
        flavor: WowFlavorValue::Retail,
        product_root: PathBuf::from("World of Warcraft"),
        flavor_root: PathBuf::from("World of Warcraft/_retail_"),
        interface_dir: PathBuf::from("World of Warcraft/_retail_/Interface"),
        addon_dir: PathBuf::from("World of Warcraft/_retail_/Interface/AddOns"),
        wtf_dir: PathBuf::from("World of Warcraft/_retail_/WTF"),
        fonts_dir: PathBuf::from("World of Warcraft/_retail_/Fonts"),
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
            interface_assets: vec!["Interface/Buttons".to_string()],
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
