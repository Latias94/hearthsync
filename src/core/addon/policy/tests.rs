use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use tempfile::tempdir;
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use super::{
    AddonPolicyPackageEntry, AddonPolicyPin, AddonPolicyState, AddonReleaseChannel,
    RemoveAddonPolicyRequest, SetAddonPolicyRequest, inspect_addon_policy, policy_path,
    remove_addon_policy, set_addon_policy,
};
use crate::core::addon::{InstallAddonRequest, RemoveAddonRequest, install_addon, remove_addons};
use crate::core::error::AppError;
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};

fn addon_state_paths(
    installation: &DetectedFlavorInstallation,
) -> crate::core::addon::AddonStatePaths {
    crate::core::addon::AddonStatePaths::for_installation(
        crate::core::addon::AddonStateStorageKind::default(),
        installation,
    )
    .expect("addon state paths")
}

#[test]
fn inspect_addon_policy_returns_empty_when_state_file_is_missing() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());

    let inspection = inspect_addon_policy(&installation, &addon_state_paths(&installation))
        .expect("inspect addon policy");

    assert_eq!(inspection.package_count, 0);
    assert!(inspection.packages.is_empty());
    assert_eq!(
        inspection.policy_path,
        policy_path(&addon_state_paths(&installation))
    );
}

#[test]
fn set_addon_policy_persists_and_inspects_tracked_package_preferences() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("weakauras.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "WeakAuras/WeakAuras.toc",
            "## Interface: 110000\n## Title: WeakAuras\n",
        )],
    );

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install addon");

    let written = set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "WeakAuras".to_string(),
        ignored: Some(true),
        pinned_version: Some("2.4.6".to_string()),
        pinned_file_id: None,
        release_channel: Some(AddonReleaseChannel::Beta),
        allow_prerelease: Some(true),
        install_dependencies: Some(false),
    })
    .expect("set addon policy");

    assert_eq!(written.package_count, 1);
    assert!(!written.entry_removed);
    let package = written.package.expect("written package");
    assert_eq!(package.package_id, "weakauras");
    assert_eq!(package.package_name.as_deref(), Some("WeakAuras"));
    assert_eq!(package.addon_directories, vec!["WeakAuras".to_string()]);
    assert_eq!(package.ignored, Some(true));
    assert_eq!(
        package.pin,
        Some(AddonPolicyPin::Version {
            value: "2.4.6".to_string()
        })
    );
    assert_eq!(package.release_channel, Some(AddonReleaseChannel::Beta));
    assert_eq!(package.allow_prerelease, Some(true));
    assert_eq!(package.install_dependencies, Some(false));

    let inspection = inspect_addon_policy(&installation, &addon_state_paths(&installation))
        .expect("inspect addon policy");
    assert_eq!(inspection.package_count, 1);
    assert_eq!(inspection.packages.len(), 1);
    assert!(policy_path(&addon_state_paths(&installation)).exists());
    assert!(
        fs::read_to_string(policy_path(&addon_state_paths(&installation)))
            .expect("policy file")
            .contains("release_channel = \"beta\"")
    );
}

#[test]
fn remove_addon_policy_removes_last_policy_file() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("details.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Title: Details!\n",
        )],
    );

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install addon");

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "details".to_string(),
        ignored: Some(false),
        pinned_version: None,
        pinned_file_id: Some(12345),
        release_channel: Some(AddonReleaseChannel::Stable),
        allow_prerelease: None,
        install_dependencies: Some(true),
    })
    .expect("set addon policy");

    let removed = remove_addon_policy(RemoveAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "details".to_string(),
    })
    .expect("remove addon policy");

    assert!(removed.entry_removed);
    assert_eq!(removed.package_count, 0);
    assert_eq!(removed.package_id, "details");
    assert!(!policy_path(&addon_state_paths(&installation)).exists());
}

#[test]
fn set_addon_policy_can_update_existing_untracked_entry_by_package_id() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("plater.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Title: Plater\n",
        )],
    );

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install addon");

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "plater".to_string(),
        ignored: Some(true),
        pinned_version: None,
        pinned_file_id: None,
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: None,
    })
    .expect("set addon policy");

    remove_addons(RemoveAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        name: "plater".to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
    })
    .expect("remove addon");

    let updated = set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "plater".to_string(),
        ignored: None,
        pinned_version: Some("3.0.0".to_string()),
        pinned_file_id: None,
        release_channel: Some(AddonReleaseChannel::Alpha),
        allow_prerelease: Some(true),
        install_dependencies: None,
    })
    .expect("update untracked addon policy");

    let package = updated.package.expect("updated package");
    assert!(!package.tracked);
    assert_eq!(package.package_id, "plater");
    assert_eq!(package.ignored, Some(true));
    assert_eq!(
        package.pin,
        Some(AddonPolicyPin::Version {
            value: "3.0.0".to_string()
        })
    );
    assert_eq!(package.release_channel, Some(AddonReleaseChannel::Alpha));
}

#[test]
fn set_addon_policy_rejects_conflicting_pin_inputs() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let error = set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation),
        installation,
        package: "weakauras".to_string(),
        ignored: None,
        pinned_version: Some("1.0.0".to_string()),
        pinned_file_id: Some(42),
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: None,
    })
    .expect_err("conflicting pin inputs should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(
        error
            .to_string()
            .contains("cannot pin both a version and a file id")
    );
}

#[test]
fn inspect_addon_policy_rejects_invalid_persisted_state_contracts() {
    let blank_updated_at = AddonPolicyState {
        updated_at: " ".to_string(),
        ..valid_policy_state(vec![valid_policy_entry("details")])
    };
    let blank_package_id = valid_policy_state(vec![valid_policy_entry(" ")]);
    let duplicate_package_ids = valid_policy_state(vec![
        valid_policy_entry(" Details "),
        valid_policy_entry("details"),
    ]);
    let mut empty_version_pin = valid_policy_entry("details");
    empty_version_pin.pin = Some(AddonPolicyPin::Version {
        value: " ".to_string(),
    });
    let mut zero_file_id_pin = valid_policy_entry("details");
    zero_file_id_pin.pin = Some(AddonPolicyPin::FileId { value: 0 });
    let no_op_entry = valid_policy_state(vec![AddonPolicyPackageEntry::new("details".to_string())]);

    for (case_name, state, expected_message) in [
        (
            "blank updated_at",
            blank_updated_at,
            "updated_at must not be empty",
        ),
        (
            "blank package id",
            blank_package_id,
            "package id cannot be empty",
        ),
        (
            "duplicate package ids",
            duplicate_package_ids,
            "duplicate addon policy package id",
        ),
        (
            "empty version pin",
            valid_policy_state(vec![empty_version_pin]),
            "pinned version cannot be empty",
        ),
        (
            "zero file id pin",
            valid_policy_state(vec![zero_file_id_pin]),
            "pinned file id must be greater than zero",
        ),
        (
            "no-op policy entry",
            no_op_entry,
            "must contain at least one policy setting",
        ),
    ] {
        let temp = tempdir().expect("temp dir");
        let installation = create_fixture_installation(temp.path());
        let state_paths = sidecar_addon_state_paths(&installation);
        write_policy_state(&state_paths, &state);

        let error = inspect_addon_policy(&installation, &state_paths).expect_err(case_name);

        assert!(matches!(error, AppError::Validation(_)));
        assert!(
            error.to_string().contains(expected_message),
            "{case_name}: expected `{expected_message}`, got `{error}`"
        );
    }
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

fn sidecar_addon_state_paths(
    installation: &DetectedFlavorInstallation,
) -> crate::core::addon::AddonStatePaths {
    crate::core::addon::AddonStatePaths::for_installation(
        crate::core::addon::AddonStateStorageKind::Sidecar,
        installation,
    )
    .expect("addon state paths")
}

fn valid_policy_state(packages: Vec<AddonPolicyPackageEntry>) -> AddonPolicyState {
    AddonPolicyState {
        schema_version: 1,
        updated_at: "2026-04-29T00:00:00Z".to_string(),
        packages,
    }
}

fn valid_policy_entry(package_id: &str) -> AddonPolicyPackageEntry {
    let mut entry = AddonPolicyPackageEntry::new(package_id.to_string());
    entry.ignored = Some(false);
    entry
}

fn write_policy_state(state_paths: &crate::core::addon::AddonStatePaths, state: &AddonPolicyState) {
    let path = policy_path(state_paths);
    fs::create_dir_all(path.parent().expect("policy parent")).expect("policy parent dir");
    fs::write(
        path,
        toml::to_string_pretty(state).expect("serialized policy state"),
    )
    .expect("policy file");
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
