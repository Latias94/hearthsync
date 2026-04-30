use super::*;

#[test]
fn addon_index_service_install_with_runtime_uses_injected_provider() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("WeakAuras-runtime.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "WeakAuras/WeakAuras.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    let index_path = temp.path().join("addon-index.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "weakauras"
name = "WeakAuras"
version = "1.0.0"
source = { kind = "http_archive", url = "https://example.invalid/WeakAuras.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");

    let service =
        AddonIndexService::with_runtime(AppRuntime::with_addon_provider(FakeAddonProvider {
            archive_path: archive_path.clone(),
        }));
    let result = service
        .install(InstallAddonIndexAppRequest {
            installation,
            index_path,
            name: "weakauras".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
        })
        .expect("install from index through injected provider");

    assert_eq!(result.package.id, "weakauras");
    assert_eq!(result.install.package_id, "weakauras");
    assert_eq!(
        result.install.source.kind,
        crate::core::app::AddonSourceKindResult::HttpArchive
    );
    assert_eq!(
        result.install.source.url.as_deref(),
        Some("https://example.invalid/WeakAuras.zip")
    );
}

#[test]
fn addon_index_service_install_collecting_progress_includes_download_byte_events() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("WeakAuras-progress.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "WeakAuras/WeakAuras.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    let index_path = temp.path().join("addon-index-http.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "weakauras"
name = "WeakAuras"
version = "1.0.0"
source = { kind = "http_archive", url = "https://example.invalid/WeakAuras.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");

    let service = AddonIndexService::with_runtime(AppRuntime::with_addon_provider(
        FakeDownloadProgressAddonProvider {
            archive_path: archive_path.clone(),
        },
    ));
    let run = service
        .install_collecting_progress(InstallAddonIndexAppRequest {
            installation,
            index_path,
            name: "weakauras".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
        })
        .expect("install from index with byte progress");

    let download_events = run
        .progress
        .iter()
        .filter(|event| event.code == Some(TaskProgressCode::DownloadArchive))
        .collect::<Vec<_>>();
    assert_eq!(download_events.len(), 2);
    assert!(
        download_events
            .iter()
            .all(|event| event.task == TaskKind::AddonIndexInstall)
    );
    assert!(
        download_events
            .iter()
            .all(|event| event.phase == TaskPhase::Preparing)
    );
    assert_eq!(download_events[1].bytes_current, Some(1024));
    assert_eq!(download_events[1].bytes_total, Some(1024));
    assert_eq!(download_events[1].bytes_per_second, Some(512));
}
