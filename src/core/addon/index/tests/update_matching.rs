use std::cell::RefCell;
use std::fs;

use tempfile::tempdir;

use super::super::update_addons_from_index_task_with_provider;
use super::{
    addon_state_paths, create_addon_archive, create_fixture_installation, write_index_package,
};
use crate::core::addon::index::AddonIndexUpdateRequest;
use crate::core::addon::{
    AddonProvider, AddonSearchRequest as ProviderAddonSearchRequest, AddonSearchResult,
    AddonSourceRef, AddonSourceResolutionPolicy, InstallAddonRequest,
    MaterializeSourceInputRequest, MaterializeSourceRefRequest, MaterializedAddonSource,
    install_addon_task_with_provider,
    policy::{AddonReleaseChannel, SetAddonPolicyRequest, set_addon_policy},
};
use crate::core::error::AppResult;
use crate::core::task::{NeverCancel, NoopProgressSink};

#[derive(Default)]
struct CuratedSourceProvider {
    update_requests: RefCell<Vec<(AddonSourceRef, AddonSourceResolutionPolicy)>>,
}

impl AddonProvider for CuratedSourceProvider {
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

struct GithubMatchingProvider {
    install_source: AddonSourceRef,
    install_toc: &'static str,
    update_toc: &'static str,
}

impl AddonProvider for GithubMatchingProvider {
    fn materialize_source_input(
        &self,
        request: MaterializeSourceInputRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        let archive_path = request.stage_root.join("github-addon.zip");
        create_addon_archive(&archive_path, &[("Plater/Plater.toc", self.install_toc)]);
        Ok(MaterializedAddonSource {
            source_ref: self.install_source.clone(),
            archive_path,
        })
    }

    fn materialize_source_ref(
        &self,
        request: MaterializeSourceRefRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        let archive_path = request.stage_root.join("github-addon-update.zip");
        create_addon_archive(&archive_path, &[("Plater/Plater.toc", self.update_toc)]);
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

fn github_release(owner: &str, repo: &str, asset_name: &str) -> AddonSourceRef {
    AddonSourceRef::GitHubRelease {
        owner: owner.to_string(),
        repo: repo.to_string(),
        tag: None,
        asset_name: Some(asset_name.to_string()),
    }
}

#[test]
fn update_addons_from_index_keeps_curated_source_authority_over_pin_and_release_policy() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = CuratedSourceProvider::default();
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
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = GithubMatchingProvider {
        install_source: github_release("owner", "repo", "plater.zip"),
        install_toc: "## Interface: 110000\n## Version: 1.0.0\n",
        update_toc: "## Interface: 120000\n## Version: 2.0.0\n",
    };
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
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = GithubMatchingProvider {
        install_source: github_release("legacy-owner", "legacy-repo", "plater.zip"),
        install_toc: "## Interface: 110000\n## Title: Plater\n## Version: 1.0.0\n",
        update_toc: "## Interface: 120000\n## Title: Plater\n## Version: 2.0.0\n",
    };
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
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = GithubMatchingProvider {
        install_source: github_release("legacy-owner", "legacy-repo", "plater.zip"),
        install_toc: "## Interface: 110000\n## Version: 1.0.0\n",
        update_toc: "## Interface: 120000\n## Version: 2.0.0\n",
    };
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
