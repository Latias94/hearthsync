use std::fs;
use std::path::PathBuf;

use tempfile::tempdir;

use super::{create_fixture_installation, sidecar_addon_state_paths, tracked_package};
use crate::core::addon::provider::AddonSourceRef;
use crate::core::addon::{
    AddonPackageMetadata, AddonRegistry, load_registry, save_registry,
    select_single_tracked_package, select_tracked_packages,
};
use crate::core::error::AppError;

#[test]
fn load_registry_rejects_case_insensitive_duplicate_package_ids() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let state_paths = sidecar_addon_state_paths(&installation);
    let registry = AddonRegistry {
        schema_version: 1,
        packages: vec![
            tracked_package("Details", "Details"),
            tracked_package("details", "Omen"),
        ],
    };
    fs::create_dir_all(state_paths.registry_path.parent().expect("registry parent"))
        .expect("registry parent dir");
    fs::write(
        &state_paths.registry_path,
        toml::to_string_pretty(&registry).expect("registry toml"),
    )
    .expect("write invalid registry");

    let error = load_registry(&installation, &state_paths)
        .expect_err("case-insensitive duplicate package ids should fail");

    assert!(matches!(error, AppError::Validation(_)));
    let message = error.to_string();
    assert!(message.contains("duplicate tracked addon package id"));
    assert!(message.contains("Details"));
    assert!(message.contains("details"));
}

#[test]
fn save_registry_rejects_case_insensitive_duplicate_addon_directory_owners() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let state_paths = sidecar_addon_state_paths(&installation);
    let registry = AddonRegistry {
        schema_version: 1,
        packages: vec![
            tracked_package("details", "Details"),
            tracked_package("details-alt", "details"),
        ],
    };

    let error = save_registry(&installation, &state_paths, &registry)
        .expect_err("case-insensitive duplicate addon directory owners should fail");

    assert!(matches!(error, AppError::Validation(_)));
    let message = error.to_string();
    assert!(message.contains("addon directory `details`"));
    assert!(message.contains("tracked package `details-alt`"));
    assert!(message.contains("tracked package `details`"));
    assert!(!state_paths.registry_path.exists());
}

#[test]
fn save_registry_rejects_non_portable_addon_directory_names() {
    for addon_directory in ["Bad/Addon", "CON", "Weak:Auras"] {
        let temp = tempdir().expect("temp dir");
        let installation = create_fixture_installation(temp.path());
        let state_paths = sidecar_addon_state_paths(&installation);
        let registry = AddonRegistry {
            schema_version: 1,
            packages: vec![tracked_package("details", addon_directory)],
        };

        let error = save_registry(&installation, &state_paths, &registry)
            .expect_err("non-portable addon directory should fail");

        assert!(matches!(error, AppError::Validation(_)));
        let message = error.to_string();
        assert!(message.contains("invalid addon directory name"));
        assert!(message.contains("tracked package `details`"));
        assert!(!state_paths.registry_path.exists());
    }
}

#[test]
fn save_registry_rejects_relative_local_archive_sources() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let state_paths = sidecar_addon_state_paths(&installation);
    let mut package = tracked_package("details", "Details");
    package.source = AddonSourceRef::LocalArchive {
        path: PathBuf::from("archives/details.zip"),
    };
    let registry = AddonRegistry {
        schema_version: 1,
        packages: vec![package],
    };

    let error = save_registry(&installation, &state_paths, &registry)
        .expect_err("relative local archive source should fail");

    assert!(matches!(error, AppError::Validation(_)));
    let message = error.to_string();
    assert!(message.contains("tracked addon package `details`"));
    assert!(message.contains("must be absolute"));
    assert!(!state_paths.registry_path.exists());
}

#[test]
fn save_registry_rejects_invalid_remote_source_refs() {
    for (source, expected_message) in [
        (
            AddonSourceRef::HttpArchive { url: String::new() },
            "HTTP archive source URL",
        ),
        (
            AddonSourceRef::HttpArchive {
                url: "ftp://example.invalid/details.zip".to_string(),
            },
            "HTTP archive source URL",
        ),
        (
            AddonSourceRef::CurseForgeMod {
                mod_id: 0,
                file_id: None,
            },
            "CurseForge mod id",
        ),
        (
            AddonSourceRef::CurseForgeMod {
                mod_id: 12345,
                file_id: Some(0),
            },
            "CurseForge file id",
        ),
        (
            AddonSourceRef::GitHubRelease {
                owner: String::new(),
                repo: "details".to_string(),
                tag: None,
                asset_name: None,
            },
            "GitHub owner",
        ),
        (
            AddonSourceRef::GitHubRelease {
                owner: "owner".to_string(),
                repo: " ".to_string(),
                tag: None,
                asset_name: None,
            },
            "GitHub repo",
        ),
        (
            AddonSourceRef::GitHubRelease {
                owner: "owner".to_string(),
                repo: "bad/repo".to_string(),
                tag: None,
                asset_name: None,
            },
            "GitHub repo",
        ),
        (
            AddonSourceRef::GitHubRelease {
                owner: "owner".to_string(),
                repo: "details".to_string(),
                tag: Some(String::new()),
                asset_name: None,
            },
            "GitHub tag",
        ),
        (
            AddonSourceRef::GitHubRelease {
                owner: "owner".to_string(),
                repo: "details".to_string(),
                tag: Some(" v1.2.3".to_string()),
                asset_name: None,
            },
            "GitHub tag",
        ),
        (
            AddonSourceRef::GitHubRelease {
                owner: "owner".to_string(),
                repo: "details".to_string(),
                tag: None,
                asset_name: Some(String::new()),
            },
            "GitHub asset name",
        ),
        (
            AddonSourceRef::GitHubRelease {
                owner: "owner".to_string(),
                repo: "details".to_string(),
                tag: None,
                asset_name: Some("bad/name.zip".to_string()),
            },
            "GitHub asset name",
        ),
    ] {
        let temp = tempdir().expect("temp dir");
        let installation = create_fixture_installation(temp.path());
        let state_paths = sidecar_addon_state_paths(&installation);
        let mut package = tracked_package("details", "Details");
        package.source = source;
        let registry = AddonRegistry {
            schema_version: 1,
            packages: vec![package],
        };

        let error = save_registry(&installation, &state_paths, &registry)
            .expect_err("invalid remote source should fail");

        assert!(matches!(error, AppError::Validation(_)));
        let message = error.to_string();
        assert!(message.contains("invalid source for tracked addon package `details`"));
        assert!(message.contains(expected_message));
        assert!(!state_paths.registry_path.exists());
    }
}

#[test]
fn save_registry_rejects_blank_metadata_values() {
    for (expected_message, metadata) in [
        (
            "index_name",
            AddonPackageMetadata {
                index_name: Some(" ".to_string()),
                ..Default::default()
            },
        ),
        (
            "index_package_id",
            AddonPackageMetadata {
                index_package_id: Some(String::new()),
                ..Default::default()
            },
        ),
        (
            "package_name",
            AddonPackageMetadata {
                package_name: Some(" ".to_string()),
                ..Default::default()
            },
        ),
        (
            "version",
            AddonPackageMetadata {
                version: Some(String::new()),
                ..Default::default()
            },
        ),
        (
            "source_url",
            AddonPackageMetadata {
                source_url: Some(" ".to_string()),
                ..Default::default()
            },
        ),
        (
            "website_url",
            AddonPackageMetadata {
                website_url: Some(String::new()),
                ..Default::default()
            },
        ),
        (
            "source_sha256",
            AddonPackageMetadata {
                source_sha256: Some(" ".to_string()),
                ..Default::default()
            },
        ),
        (
            "supported flavor",
            AddonPackageMetadata {
                supported_flavors: vec![" ".to_string()],
                ..Default::default()
            },
        ),
    ] {
        let temp = tempdir().expect("temp dir");
        let installation = create_fixture_installation(temp.path());
        let state_paths = sidecar_addon_state_paths(&installation);
        let mut package = tracked_package("details", "Details");
        package.metadata = Some(metadata);
        let registry = AddonRegistry {
            schema_version: 1,
            packages: vec![package],
        };

        let error = save_registry(&installation, &state_paths, &registry)
            .expect_err("blank metadata should fail");

        assert!(matches!(error, AppError::Validation(_)));
        let message = error.to_string();
        assert!(message.contains("tracked addon metadata"));
        assert!(message.contains(expected_message));
        assert!(message.contains("package `details`"));
        assert!(!state_paths.registry_path.exists());
    }
}

#[test]
fn select_tracked_packages_trims_user_selector() {
    let registry = AddonRegistry {
        schema_version: 1,
        packages: vec![tracked_package("details-pack", "Details")],
    };

    let packages = select_tracked_packages(&registry, Some(" Details "))
        .expect("trimmed addon directory selector");
    let (_, single) = select_single_tracked_package(&registry, " details-pack ")
        .expect("trimmed package selector");

    assert_eq!(packages[0].package_id, "details-pack");
    assert_eq!(single.package_id, "details-pack");
}

#[test]
fn select_tracked_packages_rejects_blank_selector() {
    let registry = AddonRegistry {
        schema_version: 1,
        packages: vec![tracked_package("details-pack", "Details")],
    };

    for error in [
        select_tracked_packages(&registry, Some(" ")).expect_err("blank multi selector"),
        select_single_tracked_package(&registry, "").expect_err("blank single selector"),
    ] {
        assert!(matches!(error, AppError::Validation(_)));
        assert!(error.to_string().contains("selector must not be empty"));
    }
}
