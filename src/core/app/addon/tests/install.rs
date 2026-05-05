use super::*;

#[test]
fn addon_service_install_and_list_roundtrip_local_archive() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("WeakAuras.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "WeakAuras/WeakAuras.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    let service = AddonService::new();
    let installed = service
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install addon");
    let inventory = service
        .list(ListAddonsRequest { installation })
        .expect("list addons");

    assert_eq!(installed.package_id, "weakauras");
    assert_eq!(inventory.tracked_packages.len(), 1);
    assert!(inventory.untracked_addons.is_empty());
}

#[test]
fn addon_service_install_projects_wago_source_result() {
    #[derive(Clone)]
    struct FakeWagoInstallProvider {
        archive_path: PathBuf,
    }

    impl AddonProvider for FakeWagoInstallProvider {
        fn materialize_source_input(
            &self,
            request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            assert_eq!(request.source, "wago:qv63A7Gb");
            assert_eq!(request.context.target_flavor, Some(WowFlavor::Retail));
            Ok(MaterializedAddonSource {
                source_ref: AddonSourceRef::WagoAddon {
                    project_id: "qv63A7Gb".to_string(),
                    release_id: None,
                },
                archive_path: self.archive_path.clone(),
            })
        }

        fn materialize_source_ref(
            &self,
            request: MaterializeSourceRefRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            Ok(MaterializedAddonSource {
                source_ref: request.source.clone(),
                archive_path: self.archive_path.clone(),
            })
        }

        fn search_addons(
            &self,
            _request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            Ok(Vec::new())
        }
    }

    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("Details.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    let service =
        AddonService::with_runtime(AppRuntime::with_addon_provider(FakeWagoInstallProvider {
            archive_path,
        }));

    let installed = service
        .install(InstallAddonAppRequest {
            installation,
            source: "wago:qv63A7Gb".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install wago addon");

    assert_eq!(installed.package_id, "wago-qv63a7gb");
    assert_eq!(
        installed.source.kind,
        crate::core::app::AddonSourceKindResult::WagoAddon
    );
    assert_eq!(installed.source.project_id.as_deref(), Some("qv63A7Gb"));
    assert_eq!(installed.source.release_id, None);
}

#[test]
fn addon_service_install_resolves_relative_local_archive_against_runtime_base() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let source_dir = temp.path().join("sources");
    fs::create_dir_all(&source_dir).expect("source dir");
    let archive_path = source_dir.join("WeakAuras.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "WeakAuras/WeakAuras.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    let service = AddonService::with_runtime(
        AppRuntime::builder()
            .with_relative_path_base(Some(source_dir.clone()))
            .build()
            .expect("runtime"),
    );
    let installed = service
        .install(InstallAddonAppRequest {
            installation,
            source: "WeakAuras.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install relative addon archive");

    assert_eq!(
        installed.source.local_archive_path,
        Some(
            crate::core::addon::canonicalize_local_archive_path(&archive_path)
                .expect("canonical source path")
        )
    );
}

#[test]
fn addon_service_install_rejects_relative_local_archive_without_runtime_base() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("WeakAuras.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "WeakAuras/WeakAuras.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    let service = AddonService::new();
    let error = service
        .install(InstallAddonAppRequest {
            installation,
            source: "WeakAuras.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect_err("relative archive without runtime base should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("relative path base"));
}

#[test]
fn addon_service_install_rejects_relative_runtime_base_for_relative_archives() {
    let error = AppRuntime::builder()
        .with_relative_path_base(Some(PathBuf::from("sources")))
        .build()
        .expect_err("relative runtime base should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("base must be absolute"));
}
