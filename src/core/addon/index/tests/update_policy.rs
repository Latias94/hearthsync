use std::cell::Cell;
use std::fs;

use tempfile::tempdir;

use super::super::update_addons_from_index_task_with_provider;
use super::{
    addon_state_paths, create_addon_archive, create_fixture_installation, write_index_package,
};
use crate::core::addon::index::AddonIndexUpdateRequest;
use crate::core::addon::{
    AddonProvider, AddonSearchRequest as ProviderAddonSearchRequest, AddonSearchResult,
    AddonSourceRef, InstallAddonRequest, MaterializeSourceInputRequest,
    MaterializeSourceRefRequest, MaterializedAddonSource, install_addon_task_with_provider,
    policy::{SetAddonPolicyRequest, set_addon_policy},
};
use crate::core::error::{AppError, AppResult};
use crate::core::task::{NeverCancel, NoopProgressSink};

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
