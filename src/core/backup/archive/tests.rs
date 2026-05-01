use std::path::PathBuf;

use crate::core::archive_io::PortableArchivePathSet;
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};

use super::create::register_backup_archive_output;
use super::restore::parse_backup_entry_target;
use crate::core::backup::BackupGroup;

#[test]
fn register_backup_archive_output_rejects_case_insensitive_metadata_collisions() {
    let mut archive_outputs = PortableArchivePathSet::new();
    register_backup_archive_output(&mut archive_outputs, "backup.toml", false)
        .expect("backup metadata should register");

    let error = register_backup_archive_output(&mut archive_outputs, "BACKUP.toml", false)
        .expect_err("case-only metadata collision should fail");

    let message = error.to_string();
    assert!(message.contains("case-insensitive archive path collisions"));
    assert!(message.contains("backup.toml"));
    assert!(message.contains("BACKUP.toml"));
}

#[test]
fn register_backup_archive_output_rejects_file_as_ancestor_conflicts() {
    let mut archive_outputs = PortableArchivePathSet::new();
    register_backup_archive_output(&mut archive_outputs, "addons/WeakAuras", false)
        .expect("file output should register");

    let error =
        register_backup_archive_output(&mut archive_outputs, "addons/WeakAuras/Config.lua", false)
            .expect_err("file ancestor conflict should fail");

    let message = error.to_string();
    assert!(message.contains("conflicting file and directory archive paths"));
    assert!(message.contains("addons/WeakAuras"));
    assert!(message.contains("addons/WeakAuras/Config.lua"));
}

#[test]
fn register_backup_archive_output_allows_directory_ancestors() {
    let mut archive_outputs = PortableArchivePathSet::new();
    register_backup_archive_output(&mut archive_outputs, "addons/WeakAuras", true)
        .expect("directory output should register");
    register_backup_archive_output(&mut archive_outputs, "addons/WeakAuras/Config.lua", false)
        .expect("directory ancestors should stay legal");
}

#[test]
fn parse_backup_entry_target_maps_group_and_destination() {
    let installation = fixture_installation();

    let target = parse_backup_entry_target(
        "wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua",
        &installation,
    )
    .expect("portable backup entry should parse");

    assert_eq!(target.group, BackupGroup::Wtf);
    assert_eq!(
        target.destination,
        installation
            .wtf_dir
            .join("common")
            .join("accounts")
            .join("ACCOUNT")
            .join("SavedVariables")
            .join("Details.lua")
    );
}

#[test]
fn parse_backup_entry_target_preserves_root_vs_path_errors() {
    let installation = fixture_installation();

    let missing_rest = parse_backup_entry_target("addons", &installation)
        .expect_err("root-only entry path should fail");
    assert!(
        missing_rest
            .to_string()
            .contains("backup archive contains unsupported entry path")
    );

    let unsupported_root = parse_backup_entry_target("metadata/backup.toml", &installation)
        .expect_err("unsupported root should fail");
    assert!(
        unsupported_root
            .to_string()
            .contains("backup archive contains unsupported root entry")
    );
}

fn fixture_installation() -> DetectedFlavorInstallation {
    let product_root = PathBuf::from("C:/Games/World of Warcraft");
    let flavor_root = product_root.join("_retail_");
    let interface_dir = flavor_root.join("Interface");
    let addon_dir = interface_dir.join("AddOns");
    let wtf_dir = flavor_root.join("WTF");
    let fonts_dir = flavor_root.join("Fonts");

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
