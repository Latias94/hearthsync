use super::*;

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
