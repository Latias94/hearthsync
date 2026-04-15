use std::fs;
use std::io::Write;
use std::path::Path;

use tempfile::tempdir;
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use super::{
    AddonIndexInstallRequest, inspect_addon_index, install_addon_from_index,
    update_addons_from_index,
};
use crate::core::addon::index::AddonIndexUpdateRequest;
use crate::core::addon::{AddonSourceRef, InstallAddonRequest, install_addon, list_addons};
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};

#[test]
fn inspect_addon_index_reads_packages() {
    let temp = tempdir().expect("temp dir");
    let archive_path = temp.path().join("details.zip");
    let index_path = write_index(temp.path(), &archive_path);

    let inspection = inspect_addon_index(&index_path).expect("inspect index");

    assert_eq!(inspection.index.name, "Fixture Index");
    assert_eq!(inspection.package_count, 1);
    assert_eq!(inspection.index.packages[0].id, "details");
}

#[test]
fn install_addon_from_index_installs_selected_package() {
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
    let index_path = write_index(temp.path(), &archive_path);

    let result = install_addon_from_index(AddonIndexInstallRequest {
        installation: installation.clone(),
        index_path,
        name: "details".to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
    })
    .expect("install from index");

    assert_eq!(result.package.id, "details");
    assert!(
        installation
            .addon_dir
            .join("Details")
            .join("Details.toc")
            .exists()
    );
}

#[test]
fn update_addons_from_index_uses_index_source_and_skips_unselected_packages() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let installed_archive_path = temp.path().join("details-installed.zip");
    let updated_archive_path = temp.path().join("details-updated.zip");
    let extra_archive_path = temp.path().join("omen.zip");
    create_addon_archive(
        &installed_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &updated_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 120000\n## Version: 2.0.0\n",
        )],
    );
    create_addon_archive(
        &extra_archive_path,
        &[("Omen/Omen.toc", "## Interface: 110000\n## Version: 1.0.0\n")],
    );
    let index_path = write_index(temp.path(), &updated_archive_path);

    install_addon(InstallAddonRequest {
        installation: installation.clone(),
        source: installed_archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install details");
    install_addon(InstallAddonRequest {
        installation: installation.clone(),
        source: extra_archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install omen");

    let result = update_addons_from_index(AddonIndexUpdateRequest {
        installation: installation.clone(),
        index_path,
        name: None,
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
    })
    .expect("update from index");

    assert_eq!(result.selected_packages.len(), 1);
    assert!(
        fs::read_to_string(installation.addon_dir.join("Details").join("Details.toc"))
            .expect("toc")
            .contains("2.0.0")
    );
    assert!(
        fs::read_to_string(installation.addon_dir.join("Omen").join("Omen.toc"))
            .expect("omen toc")
            .contains("1.0.0")
    );

    let inventory = list_addons(&installation).expect("inventory");
    let details_package = inventory
        .tracked_packages
        .iter()
        .find(|package| {
            package
                .addons
                .iter()
                .any(|addon| addon.directory_name == "Details")
        })
        .expect("details package");
    assert_eq!(
        details_package.source,
        AddonSourceRef::LocalArchive {
            path: updated_archive_path,
        }
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

fn write_index(root: &Path, archive_path: &Path) -> std::path::PathBuf {
    let index_path = root.join("index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "details"
name = "Details"
version = "1.0.0"
source = {{ kind = "local_archive", path = "{}" }}
supported_flavors = ["retail"]
"#,
            archive_path.display().to_string().replace('\\', "\\\\")
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
