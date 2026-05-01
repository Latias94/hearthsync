use super::*;

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
fn install_addon_request_rejects_invalid_app_metadata() {
    let base = std::env::current_dir().expect("cwd");
    let runtime = runtime_with_relative_path_base(base);

    let error = InstallAddonAppRequest {
        installation: sample_installation(),
        source: "https://example.invalid/weakauras.zip".to_string(),
        dry_run: false,
        backup_output_path: None,
        replace_existing: true,
        metadata: Some(AddonPackageMetadataValue {
            index_package_id: Some(" ".to_string()),
            ..AddonPackageMetadataValue::default()
        }),
    }
    .into_domain_request(&runtime)
    .expect_err("invalid addon metadata should fail closed");

    assert!(
        error
            .to_string()
            .contains("addon package metadata index_package_id must not be empty")
    );
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
