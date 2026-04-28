use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use crate::core::app::{
    AddonService, AppRuntime, HealthStatusValue, HostPlatformValue, InspectInstallationRequest,
    InstallationService, ListAddonsRequest, ResolveInstallationRequest, WowFlavorValue,
};

#[test]
fn installation_service_scan_uses_runtime_scan_roots_and_host_platform() {
    let temp = tempdir().expect("temp dir");
    let product_root = temp.path().join("World of Warcraft");
    let flavor_root = product_root.join("_retail_");

    fs::create_dir_all(flavor_root.join("Interface").join("AddOns")).expect("addons dir");
    fs::create_dir_all(flavor_root.join("WTF")).expect("wtf dir");

    let service = InstallationService::with_runtime(
        AppRuntime::builder()
            .with_host_platform(HostPlatformValue::MacOs)
            .with_install_scan_roots(Some(vec![product_root.clone()]))
            .build()
            .expect("runtime"),
    );
    let installations = service.scan().expect("scan installations");

    assert_eq!(installations.installation_count, 1);
    assert_eq!(
        installations.installations[0].platform,
        HostPlatformValue::MacOs
    );
    assert_eq!(installations.installations[0].product_root, product_root);
}

#[test]
fn installation_service_scan_resolves_relative_roots_against_runtime_base() {
    let temp = tempdir().expect("temp dir");
    let product_root = temp.path().join("World of Warcraft");
    let flavor_root = product_root.join("_retail_");

    fs::create_dir_all(flavor_root.join("Interface").join("AddOns")).expect("addons dir");
    fs::create_dir_all(flavor_root.join("WTF")).expect("wtf dir");

    let service = InstallationService::with_runtime(
        AppRuntime::builder()
            .with_host_platform(HostPlatformValue::MacOs)
            .with_relative_path_base(Some(temp.path().to_path_buf()))
            .with_install_scan_roots(Some(vec![PathBuf::from("World of Warcraft")]))
            .build()
            .expect("runtime"),
    );
    let installations = service.scan().expect("scan installations");

    assert_eq!(installations.installation_count, 1);
    assert!(
        installations.installations[0]
            .product_root
            .ends_with(Path::new("World of Warcraft"))
    );
}

#[test]
fn installation_service_scan_rejects_relative_roots_without_runtime_base() {
    let error = AppRuntime::builder()
        .with_install_scan_roots(Some(vec![PathBuf::from("World of Warcraft")]))
        .build()
        .expect_err("relative scan root should fail closed");

    assert!(
        error
            .to_string()
            .contains("installation scan root relative path requires")
    );
}

#[test]
fn installation_service_inspect_and_resolve_use_runtime_host_platform() {
    let temp = tempdir().expect("temp dir");
    let product_root = temp.path().join("World of Warcraft");
    let flavor_root = product_root.join("_retail_");

    fs::create_dir_all(flavor_root.join("Interface").join("AddOns")).expect("addons dir");
    fs::create_dir_all(flavor_root.join("WTF")).expect("wtf dir");
    fs::write(
        flavor_root.join("WTF").join("Config.wtf"),
        "SET locale enUS",
    )
    .expect("config");

    let service = InstallationService::with_runtime(
        AppRuntime::builder()
            .with_host_platform(HostPlatformValue::MacOs)
            .build()
            .expect("runtime"),
    );
    let inspection = service
        .inspect(InspectInstallationRequest {
            path: product_root.clone(),
            flavor: Some(WowFlavorValue::Retail),
        })
        .expect("inspect");
    let resolved = service
        .resolve(ResolveInstallationRequest {
            path: product_root,
            flavor: Some(WowFlavorValue::Retail),
        })
        .expect("resolve");

    assert_eq!(inspection.installation.platform, HostPlatformValue::MacOs);
    assert_eq!(inspection.health.status, HealthStatusValue::Warning);
    assert_eq!(resolved.platform, HostPlatformValue::MacOs);
    assert!(
        resolved
            .flavor_root
            .ends_with(Path::new("World of Warcraft").join("_retail_"))
    );

    let inventory = AddonService::new()
        .list(ListAddonsRequest {
            installation: resolved,
        })
        .expect("list addons from resolved installation value");
    assert_eq!(inventory.tracked_packages.len(), 0);
}

#[test]
fn installation_service_resolves_relative_paths_against_runtime_base() {
    let temp = tempdir().expect("temp dir");
    let product_root = temp.path().join("World of Warcraft");
    let flavor_root = product_root.join("_retail_");

    fs::create_dir_all(flavor_root.join("Interface").join("AddOns")).expect("addons dir");
    fs::create_dir_all(flavor_root.join("WTF")).expect("wtf dir");

    let service = InstallationService::with_runtime(
        AppRuntime::builder()
            .with_host_platform(HostPlatformValue::MacOs)
            .with_relative_path_base(Some(temp.path().to_path_buf()))
            .build()
            .expect("runtime"),
    );
    let inspection = service
        .inspect(InspectInstallationRequest {
            path: PathBuf::from("World of Warcraft"),
            flavor: Some(WowFlavorValue::Retail),
        })
        .expect("inspect relative installation");
    let resolved = service
        .resolve(ResolveInstallationRequest {
            path: PathBuf::from("World of Warcraft"),
            flavor: Some(WowFlavorValue::Retail),
        })
        .expect("resolve relative installation");

    assert!(inspection.product_root.exists());
    assert!(resolved.flavor_root.exists());
    assert!(
        inspection
            .product_root
            .ends_with(Path::new("World of Warcraft"))
    );
    assert!(
        resolved
            .flavor_root
            .ends_with(Path::new("World of Warcraft").join("_retail_"))
    );
}

#[test]
fn installation_service_rejects_relative_paths_without_runtime_base() {
    let service = InstallationService::with_runtime(AppRuntime::new());

    let inspect_error = service
        .inspect(InspectInstallationRequest {
            path: PathBuf::from("World of Warcraft"),
            flavor: Some(WowFlavorValue::Retail),
        })
        .expect_err("relative inspect path should fail closed");
    let resolve_error = service
        .resolve(ResolveInstallationRequest {
            path: PathBuf::from("World of Warcraft"),
            flavor: Some(WowFlavorValue::Retail),
        })
        .expect_err("relative resolve path should fail closed");

    assert!(
        inspect_error
            .to_string()
            .contains("installation path relative path requires")
    );
    assert!(
        resolve_error
            .to_string()
            .contains("installation path relative path requires")
    );
}
