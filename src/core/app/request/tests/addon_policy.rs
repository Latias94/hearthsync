use super::*;

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
