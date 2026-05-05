use super::*;

#[test]
fn inspect_addon_lock_reads_wago_sources() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    write_lock_fixture(
        &installation,
        &lock_fixture_toml(
            r#"{ kind = "wago_addon", project_id = "qv63A7Gb", release_id = "vdx1042w" }"#,
            VALID_LOCK_SHA256,
            r#"["Details"]"#,
            "",
        ),
    );

    let inspection =
        inspect_addon_lock(&installation, &addon_state_paths(&installation)).expect("inspect lock");

    assert_eq!(
        inspection.lock.packages[0].source,
        AddonSourceRef::WagoAddon {
            project_id: "qv63A7Gb".to_string(),
            release_id: Some("vdx1042w".to_string()),
        }
    );
}

#[test]
fn inspect_addon_lock_rejects_invalid_source_refs() {
    for (source_toml, expected_message) in [
        (
            r#"{ kind = "local_archive", path = "" }"#,
            "local archive source path",
        ),
        (
            r#"{ kind = "local_archive", path = "sources/details.zip" }"#,
            "must be absolute",
        ),
        (
            r#"{ kind = "http_archive", url = "" }"#,
            "HTTP archive source URL",
        ),
        (
            r#"{ kind = "http_archive", url = "ftp://example.invalid/details.zip" }"#,
            "HTTP archive source URL",
        ),
        (
            r#"{ kind = "curseforge_mod", mod_id = 0 }"#,
            "CurseForge mod id",
        ),
        (
            r#"{ kind = "curseforge_mod", mod_id = 12345, file_id = 0 }"#,
            "CurseForge file id",
        ),
        (
            r#"{ kind = "github_release", owner = "", repo = "details" }"#,
            "GitHub owner",
        ),
        (
            r#"{ kind = "github_release", owner = "owner", repo = " " }"#,
            "GitHub repo",
        ),
        (
            r#"{ kind = "github_release", owner = "owner", repo = "details", tag = "" }"#,
            "GitHub tag",
        ),
        (
            r#"{ kind = "github_release", owner = "owner", repo = "details", asset_name = "" }"#,
            "GitHub asset name",
        ),
        (
            r#"{ kind = "wago_addon", project_id = "" }"#,
            "Wago project id",
        ),
        (
            r#"{ kind = "wago_addon", project_id = "qv63A7Gb", release_id = "bad/id" }"#,
            "Wago release id",
        ),
    ] {
        let temp = tempdir().expect("temp dir");
        let installation = create_fixture_installation(temp.path());
        write_lock_fixture(
            &installation,
            &lock_fixture_toml(source_toml, VALID_LOCK_SHA256, r#"["Details"]"#, ""),
        );

        let error = inspect_addon_lock(&installation, &addon_state_paths(&installation))
            .expect_err("invalid source should fail");

        assert!(matches!(error, AppError::Validation(_)));
        let message = error.to_string();
        assert!(message.contains("invalid source for addon lock package `details`"));
        assert!(message.contains(expected_message));
    }
}

#[test]
fn inspect_addon_lock_rejects_non_portable_addon_directories() {
    for addon_directory in ["Bad/Addon", "CON", "Weak:Auras"] {
        let temp = tempdir().expect("temp dir");
        let installation = create_fixture_installation(temp.path());
        write_lock_fixture(
            &installation,
            &lock_fixture_toml(
                r#"{ kind = "http_archive", url = "https://example.invalid/details.zip" }"#,
                VALID_LOCK_SHA256,
                &format!(r#"["{addon_directory}"]"#),
                "",
            ),
        );

        let error = inspect_addon_lock(&installation, &addon_state_paths(&installation))
            .expect_err("non-portable addon directory should fail");

        assert!(matches!(error, AppError::Validation(_)));
        let message = error.to_string();
        assert!(message.contains("invalid addon directory name"));
        assert!(message.contains("addon lock package `details`"));
    }
}

#[test]
fn inspect_addon_lock_rejects_duplicate_addon_directories() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    write_lock_fixture(
        &installation,
        &lock_fixture_toml(
            r#"{ kind = "http_archive", url = "https://example.invalid/details.zip" }"#,
            VALID_LOCK_SHA256,
            r#"["Details", "details"]"#,
            "",
        ),
    );

    let error = inspect_addon_lock(&installation, &addon_state_paths(&installation))
        .expect_err("duplicate addon directory should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("duplicate addon directory"));
}

#[test]
fn inspect_addon_lock_rejects_blank_metadata_fields() {
    for (field, field_toml) in [
        ("index_name", r#"index_name = " ""#),
        ("index_package_id", r#"index_package_id = """#),
        ("name", r#"name = " ""#),
        ("version", r#"version = """#),
        ("source_url", r#"source_url = " ""#),
        ("website_url", r#"website_url = """#),
        ("source_sha256", r#"source_sha256 = " ""#),
    ] {
        let temp = tempdir().expect("temp dir");
        let installation = create_fixture_installation(temp.path());
        write_lock_fixture(
            &installation,
            &lock_fixture_toml(
                r#"{ kind = "http_archive", url = "https://example.invalid/details.zip" }"#,
                VALID_LOCK_SHA256,
                r#"["Details"]"#,
                field_toml,
            ),
        );

        let error = inspect_addon_lock(&installation, &addon_state_paths(&installation))
            .expect_err("blank metadata should fail");

        assert!(matches!(error, AppError::Validation(_)));
        let message = error.to_string();
        assert!(message.contains(field));
        assert!(message.contains("must not be blank"));
        assert!(message.contains("addon lock package `details`"));
    }
}

#[test]
fn inspect_addon_lock_rejects_invalid_content_hash() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    write_lock_fixture(
        &installation,
        &lock_fixture_toml(
            r#"{ kind = "http_archive", url = "https://example.invalid/details.zip" }"#,
            "not-a-sha256",
            r#"["Details"]"#,
            "",
        ),
    );

    let error = inspect_addon_lock(&installation, &addon_state_paths(&installation))
        .expect_err("invalid content hash should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("content_sha256"));
    assert!(error.to_string().contains("64-character"));
}
