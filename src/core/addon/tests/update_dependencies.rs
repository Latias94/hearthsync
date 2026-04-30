use std::cell::RefCell;
use std::fs;

use tempfile::tempdir;

use super::{addon_state_paths, create_addon_archive, create_fixture_installation};
use crate::core::addon::policy::{SetAddonPolicyRequest, set_addon_policy};
use crate::core::addon::provider::{
    AddonDependencyResolutionCapability, AddonProvider,
    AddonSearchRequest as ProviderAddonSearchRequest, AddonSearchResult, AddonSourceRef,
    MaterializeSourceInputRequest, MaterializeSourceRefRequest, MaterializedAddonSource,
    ResolveAddonDependenciesRequest, ResolvedAddonDependencies,
};
use crate::core::addon::{
    InstallAddonRequest, UpdateAddonRequest, install_addon_task_with_provider, list_addons,
    update_addons_task_with_provider,
};
use crate::core::error::{AppError, AppResult};
use crate::core::task::{NeverCancel, NoopProgressSink};

#[test]
fn update_addons_installs_missing_required_dependencies_when_policy_enabled() {
    #[derive(Default)]
    struct FakeProvider {
        dependency_requests: RefCell<Vec<AddonSourceRef>>,
    }

    impl AddonProvider for FakeProvider {
        fn materialize_source_input(
            &self,
            request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("curse-addon.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "WeakAuras/WeakAuras.toc",
                    "## Interface: 110000\n## Version: 1.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: AddonSourceRef::CurseForgeMod {
                    mod_id: 42,
                    file_id: None,
                },
                archive_path,
            })
        }

        fn materialize_source_ref(
            &self,
            request: MaterializeSourceRefRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = match request.source {
                AddonSourceRef::CurseForgeMod { mod_id: 42, .. } => {
                    request.stage_root.join("curse-addon-update.zip")
                }
                AddonSourceRef::CurseForgeMod { mod_id: 99, .. } => {
                    request.stage_root.join("sharedmedia-addon.zip")
                }
                source => {
                    return Err(AppError::Validation(format!(
                        "unexpected source during dependency install test: {}",
                        source.display_name()
                    )));
                }
            };

            let entries = match request.source {
                AddonSourceRef::CurseForgeMod { mod_id: 42, .. } => vec![(
                    "WeakAuras/WeakAuras.toc",
                    "## Interface: 120000\n## Version: 2.0.0\n",
                )],
                AddonSourceRef::CurseForgeMod { mod_id: 99, .. } => vec![(
                    "SharedMedia/SharedMedia.toc",
                    "## Interface: 120000\n## Version: 1.0.0\n",
                )],
                _ => unreachable!(),
            };
            create_addon_archive(&archive_path, &entries);

            Ok(MaterializedAddonSource {
                source_ref: request.source.clone(),
                archive_path,
            })
        }

        fn dependency_resolution_capability(
            &self,
            source: &AddonSourceRef,
        ) -> AddonDependencyResolutionCapability {
            match source {
                AddonSourceRef::CurseForgeMod { .. } => {
                    AddonDependencyResolutionCapability::missing_required_only()
                }
                _ => AddonDependencyResolutionCapability::Unsupported,
            }
        }

        fn resolve_addon_dependencies(
            &self,
            request: ResolveAddonDependenciesRequest<'_>,
        ) -> AppResult<ResolvedAddonDependencies> {
            self.dependency_requests
                .borrow_mut()
                .push(request.source.clone());
            match request.source {
                AddonSourceRef::CurseForgeMod { mod_id: 42, .. } => {
                    Ok(ResolvedAddonDependencies::missing_required_only(vec![
                        AddonSourceRef::CurseForgeMod {
                            mod_id: 99,
                            file_id: None,
                        },
                    ]))
                }
                AddonSourceRef::CurseForgeMod { mod_id: 99, .. } => {
                    Ok(ResolvedAddonDependencies::missing_required_only(Vec::new()))
                }
                source => Err(AppError::Validation(format!(
                    "unexpected source during dependency resolution test: {}",
                    source.display_name()
                ))),
            }
        }

        fn search_addons(
            &self,
            _request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            Ok(Vec::new())
        }
    }

    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = FakeProvider::default();
    let mut progress = NoopProgressSink;

    install_addon_task_with_provider(
        &provider,
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: "curseforge:42".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("install provider-backed addon");

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "curseforge-42".to_string(),
        ignored: None,
        pinned_version: None,
        pinned_file_id: None,
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: Some(true),
    })
    .expect("enable dependency installation");

    let result = update_addons_task_with_provider(
        &provider,
        UpdateAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            name: None,
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("update addon with dependency installation");

    assert_eq!(result.updated_packages.len(), 1);
    assert_eq!(result.installed_dependency_packages.len(), 1);
    assert_eq!(
        result.installed_dependency_packages[0].package_id,
        "curseforge-99"
    );
    assert_eq!(
        provider.dependency_requests.borrow().as_slice(),
        &[
            AddonSourceRef::CurseForgeMod {
                mod_id: 42,
                file_id: None,
            },
            AddonSourceRef::CurseForgeMod {
                mod_id: 99,
                file_id: None,
            },
        ]
    );

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    assert_eq!(inventory.tracked_packages.len(), 2);
    assert!(
        inventory
            .tracked_packages
            .iter()
            .any(|package| package.package_id == "curseforge-42")
    );
    assert!(
        inventory
            .tracked_packages
            .iter()
            .any(|package| package.package_id == "curseforge-99")
    );
}

#[test]
fn update_addons_rolls_back_when_dependency_install_fails_after_primary_update() {
    #[derive(Default)]
    struct FakeProvider {
        dependency_requests: RefCell<Vec<AddonSourceRef>>,
    }

    impl AddonProvider for FakeProvider {
        fn materialize_source_input(
            &self,
            request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("curse-addon.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "WeakAuras/WeakAuras.toc",
                    "## Interface: 110000\n## Version: 1.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: AddonSourceRef::CurseForgeMod {
                    mod_id: 42,
                    file_id: None,
                },
                archive_path,
            })
        }

        fn materialize_source_ref(
            &self,
            request: MaterializeSourceRefRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = match request.source {
                AddonSourceRef::CurseForgeMod { mod_id: 42, .. } => {
                    request.stage_root.join("curse-addon-update.zip")
                }
                AddonSourceRef::CurseForgeMod { mod_id: 99, .. } => {
                    request.stage_root.join("sharedmedia-addon.zip")
                }
                source => {
                    return Err(AppError::Validation(format!(
                        "unexpected source during dependency rollback test: {}",
                        source.display_name()
                    )));
                }
            };

            let entries = match request.source {
                AddonSourceRef::CurseForgeMod { mod_id: 42, .. } => vec![(
                    "WeakAuras/WeakAuras.toc",
                    "## Interface: 120000\n## Version: 2.0.0\n",
                )],
                AddonSourceRef::CurseForgeMod { mod_id: 99, .. } => vec![(
                    "SharedMedia/SharedMedia.toc",
                    "## Interface: 120000\n## Version: 1.0.0\n",
                )],
                _ => unreachable!(),
            };
            create_addon_archive(&archive_path, &entries);

            Ok(MaterializedAddonSource {
                source_ref: request.source.clone(),
                archive_path,
            })
        }

        fn dependency_resolution_capability(
            &self,
            source: &AddonSourceRef,
        ) -> AddonDependencyResolutionCapability {
            match source {
                AddonSourceRef::CurseForgeMod { .. } => {
                    AddonDependencyResolutionCapability::missing_required_only()
                }
                _ => AddonDependencyResolutionCapability::Unsupported,
            }
        }

        fn resolve_addon_dependencies(
            &self,
            request: ResolveAddonDependenciesRequest<'_>,
        ) -> AppResult<ResolvedAddonDependencies> {
            self.dependency_requests
                .borrow_mut()
                .push(request.source.clone());
            match request.source {
                AddonSourceRef::CurseForgeMod { mod_id: 42, .. } => {
                    Ok(ResolvedAddonDependencies::missing_required_only(vec![
                        AddonSourceRef::CurseForgeMod {
                            mod_id: 99,
                            file_id: None,
                        },
                    ]))
                }
                AddonSourceRef::CurseForgeMod { mod_id: 99, .. } => {
                    Ok(ResolvedAddonDependencies::missing_required_only(Vec::new()))
                }
                source => Err(AppError::Validation(format!(
                    "unexpected source during dependency rollback resolution test: {}",
                    source.display_name()
                ))),
            }
        }

        fn search_addons(
            &self,
            _request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            Ok(Vec::new())
        }
    }

    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = FakeProvider::default();
    let mut progress = NoopProgressSink;

    install_addon_task_with_provider(
        &provider,
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: "curseforge:42".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("install provider-backed addon");

    let local_dependency_dir = installation.addon_dir.join("SharedMedia");
    fs::create_dir_all(&local_dependency_dir).expect("create local dependency conflict");
    fs::write(
        local_dependency_dir.join("SharedMedia.toc"),
        "## Interface: 110000\n## Version: local\n",
    )
    .expect("write local dependency conflict");

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "curseforge-42".to_string(),
        ignored: None,
        pinned_version: None,
        pinned_file_id: None,
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: Some(true),
    })
    .expect("enable dependency installation");

    let error = update_addons_task_with_provider(
        &provider,
        UpdateAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            name: None,
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect_err("dependency install conflict should roll back");

    let message = error.to_string();
    assert!(message.contains("rollback restored"));
    assert!(message.contains("addon directory already exists"));
    assert!(
        fs::read_to_string(
            installation
                .addon_dir
                .join("WeakAuras")
                .join("WeakAuras.toc")
        )
        .expect("weakauras toc after rollback")
        .contains("1.0.0")
    );
    assert!(
        fs::read_to_string(local_dependency_dir.join("SharedMedia.toc"))
            .expect("local dependency after rollback")
            .contains("local")
    );

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    assert_eq!(inventory.tracked_packages.len(), 1);
    assert_eq!(inventory.tracked_packages[0].package_id, "curseforge-42");
}

#[test]
fn update_addons_rejects_dependency_installation_for_unsupported_sources() {
    struct FakeProvider;

    impl AddonProvider for FakeProvider {
        fn materialize_source_input(
            &self,
            request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("github-addon.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "Plater/Plater.toc",
                    "## Interface: 110000\n## Version: 1.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: AddonSourceRef::GitHubRelease {
                    owner: "owner".to_string(),
                    repo: "repo".to_string(),
                    tag: None,
                    asset_name: Some("plater.zip".to_string()),
                },
                archive_path,
            })
        }

        fn materialize_source_ref(
            &self,
            request: MaterializeSourceRefRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("github-addon-update.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "Plater/Plater.toc",
                    "## Interface: 120000\n## Version: 2.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: request.source.clone(),
                archive_path,
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
    let installation = create_fixture_installation(temp.path());
    let provider = FakeProvider;
    let mut progress = NoopProgressSink;

    install_addon_task_with_provider(
        &provider,
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: "github:owner/repo#plater.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("install provider-backed addon");

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "plater".to_string(),
        ignored: None,
        pinned_version: None,
        pinned_file_id: None,
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: Some(true),
    })
    .expect("enable dependency installation");

    let error = update_addons_task_with_provider(
        &provider,
        UpdateAddonRequest {
            state_paths: addon_state_paths(&installation),
            installation,
            name: None,
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect_err("unsupported dependency installation should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("not supported"));
}
