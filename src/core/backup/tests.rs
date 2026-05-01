use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use super::{
    BackupGroup, BackupMetadata, BackupRequest, RestoreBackupRequest, create_backup, list_backups,
    restore_backup, restore_backup_selection, restore_backup_selection_task,
};
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};
use crate::core::task::{NeverCancel, TaskKind, TaskPhase, TaskProgressEvent, VecTaskProgressSink};

mod catalog;
mod create;
mod restore;
mod selection;

enum TestBackupArchiveEntry<'a> {
    File { name: &'a str, content: &'a str },
    Symlink { name: &'a str, target: &'a str },
}

fn write_test_backup_archive(path: &Path, metadata: BackupMetadata) {
    write_test_backup_archive_with_entries(path, metadata, &[]);
}

fn write_test_backup_archive_with_symlink_metadata(path: &Path, target: &str) {
    let file = File::create(path).expect("archive file");
    let mut zip = ZipWriter::new(file);
    zip.add_symlink("backup.toml", target, SimpleFileOptions::default())
        .expect("add backup metadata symlink");
    zip.finish().expect("finish archive");
}

fn write_test_backup_archive_with_entries(
    path: &Path,
    metadata: BackupMetadata,
    entries: &[TestBackupArchiveEntry<'_>],
) {
    let file = File::create(path).expect("archive file");
    let mut zip = ZipWriter::new(file);
    zip.start_file("backup.toml", SimpleFileOptions::default())
        .expect("start backup metadata");
    zip.write_all(
        toml::to_string_pretty(&metadata)
            .expect("serialize metadata")
            .as_bytes(),
    )
    .expect("write backup metadata");
    for entry in entries {
        match entry {
            TestBackupArchiveEntry::File { name, content } => {
                zip.start_file(*name, SimpleFileOptions::default())
                    .expect("start backup entry");
                zip.write_all(content.as_bytes())
                    .expect("write backup entry");
            }
            TestBackupArchiveEntry::Symlink { name, target } => {
                zip.add_symlink(*name, *target, SimpleFileOptions::default())
                    .expect("add backup symlink entry");
            }
        }
    }
    zip.finish().expect("finish archive");
}

fn create_fixture_installation(root: &Path, flavor: WowFlavor) -> DetectedFlavorInstallation {
    let product_root = root.join("World of Warcraft");
    let flavor_root = product_root.join(flavor.folder_name());
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
        flavor,
        interface_dir,
        addon_dir,
        wtf_dir,
        fonts_dir,
    }
}

fn assert_backup_restore_task_progress(events: &[TaskProgressEvent]) {
    let phases = events
        .iter()
        .map(|event| (event.task, event.phase))
        .collect::<Vec<_>>();

    assert_eq!(
        phases.first(),
        Some(&(TaskKind::BackupRestore, TaskPhase::Preparing))
    );
    assert_eq!(
        phases.last(),
        Some(&(TaskKind::BackupRestore, TaskPhase::Completed))
    );
    assert!(phases.contains(&(TaskKind::BackupRestore, TaskPhase::BackingUp)));
    assert!(
        phases
            .iter()
            .any(|phase| { *phase == (TaskKind::BackupRestore, TaskPhase::Executing) })
    );
    assert!(events.iter().any(|event| {
        event.task == TaskKind::BackupRestore
            && event.phase == TaskPhase::Executing
            && (event.message.contains("Clearing restore target group")
                || event.message.contains("Restoring backup entry"))
    }));
}
