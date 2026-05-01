use super::*;

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
