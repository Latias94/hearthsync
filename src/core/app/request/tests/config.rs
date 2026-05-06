use super::*;

#[test]
fn export_config_request_converts_config_owned_export_policy() {
    let temp = tempfile::tempdir().expect("temp dir");
    let domain = ExportConfigBundleAppRequest {
        config_package: sample_config_package_request(),
        sharing_mode: ExternalPackageSharingModeValue::Public,
        allow_public_sharing_risks: true,
        excluded_wtf_scopes: vec![WtfScopeValue::AccountSavedVariables],
    }
    .into_external_request()
    .into_domain_request(&runtime_with_relative_path_base(temp.path().to_path_buf()))
    .expect("domain request");

    assert_eq!(
        domain.layout,
        crate::core::bundle::ExternalPackageLayout::Auto
    );
    assert_eq!(
        domain.sharing_mode,
        crate::core::bundle::ExternalPackageSharingMode::Public
    );
    assert!(domain.allow_public_sharing_risks);
    assert_eq!(
        domain.excluded_wtf_scopes,
        vec![crate::core::bundle::WtfScope::AccountSavedVariables]
    );
}
