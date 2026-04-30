use std::fs;

use tempfile::tempdir;

use super::super::{
    AddonIndexRelinkRequest, relink_addon_from_index, relink_addon_from_index_task,
};
use super::{addon_state_paths, create_addon_archive, create_fixture_installation};
use crate::core::addon::{InstallAddonRequest, install_addon, list_addons};
use crate::core::task::{NeverCancel, TaskKind, TaskPhase, VecTaskProgressSink};

#[test]
fn relink_addon_from_index_updates_curated_metadata_without_reinstalling_files() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("details.zip");
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
source_url = "https://example.invalid/details.zip"
website_url = "https://example.invalid/details"
addon_directories = ["Details"]
supported_flavors = ["retail"]
"#,
            archive_path.display().to_string().replace('\\', "\\\\")
        ),
    )
    .expect("write index");

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install details");

    let result = relink_addon_from_index(AddonIndexRelinkRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        index_path,
        name: "curated-details".to_string(),
        target: Some("details".to_string()),
        dry_run: false,
    })
    .expect("relink from index");

    assert_eq!(result.tracked_package_id, "details");
    assert!(!result.source_changed);
    assert!(result.metadata_changed);
    assert_eq!(
        result.metadata.index_package_id.as_deref(),
        Some("curated-details")
    );

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    assert_eq!(
        inventory.tracked_packages[0]
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.index_package_id.as_deref()),
        Some("curated-details")
    );
    assert!(
        fs::read_to_string(installation.addon_dir.join("Details").join("Details.toc"))
            .expect("details toc")
            .contains("1.0.0")
    );
}

#[test]
fn relink_addon_from_index_task_reports_relink_progress() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("details.zip");
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
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install details");

    let cancellation = NeverCancel;
    let mut progress = VecTaskProgressSink::default();
    let result = relink_addon_from_index_task(
        AddonIndexRelinkRequest {
            state_paths: addon_state_paths(&installation),
            installation,
            index_path,
            name: "curated-details".to_string(),
            target: Some("details".to_string()),
            dry_run: false,
        },
        &cancellation,
        &mut progress,
    )
    .expect("relink from index task");

    assert_eq!(result.tracked_package_id, "details");
    let phases = progress
        .events()
        .iter()
        .map(|event| (event.task, event.phase))
        .collect::<Vec<_>>();
    assert_eq!(
        phases.first(),
        Some(&(TaskKind::AddonIndexRelink, TaskPhase::Preparing))
    );
    assert_eq!(
        phases.last(),
        Some(&(TaskKind::AddonIndexRelink, TaskPhase::Completed))
    );
    assert!(phases.contains(&(TaskKind::AddonIndexRelink, TaskPhase::Executing)));
}
