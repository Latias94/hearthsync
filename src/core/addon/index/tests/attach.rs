use std::fs;

use tempfile::tempdir;

use super::super::{
    AddonIndexAttachPackageStatus, AddonIndexAttachRequest, attach_addons_from_index,
    attach_addons_from_index_task,
};
use super::{addon_state_paths, create_addon_archive, create_fixture_installation};
use crate::core::addon::{InstallAddonRequest, install_addon, list_addons};
use crate::core::task::{NeverCancel, TaskKind, TaskPhase, VecTaskProgressSink};

#[test]
fn attach_addons_from_index_blocks_without_writing_registry_when_any_package_cannot_attach() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("Plater.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Title: Plater\n## Version: 1.0.0\n",
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
    .expect("install tracked addon");

    let index_path = temp.path().join("index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-plater"
name = "Curated Plater"
version = "2.0.0"
source = {{ kind = "local_archive", path = "{}" }}
supported_flavors = ["retail"]

[[packages]]
id = "unknown-addon"
name = "Unknown Addon"
version = "1.0.0"
source = {{ kind = "http_archive", url = "https://example.invalid/unknown.zip" }}
supported_flavors = ["retail"]

[[packages]]
id = "classic-only"
name = "Classic Only"
version = "1.0.0"
source = {{ kind = "http_archive", url = "https://example.invalid/classic.zip" }}
supported_flavors = ["classic"]
"#,
            archive_path.display().to_string().replace('\\', "\\\\")
        ),
    )
    .expect("write index");

    let result = attach_addons_from_index(AddonIndexAttachRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        index_path,
        name: None,
        dry_run: false,
        apply_ready_only: false,
    })
    .expect("attach from index");

    assert!(!result.ready);
    assert!(!result.applied);
    assert_eq!(result.change_package_count, 1);
    assert_eq!(result.attached_package_count, 0);
    assert_eq!(result.blocked_package_count, 1);
    assert_eq!(result.skipped_unsupported_flavor_package_count, 1);
    let curated = result
        .packages
        .iter()
        .find(|package| package.package.id == "curated-plater")
        .expect("curated package");
    assert!(matches!(
        curated.status,
        AddonIndexAttachPackageStatus::WouldAttach
    ));
    assert_eq!(
        curated.matched_tracked_package_id.as_deref(),
        Some("plater")
    );
    assert!(!curated.source_changed);
    assert!(curated.metadata_changed);

    let unknown = result
        .packages
        .iter()
        .find(|package| package.package.id == "unknown-addon")
        .expect("unknown package");
    assert!(matches!(
        unknown.status,
        AddonIndexAttachPackageStatus::NoLocalMatch
    ));

    let classic = result
        .packages
        .iter()
        .find(|package| package.package.id == "classic-only")
        .expect("classic package");
    assert!(matches!(
        classic.status,
        AddonIndexAttachPackageStatus::SkippedUnsupportedFlavor
    ));

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    assert!(
        inventory.tracked_packages[0].metadata.is_none(),
        "blocked bulk attach must not partially write curated metadata"
    );
}

#[test]
fn attach_addons_from_index_can_apply_ready_packages_when_partial_apply_is_explicit() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("Plater.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Title: Plater\n## Version: 1.0.0\n",
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
    .expect("install tracked addon");

    let index_path = temp.path().join("index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-plater"
name = "Curated Plater"
version = "2.0.0"
source = {{ kind = "local_archive", path = "{}" }}
supported_flavors = ["retail"]

[[packages]]
id = "unknown-addon"
name = "Unknown Addon"
version = "1.0.0"
source = {{ kind = "http_archive", url = "https://example.invalid/unknown.zip" }}
supported_flavors = ["retail"]
"#,
            archive_path.display().to_string().replace('\\', "\\\\")
        ),
    )
    .expect("write index");

    let result = attach_addons_from_index(AddonIndexAttachRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        index_path,
        name: None,
        dry_run: false,
        apply_ready_only: true,
    })
    .expect("attach ready packages from index");

    assert!(!result.ready);
    assert!(result.applied);
    assert!(result.partial_apply);
    assert_eq!(result.change_package_count, 1);
    assert_eq!(result.attached_package_count, 1);
    assert_eq!(result.blocked_package_count, 1);

    let curated = result
        .packages
        .iter()
        .find(|package| package.package.id == "curated-plater")
        .expect("curated package");
    assert!(matches!(
        curated.status,
        AddonIndexAttachPackageStatus::Attached
    ));
    let unknown = result
        .packages
        .iter()
        .find(|package| package.package.id == "unknown-addon")
        .expect("unknown package");
    assert!(matches!(
        unknown.status,
        AddonIndexAttachPackageStatus::NoLocalMatch
    ));

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    assert_eq!(
        inventory.tracked_packages[0]
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.index_package_id.as_deref()),
        Some("curated-plater")
    );
}

#[test]
fn attach_addons_from_index_attaches_all_ready_packages_without_reinstalling_files() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let details_archive_path = temp.path().join("Details.zip");
    let plater_archive_path = temp.path().join("Plater.zip");
    create_addon_archive(
        &details_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &plater_archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Title: Plater\n## Version: 1.0.0\n",
        )],
    );

    for archive_path in [&details_archive_path, &plater_archive_path] {
        install_addon(InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install tracked addon");
    }

    let index_path = temp.path().join("index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-details"
name = "Curated Details"
version = "2.0.0"
source = {{ kind = "local_archive", path = "{}" }}
addon_directories = ["Details"]
supported_flavors = ["retail"]

[[packages]]
id = "curated-plater"
name = "Curated Plater"
version = "2.0.0"
source = {{ kind = "local_archive", path = "{}" }}
addon_directories = ["Plater"]
supported_flavors = ["retail"]
"#,
            details_archive_path
                .display()
                .to_string()
                .replace('\\', "\\\\"),
            plater_archive_path
                .display()
                .to_string()
                .replace('\\', "\\\\"),
        ),
    )
    .expect("write index");

    let result = attach_addons_from_index(AddonIndexAttachRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        index_path,
        name: None,
        dry_run: false,
        apply_ready_only: false,
    })
    .expect("attach from index");

    assert!(result.ready);
    assert!(result.applied);
    assert_eq!(result.change_package_count, 2);
    assert_eq!(result.attached_package_count, 2);
    assert_eq!(result.blocked_package_count, 0);
    assert!(
        result
            .packages
            .iter()
            .all(|package| { matches!(package.status, AddonIndexAttachPackageStatus::Attached) })
    );

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    assert_eq!(inventory.tracked_packages.len(), 2);
    assert!(inventory.tracked_packages.iter().any(|package| {
        package.package_id == "details"
            && package
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.index_package_id.as_deref())
                == Some("curated-details")
    }));
    assert!(inventory.tracked_packages.iter().any(|package| {
        package.package_id == "plater"
            && package
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.index_package_id.as_deref())
                == Some("curated-plater")
    }));
    assert!(
        fs::read_to_string(installation.addon_dir.join("Details").join("Details.toc"))
            .expect("details toc")
            .contains("1.0.0")
    );
    assert!(
        fs::read_to_string(installation.addon_dir.join("Plater").join("Plater.toc"))
            .expect("plater toc")
            .contains("1.0.0")
    );
}

#[test]
fn attach_addons_from_index_task_reports_attach_progress() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("Details.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    let index_path = temp.path().join("index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-details"
name = "Curated Details"
version = "2.0.0"
source = {{ kind = "local_archive", path = "{}" }}
addon_directories = ["Details"]
supported_flavors = ["retail"]
"#,
            archive_path.display().to_string().replace('\\', "\\\\")
        ),
    )
    .expect("write index");

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation),
        installation,
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install details");

    let cancellation = NeverCancel;
    let mut progress = VecTaskProgressSink::default();
    let result = attach_addons_from_index_task(
        AddonIndexAttachRequest {
            state_paths: addon_state_paths(&create_fixture_installation(temp.path())),
            installation: create_fixture_installation(temp.path()),
            index_path,
            name: Some("curated-details".to_string()),
            dry_run: false,
            apply_ready_only: false,
        },
        &cancellation,
        &mut progress,
    )
    .expect("attach from index task");

    assert!(result.applied);
    let phases = progress
        .events()
        .iter()
        .map(|event| (event.task, event.phase))
        .collect::<Vec<_>>();
    assert_eq!(
        phases.first(),
        Some(&(TaskKind::AddonIndexAttach, TaskPhase::Preparing))
    );
    assert_eq!(
        phases.last(),
        Some(&(TaskKind::AddonIndexAttach, TaskPhase::Completed))
    );
    assert!(phases.contains(&(TaskKind::AddonIndexAttach, TaskPhase::Executing)));
}
