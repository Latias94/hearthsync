use std::fs;

use tempfile::tempdir;
use zip::ZipArchive;

use super::support::*;
use crate::core::addon::lock::plan_addon_lock_sync;
use crate::core::addon::{InstallAddonRequest, install_addon};
use crate::core::bundle::*;
use crate::core::manifest::{CharacterMappingMode, ResourceApplyPolicy};

#[test]
fn pack_bundle_writes_normalized_layout() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path(), true);
    let bundle_path = temp.path().join("bundle.zip");

    let bundle = pack_bundle(PackBundleRequest {
        installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    assert_eq!(bundle.archive_path, bundle_path);

    let file = fs::File::open(bundle.archive_path).expect("bundle file");
    let mut archive = ZipArchive::new(file).expect("zip archive");

    assert!(archive.by_name("manifest.toml").is_ok());
    assert!(archive.by_name("addons/WeakAuras/WeakAuras.toc").is_ok());
    assert!(archive.by_name("addons/WeakAuras/WeakAuras.lua").is_ok());
    assert!(archive.by_name("wtf/common/Config.wtf").is_ok());
    assert!(
        archive
            .by_name("wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua")
            .is_ok()
    );
    assert!(
        archive
            .by_name("wtf/characters/ACCOUNT/Illidan/Examplemage/AddOns.txt")
            .is_ok()
    );
    assert!(archive.by_name("fonts/FRIZQT__.ttf").is_ok());
    assert!(archive.by_name("interface/SharedXML/texture.blp").is_ok());
}

#[test]
fn pack_bundle_defaults_output_path_relative_to_manifest_base_dir() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path(), true);
    let manifest_dir = temp.path().join("manifest-root");
    fs::create_dir_all(&manifest_dir).expect("manifest dir");

    let bundle = pack_bundle(PackBundleRequest {
        installation,
        manifest: sample_manifest(),
        output_path: None,
        manifest_base_dir: Some(manifest_dir.clone()),
    })
    .expect("pack bundle");

    assert_eq!(bundle.archive_path.parent(), Some(manifest_dir.as_path()));
    assert_eq!(
        bundle
            .archive_path
            .extension()
            .and_then(|item| item.to_str()),
        Some("zip")
    );
    assert!(bundle.archive_path.is_file());
}

#[test]
fn pack_bundle_resolves_relative_output_path_against_manifest_base_dir() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path(), true);
    let manifest_dir = temp.path().join("manifest-root");
    let expected_output_dir = manifest_dir.join("exports");
    fs::create_dir_all(&manifest_dir).expect("manifest dir");

    let bundle = pack_bundle(PackBundleRequest {
        installation,
        manifest: sample_manifest(),
        output_path: Some(std::path::PathBuf::from("exports")),
        manifest_base_dir: Some(manifest_dir.clone()),
    })
    .expect("pack bundle");

    assert_eq!(
        bundle.archive_path.parent(),
        Some(expected_output_dir.as_path())
    );
    assert!(bundle.archive_path.is_file());
}

#[test]
fn pack_bundle_defaults_output_path_next_to_installation_without_manifest_base_dir() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path(), true);

    let bundle = pack_bundle(PackBundleRequest {
        installation,
        manifest: sample_manifest(),
        output_path: None,
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    assert_eq!(bundle.archive_path.parent(), Some(temp.path()));
    assert!(bundle.archive_path.is_file());
}

#[test]
fn pack_bundle_rejects_relative_addon_index_without_manifest_base_dir() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path(), true);
    let mut manifest = sample_manifest();
    manifest.resources.addon_indexes = vec!["indexes/addon-index.toml".to_string()];

    let error = pack_bundle(PackBundleRequest {
        installation,
        manifest,
        output_path: Some(temp.path().join("bundle.zip")),
        manifest_base_dir: None,
    })
    .expect_err("relative addon index without manifest base dir should fail");

    assert!(
        error
            .to_string()
            .contains("relative addon index path requires `manifest_base_dir`")
    );
}

#[test]
fn pack_bundle_embeds_addon_lock_and_indexes_as_sidecar_metadata() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), false);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let archive_path = source.path().join("WeakAuras.zip");
    let index_path = source.path().join("addon-index.toml");

    create_addon_archive(
        &archive_path,
        &[(
            "WeakAuras/WeakAuras.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    install_addon(InstallAddonRequest {
        installation: source_installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(source.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install tracked addon");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "weakauras"
name = "WeakAuras"
version = "1.0.0"
source = { kind = "local_archive", path = "WeakAuras.zip" }
"#,
    )
    .expect("index");

    let mut manifest = sample_manifest();
    manifest.resources.addons = Vec::new();
    manifest.resources.wtf_common = false;
    manifest.resources.wtf_characters = Vec::new();
    manifest.resources.fonts = false;
    manifest.resources.interface_assets = Vec::new();
    manifest.resources.addon_lock = true;
    manifest.resources.addon_indexes = vec!["addon-index.toml".to_string()];
    manifest.mapping.character_mode = CharacterMappingMode::KeepOriginal;
    manifest.apply.addons = ResourceApplyPolicy::Mirror;

    let bundle = pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: Some(source.path().to_path_buf()),
    })
    .expect("pack bundle");

    let file = fs::File::open(&bundle.archive_path).expect("bundle file");
    let mut archive = ZipArchive::new(file).expect("zip archive");
    assert!(archive.by_name("metadata/addons/lock.toml").is_ok());
    assert!(archive.by_name("metadata/addons/sources.toml").is_ok());
    assert!(
        archive
            .by_name("metadata/addons/sources/addons-weakauras.zip")
            .is_ok()
    );
    assert!(
        archive
            .by_name("metadata/addons/indexes/addon-index.toml")
            .is_ok()
    );

    let inspection = inspect_bundle(&bundle.archive_path).expect("inspect bundle");
    assert_eq!(inspection.entries.metadata, 5);
    assert_eq!(
        inspection.manifest.apply.addons,
        ResourceApplyPolicy::Mirror
    );
    fs::remove_file(&archive_path).expect("remove original addon source");

    unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("unpack bundle");

    let sidecar_root = target_installation
        .addon_dir
        .join(".hearthsync")
        .join("bundles")
        .join("test-ui");
    assert!(sidecar_root.join("addons").join("lock.toml").exists());
    assert!(
        sidecar_root
            .join("addons")
            .join("indexes")
            .join("addon-index.toml")
            .exists()
    );

    let sidecar_plan = plan_addon_lock_sync(
        &target_installation,
        Some(&sidecar_root.join("addons").join("lock.toml")),
    )
    .expect("sidecar addon plan");
    assert_eq!(sidecar_plan.install_count, 1);
    assert_eq!(sidecar_plan.blocked_count, 0);

    let addon_plan =
        plan_bundle_addon_lock(&bundle.archive_path, &target_installation).expect("addon plan");
    assert_eq!(addon_plan.plan.install_count, 1);
    assert_eq!(addon_plan.plan.update_count, 0);
    assert_eq!(addon_plan.plan.remove_count, 0);
    assert_eq!(addon_plan.plan.blocked_count, 0);

    let addon_apply = apply_bundle_addon_lock(BundleAddonLockApplyRequest {
        bundle_path: bundle.archive_path,
        installation: target_installation.clone(),
        backup_output_path: Some(target.path().join("addon-backups")),
        replace_existing: false,
    })
    .expect("addon apply");
    assert!(addon_apply.apply.verification.matches);
    assert!(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
}
