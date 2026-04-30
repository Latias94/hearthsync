use std::cell::RefCell;
use std::fs;

use tempfile::tempdir;

use super::super::update_addons_from_index_task_with_provider;
use super::{
    addon_state_paths, create_addon_archive, create_fixture_installation, write_index_package,
};
use crate::core::addon::index::AddonIndexUpdateRequest;
use crate::core::addon::provider::ResolveAddonDependenciesRequest;
use crate::core::addon::{
    AddonDependencyResolutionCapability, AddonProvider,
    AddonSearchRequest as ProviderAddonSearchRequest, AddonSearchResult, AddonSourceRef,
    InstallAddonRequest, MaterializeSourceInputRequest, MaterializeSourceRefRequest,
    MaterializedAddonSource, ResolvedAddonDependencies, install_addon_task_with_provider,
    list_addons,
    policy::{SetAddonPolicyRequest, set_addon_policy},
};
use crate::core::error::{AppError, AppResult};
use crate::core::task::{NeverCancel, NoopProgressSink};

#[derive(Default)]
struct DependencyProvider {
    dependency_requests: RefCell<Vec<AddonSourceRef>>,
}

impl AddonProvider for DependencyProvider {
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

#[test]
fn update_addons_from_index_installs_missing_required_dependencies_when_policy_enabled() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = DependencyProvider::default();
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
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = DependencyProvider::default();
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
