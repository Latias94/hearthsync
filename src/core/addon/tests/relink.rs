use std::fs;

use tempfile::tempdir;

use super::{addon_state_paths, create_addon_archive, create_fixture_installation};
use crate::core::addon::provider::AddonSourceRef;
use crate::core::addon::{
    AddonPackageMetadata, InstallAddonRequest, RelinkAddonRequest, canonicalize_local_archive_path,
    install_addon, list_addons, relink_addon,
};
use crate::core::error::AppError;

#[test]
fn relink_addon_updates_tracked_source_and_clears_metadata_without_reinstalling_files() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let installed_archive = temp.path().join("Details.zip");
    let relink_archive = temp.path().join("Details-release.zip");
    create_addon_archive(
        &installed_archive,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &relink_archive,
        &[(
            "Details/Details.toc",
            "## Interface: 120000\n## Version: 2.0.0\n",
        )],
    );

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: installed_archive.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: Some(AddonPackageMetadata {
            index_name: Some("curated".to_string()),
            index_package_id: Some("details".to_string()),
            package_name: Some("Details".to_string()),
            version: Some("1.0.0".to_string()),
            source_url: Some("https://example.invalid/details.zip".to_string()),
            website_url: Some("https://example.invalid/details".to_string()),
            source_sha256: Some("abc123".to_string()),
            supported_flavors: vec!["retail".to_string()],
        }),
    })
    .expect("install addon");

    let result = relink_addon(RelinkAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        name: "Details".to_string(),
        source: relink_archive.display().to_string(),
        dry_run: false,
    })
    .expect("relink addon");

    assert_eq!(result.package_id, "details");
    assert!(result.cleared_metadata);

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("list addons");
    assert_eq!(inventory.tracked_packages.len(), 1);
    assert_eq!(
        inventory.tracked_packages[0].source,
        AddonSourceRef::LocalArchive {
            path: canonicalize_local_archive_path(&relink_archive)
                .expect("normalized relink archive"),
        }
    );
    assert!(inventory.tracked_packages[0].metadata.is_none());
    assert!(
        fs::read_to_string(installation.addon_dir.join("Details").join("Details.toc"))
            .expect("installed toc")
            .contains("1.0.0")
    );
}

#[test]
fn relink_addon_rejects_incompatible_addon_directory_sets() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let installed_archive = temp.path().join("Plater.zip");
    let incompatible_archive = temp.path().join("Plater-remote.zip");
    create_addon_archive(
        &installed_archive,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &incompatible_archive,
        &[(
            "PlaterOptions/PlaterOptions.toc",
            "## Interface: 120000\n## Version: 2.0.0\n",
        )],
    );

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: installed_archive.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install addon");

    let error = relink_addon(RelinkAddonRequest {
        state_paths: addon_state_paths(&installation),
        installation,
        name: "Plater".to_string(),
        source: incompatible_archive.display().to_string(),
        dry_run: false,
    })
    .expect_err("incompatible relink should fail");

    assert!(matches!(error, AppError::Validation(_)));
    let message = error.to_string();
    assert!(message.contains("addon directory sets must match exactly"));
    assert!(message.contains("missing from source: Plater"));
    assert!(message.contains("extra from source: PlaterOptions"));
}
