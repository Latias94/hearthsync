use std::cell::Cell;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use tempfile::tempdir;
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use super::{
    AddonLockApplyRequest, apply_addon_lock_sync, apply_addon_lock_sync_task, diff_addon_locks,
    inspect_addon_lock, lock_path, plan_addon_lock_sync, verify_addon_lock, write_addon_lock,
};
use crate::core::addon::{
    AddonPackageMetadata, AddonSourceRef, InstallAddonRequest, RemoveAddonRequest, install_addon,
    list_addons, remove_addons,
};
use crate::core::error::AppError;
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};
use crate::core::task::{
    CancellationToken, NeverCancel, TaskKind, TaskPhase, TaskProgressEvent, TaskProgressSink,
    VecTaskProgressSink,
};

const VALID_LOCK_SHA256: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn addon_state_paths(
    installation: &DetectedFlavorInstallation,
) -> crate::core::addon::AddonStatePaths {
    crate::core::addon::AddonStatePaths::for_installation(
        crate::core::addon::AddonStateStorageKind::default(),
        installation,
    )
    .expect("addon state paths")
}

mod apply;
mod diff_verify;
mod inspect;
mod plan;
mod storage;

#[derive(Default)]
struct CancelDuringVerifying {
    cancel_requested: Cell<bool>,
}

impl CancellationToken for CancelDuringVerifying {
    fn is_cancelled(&self) -> bool {
        self.cancel_requested.get()
    }
}

struct CancelDuringVerifyingProgressSink<'a> {
    cancel_requested: &'a Cell<bool>,
    inner: VecTaskProgressSink,
}

impl<'a> CancelDuringVerifyingProgressSink<'a> {
    fn new(cancel_requested: &'a Cell<bool>) -> Self {
        Self {
            cancel_requested,
            inner: VecTaskProgressSink::default(),
        }
    }

    fn events(&self) -> &[TaskProgressEvent] {
        self.inner.events()
    }
}

impl TaskProgressSink for CancelDuringVerifyingProgressSink<'_> {
    fn push(&mut self, event: TaskProgressEvent) {
        if event.task == TaskKind::AddonLockApply && event.phase == TaskPhase::Verifying {
            self.cancel_requested.set(true);
        }
        self.inner.push(event);
    }

    fn task_id(&self) -> Option<&str> {
        self.inner.task_id()
    }
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

fn desired_lock_with_single_addon(
    root: &Path,
    platform: HostPlatform,
    addon_name: &str,
    toc_entry: &str,
) -> std::path::PathBuf {
    let archive_path = root.join(format!("{addon_name}.zip"));
    create_addon_archive(
        &archive_path,
        &[(
            toc_entry,
            "## Interface: 110000\n## Title: Fixture\n## Version: 1.0.0\n",
        )],
    );
    let desired_installation =
        create_fixture_installation_for_platform(&root.join("desired"), platform);
    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&desired_installation.clone()),
        installation: desired_installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(root.join("desired-backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install desired addon");

    write_addon_lock(
        &desired_installation,
        &addon_state_paths(&desired_installation),
    )
    .expect("write desired lock")
    .lock_path
}

fn rewrite_first_lock_package_source_to_relative_local_archive(lock_path: &Path) {
    let content = fs::read_to_string(lock_path).expect("lock content");
    let mut lock = toml::from_str::<super::AddonLock>(&content).expect("lock toml");
    lock.packages[0].source = AddonSourceRef::LocalArchive {
        path: PathBuf::from("sources/Details.zip"),
    };
    fs::write(
        lock_path,
        toml::to_string_pretty(&lock).expect("rewritten lock toml"),
    )
    .expect("rewrite lock");
}

fn write_lock_fixture(installation: &DetectedFlavorInstallation, content: &str) -> PathBuf {
    let path = lock_path(&addon_state_paths(installation));
    fs::create_dir_all(path.parent().expect("lock parent")).expect("lock parent");
    fs::write(&path, content).expect("write lock fixture");
    path
}

fn lock_fixture_toml(
    source_toml: &str,
    content_sha256: &str,
    addon_directories_toml: &str,
    package_extra_toml: &str,
) -> String {
    format!(
        r#"
schema_version = 1
generated_at = "2026-04-15T00:00:00Z"

[[packages]]
package_id = "details"
source = {source_toml}
content_sha256 = "{content_sha256}"
installed_at = "2026-04-15T00:00:00Z"
updated_at = "2026-04-15T00:00:00Z"
addon_directories = {addon_directories_toml}
addons = []
{package_extra_toml}
"#
    )
}

fn create_untracked_addon(installation: &DetectedFlavorInstallation, addon_name: &str) {
    let addon_dir = installation.addon_dir.join(addon_name);
    fs::create_dir_all(&addon_dir).expect("untracked addon dir");
    fs::write(
        addon_dir.join(format!("{addon_name}.toc")),
        "## Interface: 110000\n## Version: local\n",
    )
    .expect("untracked addon toc");
}

fn create_fixture_installation(root: &Path) -> DetectedFlavorInstallation {
    create_fixture_installation_for_platform(root, HostPlatform::Windows)
}

fn create_fixture_installation_for_platform(
    root: &Path,
    platform: HostPlatform,
) -> DetectedFlavorInstallation {
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
        platform,
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
