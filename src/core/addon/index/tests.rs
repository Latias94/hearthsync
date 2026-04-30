use std::cell::{Cell, RefCell};
use std::fs;
use std::io::Write;
use std::path::Path;

use tempfile::tempdir;
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use super::update_addons_from_index_task_with_provider;
use crate::core::addon::index::AddonIndexUpdateRequest;
use crate::core::addon::provider::ResolveAddonDependenciesRequest;
use crate::core::addon::{
    AddonDependencyResolutionCapability, AddonProvider,
    AddonSearchRequest as ProviderAddonSearchRequest, AddonSearchResult, AddonSourceRef,
    AddonSourceResolutionPolicy, InstallAddonRequest, MaterializeSourceInputRequest,
    MaterializeSourceRefRequest, MaterializedAddonSource, ResolvedAddonDependencies,
    canonicalize_local_archive_path, install_addon_task_with_provider, list_addons,
    policy::{AddonReleaseChannel, SetAddonPolicyRequest, set_addon_policy},
};
use crate::core::error::{AppError, AppResult};
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};
use crate::core::task::{NeverCancel, NoopProgressSink, TaskKind, TaskPhase, TaskProgressEvent};

mod attach;
mod curation;
mod inspect;
mod install;
mod relink;
mod update_basic;

fn addon_state_paths(
    installation: &DetectedFlavorInstallation,
) -> crate::core::addon::AddonStatePaths {
    crate::core::addon::AddonStatePaths::for_installation(
        crate::core::addon::AddonStateStorageKind::default(),
        installation,
    )
    .expect("addon state paths")
}

#[test]
fn update_addons_from_index_skips_ignored_preflight_match_without_provider_prepare() {
    #[derive(Default)]
    struct FakeProvider {
        materialize_ref_calls: Cell<usize>,
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
            self.materialize_ref_calls
                .set(self.materialize_ref_calls.get() + 1);
            Err(AppError::Validation(format!(
                "ignored package should not be prepared: {}",
                request.source.display_name()
            )))
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
    let index_path = write_index_package(
        temp.path(),
        "weakauras",
        "WeakAuras",
        "2.0.0",
        r#"{ kind = "curseforge_mod", mod_id = 42 }"#,
    );
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
    .expect("install tracked curseforge addon");

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "curseforge-42".to_string(),
        ignored: Some(true),
        pinned_version: None,
        pinned_file_id: None,
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: None,
    })
    .expect("set ignored policy");

    let result = update_addons_from_index_task_with_provider(
        &provider,
        AddonIndexUpdateRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            index_path,
            name: None,
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("ignored index update should not prepare provider source");

    assert_eq!(provider.materialize_ref_calls.get(), 0);
    assert!(result.selected_packages.is_empty());
    assert_eq!(
        result.update.ignored_packages,
        vec!["curseforge-42".to_string()]
    );
    assert!(result.update.backup_path.is_none());
}

#[test]
fn update_addons_from_index_installs_missing_required_dependencies_when_policy_enabled() {
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
                        "unexpected source during addon-index dependency test: {}",
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
                    "unexpected source during addon-index dependency resolution test: {}",
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
    let index_path = write_index_package(
        temp.path(),
        "weakauras",
        "WeakAuras",
        "2.0.0",
        r#"{ kind = "curseforge_mod", mod_id = 42 }"#,
    );
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
    .expect("install tracked curseforge addon");

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

    let result = update_addons_from_index_task_with_provider(
        &provider,
        AddonIndexUpdateRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            index_path,
            name: None,
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("update from index with dependency installation");

    assert_eq!(result.selected_packages.len(), 1);
    assert_eq!(result.update.updated_packages.len(), 1);
    assert_eq!(result.update.installed_dependency_packages.len(), 1);
    assert_eq!(
        result.update.installed_dependency_packages[0].package_id,
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
fn update_addons_from_index_rolls_back_when_dependency_install_fails_after_primary_update() {
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
                        "unexpected source during addon-index dependency rollback test: {}",
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
                    "unexpected source during addon-index dependency rollback resolution test: {}",
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
    let index_path = write_index_package(
        temp.path(),
        "weakauras",
        "WeakAuras",
        "2.0.0",
        r#"{ kind = "curseforge_mod", mod_id = 42 }"#,
    );
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
    .expect("install tracked curseforge addon");

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

    let error = update_addons_from_index_task_with_provider(
        &provider,
        AddonIndexUpdateRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            index_path,
            name: None,
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
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
fn update_addons_from_index_keeps_curated_source_authority_over_pin_and_release_policy() {
    #[derive(Default)]
    struct FakeProvider {
        update_requests: RefCell<Vec<(AddonSourceRef, AddonSourceResolutionPolicy)>>,
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
            self.update_requests
                .borrow_mut()
                .push((request.source.clone(), request.context.resolution_policy()));
            let archive_path = request.stage_root.join("curse-addon-update.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "WeakAuras/WeakAuras.toc",
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
    let provider = FakeProvider::default();
    let index_path = write_index_package(
        temp.path(),
        "weakauras",
        "WeakAuras",
        "2.0.0",
        r#"{ kind = "curseforge_mod", mod_id = 42 }"#,
    );
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
    .expect("install tracked curseforge addon");

    provider.update_requests.borrow_mut().clear();

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "curseforge-42".to_string(),
        ignored: None,
        pinned_version: None,
        pinned_file_id: Some(777),
        release_channel: Some(AddonReleaseChannel::Alpha),
        allow_prerelease: Some(true),
        install_dependencies: None,
    })
    .expect("set source override policy");

    let result = update_addons_from_index_task_with_provider(
        &provider,
        AddonIndexUpdateRequest {
            state_paths: addon_state_paths(&installation),
            installation,
            index_path,
            name: None,
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("update from index with curated source authority");

    assert_eq!(result.selected_packages.len(), 1);
    assert_eq!(result.update.updated_packages.len(), 1);
    assert_eq!(
        result.update.updated_packages[0].source,
        AddonSourceRef::CurseForgeMod {
            mod_id: 42,
            file_id: None,
        }
    );
    assert_eq!(
        provider.update_requests.borrow().as_slice(),
        &[(
            AddonSourceRef::CurseForgeMod {
                mod_id: 42,
                file_id: None,
            },
            AddonSourceResolutionPolicy::default(),
        )]
    );
}

#[test]
fn update_addons_from_index_matches_tracked_package_by_source_family_identity_when_github_asset_changes()
 {
    #[derive(Default)]
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
    let index_path = write_index_package(
        temp.path(),
        "curated-plater",
        "Curated Plater",
        "2.0.0",
        r#"{ kind = "github_release", owner = "owner", repo = "repo", asset_name = "release.zip" }"#,
    );
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
    .expect("install tracked github addon");

    let result = update_addons_from_index_task_with_provider(
        &provider,
        AddonIndexUpdateRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            index_path,
            name: Some("curated-plater".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("update from index by source family identity");

    assert_eq!(result.selected_packages.len(), 1);
    assert_eq!(result.selected_packages[0].id, "curated-plater");
    assert_eq!(result.update.updated_packages.len(), 1);
    assert_eq!(result.update.updated_packages[0].package_id, "plater");
    assert!(
        fs::read_to_string(installation.addon_dir.join("Plater").join("Plater.toc"))
            .expect("toc")
            .contains("2.0.0")
    );
}

#[test]
fn update_addons_from_index_matches_tracked_package_by_display_name_when_source_family_changes() {
    #[derive(Default)]
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
                    "## Interface: 110000\n## Title: Plater\n## Version: 1.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: AddonSourceRef::GitHubRelease {
                    owner: "legacy-owner".to_string(),
                    repo: "legacy-repo".to_string(),
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
                    "## Interface: 120000\n## Title: Plater\n## Version: 2.0.0\n",
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
    let index_path = write_index_package(
        temp.path(),
        "curated-plater",
        "Plater",
        "2.0.0",
        r#"{ kind = "github_release", owner = "new-owner", repo = "new-repo", asset_name = "release.zip" }"#,
    );
    let mut progress = NoopProgressSink;

    install_addon_task_with_provider(
        &provider,
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: "github:legacy-owner/legacy-repo#plater.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("install tracked github addon");

    let result = update_addons_from_index_task_with_provider(
        &provider,
        AddonIndexUpdateRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            index_path,
            name: Some("curated-plater".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("update from index by display name continuity");

    assert_eq!(result.selected_packages.len(), 1);
    assert_eq!(result.selected_packages[0].id, "curated-plater");
    assert_eq!(result.update.updated_packages.len(), 1);
    assert_eq!(result.update.updated_packages[0].package_id, "plater");
    assert!(
        fs::read_to_string(installation.addon_dir.join("Plater").join("Plater.toc"))
            .expect("toc")
            .contains("2.0.0")
    );
}

#[test]
fn update_addons_from_index_matches_tracked_package_by_curated_package_hint_when_source_family_changes()
 {
    #[derive(Default)]
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
                    owner: "legacy-owner".to_string(),
                    repo: "legacy-repo".to_string(),
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
    let index_path = temp.path().join("index.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-plater-v2"
name = "Curated Plater Package"
version = "2.0.0"
match_package_ids = ["plater"]
source = { kind = "github_release", owner = "new-owner", repo = "new-repo", asset_name = "release.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");
    let mut progress = NoopProgressSink;

    install_addon_task_with_provider(
        &provider,
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: "github:legacy-owner/legacy-repo#plater.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("install tracked github addon");

    let result = update_addons_from_index_task_with_provider(
        &provider,
        AddonIndexUpdateRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            index_path,
            name: Some("curated-plater-v2".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("update from index by curated package hint");

    assert_eq!(result.selected_packages.len(), 1);
    assert_eq!(result.selected_packages[0].id, "curated-plater-v2");
    assert_eq!(
        result.selected_packages[0].match_package_ids,
        vec!["plater".to_string()]
    );
    assert_eq!(result.update.updated_packages.len(), 1);
    assert_eq!(result.update.updated_packages[0].package_id, "plater");
    assert!(
        fs::read_to_string(installation.addon_dir.join("Plater").join("Plater.toc"))
            .expect("toc")
            .contains("2.0.0")
    );
}

#[test]
fn update_addons_from_index_rejects_dependency_installation_for_unsupported_sources() {
    #[derive(Default)]
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
    let index_path = write_index_package(
        temp.path(),
        "curated-plater",
        "Curated Plater",
        "2.0.0",
        r#"{ kind = "github_release", owner = "owner", repo = "repo", asset_name = "release.zip" }"#,
    );
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
    .expect("install tracked github addon");

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

    let error = update_addons_from_index_task_with_provider(
        &provider,
        AddonIndexUpdateRequest {
            state_paths: addon_state_paths(&installation),
            installation,
            index_path,
            name: Some("curated-plater".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect_err("unsupported dependency installation should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("not supported"));
}

#[test]
fn update_addons_from_index_explains_deferred_dependency_policy_failure_when_preflight_cannot_match()
 {
    #[derive(Default)]
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
                    owner: "legacy-owner".to_string(),
                    repo: "legacy-repo".to_string(),
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
    let index_path = temp.path().join("index.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-plater-v3"
name = "Curated Plater Package"
version = "2.0.0"
source = { kind = "github_release", owner = "new-owner", repo = "new-repo", asset_name = "release.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");
    let mut progress = NoopProgressSink;

    install_addon_task_with_provider(
        &provider,
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: "github:legacy-owner/legacy-repo#plater.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("install tracked github addon");

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

    let error = update_addons_from_index_task_with_provider(
        &provider,
        AddonIndexUpdateRequest {
            state_paths: addon_state_paths(&installation),
            installation,
            index_path,
            name: Some("curated-plater-v3".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect_err("deferred dependency-policy validation should fail with guidance");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("not supported"));
    assert!(
        error
            .to_string()
            .contains("app preflight could not determine")
    );
    assert!(error.to_string().contains("match_package_ids"));
    assert!(error.to_string().contains("addon_directories"));
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
    write_index_package(
        root,
        "details",
        "Details",
        "1.0.0",
        &format!(
            r#"{{ kind = "local_archive", path = "{}" }}"#,
            archive_path.display().to_string().replace('\\', "\\\\")
        ),
    )
}

fn write_index_package(
    root: &Path,
    package_id: &str,
    package_name: &str,
    version: &str,
    source_toml: &str,
) -> std::path::PathBuf {
    let index_path = root.join("index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "{package_id}"
name = "{package_name}"
version = "{version}"
source = {source_toml}
supported_flavors = ["retail"]
"#
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

fn normalized_archive_path(path: &Path) -> std::path::PathBuf {
    canonicalize_local_archive_path(path).expect("normalized archive path")
}

fn assert_addon_index_task_progress(
    events: &[TaskProgressEvent],
    task: TaskKind,
    executing_detail: &str,
) {
    let phases = events
        .iter()
        .map(|event| (event.task, event.phase))
        .collect::<Vec<_>>();

    assert_eq!(phases.first(), Some(&(task, TaskPhase::Preparing)));
    assert_eq!(phases.last(), Some(&(task, TaskPhase::Completed)));
    assert!(phases.contains(&(task, TaskPhase::BackingUp)));
    assert!(phases.contains(&(task, TaskPhase::Executing)));
    assert!(events.iter().any(|event| {
        event.task == task
            && event.phase == TaskPhase::Executing
            && event.message.contains(executing_detail)
    }));
}
