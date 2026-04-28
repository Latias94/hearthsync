use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::core::bundle::CreateExternalPackageBundleRequest;
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};
use crate::core::manifest::{
    ApplyDefaults, BundleManifest, BundleResources, CharacterMappingMode, CharacterResource,
    MappingRules, PackageMetadata, ResourceApplyPolicy, SourceInstallation,
};

pub(super) fn create_fixture_installation(
    root: &Path,
    with_content: bool,
) -> DetectedFlavorInstallation {
    create_fixture_installation_on_platform(root, with_content, HostPlatform::Windows)
}

pub(super) fn create_fixture_installation_on_platform(
    root: &Path,
    with_content: bool,
    platform: HostPlatform,
) -> DetectedFlavorInstallation {
    let product_root = root.join("World of Warcraft");
    let flavor_root = product_root.join("_retail_");
    let interface_dir = flavor_root.join("Interface");
    let addon_dir = interface_dir.join("AddOns");
    let wtf_dir = flavor_root.join("WTF");
    let fonts_dir = flavor_root.join("Fonts");

    fs::create_dir_all(&addon_dir).expect("addon root");
    fs::create_dir_all(&wtf_dir).expect("wtf root");
    fs::create_dir_all(&fonts_dir).expect("fonts root");

    if with_content {
        fs::create_dir_all(addon_dir.join("WeakAuras")).expect("addon dir");
        fs::write(
            addon_dir.join("WeakAuras").join("WeakAuras.toc"),
            "## Interface: 110000",
        )
        .expect("toc");
        fs::write(
            addon_dir.join("WeakAuras").join("WeakAuras.lua"),
            r#"
WeakAurasSaved = {
  ["profileKeys"] = {
    ["Examplemage - Illidan"] = "Default",
  },
  ["player"] = "Examplemage",
}
"#,
        )
        .expect("addon lua");

        fs::write(wtf_dir.join("Config.wtf"), "SET locale enUS").expect("config");
        fs::create_dir_all(
            wtf_dir
                .join("Account")
                .join("ACCOUNT")
                .join("SavedVariables"),
        )
        .expect("saved variables");
        fs::write(
            wtf_dir
                .join("Account")
                .join("ACCOUNT")
                .join("SavedVariables")
                .join("Details.lua"),
            r#"
DetailsDB = {
  ["profileKeys"] = {
    ["Examplemage - Illidan"] = "Default",
  },
  ["profiles"] = {
    ["Default.Illidan.Examplemage"] = {},
  },
}
"#,
        )
        .expect("saved variable");
        fs::create_dir_all(
            wtf_dir
                .join("Account")
                .join("ACCOUNT")
                .join("Illidan")
                .join("Examplemage"),
        )
        .expect("character");
        fs::create_dir_all(
            wtf_dir
                .join("Account")
                .join("ACCOUNT")
                .join("Illidan")
                .join("Examplemage")
                .join("SavedVariables"),
        )
        .expect("character saved variables");
        fs::write(
            wtf_dir
                .join("Account")
                .join("ACCOUNT")
                .join("Illidan")
                .join("Examplemage")
                .join("AddOns.txt"),
            "WeakAuras: enabled",
        )
        .expect("addons state");
        fs::write(
            wtf_dir
                .join("Account")
                .join("ACCOUNT")
                .join("Illidan")
                .join("Examplemage")
                .join("SavedVariables")
                .join("Pawn.lua"),
            r#"
PawnOptions = {
  ["LastPlayerFullName"] = "Examplemage",
  ["LastRealm"] = "Illidan",
}
"#,
        )
        .expect("character lua");

        fs::write(fonts_dir.join("FRIZQT__.ttf"), "font").expect("font");
        fs::create_dir_all(interface_dir.join("SharedXML")).expect("asset dir");
        fs::write(
            interface_dir.join("SharedXML").join("texture.blp"),
            "texture",
        )
        .expect("asset");
    }

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

pub(super) fn seed_external_package_policy_target(installation: &DetectedFlavorInstallation) {
    fs::create_dir_all(installation.addon_dir.join("WeakAuras")).expect("addon dir");
    fs::write(
        installation.addon_dir.join("WeakAuras").join("Stale.lua"),
        "print('stale')",
    )
    .expect("stale addon");

    fs::write(installation.wtf_dir.join("Config.wtf"), "SET locale zhCN").expect("config");
    fs::create_dir_all(
        installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("Illidan")
            .join("Examplemage")
            .join("SavedVariables"),
    )
    .expect("character dir");
    fs::write(
        installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("Illidan")
            .join("Examplemage")
            .join("StaleCharacter.txt"),
        "stale-character",
    )
    .expect("stale character root");
    fs::write(
        installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("Illidan")
            .join("Examplemage")
            .join("SavedVariables")
            .join("Old.lua"),
        "OldSaved = true",
    )
    .expect("stale character saved variables");

    fs::write(installation.fonts_dir.join("FRIZQT__.ttf"), "mac-font").expect("font");
    fs::create_dir_all(installation.interface_dir.join("SharedXML")).expect("shared xml");
    fs::write(
        installation.interface_dir.join("SharedXML").join("old.blp"),
        "old-texture",
    )
    .expect("old texture");
}

pub(super) fn bundle_testdata_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("core")
        .join("bundle")
        .join("testdata")
        .join(name)
}

pub(super) fn external_package_fixture_root() -> PathBuf {
    bundle_testdata_path("external_package_author_ui_wrapped")
}

pub(super) fn external_package_dirty_fixture_root() -> PathBuf {
    bundle_testdata_path("external_package_dirty_mixed_case")
}

pub(super) fn external_package_conflict_fixture_root() -> PathBuf {
    bundle_testdata_path("external_package_conflicting_duplicates")
}

pub(super) fn create_external_package_fixture_archive(archive_path: &Path) {
    create_archive_from_directory(&external_package_fixture_root(), archive_path);
}

pub(super) fn create_archive_from_directory(source_root: &Path, archive_path: &Path) {
    let file = fs::File::create(archive_path).expect("archive file");
    let mut zip = ZipWriter::new(file);
    add_directory_entries_to_zip(&mut zip, source_root, source_root);
    zip.finish().expect("finish archive");
}

pub(super) fn create_archive_with_raw_entries(archive_path: &Path, entries: &[(&str, &str)]) {
    let file = fs::File::create(archive_path).expect("archive file");
    let mut zip = ZipWriter::new(file);
    for (name, content) in entries {
        zip.start_file(
            *name,
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
        )
        .expect("start raw archive file");
        zip.write_all(content.as_bytes())
            .expect("write raw archive file");
    }
    zip.finish().expect("finish raw archive");
}

pub(super) fn create_archive_with_owned_raw_entries(
    archive_path: &Path,
    entries: &[(String, String)],
) {
    let file = fs::File::create(archive_path).expect("archive file");
    let mut zip = ZipWriter::new(file);
    for (name, content) in entries {
        zip.start_file(
            name,
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
        )
        .expect("start raw archive file");
        zip.write_all(content.as_bytes())
            .expect("write raw archive file");
    }
    zip.finish().expect("finish raw archive");
}

pub(super) fn create_archive_with_raw_directories(archive_path: &Path, entries: &[&str]) {
    let file = fs::File::create(archive_path).expect("archive file");
    let mut zip = ZipWriter::new(file);
    for name in entries {
        zip.add_directory(
            *name,
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
        )
        .expect("add raw archive directory");
    }
    zip.finish().expect("finish raw directory archive");
}

pub(super) fn create_archive_with_symlink_entry(archive_path: &Path, name: &str, target: &str) {
    let file = fs::File::create(archive_path).expect("archive file");
    let mut zip = ZipWriter::new(file);
    zip.add_symlink(name, target, SimpleFileOptions::default())
        .expect("add raw symlink entry");
    zip.finish().expect("finish raw symlink archive");
}

pub(super) fn create_archive_with_raw_entries_and_symlink(
    archive_path: &Path,
    entries: &[(&str, &str)],
    symlink_name: &str,
    symlink_target: &str,
) {
    let file = fs::File::create(archive_path).expect("archive file");
    let mut zip = ZipWriter::new(file);
    for (name, content) in entries {
        zip.start_file(
            *name,
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
        )
        .expect("start raw archive file");
        zip.write_all(content.as_bytes())
            .expect("write raw archive file");
    }
    zip.add_symlink(symlink_name, symlink_target, SimpleFileOptions::default())
        .expect("add raw symlink entry");
    zip.finish().expect("finish raw archive");
}

pub(super) fn add_directory_entries_to_zip(
    zip: &mut ZipWriter<fs::File>,
    source_root: &Path,
    current: &Path,
) {
    let mut entries = fs::read_dir(current)
        .expect("read dir")
        .map(|entry| entry.expect("dir entry").path())
        .collect::<Vec<_>>();
    entries.sort();

    for entry_path in entries {
        if entry_path.is_dir() {
            add_directory_entries_to_zip(zip, source_root, &entry_path);
            continue;
        }

        let archive_name = entry_path
            .strip_prefix(source_root)
            .expect("relative fixture path")
            .to_string_lossy()
            .replace('\\', "/");
        zip.start_file(
            archive_name,
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
        )
        .expect("start fixture file");
        zip.write_all(&fs::read(&entry_path).expect("fixture bytes"))
            .expect("write fixture file");
    }
}

pub(super) fn create_addon_archive(path: &Path, entries: &[(&str, &str)]) {
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

pub(super) fn sample_manifest() -> BundleManifest {
    BundleManifest {
        schema_version: 1,
        package: PackageMetadata {
            id: "test-ui".to_string(),
            name: "Test UI".to_string(),
            created_by: "test".to_string(),
            description: None,
        },
        source: SourceInstallation {
            flavor: WowFlavor::Retail,
            platform: None,
            exported_at: None,
            supported_targets: vec![WowFlavor::Retail],
        },
        resources: BundleResources {
            addons: vec!["WeakAuras".to_string()],
            wtf_common: true,
            wtf_characters: vec![CharacterResource {
                source_account: Some("ACCOUNT".to_string()),
                source_server: "Illidan".to_string(),
                source_character: "Examplemage".to_string(),
                target_hint: None,
            }],
            fonts: true,
            interface_assets: vec!["SharedXML".to_string()],
            addon_lock: false,
            addon_indexes: Vec::new(),
        },
        mapping: MappingRules {
            character_mode: CharacterMappingMode::KeepOriginal,
            rewrite_profile_keys: false,
            rewrite_identity_strings: false,
            allow_cross_platform: true,
        },
        apply: ApplyDefaults {
            create_backup: true,
            addons: ResourceApplyPolicy::Merge,
            wtf_common: ResourceApplyPolicy::Merge,
            wtf_characters: ResourceApplyPolicy::Merge,
            fonts: ResourceApplyPolicy::Merge,
            interface_assets: ResourceApplyPolicy::Merge,
        },
    }
}

pub(super) fn sample_manifest_with_rewrite() -> BundleManifest {
    let mut manifest = sample_manifest();
    manifest.mapping.rewrite_profile_keys = true;
    manifest.mapping.rewrite_identity_strings = true;
    manifest
}

pub(super) fn sample_external_package_request_with_apply_defaults(
    source_path: PathBuf,
    apply_defaults: Option<ApplyDefaults>,
) -> CreateExternalPackageBundleRequest {
    CreateExternalPackageBundleRequest {
        source_path,
        source_flavor: WowFlavor::Retail,
        source_platform: Some(HostPlatform::Windows),
        supported_targets: vec![WowFlavor::Retail],
        output_path: None,
        package_id: Some("author-ui-import".to_string()),
        package_name: Some("Author UI Import".to_string()),
        created_by: Some("hearthsync-test".to_string()),
        description: Some("fixture external package".to_string()),
        apply_defaults,
    }
}
