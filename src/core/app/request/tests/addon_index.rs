use super::*;

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
