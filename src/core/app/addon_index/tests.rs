use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::core::addon::{
    AddonProvider, AddonSearchRequest as ProviderAddonSearchRequest, AddonSearchResult,
    AddonSourceRef, InstallAddonRequest, MaterializeSourceInputRequest,
    MaterializeSourceRefRequest, MaterializedAddonSource, install_addon,
};
use crate::core::app::{
    AddonDependencyResolutionCapabilityValue, AddonIndexAttachPackageStatusResult,
    AddonIndexInspectionWarningCodeResult, AddonIndexInspectionWarningSeverityResult,
    AddonIndexPackageSuggestionStatusResult, AddonIndexScaffoldResult, AddonIndexService,
    AddonIndexTrackedMatchStrategyResult, AddonIndexUpdateResult, AddonPolicyService, AddonService,
    AppRuntime, AttachAddonIndexAppRequest, InspectAddonIndexRequest, InstallAddonAppRequest,
    InstallAddonIndexAppRequest, RelinkAddonIndexAppRequest, ResolvedInstallationValue,
    ScaffoldAddonIndexRequest, SetAddonPolicyAppRequest, SuggestAddonIndexRequest,
    UpdateAddonIndexAppRequest,
};
use crate::core::error::{AppError, AppResult};
use crate::core::install::{HostPlatform, WowFlavor};
use crate::core::task::{TaskKind, TaskPhase, TaskProgressCode, TaskProgressEvent};

fn addon_state_paths(
    installation: &crate::core::install::DetectedFlavorInstallation,
) -> crate::core::addon::AddonStatePaths {
    crate::core::addon::AddonStatePaths::for_installation(
        crate::core::addon::AddonStateStorageKind::default(),
        installation,
    )
    .expect("addon state paths")
}

#[test]
fn addon_index_service_inspects_index_file() {
    let temp = tempdir().expect("temp dir");
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
match_package_ids = ["legacy-weakauras"]
source = { kind = "local_archive", path = "WeakAuras.zip" }
"#,
    )
    .expect("write index");

    let service = AddonIndexService::new();
    let inspection = service
        .inspect(InspectAddonIndexRequest { index_path })
        .expect("inspect addon index");

    assert_eq!(inspection.package_count, 1);
    assert_eq!(inspection.name, "Fixture Index");
    assert_eq!(inspection.packages[0].id, "weakauras");
    assert_eq!(
        inspection
            .identity_hint_coverage
            .package_count_with_both_exact_hints,
        0
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .package_count_with_any_exact_hints,
        1
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .package_count_with_match_package_ids,
        1
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .package_count_with_addon_directories,
        0
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .package_count_without_match_package_ids,
        0
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .package_count_without_addon_directories,
        1
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .package_count_without_exact_hints,
        0
    );
    assert_eq!(inspection.warning_count, 1);
    assert_eq!(inspection.blocking_warning_count, 0);
    assert_eq!(inspection.advisory_warning_count, 1);
    assert_eq!(inspection.warnings.len(), 1);
    assert!(matches!(
        inspection.warnings[0].code,
        AddonIndexInspectionWarningCodeResult::MissingAddonDirectories
    ));
    assert!(matches!(
        inspection.warnings[0].severity,
        AddonIndexInspectionWarningSeverityResult::Advisory
    ));
    assert_eq!(
        inspection.packages[0].match_package_ids,
        vec!["legacy-weakauras".to_string()]
    );
    assert_eq!(
        inspection.packages[0]
            .source
            .dependency_resolution_capability,
        AddonDependencyResolutionCapabilityValue::Unsupported
    );
}

#[test]
fn addon_index_service_install_collecting_progress_returns_index_task_events() {
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
    let index_path = write_index(temp.path(), &archive_path);

    let service = AddonIndexService::new();
    let run = service
        .install_collecting_progress(InstallAddonIndexAppRequest {
            installation,
            index_path,
            name: "weakauras".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
        })
        .expect("install from index with collected progress");

    assert_eq!(run.result.package.id, "weakauras");
    assert_addon_index_task_progress(
        &run.progress,
        TaskKind::AddonIndexInstall,
        "Installing addon directory",
    );
}

#[test]
fn addon_index_service_relink_attaches_curated_metadata_without_reinstalling_files() {
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
    let index_path = temp.path().join("addon-index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-details"
name = "Curated Details"
version = "2.0.0"
source = {{ kind = "local_archive", path = "{}" }}
source_url = "https://example.invalid/details.zip"
supported_flavors = ["retail"]
addon_directories = ["Details"]
"#,
            archive_path.display().to_string().replace('\\', "\\\\")
        ),
    )
    .expect("write index");

    AddonService::new()
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install details");

    let service = AddonIndexService::new();
    let relinked = service
        .relink(RelinkAddonIndexAppRequest {
            installation: installation.clone(),
            index_path,
            name: "curated-details".to_string(),
            target: Some("details".to_string()),
            dry_run: false,
        })
        .expect("relink addon index");
    let inventory = AddonService::new()
        .list(crate::core::app::ListAddonsRequest {
            installation: installation.clone(),
        })
        .expect("list addons");

    assert_eq!(relinked.tracked_package_id, "details");
    assert!(!relinked.source_changed);
    assert!(relinked.metadata_changed);
    assert_eq!(
        relinked.metadata.index_package_id.as_deref(),
        Some("curated-details")
    );
    assert_eq!(
        inventory.tracked_packages[0]
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.index_package_id.as_deref()),
        Some("curated-details")
    );
    assert!(
        fs::read_to_string(installation.addon_dir.join("Details").join("Details.toc"))
            .expect("details toc")
            .contains("1.0.0")
    );
}

#[test]
fn addon_index_service_attach_blocks_without_partial_registry_writes() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("Plater.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Title: Plater\n## Version: 1.0.0\n",
        )],
    );

    AddonService::new()
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install tracked addon");

    let index_path = temp.path().join("addon-index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-plater"
name = "Curated Plater"
version = "2.0.0"
source = {{ kind = "local_archive", path = "{}" }}
supported_flavors = ["retail"]

[[packages]]
id = "unknown-addon"
name = "Unknown Addon"
version = "1.0.0"
source = {{ kind = "http_archive", url = "https://example.invalid/unknown.zip" }}
supported_flavors = ["retail"]
"#,
            archive_path.display().to_string().replace('\\', "\\\\")
        ),
    )
    .expect("write index");

    let service = AddonIndexService::new();
    let result = service
        .attach(AttachAddonIndexAppRequest {
            installation: installation.clone(),
            index_path,
            name: None,
            dry_run: false,
        })
        .expect("attach addon index");

    assert!(!result.ready);
    assert!(!result.applied);
    assert_eq!(result.blocked_package_count, 1);
    assert_eq!(result.change_package_count, 1);
    assert!(matches!(
        result.packages[0].status,
        AddonIndexAttachPackageStatusResult::WouldAttach
    ));
    assert!(matches!(
        result.packages[1].status,
        AddonIndexAttachPackageStatusResult::NoLocalMatch
    ));

    let inventory = AddonService::new()
        .list(crate::core::app::ListAddonsRequest { installation })
        .expect("list addons");
    assert!(inventory.tracked_packages[0].metadata.is_none());
}

#[test]
fn addon_index_service_attach_collecting_progress_returns_attach_task_events() {
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
    let index_path = temp.path().join("addon-index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-details"
name = "Curated Details"
version = "2.0.0"
source = {{ kind = "local_archive", path = "{}" }}
addon_directories = ["Details"]
supported_flavors = ["retail"]
"#,
            archive_path.display().to_string().replace('\\', "\\\\")
        ),
    )
    .expect("write index");

    AddonService::new()
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install details");

    let service = AddonIndexService::new();
    let run = service
        .attach_collecting_progress(AttachAddonIndexAppRequest {
            installation,
            index_path,
            name: Some("curated-details".to_string()),
            dry_run: false,
        })
        .expect("attach from index with collected progress");

    assert!(run.result.applied);
    let phases = run
        .progress
        .iter()
        .map(|event| (event.task, event.phase))
        .collect::<Vec<_>>();
    assert_eq!(
        phases.first(),
        Some(&(TaskKind::AddonIndexAttach, TaskPhase::Preparing))
    );
    assert_eq!(
        phases.last(),
        Some(&(TaskKind::AddonIndexAttach, TaskPhase::Completed))
    );
    assert!(phases.contains(&(TaskKind::AddonIndexAttach, TaskPhase::Executing)));
}

#[test]
fn addon_index_service_validate_reports_warning_summary() {
    let temp = tempdir().expect("temp dir");
    let index_path = temp.path().join("addon-index.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-plater"
name = "Curated Plater"
version = "1.0.0"
source = { kind = "http_archive", url = "https://example.invalid/plater.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");

    let service = AddonIndexService::new();
    let result = service
        .validate(InspectAddonIndexRequest { index_path })
        .expect("validate addon index");

    assert!(!result.valid);
    assert_eq!(result.warning_count, 1);
    assert_eq!(result.blocking_warning_count, 1);
    assert_eq!(result.advisory_warning_count, 0);
    assert_eq!(result.warnings.len(), 1);
    assert!(matches!(
        result.warnings[0].code,
        AddonIndexInspectionWarningCodeResult::MissingExactIdentityHints
    ));
    assert!(matches!(
        result.warnings[0].severity,
        AddonIndexInspectionWarningSeverityResult::Blocking
    ));
    assert_eq!(result.warnings[0].package_id, "curated-plater");
}

#[test]
fn addon_index_service_validate_keeps_advisory_identity_gaps_non_blocking() {
    let temp = tempdir().expect("temp dir");
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
match_package_ids = ["legacy-weakauras"]
source = { kind = "http_archive", url = "https://example.invalid/weakauras.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");

    let service = AddonIndexService::new();
    let result = service
        .validate(InspectAddonIndexRequest { index_path })
        .expect("validate addon index");

    assert!(result.valid);
    assert_eq!(result.warning_count, 1);
    assert_eq!(result.blocking_warning_count, 0);
    assert_eq!(result.advisory_warning_count, 1);
    assert_eq!(result.warnings.len(), 1);
    assert!(matches!(
        result.warnings[0].code,
        AddonIndexInspectionWarningCodeResult::MissingAddonDirectories
    ));
    assert!(matches!(
        result.warnings[0].severity,
        AddonIndexInspectionWarningSeverityResult::Advisory
    ));
    assert_eq!(result.warnings[0].package_id, "weakauras");
}

#[test]
fn addon_index_service_suggests_exact_identity_hints_from_local_registry() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("Plater.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Title: Plater\n## Version: 1.0.0\n",
        )],
    );

    AddonService::new()
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install tracked addon");

    let index_path = temp.path().join("addon-index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-plater"
name = "Curated Plater"
version = "2.0.0"
source = {{ kind = "local_archive", path = "{}" }}
supported_flavors = ["retail"]
"#,
            archive_path.display().to_string().replace('\\', "\\\\"),
        ),
    )
    .expect("write index");

    let service = AddonIndexService::new();
    let result = service
        .suggest(SuggestAddonIndexRequest {
            installation,
            index_path,
            name: None,
        })
        .expect("suggest addon index hints");

    assert_eq!(result.index_name, "Fixture Index");
    assert_eq!(result.considered_package_count, 1);
    assert_eq!(result.suggested_package_count, 1);
    assert_eq!(result.complete_package_count, 0);
    assert_eq!(result.no_match_package_count, 0);
    let package = &result.packages[0];
    assert_eq!(package.package_id, "curated-plater");
    assert!(matches!(
        package.status,
        AddonIndexPackageSuggestionStatusResult::Suggested
    ));
    assert_eq!(
        package.matched_tracked_package_id.as_deref(),
        Some("plater")
    );
    assert!(matches!(
        package.match_strategy,
        Some(AddonIndexTrackedMatchStrategyResult::SourceIdentity)
    ));
    assert_eq!(package.match_package_ids_to_add, vec!["plater".to_string()]);
    assert_eq!(package.addon_directories_to_add, vec!["Plater".to_string()]);
}

#[test]
fn addon_index_service_scaffolds_index_from_tracked_registry() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("Plater.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Title: Plater\n## Version: 1.0.0\n",
        )],
    );

    AddonService::new()
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install tracked addon");

    let index_path = temp.path().join("addon-index.toml");
    let service = AddonIndexService::new();
    let result: AddonIndexScaffoldResult = service
        .scaffold(ScaffoldAddonIndexRequest {
            installation,
            index_path: index_path.clone(),
            index_name: "Guild UI".to_string(),
            description: Some("Scaffolded".to_string()),
            name: Some("plater".to_string()),
            overwrite: false,
        })
        .expect("scaffold addon index");

    assert_eq!(result.index_path, index_path);
    assert_eq!(result.index_name, "Guild UI");
    assert_eq!(result.package_count, 1);
    assert_eq!(result.package_ids, vec!["plater".to_string()]);
    assert_eq!(result.inferred_name_package_count, 1);
    assert_eq!(result.inferred_version_package_count, 1);
    assert_eq!(result.placeholder_version_package_count, 0);
    assert!(index_path.exists());
}

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

#[test]
fn addon_index_service_update_with_callbacks_uses_plain_closures() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let installed_archive_path = temp.path().join("Details-installed.zip");
    let updated_archive_path = temp.path().join("Details-updated.zip");
    create_addon_archive(
        &installed_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &updated_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 120000\n## Version: 2.0.0\n",
        )],
    );
    let index_path = write_index(temp.path(), &updated_archive_path);

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone().into_domain()),
        installation: installation.clone().into_domain(),
        source: installed_archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install tracked addon");

    let seen = std::cell::RefCell::new(Vec::new());
    let cancellation_checks = std::cell::Cell::new(0usize);
    let result = service_update_with_callbacks(
        &AddonIndexService::new(),
        UpdateAddonIndexAppRequest {
            installation,
            index_path,
            name: Some("details".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
        },
        &seen,
        &cancellation_checks,
    )
    .expect("update from index with callbacks");

    assert_eq!(result.selected_packages.len(), 1);
    assert!(seen.borrow().len() >= 4);
    assert!(seen.borrow().iter().any(|event| {
        event.task == TaskKind::AddonIndexUpdate
            && event.phase == TaskPhase::Executing
            && event.message.contains("Writing updated addon directory")
    }));
    assert!(cancellation_checks.get() >= 3);
}

fn service_update_with_callbacks(
    service: &AddonIndexService,
    request: UpdateAddonIndexAppRequest,
    seen: &std::cell::RefCell<Vec<TaskProgressEvent>>,
    cancellation_checks: &std::cell::Cell<usize>,
) -> AppResult<AddonIndexUpdateResult> {
    service.update_with_callbacks(
        request,
        || {
            let next = cancellation_checks.get() + 1;
            cancellation_checks.set(next);
            false
        },
        |event| seen.borrow_mut().push(event),
    )
}

#[test]
fn addon_index_service_update_preflights_unsupported_dependency_policy_before_domain_prepare() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("Plater.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    let index_path = temp.path().join("addon-index-github.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-plater"
name = "Curated Plater"
version = "2.0.0"
source = { kind = "github_release", owner = "owner", repo = "repo", asset_name = "release.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");

    let update_attempts = Arc::new(AtomicUsize::new(0));
    let runtime = AppRuntime::with_addon_provider(FakeUnsupportedDependencyAddonProvider {
        archive_path: archive_path.clone(),
        update_attempts: update_attempts.clone(),
    });
    let addon_service = AddonService::with_runtime(runtime.clone());
    let index_service = AddonIndexService::with_runtime(runtime);

    addon_service
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: "github:owner/repo#plater.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install tracked github addon");

    AddonPolicyService::new()
        .set(SetAddonPolicyAppRequest {
            installation: installation.clone(),
            package: "plater".to_string(),
            ignored: None,
            pin: None,
            release_channel: None,
            allow_prerelease: None,
            install_dependencies: Some(true),
        })
        .expect("enable dependency installation");

    let error = index_service
        .update(UpdateAddonIndexAppRequest {
            installation,
            index_path,
            name: Some("curated-plater".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        })
        .expect_err("unsupported dependency policy should fail in app preflight");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("not supported"));
    assert_eq!(update_attempts.load(Ordering::SeqCst), 0);
}

#[test]
fn addon_index_service_update_preflights_unsupported_dependency_policy_by_display_name_when_source_family_changes()
 {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("Plater.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Title: Plater\n## Version: 1.0.0\n",
        )],
    );
    let index_path = temp.path().join("addon-index-github-display-name.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-plater"
name = "Plater"
version = "2.0.0"
source = { kind = "github_release", owner = "new-owner", repo = "new-repo", asset_name = "release.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");

    let update_attempts = Arc::new(AtomicUsize::new(0));
    let runtime = AppRuntime::with_addon_provider(FakeDisplayNamePreflightAddonProvider {
        archive_path: archive_path.clone(),
        update_attempts: update_attempts.clone(),
    });
    let addon_service = AddonService::with_runtime(runtime.clone());
    let index_service = AddonIndexService::with_runtime(runtime);

    addon_service
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: "github:legacy-owner/legacy-repo#plater.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install tracked github addon");

    AddonPolicyService::new()
        .set(SetAddonPolicyAppRequest {
            installation: installation.clone(),
            package: "plater".to_string(),
            ignored: None,
            pin: None,
            release_channel: None,
            allow_prerelease: None,
            install_dependencies: Some(true),
        })
        .expect("enable dependency installation");

    let error = index_service
        .update(UpdateAddonIndexAppRequest {
            installation,
            index_path,
            name: Some("curated-plater".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        })
        .expect_err("unsupported dependency policy should fail in app preflight");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("not supported"));
    assert_eq!(update_attempts.load(Ordering::SeqCst), 0);
}

#[test]
fn addon_index_service_update_preflights_unsupported_dependency_policy_by_curated_package_hint_when_source_family_changes()
 {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("Plater.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    let index_path = temp.path().join("addon-index-github-hint.toml");
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

    let update_attempts = Arc::new(AtomicUsize::new(0));
    let runtime = AppRuntime::with_addon_provider(FakeDisplayNamePreflightAddonProvider {
        archive_path: archive_path.clone(),
        update_attempts: update_attempts.clone(),
    });
    let addon_service = AddonService::with_runtime(runtime.clone());
    let index_service = AddonIndexService::with_runtime(runtime);

    addon_service
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: "github:legacy-owner/legacy-repo#plater.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install tracked github addon");

    AddonPolicyService::new()
        .set(SetAddonPolicyAppRequest {
            installation: installation.clone(),
            package: "plater".to_string(),
            ignored: None,
            pin: None,
            release_channel: None,
            allow_prerelease: None,
            install_dependencies: Some(true),
        })
        .expect("enable dependency installation");

    let error = index_service
        .update(UpdateAddonIndexAppRequest {
            installation,
            index_path,
            name: Some("curated-plater-v2".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        })
        .expect_err("unsupported dependency policy should fail in app preflight");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("not supported"));
    assert_eq!(update_attempts.load(Ordering::SeqCst), 0);
}

#[test]
fn addon_index_service_update_reports_deferred_dependency_policy_guidance_when_preflight_cannot_match()
 {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("Plater.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    let index_path = temp.path().join("addon-index-github-domain-fallback.toml");
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

    let update_attempts = Arc::new(AtomicUsize::new(0));
    let runtime = AppRuntime::with_addon_provider(FakeDeferredDependencyGuidanceAddonProvider {
        archive_path: archive_path.clone(),
        update_attempts: update_attempts.clone(),
    });
    let addon_service = AddonService::with_runtime(runtime.clone());
    let index_service = AddonIndexService::with_runtime(runtime);

    addon_service
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: "github:legacy-owner/legacy-repo#plater.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install tracked github addon");

    AddonPolicyService::new()
        .set(SetAddonPolicyAppRequest {
            installation: installation.clone(),
            package: "plater".to_string(),
            ignored: None,
            pin: None,
            release_channel: None,
            allow_prerelease: None,
            install_dependencies: Some(true),
        })
        .expect("enable dependency installation");

    let error = index_service
        .update(UpdateAddonIndexAppRequest {
            installation,
            index_path,
            name: Some("curated-plater-v3".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        })
        .expect_err("domain fallback should fail with explicit guidance");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("not supported"));
    assert!(
        error
            .to_string()
            .contains("app preflight could not determine")
    );
    assert!(error.to_string().contains("match_package_ids"));
    assert_eq!(update_attempts.load(Ordering::SeqCst), 1);
}

fn create_empty_installation(root: &Path) -> ResolvedInstallationValue {
    let product_root = root.join("World of Warcraft");
    let flavor_root = product_root.join("_retail_");
    let interface_dir = flavor_root.join("Interface");
    let addon_dir = interface_dir.join("AddOns");
    let wtf_dir = flavor_root.join("WTF");
    let fonts_dir = flavor_root.join("Fonts");

    fs::create_dir_all(&addon_dir).expect("addon dir");
    fs::create_dir_all(&wtf_dir).expect("wtf dir");
    fs::create_dir_all(&fonts_dir).expect("fonts dir");

    ResolvedInstallationValue::from_domain(crate::core::install::DetectedFlavorInstallation {
        platform: HostPlatform::Windows,
        product_root,
        flavor_root,
        flavor: WowFlavor::Retail,
        interface_dir,
        addon_dir,
        wtf_dir,
        fonts_dir,
    })
}

fn write_index(root: &Path, archive_path: &Path) -> std::path::PathBuf {
    let index_path = root.join("addon-index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "weakauras"
name = "WeakAuras"
version = "1.0.0"
source = {{ kind = "local_archive", path = "{}" }}
supported_flavors = ["retail"]

[[packages]]
id = "details"
name = "Details"
version = "2.0.0"
source = {{ kind = "local_archive", path = "{}" }}
supported_flavors = ["retail"]
"#,
            archive_path.display().to_string().replace('\\', "\\\\"),
            archive_path.display().to_string().replace('\\', "\\\\")
        ),
    )
    .expect("write index");
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

#[derive(Clone)]
struct FakeAddonProvider {
    archive_path: PathBuf,
}

impl AddonProvider for FakeAddonProvider {
    fn materialize_source_input(
        &self,
        request: MaterializeSourceInputRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        Ok(MaterializedAddonSource {
            source_ref: AddonSourceRef::HttpArchive {
                url: request.source.to_string(),
            },
            archive_path: self.archive_path.clone(),
        })
    }

    fn materialize_source_ref(
        &self,
        request: MaterializeSourceRefRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        match request.source {
            AddonSourceRef::HttpArchive { url }
                if url == "https://example.invalid/WeakAuras.zip" =>
            {
                Ok(MaterializedAddonSource {
                    source_ref: request.source.clone(),
                    archive_path: self.archive_path.clone(),
                })
            }
            other => Err(AppError::Validation(format!(
                "unexpected addon source ref: {}",
                other.display_name()
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

#[derive(Clone)]
struct FakeDownloadProgressAddonProvider {
    archive_path: PathBuf,
}

impl AddonProvider for FakeDownloadProgressAddonProvider {
    fn materialize_source_input(
        &self,
        request: MaterializeSourceInputRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        Ok(MaterializedAddonSource {
            source_ref: AddonSourceRef::HttpArchive {
                url: request.source.to_string(),
            },
            archive_path: self.archive_path.clone(),
        })
    }

    fn materialize_source_ref(
        &self,
        request: MaterializeSourceRefRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        let source_ref = request.source.clone();
        request.context.report_download_progress(
            &source_ref,
            "WeakAuras-progress.zip",
            0,
            Some(1024),
            None,
        );
        request.context.report_download_progress(
            &source_ref,
            "WeakAuras-progress.zip",
            1024,
            Some(1024),
            Some(512),
        );
        Ok(MaterializedAddonSource {
            source_ref,
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

#[derive(Clone)]
struct FakeUnsupportedDependencyAddonProvider {
    archive_path: PathBuf,
    update_attempts: Arc<AtomicUsize>,
}

impl AddonProvider for FakeUnsupportedDependencyAddonProvider {
    fn materialize_source_input(
        &self,
        request: MaterializeSourceInputRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        if request.source != "github:owner/repo#plater.zip" {
            return Err(AppError::Validation(format!(
                "unexpected addon source input: {}",
                request.source
            )));
        }

        Ok(MaterializedAddonSource {
            source_ref: AddonSourceRef::GitHubRelease {
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                tag: None,
                asset_name: Some("plater.zip".to_string()),
            },
            archive_path: self.archive_path.clone(),
        })
    }

    fn materialize_source_ref(
        &self,
        request: MaterializeSourceRefRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        self.update_attempts.fetch_add(1, Ordering::SeqCst);
        Err(AppError::Validation(format!(
            "app preflight should have rejected addon-index update before materializing `{}`",
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

#[derive(Clone)]
struct FakeDisplayNamePreflightAddonProvider {
    archive_path: PathBuf,
    update_attempts: Arc<AtomicUsize>,
}

impl AddonProvider for FakeDisplayNamePreflightAddonProvider {
    fn materialize_source_input(
        &self,
        request: MaterializeSourceInputRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        if request.source != "github:legacy-owner/legacy-repo#plater.zip" {
            return Err(AppError::Validation(format!(
                "unexpected addon source input: {}",
                request.source
            )));
        }

        Ok(MaterializedAddonSource {
            source_ref: AddonSourceRef::GitHubRelease {
                owner: "legacy-owner".to_string(),
                repo: "legacy-repo".to_string(),
                tag: None,
                asset_name: Some("plater.zip".to_string()),
            },
            archive_path: self.archive_path.clone(),
        })
    }

    fn materialize_source_ref(
        &self,
        request: MaterializeSourceRefRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        self.update_attempts.fetch_add(1, Ordering::SeqCst);
        Err(AppError::Validation(format!(
            "app preflight should have rejected addon-index update before materializing `{}`",
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

#[derive(Clone)]
struct FakeDeferredDependencyGuidanceAddonProvider {
    archive_path: PathBuf,
    update_attempts: Arc<AtomicUsize>,
}

impl AddonProvider for FakeDeferredDependencyGuidanceAddonProvider {
    fn materialize_source_input(
        &self,
        request: MaterializeSourceInputRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        if request.source != "github:legacy-owner/legacy-repo#plater.zip" {
            return Err(AppError::Validation(format!(
                "unexpected addon source input: {}",
                request.source
            )));
        }

        Ok(MaterializedAddonSource {
            source_ref: AddonSourceRef::GitHubRelease {
                owner: "legacy-owner".to_string(),
                repo: "legacy-repo".to_string(),
                tag: None,
                asset_name: Some("plater.zip".to_string()),
            },
            archive_path: self.archive_path.clone(),
        })
    }

    fn materialize_source_ref(
        &self,
        request: MaterializeSourceRefRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        self.update_attempts.fetch_add(1, Ordering::SeqCst);
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
    assert!(
        phases
            .iter()
            .any(|phase| *phase == (task, TaskPhase::Executing))
    );
    assert!(events.iter().any(|event| {
        event.task == task
            && event.phase == TaskPhase::Executing
            && event.message.contains(executing_detail)
    }));
}
