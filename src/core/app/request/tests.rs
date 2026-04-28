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
    CreateBackupAppRequest, CreateExternalPackageBundleAppRequest, HostPlatformValue,
    InspectAddonPolicyRequest, InspectConfigAppRequest, InstallAddonAppRequest,
    InstallAddonIndexAppRequest, ListAddonsRequest, ListBackupsRequest, PackBundleAppRequest,
    PlanAddonLockSyncRequest, PlanBundleApplyRequest, PlanConfigApplyAppRequest,
    PlanExternalPackageApplyAppRequest, RelinkAddonAppRequest, RelinkAddonIndexAppRequest,
    RemoveAddonAppRequest, RemoveAddonPolicyAppRequest, ResolvedInstallationValue,
    ResourceApplyPolicyValue, RestoreBackupAppRequest, SetAddonPolicyAppRequest,
    UpdateAddonAppRequest, UpdateAddonIndexAppRequest, WowFlavorValue,
};
use crate::core::bundle::{
    CreateExternalPackageBundleRequest as DomainCreateExternalPackageBundleRequest,
    PackBundleRequest as DomainPackBundleRequest, UnpackBundleRequest as DomainUnpackBundleRequest,
};
use crate::core::manifest::{CharacterMappingMode, ResourceApplyPolicy};

#[test]
fn addon_family_requests_apply_runtime_backup_defaults() {
    let base = std::env::current_dir().expect("cwd");
    let backup_dir = base.join("runtime-backups");
    let runtime = runtime_with_default_backup_dir(backup_dir.clone());

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

    assert_eq!(install.backup_output_path, Some(backup_dir.clone()));
    assert_eq!(update.backup_output_path, Some(backup_dir.clone()));
    assert_eq!(remove.backup_output_path, Some(backup_dir.clone()));
    assert_eq!(index_install.backup_output_path, Some(backup_dir.clone()));
    assert_eq!(index_update.backup_output_path, Some(backup_dir.clone()));
    assert_eq!(lock_apply.backup_output_path, Some(backup_dir));
}

#[test]
fn backup_requests_apply_runtime_defaults() {
    let base = std::env::current_dir().expect("cwd");
    let backup_dir = base.join("runtime-backups");
    let runtime = runtime_with_default_backup_dir(backup_dir.clone());

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

    assert_eq!(list.backup_dir, Some(backup_dir.clone()));
    assert_eq!(create.output_path, Some(backup_dir.clone()));
    assert_eq!(restore.backup_dir, Some(backup_dir));
}

#[test]
fn bundle_requests_apply_runtime_defaults() {
    let base = std::env::current_dir().expect("cwd");
    let backup_dir = base.join("runtime-backups");
    let bundle_dir = base.join("runtime-bundles");
    let runtime = runtime_with_default_dirs(backup_dir.clone(), bundle_dir.clone());

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

    assert_eq!(pack.output_path, Some(bundle_dir));
    assert_eq!(apply.backup_output_path, Some(backup_dir.clone()));
    assert_eq!(addon_lock.backup_output_path, Some(backup_dir));
}

#[test]
fn external_package_requests_apply_runtime_defaults() {
    let base = std::env::current_dir().expect("cwd");
    let backup_dir = base.join("runtime-backups");
    let bundle_dir = base.join("runtime-bundles");
    let runtime = AppRuntime::builder()
        .with_host_platform(HostPlatformValue::MacOs)
        .with_default_backup_dir(Some(backup_dir.clone()))
        .with_default_bundle_output_dir(Some(bundle_dir.clone()))
        .build()
        .expect("runtime");

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
    assert_eq!(bundle_request.output_path, Some(bundle_dir.clone()));
    assert_eq!(
        plan_request.external_package.source_platform,
        Some(HostPlatformValue::MacOs)
    );
    assert_eq!(apply_request.external_package.output_path, Some(bundle_dir));
    assert_eq!(apply_request.backup_output_path, Some(backup_dir));
}

#[test]
fn config_requests_apply_runtime_defaults() {
    let base = std::env::current_dir().expect("cwd");
    let backup_dir = base.join("runtime-backups");
    let bundle_dir = base.join("runtime-bundles");
    let runtime = AppRuntime::builder()
        .with_host_platform(HostPlatformValue::MacOs)
        .with_default_backup_dir(Some(backup_dir.clone()))
        .with_default_bundle_output_dir(Some(bundle_dir.clone()))
        .build()
        .expect("runtime");

    let inspect = InspectConfigAppRequest {
        source_path: PathBuf::from("author-ui.zip"),
    };
    let config_package = sample_config_package_request().apply_runtime_defaults(&runtime);
    let plan_request = PlanConfigApplyAppRequest {
        config_package: sample_config_package_request(),
        installation: sample_installation(),
        apply_mappings: BundleApplyMappingsValue::default(),
    }
    .apply_runtime_defaults(&runtime);
    let apply_request = ApplyConfigAppRequest {
        config_package: sample_config_package_request(),
        installation: sample_installation(),
        dry_run: false,
        backup_output_path: None,
        apply_mappings: BundleApplyMappingsValue::default(),
    }
    .apply_runtime_defaults(&runtime);

    assert_eq!(inspect.source_path, PathBuf::from("author-ui.zip"));
    assert_eq!(
        config_package.source_platform,
        Some(HostPlatformValue::MacOs)
    );
    assert_eq!(config_package.output_path, Some(bundle_dir.clone()));
    assert_eq!(
        plan_request.config_package.source_platform,
        Some(HostPlatformValue::MacOs)
    );
    assert_eq!(apply_request.config_package.output_path, Some(bundle_dir));
    assert_eq!(apply_request.backup_output_path, Some(backup_dir));
}

#[test]
fn runtime_backed_request_helpers_compose_defaults_and_domain_projection() {
    let base = std::env::current_dir().expect("cwd");
    let runtime = AppRuntime::builder()
        .with_host_platform(HostPlatformValue::MacOs)
        .with_relative_path_base(Some(base.clone()))
        .with_default_backup_dir(Some(PathBuf::from("runtime-backups")))
        .with_default_bundle_output_dir(Some(PathBuf::from("runtime-bundles")))
        .build()
        .expect("runtime");

    let install = InstallAddonAppRequest {
        installation: sample_installation(),
        source: "https://example.invalid/weakauras.zip".to_string(),
        dry_run: false,
        backup_output_path: None,
        replace_existing: true,
        metadata: None,
    }
    .into_domain_request(&runtime)
    .expect("install domain request");
    let backup_dir = ListBackupsRequest { backup_dir: None }
        .into_backup_dir(&runtime)
        .expect("backup dir");
    let external_bundle = sample_external_package_bundle_request()
        .into_domain_request(&runtime)
        .expect("external package domain request");

    assert_eq!(
        install.backup_output_path,
        Some(base.join("runtime-backups"))
    );
    assert_eq!(backup_dir, Some(base.join("runtime-backups")));
    assert_eq!(
        external_bundle.source_platform,
        Some(crate::core::install::HostPlatform::MacOs)
    );
    assert_eq!(
        external_bundle.output_path,
        Some(base.join("runtime-bundles"))
    );
    assert_eq!(external_bundle.source_path, base.join("author-ui.zip"));
}

#[test]
fn thin_installation_requests_project_domain_inputs() {
    let installation = sample_installation();
    let base = std::env::current_dir().expect("cwd");
    let runtime = runtime_with_relative_path_base(base.clone());
    let domain_installation = ListAddonsRequest {
        installation: installation.clone(),
    }
    .into_domain_installation()
    .expect("list request");
    let (lock_installation, _lock_state_paths, lock_path) = PlanAddonLockSyncRequest {
        installation: installation.clone(),
        lock_path: Some(PathBuf::from("lock.toml")),
    }
    .into_domain_inputs(&runtime)
    .expect("lock request");
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
    .into_domain_inputs(&runtime)
    .expect("bundle request");
    let expected_installation = sample_installation();

    assert_eq!(
        domain_installation.product_root,
        expected_installation.product_root
    );
    assert_eq!(
        lock_installation.flavor_root,
        expected_installation.flavor_root
    );
    assert_eq!(lock_path, Some(base.join("lock.toml")));
    assert_eq!(bundle_path, base.join("bundle.zip"));
    assert_eq!(
        bundle_installation.addon_dir,
        expected_installation.addon_dir
    );
    assert_eq!(apply_mappings.target_account.as_deref(), Some("AccountA"));
    assert_eq!(apply_mappings.target_server.as_deref(), Some("Illidan"));
    assert_eq!(apply_mappings.target_character.as_deref(), Some("Main"));
}

#[test]
fn apply_addon_lock_request_resolves_relative_lock_and_source_overrides() {
    let base = std::env::current_dir().expect("cwd");
    let runtime = runtime_with_relative_path_base(base.clone());

    let domain = ApplyAddonLockAppRequest {
        installation: sample_installation(),
        lock_path: Some(PathBuf::from("locks/addons.lock.toml")),
        backup_output_path: Some(PathBuf::from("backup")),
        replace_existing: true,
        source_overrides: vec![AddonLockSourceOverrideRequest {
            comparison_key: "addons:details".to_string(),
            archive_path: PathBuf::from("sources/Details.zip"),
        }],
    }
    .into_domain_request(&runtime)
    .expect("addon lock apply request");

    assert_eq!(domain.lock_path, Some(base.join("locks/addons.lock.toml")));
    assert_eq!(domain.backup_output_path, Some(base.join("backup")));
    assert_eq!(
        domain.source_overrides[0].archive_path,
        base.join("sources/Details.zip")
    );
    assert_eq!(domain.source_overrides[0].comparison_key, "addons:details");
}

#[test]
fn addon_policy_requests_project_domain_inputs() {
    let runtime = AppRuntime::new();
    let (inspection_installation, _inspection_state_paths) = InspectAddonPolicyRequest {
        installation: sample_installation(),
    }
    .into_domain_inputs(&runtime)
    .expect("inspection request");
    let set_request: DomainSetAddonPolicyRequest = SetAddonPolicyAppRequest {
        installation: sample_installation(),
        package: "WeakAuras".to_string(),
        ignored: Some(true),
        pin: Some(AddonPolicyPinValue::Version {
            value: "2.0.0".to_string(),
        }),
        release_channel: Some(AddonReleaseChannelValue::Beta),
        allow_prerelease: Some(true),
        install_dependencies: Some(false),
    }
    .into_domain_request(&runtime)
    .expect("set request");
    let remove_request: DomainRemoveAddonPolicyRequest = RemoveAddonPolicyAppRequest {
        installation: sample_installation(),
        package: "WeakAuras".to_string(),
    }
    .into_domain_request(&runtime)
    .expect("remove request");

    assert_eq!(
        inspection_installation.product_root,
        sample_installation().product_root
    );
    assert_eq!(set_request.package, "WeakAuras");
    assert_eq!(set_request.ignored, Some(true));
    assert_eq!(
        set_request.release_channel,
        Some(crate::core::addon::policy::AddonReleaseChannel::Beta)
    );
    assert_eq!(set_request.allow_prerelease, Some(true));
    assert_eq!(set_request.install_dependencies, Some(false));
    assert_eq!(set_request.pinned_version, Some("2.0.0".to_string()));
    assert_eq!(set_request.pinned_file_id, None);
    assert_eq!(remove_request.package, "WeakAuras");
}

#[test]
fn adopt_addons_request_resolves_relative_archive_output() {
    let base = std::env::current_dir().expect("cwd");
    let runtime = runtime_with_relative_path_base(base.clone());

    let domain = AdoptAddonsAppRequest {
        installation: sample_installation(),
        addon_directories: vec!["WeakAuras".to_string()],
        package_id: Some("weak-auras".to_string()),
        archive_output_path: Some(PathBuf::from("snapshots/WeakAuras.zip")),
        dry_run: true,
    }
    .into_domain_request(&runtime)
    .expect("adopt addons request");

    assert_eq!(
        domain.archive_output_path,
        Some(base.join("snapshots/WeakAuras.zip"))
    );
}

#[test]
fn apply_bundle_request_converts_app_owned_apply_mappings() {
    let base = std::env::current_dir().expect("cwd");
    let runtime = runtime_with_relative_path_base(base.clone());

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
    .into_domain_request(&runtime)
    .expect("apply bundle request");

    assert_eq!(domain.bundle_path, base.join("bundle.zip"));
    assert_eq!(domain.backup_output_path, Some(base.join("backup")));
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
    let base = std::env::current_dir().expect("cwd");
    let runtime = runtime_with_relative_path_base(base.clone());

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
    .into_domain_request(&runtime)
    .expect("external package request");

    assert_eq!(domain.source_path, base.join("author-ui.zip"));
    assert_eq!(domain.output_path, Some(base.join("out")));
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
    let base = std::env::current_dir().expect("cwd");
    let runtime = runtime_with_relative_path_base(base.clone());

    let domain: DomainPackBundleRequest = PackBundleAppRequest {
        installation: sample_installation(),
        manifest: sample_manifest(),
        output_path: Some(PathBuf::from("bundle.zip")),
        manifest_base_dir: Some(PathBuf::from("manifest-dir")),
    }
    .into_domain_request(&runtime)
    .expect("pack bundle request");

    assert_eq!(domain.manifest_base_dir, Some(base.join("manifest-dir")));
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
    let base = std::env::current_dir().expect("cwd");
    let runtime = runtime_with_relative_path_base(base.clone());

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
    .into_domain_request(&runtime)
    .expect("install request");

    assert_eq!(domain.backup_output_path, Some(base.join("backup")));
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

#[test]
fn relink_addon_request_projects_domain_inputs() {
    let runtime = AppRuntime::new();
    let domain: DomainRelinkAddonRequest = RelinkAddonAppRequest {
        installation: sample_installation(),
        name: "WeakAuras".to_string(),
        source: "github:WeakAuras/WeakAuras2".to_string(),
        dry_run: true,
    }
    .into_domain_request(&runtime)
    .expect("relink request");

    assert_eq!(domain.name, "WeakAuras");
    assert_eq!(domain.source, "github:WeakAuras/WeakAuras2");
    assert!(domain.dry_run);
    assert_eq!(
        domain.installation.addon_dir,
        sample_installation().addon_dir
    );
}

#[test]
fn relink_addon_index_request_projects_domain_inputs() {
    let base = std::env::current_dir().expect("cwd");
    let runtime = runtime_with_relative_path_base(base.clone());
    let domain: DomainAddonIndexRelinkRequest = RelinkAddonIndexAppRequest {
        installation: sample_installation(),
        index_path: PathBuf::from("addons.index.toml"),
        name: "details".to_string(),
        target: Some("details-local".to_string()),
        dry_run: true,
    }
    .into_domain_request(&runtime)
    .expect("relink addon index request");

    assert_eq!(domain.index_path, base.join("addons.index.toml"));
    assert_eq!(domain.name, "details");
    assert_eq!(domain.target.as_deref(), Some("details-local"));
    assert!(domain.dry_run);
}

#[test]
fn attach_addon_index_request_projects_domain_inputs() {
    let base = std::env::current_dir().expect("cwd");
    let runtime = runtime_with_relative_path_base(base.clone());
    let domain: DomainAddonIndexAttachRequest = AttachAddonIndexAppRequest {
        installation: sample_installation(),
        index_path: PathBuf::from("addons.index.toml"),
        name: Some("details".to_string()),
        dry_run: true,
        apply_ready_only: true,
    }
    .into_domain_request(&runtime)
    .expect("attach addon index request");

    assert_eq!(domain.index_path, base.join("addons.index.toml"));
    assert_eq!(domain.name.as_deref(), Some("details"));
    assert!(domain.dry_run);
    assert!(domain.apply_ready_only);
}

#[test]
fn addon_policy_request_converts_file_id_pin() {
    let runtime = AppRuntime::new();
    let domain: DomainSetAddonPolicyRequest = SetAddonPolicyAppRequest {
        installation: sample_installation(),
        package: "details".to_string(),
        ignored: Some(false),
        pin: Some(AddonPolicyPinValue::FileId { value: 123 }),
        release_channel: Some(AddonReleaseChannelValue::Stable),
        allow_prerelease: None,
        install_dependencies: Some(true),
    }
    .into_domain_request(&runtime)
    .expect("addon policy request");

    assert_eq!(domain.package, "details");
    assert_eq!(domain.pinned_version, None);
    assert_eq!(domain.pinned_file_id, Some(123));
    assert_eq!(
        AddonPolicyPinValue::from_domain(DomainAddonPolicyPin::FileId { value: 123 }),
        AddonPolicyPinValue::FileId { value: 123 }
    );
}

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
