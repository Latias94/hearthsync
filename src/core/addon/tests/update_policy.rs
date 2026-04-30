use std::cell::RefCell;

use tempfile::tempdir;

use super::{addon_state_paths, create_addon_archive, create_fixture_installation};
use crate::core::addon::policy::{AddonReleaseChannel, SetAddonPolicyRequest, set_addon_policy};
use crate::core::addon::provider::{
    AddonProvider, AddonSearchRequest as ProviderAddonSearchRequest, AddonSearchResult,
    AddonSourceRef, AddonSourceResolutionPolicy, MaterializeSourceInputRequest,
    MaterializeSourceRefRequest, MaterializedAddonSource,
};
use crate::core::addon::{
    InstallAddonRequest, UpdateAddonRequest, install_addon_task_with_provider, list_addons,
    update_addons_task_with_provider,
};
use crate::core::error::AppResult;
use crate::core::task::{NeverCancel, NoopProgressSink};

#[test]
fn update_addons_applies_curseforge_file_pin_policy() {
    #[derive(Default)]
    struct FakeProvider {
        update_sources: RefCell<Vec<AddonSourceRef>>,
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
            self.update_sources
                .borrow_mut()
                .push(request.source.clone());
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
        pinned_file_id: Some(777),
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: None,
    })
    .expect("set file pin");

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
    .expect("update addon with file pin");

    assert_eq!(result.updated_packages.len(), 1);
    assert_eq!(
        provider.update_sources.borrow().as_slice(),
        &[AddonSourceRef::CurseForgeMod {
            mod_id: 42,
            file_id: Some(777),
        }]
    );
    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    assert_eq!(
        inventory.tracked_packages[0].source,
        AddonSourceRef::CurseForgeMod {
            mod_id: 42,
            file_id: Some(777),
        }
    );
    assert_eq!(inventory.tracked_packages[0].package_id, "curseforge-42");
}

#[test]
fn update_addons_applies_github_tag_pin_policy() {
    #[derive(Default)]
    struct FakeProvider {
        update_sources: RefCell<Vec<AddonSourceRef>>,
    }

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
            self.update_sources
                .borrow_mut()
                .push(request.source.clone());
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
    let provider = FakeProvider::default();
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
        pinned_version: Some("v2.5.0".to_string()),
        pinned_file_id: None,
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: None,
    })
    .expect("set tag pin");

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
    .expect("update addon with tag pin");

    assert_eq!(result.updated_packages.len(), 1);
    assert_eq!(
        provider.update_sources.borrow().as_slice(),
        &[AddonSourceRef::GitHubRelease {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            tag: Some("v2.5.0".to_string()),
            asset_name: Some("plater.zip".to_string()),
        }]
    );
    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    assert_eq!(
        inventory.tracked_packages[0].source,
        AddonSourceRef::GitHubRelease {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            tag: Some("v2.5.0".to_string()),
            asset_name: Some("plater.zip".to_string()),
        }
    );
    assert_eq!(inventory.tracked_packages[0].package_id, "plater");
}

#[test]
fn update_addons_forwards_resolution_policy_into_provider_context() {
    #[derive(Default)]
    struct FakeProvider {
        update_requests: RefCell<Vec<(AddonSourceRef, AddonSourceResolutionPolicy)>>,
    }

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
            self.update_requests
                .borrow_mut()
                .push((request.source.clone(), request.context.resolution_policy()));
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
    let provider = FakeProvider::default();
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
        release_channel: Some(AddonReleaseChannel::Beta),
        allow_prerelease: Some(true),
        install_dependencies: None,
    })
    .expect("set resolution policy");

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
    .expect("update addon with resolution policy");

    assert_eq!(result.updated_packages.len(), 1);
    assert_eq!(
        provider.update_requests.borrow().as_slice(),
        &[(
            AddonSourceRef::GitHubRelease {
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                tag: None,
                asset_name: Some("plater.zip".to_string()),
            },
            AddonSourceResolutionPolicy {
                release_channel: Some(AddonReleaseChannel::Beta),
                allow_prerelease: Some(true),
            },
        )]
    );
}
