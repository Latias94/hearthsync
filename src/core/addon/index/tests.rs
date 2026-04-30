use std::fs;
use std::io::Write;
use std::path::Path;

use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::core::addon::canonicalize_local_archive_path;
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};
use crate::core::task::{TaskKind, TaskPhase, TaskProgressEvent};

mod attach;
mod curation;
mod inspect;
mod install;
mod relink;
mod update_basic;
mod update_dependencies;
mod update_matching;
mod update_policy;

fn addon_state_paths(
    installation: &DetectedFlavorInstallation,
) -> crate::core::addon::AddonStatePaths {
    crate::core::addon::AddonStatePaths::for_installation(
        crate::core::addon::AddonStateStorageKind::default(),
        installation,
    )
    .expect("addon state paths")
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

fn write_index(root: &Path, archive_path: &Path) -> std::path::PathBuf {
    write_index_package(
        root,
        "details",
        "Details",
        "1.0.0",
        &format!(
            r#"{{ kind = "local_archive", path = "{}" }}"#,
            archive_path.display().to_string().replace('\\', "\\\\")
        ),
    )
}

fn write_index_package(
    root: &Path,
    package_id: &str,
    package_name: &str,
    version: &str,
    source_toml: &str,
) -> std::path::PathBuf {
    let index_path = root.join("index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "{package_id}"
name = "{package_name}"
version = "{version}"
source = {source_toml}
supported_flavors = ["retail"]
"#
        ),
    )
    .expect("index");
    index_path
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

fn normalized_archive_path(path: &Path) -> std::path::PathBuf {
    canonicalize_local_archive_path(path).expect("normalized archive path")
}

fn assert_addon_index_task_progress(
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
