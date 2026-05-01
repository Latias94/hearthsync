use std::path::PathBuf;

use crate::core::app::{
    HealthStatusValue, HostPlatformValue, ResolvedInstallationValue, WowFlavorValue,
};

#[test]
fn host_platform_value_roundtrips_domain_shape() {
    let value = HostPlatformValue::MacOs;

    let domain = value.into_domain();

    assert_eq!(HostPlatformValue::from_domain(domain), value);
}

#[test]
fn wow_flavor_value_roundtrips_domain_shape() {
    let value = WowFlavorValue::ClassicEra;

    let domain = value.into_domain();

    assert_eq!(WowFlavorValue::from_domain(domain), value);
}

#[test]
fn wow_flavor_value_helpers_return_stable_strings() {
    assert_eq!(WowFlavorValue::Retail.as_str(), "retail");
    assert_eq!(WowFlavorValue::ClassicEra.as_str(), "classic_era");
}

#[test]
fn health_status_value_roundtrips_domain_shape() {
    let value = HealthStatusValue::Warning;

    let domain = value.into_domain();

    assert_eq!(HealthStatusValue::from_domain(domain), value);
}

#[test]
fn resolved_installation_value_projects_absolute_domain_shape() {
    let value = absolute_installation_value();

    let domain = value.clone().into_domain().expect("domain installation");

    assert_eq!(domain.product_root, value.product_root);
    assert_eq!(domain.flavor_root, value.flavor_root);
    assert_eq!(domain.interface_dir, value.interface_dir);
    assert_eq!(domain.addon_dir, value.addon_dir);
    assert_eq!(domain.wtf_dir, value.wtf_dir);
    assert_eq!(domain.fonts_dir, value.fonts_dir);
}

#[test]
fn resolved_installation_value_rejects_relative_paths() {
    let error = relative_installation_value()
        .into_domain()
        .expect_err("relative installation should fail closed");

    assert!(
        error
            .to_string()
            .contains("resolved installation product root must be absolute")
    );
}

fn absolute_installation_value() -> ResolvedInstallationValue {
    let product_root = std::env::current_dir()
        .expect("cwd")
        .join("World of Warcraft");
    installation_value_from_product_root(product_root)
}

fn relative_installation_value() -> ResolvedInstallationValue {
    installation_value_from_product_root(PathBuf::from("World of Warcraft"))
}

fn installation_value_from_product_root(product_root: PathBuf) -> ResolvedInstallationValue {
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
