use super::*;

#[test]
fn analyze_external_package_conflict_fixture_exposes_duplicate_normalized_paths() {
    let package_root = external_package_conflict_fixture_root();

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_root,
    })
    .expect("analyze external package conflict fixture");

    assert_eq!(analysis.summary.total_files, 2);
    assert_eq!(analysis.summary.normalized_files, 2);
    assert_eq!(analysis.summary.ignored_files, 0);
    assert_eq!(analysis.summary.warning_count, 0);
    assert!(analysis.summary.warning_groups.is_empty());
    assert!(analysis.warnings.is_empty());
    assert_eq!(analysis.resources.addons, vec!["WeakAuras".to_string()]);

    let duplicate_count = analysis
        .entries
        .iter()
        .filter(|entry| entry.normalized_path == "addons/WeakAuras/WeakAuras.toc")
        .count();
    assert_eq!(duplicate_count, 2);
}

#[test]
fn create_external_package_bundle_rejects_duplicate_normalized_paths_from_directory_fixture() {
    let error =
        create_external_package_bundle(sample_external_package_request_with_apply_defaults(
            external_package_conflict_fixture_root(),
            None,
        ))
        .expect_err("duplicate normalized paths should fail");

    let message = error.to_string();
    assert!(message.contains("normalizes multiple files onto the same target path"));
    assert!(message.contains("addons/WeakAuras/WeakAuras.toc"));
}

#[test]
fn create_external_package_bundle_rejects_duplicate_normalized_paths_from_zip_fixture() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("conflicting-author-pack.zip");
    create_archive_from_directory(&external_package_conflict_fixture_root(), &package_path);

    let error = create_external_package_bundle(
        sample_external_package_request_with_apply_defaults(package_path, None),
    )
    .expect_err("duplicate normalized paths in zip should fail");

    let message = error.to_string();
    assert!(message.contains("normalizes multiple files onto the same target path"));
    assert!(message.contains("addons/WeakAuras/WeakAuras.toc"));
}

#[test]
fn create_external_package_bundle_rejects_case_insensitive_target_path_collisions_from_zip_fixture()
{
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("case-collision-author-pack.zip");
    create_archive_with_raw_entries(
        &package_path,
        &[
            ("Fonts/FRIZQT__.ttf", "font-a"),
            ("fonts/frizqt__.ttf", "font-b"),
        ],
    );

    let error = create_external_package_bundle(
        sample_external_package_request_with_apply_defaults(package_path, None),
    )
    .expect_err("case-insensitive normalized path collisions should fail");

    let message = error.to_string();
    assert!(message.contains("case-insensitive target path collisions"));
    assert!(message.contains("fonts/FRIZQT__.ttf"));
    assert!(message.contains("fonts/frizqt__.ttf"));
}

#[test]
fn analyze_external_package_rejects_zip_with_parent_directory_segments() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("unsafe-parent.zip");
    create_archive_with_raw_entries(
        &package_path,
        &[
            ("../evil.txt", "evil"),
            ("AuthorUI/WTF/Config.wtf", "SET locale enUS"),
        ],
    );

    let error = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_path,
    })
    .expect_err("parent directory zip entry should be rejected");

    assert!(error.to_string().contains("unsafe archive path"));
}

#[test]
fn analyze_external_package_rejects_zip_with_backslash_segments() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("unsafe-backslash.zip");
    create_archive_with_raw_entries(
        &package_path,
        &[("AuthorUI\\WTF\\Config.wtf", "SET locale enUS")],
    );

    let error = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_path,
    })
    .expect_err("backslash zip entry should be rejected");

    assert!(error.to_string().contains("unsafe archive path"));
}

#[test]
fn analyze_external_package_rejects_zip_with_empty_path_segments() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("unsafe-empty-segment.zip");
    create_archive_with_raw_entries(
        &package_path,
        &[("AuthorUI//WTF/Config.wtf", "SET locale enUS")],
    );

    let error = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_path,
    })
    .expect_err("empty path segment zip entry should be rejected");

    assert!(error.to_string().contains("unsafe archive path"));
}

#[test]
fn analyze_external_package_rejects_zip_with_windows_reserved_segments() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("unsafe-reserved-segment.zip");
    create_archive_with_raw_entries(
        &package_path,
        &[("AuthorUI/Interface/AddOns/Weak:Auras/WeakAuras.toc", "toc")],
    );

    let error = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_path,
    })
    .expect_err("reserved Windows path segment should be rejected");

    assert!(error.to_string().contains("unsafe archive path"));
}

#[test]
fn analyze_external_package_rejects_zip_symlink_entries() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("symlink-author-pack.zip");
    create_archive_with_symlink_entry(
        &package_path,
        "AuthorUI/Interface/AddOns/WeakAuras/WeakAuras.lua",
        "../../outside.lua",
    );

    let error = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_path,
    })
    .expect_err("symlink zip entry should be rejected");

    let message = error.to_string();
    assert!(message.contains("unsupported symlink metadata"));
    assert!(message.contains("AuthorUI/Interface/AddOns/WeakAuras/WeakAuras.lua"));
}

#[test]
fn analyze_external_package_rejects_non_archive_file_with_clear_error() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("not-a-zip.bin");
    fs::write(&package_path, "plain text").expect("plain file");

    let error = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_path.clone(),
    })
    .expect_err("plain file should not be treated as zip");

    let message = error.to_string();
    assert!(message.contains("not a valid zip archive"));
    assert!(message.contains(&package_path.display().to_string()));
}

#[test]
fn create_external_package_bundle_rejects_zip_with_only_directory_entries() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("directory-only.zip");
    create_archive_with_raw_directories(
        &package_path,
        &[
            "AuthorUI/",
            "AuthorUI/Interface/",
            "AuthorUI/Interface/AddOns/",
            "AuthorUI/WTF/",
        ],
    );

    let error = create_external_package_bundle(
        sample_external_package_request_with_apply_defaults(package_path, None),
    )
    .expect_err("directory-only zip should not build a bundle");

    let message = error.to_string();
    assert!(message.contains("resources must include at least one addon"));
}

#[test]
fn analyze_external_package_directory_rejects_non_portable_relative_segments() {
    let error = crate::core::archive_path::safe_relative_segments(
        std::path::Path::new("AuthorUI/Interface/AddOns/Weak:Auras/WeakAuras.toc"),
        "directory entry path",
    )
    .expect_err("non-portable directory segment should fail");

    assert!(error.to_string().contains("unsafe directory entry path"));
}
