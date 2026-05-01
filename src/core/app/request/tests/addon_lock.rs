use super::*;

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
