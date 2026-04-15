use std::fs;
use std::io::Write;
use std::path::Path;

use tempfile::tempdir;
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use super::{
    AddonLockApplyRequest, apply_addon_lock_sync, diff_addon_locks, inspect_addon_lock, lock_path,
    plan_addon_lock_sync, verify_addon_lock, write_addon_lock,
};
use crate::core::addon::{
    AddonPackageMetadata, InstallAddonRequest, RemoveAddonRequest, install_addon, list_addons,
    remove_addons,
};
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};

#[test]
fn install_addon_writes_lock_with_metadata_and_content_hash() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("details.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Title: Details!\n## Version: 1.0.0\n",
        )],
    );

    install_addon(InstallAddonRequest {
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: Some(AddonPackageMetadata {
            index_name: Some("Fixture Index".to_string()),
            index_package_id: Some("details".to_string()),
            package_name: Some("Details".to_string()),
            version: Some("1.0.0".to_string()),
            source_url: Some("https://example.com/details.zip".to_string()),
            website_url: Some("https://example.com/details".to_string()),
            source_sha256: Some("source-hash".to_string()),
            supported_flavors: vec!["retail".to_string()],
        }),
    })
    .expect("install addon");

    let inspection = inspect_addon_lock(&installation).expect("inspect lock");
    assert_eq!(inspection.package_count, 1);
    assert_eq!(
        inspection.lock.packages[0].index_package_id.as_deref(),
        Some("details")
    );
    assert_eq!(inspection.lock.packages[0].name.as_deref(), Some("Details"));
    assert_eq!(
        inspection.lock.packages[0].version.as_deref(),
        Some("1.0.0")
    );
    assert_eq!(inspection.lock.packages[0].content_sha256.len(), 64);
    assert_eq!(
        inspection.lock.packages[0].addon_directories,
        vec!["Details"]
    );
}

#[test]
fn write_addon_lock_removes_stale_lock_when_registry_is_empty() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let path = lock_path(&installation);
    fs::create_dir_all(path.parent().expect("lock parent")).expect("lock parent");
    fs::write(&path, "stale").expect("stale lock");

    let result = write_addon_lock(&installation).expect("write lock");

    assert!(result.removed);
    assert!(!path.exists());
}

#[test]
fn remove_addon_cleans_lock_file_when_last_package_is_removed() {
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

    install_addon(InstallAddonRequest {
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install addon");
    assert!(lock_path(&installation).exists());

    remove_addons(RemoveAddonRequest {
        installation: installation.clone(),
        name: "Details".to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
    })
    .expect("remove addon");

    assert!(!lock_path(&installation).exists());
}

#[test]
fn diff_addon_locks_reports_changed_added_and_removed_packages() {
    let temp = tempdir().expect("temp dir");
    let left_path = temp.path().join("left.toml");
    let right_path = temp.path().join("right.toml");

    fs::write(
        &left_path,
        r#"
schema_version = 1
generated_at = "2026-04-15T00:00:00Z"

[[packages]]
package_id = "details"
index_name = "Raid"
index_package_id = "details"
name = "Details"
version = "1.0.0"
source = { kind = "local_archive", path = "C:\\details.zip" }
content_sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
installed_at = "2026-04-15T00:00:00Z"
updated_at = "2026-04-15T00:00:00Z"
addon_directories = ["Details"]
addons = []

[[packages]]
package_id = "omen"
name = "Omen"
source = { kind = "local_archive", path = "C:\\omen.zip" }
content_sha256 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
installed_at = "2026-04-15T00:00:00Z"
updated_at = "2026-04-15T00:00:00Z"
addon_directories = ["Omen"]
addons = []
"#,
    )
    .expect("left lock");

    fs::write(
        &right_path,
        r#"
schema_version = 1
generated_at = "2026-04-16T00:00:00Z"

[[packages]]
package_id = "details-v2"
index_name = "Raid"
index_package_id = "details"
name = "Details"
version = "2.0.0"
source = { kind = "local_archive", path = "C:\\details-v2.zip" }
content_sha256 = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
installed_at = "2026-04-16T00:00:00Z"
updated_at = "2026-04-16T00:00:00Z"
addon_directories = ["Details"]
addons = []

[[packages]]
package_id = "bigwigs"
name = "BigWigs"
source = { kind = "local_archive", path = "C:\\bigwigs.zip" }
content_sha256 = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
installed_at = "2026-04-16T00:00:00Z"
updated_at = "2026-04-16T00:00:00Z"
addon_directories = ["BigWigs"]
addons = []
"#,
    )
    .expect("right lock");

    let diff = diff_addon_locks(&left_path, &right_path).expect("diff locks");

    assert!(!diff.identical);
    assert_eq!(diff.changed_packages.len(), 1);
    assert_eq!(diff.added_packages.len(), 1);
    assert_eq!(diff.removed_packages.len(), 1);
    assert!(
        diff.changed_packages[0]
            .changes
            .iter()
            .any(|change| change.field == "version")
    );
}

#[test]
fn verify_addon_lock_reports_drift_and_untracked_addons() {
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

    install_addon(InstallAddonRequest {
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install addon");

    fs::write(
        installation.addon_dir.join("Details").join("Details.toc"),
        "## Interface: 110000\n## Version: 2.0.0\n",
    )
    .expect("mutate toc");
    fs::create_dir_all(installation.addon_dir.join("BigWigs")).expect("untracked addon dir");
    fs::write(
        installation.addon_dir.join("BigWigs").join("BigWigs.toc"),
        "## Interface: 110000\n## Version: 1.0.0\n",
    )
    .expect("untracked addon toc");

    let verification = verify_addon_lock(&installation, None).expect("verify lock");

    assert!(!verification.matches);
    assert_eq!(verification.diff.changed_packages.len(), 1);
    assert_eq!(verification.untracked_addons, vec!["BigWigs"]);
    assert!(
        verification.diff.changed_packages[0]
            .changes
            .iter()
            .any(|change| change.field == "content_sha256")
    );
}

#[test]
fn apply_addon_lock_sync_updates_installs_and_removes_packages() {
    let temp = tempdir().expect("temp dir");
    let source_root = temp.path().join("sources");
    fs::create_dir_all(&source_root).expect("source root");

    let details_v1 = source_root.join("details-v1.zip");
    let details_v2 = source_root.join("details-v2.zip");
    let omen = source_root.join("omen.zip");
    let bigwigs = source_root.join("bigwigs.zip");
    create_addon_archive(
        &details_v1,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &details_v2,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 2.0.0\n",
        )],
    );
    create_addon_archive(
        &omen,
        &[("Omen/Omen.toc", "## Interface: 110000\n## Version: 1.0.0\n")],
    );
    create_addon_archive(
        &bigwigs,
        &[(
            "BigWigs/BigWigs.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    let desired_installation = create_fixture_installation(&temp.path().join("desired"));
    install_addon(InstallAddonRequest {
        installation: desired_installation.clone(),
        source: details_v2.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("desired-backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install desired details");
    install_addon(InstallAddonRequest {
        installation: desired_installation.clone(),
        source: bigwigs.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("desired-backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install desired bigwigs");
    let desired_lock = write_addon_lock(&desired_installation)
        .expect("write desired lock")
        .lock_path;

    let current_installation = create_fixture_installation(&temp.path().join("current"));
    install_addon(InstallAddonRequest {
        installation: current_installation.clone(),
        source: details_v1.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("current-backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install current details");
    install_addon(InstallAddonRequest {
        installation: current_installation.clone(),
        source: omen.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("current-backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install current omen");

    let plan = plan_addon_lock_sync(&current_installation, Some(&desired_lock)).expect("plan");
    assert_eq!(plan.install_count, 1);
    assert_eq!(plan.update_count, 1);
    assert_eq!(plan.remove_count, 1);
    assert_eq!(plan.blocked_count, 0);

    let apply_backup_dir = temp.path().join("apply-backups");
    let result = apply_addon_lock_sync(AddonLockApplyRequest {
        installation: current_installation.clone(),
        lock_path: Some(desired_lock.clone()),
        backup_output_path: Some(apply_backup_dir.clone()),
        replace_existing: false,
        source_overrides: Vec::new(),
    })
    .expect("apply lock sync");

    assert!(result.verification.matches);
    assert_eq!(result.install_count, 1);
    assert_eq!(result.update_count, 1);
    assert_eq!(result.remove_count, 1);
    assert!(
        fs::read_to_string(
            current_installation
                .addon_dir
                .join("Details")
                .join("Details.toc")
        )
        .expect("details toc")
        .contains("2.0.0")
    );
    assert!(current_installation.addon_dir.join("BigWigs").exists());
    assert!(!current_installation.addon_dir.join("Omen").exists());
    assert_eq!(count_backup_archives(&apply_backup_dir), 1);
}

#[test]
fn apply_addon_lock_sync_applies_metadata_only_actions_transactionally() {
    let temp = tempdir().expect("temp dir");
    let archive_path = temp.path().join("details-pack.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    let desired_installation = create_fixture_installation(&temp.path().join("desired"));
    install_addon(InstallAddonRequest {
        installation: desired_installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("desired-backups")),
        replace_existing: false,
        metadata: Some(AddonPackageMetadata {
            package_name: Some("Curated Details".to_string()),
            version: Some("1.0.0-curated".to_string()),
            source_url: Some("https://example.invalid/details".to_string()),
            website_url: Some("https://example.invalid/details/site".to_string()),
            ..AddonPackageMetadata::default()
        }),
    })
    .expect("install desired details");
    let desired_lock = write_addon_lock(&desired_installation)
        .expect("write desired lock")
        .lock_path;

    let current_installation = create_fixture_installation(&temp.path().join("current"));
    install_addon(InstallAddonRequest {
        installation: current_installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("current-backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install current details");

    let plan = plan_addon_lock_sync(&current_installation, Some(&desired_lock)).expect("plan");
    assert_eq!(plan.install_count, 0);
    assert_eq!(plan.update_count, 0);
    assert_eq!(plan.remove_count, 0);
    assert_eq!(plan.metadata_only_count, 1);

    let apply_backup_dir = temp.path().join("metadata-apply-backups");
    let result = apply_addon_lock_sync(AddonLockApplyRequest {
        installation: current_installation.clone(),
        lock_path: Some(desired_lock),
        backup_output_path: Some(apply_backup_dir.clone()),
        replace_existing: false,
        source_overrides: Vec::new(),
    })
    .expect("apply metadata-only lock sync");

    assert!(result.verification.matches);
    assert_eq!(result.metadata_only_count, 1);
    assert_eq!(count_backup_archives(&apply_backup_dir), 1);

    let inventory = list_addons(&current_installation).expect("list addons");
    let metadata = inventory.tracked_packages[0]
        .metadata
        .as_ref()
        .expect("metadata");
    assert_eq!(metadata.package_name.as_deref(), Some("Curated Details"));
    assert_eq!(metadata.version.as_deref(), Some("1.0.0-curated"));
    assert_eq!(
        metadata.source_url.as_deref(),
        Some("https://example.invalid/details")
    );
}

fn count_backup_archives(path: &Path) -> usize {
    fs::read_dir(path)
        .expect("backup dir")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
        })
        .count()
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
