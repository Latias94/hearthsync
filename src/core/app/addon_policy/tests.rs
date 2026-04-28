use std::fs;
use std::io::Write;
use std::path::Path;

use tempfile::tempdir;
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::core::app::{
    AddonPolicyPinValue, AddonPolicyService, AddonReleaseChannelValue, AddonService,
    InspectAddonPolicyRequest, InstallAddonAppRequest, RemoveAddonPolicyAppRequest,
    ResolvedInstallationValue, SetAddonPolicyAppRequest,
};
use crate::core::install::{HostPlatform, WowFlavor};

#[test]
fn addon_policy_service_set_and_inspect_roundtrip() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("WeakAuras.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "WeakAuras/WeakAuras.toc",
            "## Interface: 110000\n## Title: WeakAuras\n",
        )],
    );

    AddonService::new()
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install addon");

    let service = AddonPolicyService::new();
    let written = service
        .set(SetAddonPolicyAppRequest {
            installation: installation.clone(),
            package: "WeakAuras".to_string(),
            ignored: Some(true),
            pin: Some(AddonPolicyPinValue::FileId { value: 9001 }),
            release_channel: Some(AddonReleaseChannelValue::Beta),
            allow_prerelease: Some(true),
            install_dependencies: Some(false),
        })
        .expect("set addon policy");
    let inspection = service
        .inspect(InspectAddonPolicyRequest {
            installation: installation.clone(),
        })
        .expect("inspect addon policy");

    assert_eq!(written.package_id, "weakauras");
    assert_eq!(inspection.package_count, 1);
    assert_eq!(inspection.packages.len(), 1);
    assert_eq!(
        inspection.packages[0].package_name.as_deref(),
        Some("WeakAuras")
    );
    assert_eq!(
        inspection.packages[0].pin,
        Some(AddonPolicyPinValue::FileId { value: 9001 })
    );
    assert_eq!(
        inspection.packages[0].release_channel,
        Some(AddonReleaseChannelValue::Beta)
    );
}

#[test]
fn addon_policy_service_remove_clears_entry() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("Details.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Title: Details!\n",
        )],
    );

    AddonService::new()
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install addon");

    let service = AddonPolicyService::new();
    service
        .set(SetAddonPolicyAppRequest {
            installation: installation.clone(),
            package: "details".to_string(),
            ignored: Some(false),
            pin: None,
            release_channel: Some(AddonReleaseChannelValue::Stable),
            allow_prerelease: None,
            install_dependencies: Some(true),
        })
        .expect("set addon policy");

    let removed = service
        .remove(RemoveAddonPolicyAppRequest {
            installation: installation.clone(),
            package: "details".to_string(),
        })
        .expect("remove addon policy");
    let inspection = service
        .inspect(InspectAddonPolicyRequest { installation })
        .expect("inspect addon policy");

    assert!(removed.entry_removed);
    assert_eq!(inspection.package_count, 0);
}

fn create_empty_installation(root: &Path) -> ResolvedInstallationValue {
    let product_root = root.join("World of Warcraft");
    let flavor_root = product_root.join("_retail_");
    let interface_dir = flavor_root.join("Interface");
    let addon_dir = interface_dir.join("AddOns");
    let wtf_dir = flavor_root.join("WTF");
    let fonts_dir = flavor_root.join("Fonts");

    fs::create_dir_all(&addon_dir).expect("addon dir");
    fs::create_dir_all(&wtf_dir).expect("wtf dir");
    fs::create_dir_all(&fonts_dir).expect("fonts dir");

    ResolvedInstallationValue::from_domain(crate::core::install::DetectedFlavorInstallation {
        platform: HostPlatform::Windows,
        product_root,
        flavor_root,
        flavor: WowFlavor::Retail,
        interface_dir,
        addon_dir,
        wtf_dir,
        fonts_dir,
    })
}

fn create_addon_archive(path: &Path, entries: &[(&str, &str)]) {
    let file = fs::File::create(path).expect("archive file");
    let mut zip = ZipWriter::new(file);
    for (name, content) in entries {
        zip.start_file(
            name.replace('\\', "/"),
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
        )
        .expect("start file");
        zip.write_all(content.as_bytes()).expect("write file");
    }
    zip.finish().expect("finish zip");
}
