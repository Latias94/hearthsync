use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use tempfile::tempdir;
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use super::{
    InstallAddonRequest, RemoveAddonRequest, UpdateAddonRequest, install_addon, list_addons,
    remove_addons, update_addons,
};
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};

#[test]
fn install_addon_from_local_archive_writes_files_and_registry() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("weakauras-pack.zip");

    create_addon_archive(
        &archive_path,
        &[
            (
                "WeakAuras/WeakAuras.toc",
                "## Interface: 110000\n## Title: WeakAuras\n## Version: 1.0.0\n",
            ),
            ("WeakAuras/Core.lua", "print('wa')"),
            (
                "SharedMedia/SharedMedia.toc",
                "## Interface: 110000\n## Title: SharedMedia\n",
            ),
        ],
    );

    let result = install_addon(InstallAddonRequest {
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install addon");

    assert_eq!(result.package_id, "weakauras-pack");
    assert_eq!(result.addons.len(), 2);
    assert!(
        installation
            .addon_dir
            .join("WeakAuras")
            .join("Core.lua")
            .exists()
    );
    assert!(
        installation
            .addon_dir
            .join(".hearthsync")
            .join("addons.toml")
            .exists()
    );

    let inventory = list_addons(&installation).expect("list addons");
    assert_eq!(inventory.tracked_packages.len(), 1);
    assert!(inventory.untracked_addons.is_empty());
}

#[test]
fn update_addons_reuses_recorded_source() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("details-pack.zip");

    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    install_addon(InstallAddonRequest {
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install addon");

    create_addon_archive(
        &archive_path,
        &[
            (
                "Details/Details.toc",
                "## Interface: 120000\n## Version: 2.0.0\n",
            ),
            ("Details/Core.lua", "print('updated')"),
        ],
    );

    let result = update_addons(UpdateAddonRequest {
        installation: installation.clone(),
        name: Some("Details".to_string()),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
    })
    .expect("update addons");

    assert_eq!(result.updated_packages.len(), 1);
    assert!(
        fs::read_to_string(installation.addon_dir.join("Details").join("Details.toc"))
            .expect("toc")
            .contains("2.0.0")
    );
    assert!(
        installation
            .addon_dir
            .join("Details")
            .join("Core.lua")
            .exists()
    );
}

#[test]
fn list_addons_reports_untracked_directories() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());

    fs::create_dir_all(installation.addon_dir.join("Plater")).expect("plater dir");
    fs::write(
        installation.addon_dir.join("Plater").join("Plater.toc"),
        "## Interface: 110000",
    )
    .expect("plater toc");

    let inventory = list_addons(&installation).expect("list addons");
    assert!(inventory.tracked_packages.is_empty());
    assert_eq!(inventory.untracked_addons, vec!["Plater".to_string()]);
}

#[test]
fn remove_addons_removes_directories_and_cleans_registry_when_empty() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("plater-pack.zip");

    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    install_addon(InstallAddonRequest {
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install addon");

    let result = remove_addons(RemoveAddonRequest {
        installation: installation.clone(),
        name: "Plater".to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
    })
    .expect("remove addon");

    assert_eq!(result.removed_addons, vec!["Plater".to_string()]);
    assert!(result.registry_cleaned);
    assert!(!installation.addon_dir.join("Plater").exists());
    assert!(
        !installation
            .addon_dir
            .join(".hearthsync")
            .join("addons.toml")
            .exists()
    );
    assert!(!installation.addon_dir.join(".hearthsync").exists());
}

#[test]
fn remove_addons_dry_run_keeps_files_and_registry() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("details-pack.zip");

    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    install_addon(InstallAddonRequest {
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install addon");

    let result = remove_addons(RemoveAddonRequest {
        installation: installation.clone(),
        name: "details-pack".to_string(),
        dry_run: true,
        backup_output_path: Some(temp.path().join("backups")),
    })
    .expect("dry-run remove");

    assert_eq!(result.removed_addons, vec!["Details".to_string()]);
    assert!(!result.registry_cleaned);
    assert!(installation.addon_dir.join("Details").exists());
    assert!(
        installation
            .addon_dir
            .join(".hearthsync")
            .join("addons.toml")
            .exists()
    );
}

fn create_fixture_installation(root: &Path) -> DetectedFlavorInstallation {
    let product_root = root.join("World of Warcraft");
    let flavor_root = product_root.join("_retail_");
    let interface_dir = flavor_root.join("Interface");
    let addon_dir = interface_dir.join("AddOns");
    let wtf_dir = flavor_root.join("WTF");
    let fonts_dir = flavor_root.join("Fonts");

    fs::create_dir_all(&addon_dir).expect("addon dir");
    fs::create_dir_all(&wtf_dir).expect("wtf dir");
    fs::create_dir_all(&fonts_dir).expect("fonts dir");

    DetectedFlavorInstallation {
        platform: HostPlatform::Windows,
        product_root,
        flavor_root,
        flavor: WowFlavor::Retail,
        interface_dir,
        addon_dir,
        wtf_dir,
        fonts_dir,
    }
}

fn create_addon_archive(path: &Path, entries: &[(&str, &str)]) {
    let file = File::create(path).expect("archive file");
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
