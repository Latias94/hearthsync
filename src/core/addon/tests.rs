use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use super::provider::AddonSourceRef;
use super::{TrackedAddon, TrackedAddonPackage};
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};
use crate::core::task::TaskProgressEvent;
use crate::core::task::{TaskKind, TaskPhase};

mod adopt;
mod install;
mod provider_helpers;
mod registry;
mod relink;
mod remove;
mod update_basic;
mod update_dependencies;
mod update_policy;

fn addon_state_paths(installation: &DetectedFlavorInstallation) -> super::AddonStatePaths {
    super::AddonStatePaths::for_installation(super::AddonStateStorageKind::default(), installation)
        .expect("addon state paths")
}

fn sidecar_addon_state_paths(installation: &DetectedFlavorInstallation) -> super::AddonStatePaths {
    super::AddonStatePaths::for_installation(super::AddonStateStorageKind::Sidecar, installation)
        .expect("sidecar addon state paths")
}

fn tracked_package(package_id: &str, addon_directory: &str) -> TrackedAddonPackage {
    TrackedAddonPackage {
        package_id: package_id.to_string(),
        source: AddonSourceRef::HttpArchive {
            url: format!("https://example.invalid/{package_id}.zip"),
        },
        installed_at: "2026-04-28T00:00:00Z".to_string(),
        updated_at: "2026-04-28T00:00:00Z".to_string(),
        addons: vec![TrackedAddon {
            directory_name: addon_directory.to_string(),
            toc_file: Some(format!("{addon_directory}.toc")),
            title: Some(addon_directory.to_string()),
            version: Some("1.0.0".to_string()),
        }],
        metadata: None,
    }
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

fn assert_addon_task_progress(
    events: &[TaskProgressEvent],
    task: TaskKind,
    executing_detail: &str,
) {
    let phases = events
        .iter()
        .map(|event| (event.task, event.phase))
        .collect::<Vec<_>>();

    assert_eq!(phases.first(), Some(&(task, TaskPhase::Preparing)));
    assert_eq!(phases.last(), Some(&(task, TaskPhase::Completed)));
    assert!(phases.contains(&(task, TaskPhase::BackingUp)));
    assert!(phases.contains(&(task, TaskPhase::Executing)));
    assert!(events.iter().any(|event| {
        event.task == task
            && event.phase == TaskPhase::Executing
            && event.message.contains(executing_detail)
    }));
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

fn create_addon_archive_with_symlink_entry(path: &Path, name: &str, target: &str) {
    let file = File::create(path).expect("archive file");
    let mut zip = ZipWriter::new(file);
    zip.add_symlink(name, target, SimpleFileOptions::default())
        .expect("add symlink entry");
    zip.finish().expect("finish zip");
}
