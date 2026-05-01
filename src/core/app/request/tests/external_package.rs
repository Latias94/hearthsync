use super::*;

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
