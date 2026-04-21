use std::cell::Cell;
use std::fs;

use tempfile::tempdir;
use zip::ZipArchive;

use super::super::constants::MANIFEST_ENTRY;
use super::super::planner::pipeline::{plan_apply_from_entries_with_reader, prepare_bundle_apply};
use super::support::*;
use crate::core::bundle::*;
use crate::core::install::HostPlatform;
use crate::core::manifest::{CharacterMappingMode, CharacterResource, ResourceApplyPolicy};
use crate::core::task::{CancellationToken, NeverCancel, TaskKind, TaskPhase, VecTaskProgressSink};

#[test]
fn unpack_bundle_task_reports_progress_for_dry_run() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let mut progress = VecTaskProgressSink::default();
    let cancellation = NeverCancel;
    let result = unpack_bundle_task(
        UnpackBundleRequest {
            bundle_path,
            installation: target_installation,
            dry_run: true,
            backup_output_path: None,
            apply_mappings: BundleApplyMappings::default(),
        },
        &cancellation,
        &mut progress,
    )
    .expect("unpack bundle dry run task");

    let phases = progress
        .events()
        .iter()
        .map(|event| (event.task, event.phase))
        .collect::<Vec<_>>();
    assert_eq!(
        phases,
        vec![
            (TaskKind::BundleApply, TaskPhase::Preparing),
            (TaskKind::BundleApply, TaskPhase::Planning),
            (TaskKind::BundleApply, TaskPhase::Completed),
        ]
    );
    assert!(result.dry_run);
}

#[test]
fn unpack_bundle_task_honors_cancellation_before_execution() {
    struct CancelOnSecondCheck {
        checks: Cell<usize>,
    }

    impl CancellationToken for CancelOnSecondCheck {
        fn is_cancelled(&self) -> bool {
            let next = self.checks.get() + 1;
            self.checks.set(next);
            next >= 2
        }
    }

    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let mut progress = VecTaskProgressSink::default();
    let cancellation = CancelOnSecondCheck {
        checks: Cell::new(0),
    };
    let error = unpack_bundle_task(
        UnpackBundleRequest {
            bundle_path,
            installation: target_installation.clone(),
            dry_run: false,
            backup_output_path: Some(target.path().join("backups")),
            apply_mappings: BundleApplyMappings::default(),
        },
        &cancellation,
        &mut progress,
    )
    .expect_err("bundle task should cancel before execution");

    assert!(matches!(error, crate::core::error::AppError::Cancelled(_)));
    assert!(
        !target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
}

#[test]
fn unpack_bundle_task_reports_operation_progress_during_execution() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let mut progress = VecTaskProgressSink::default();
    let cancellation = NeverCancel;
    let result = unpack_bundle_task(
        UnpackBundleRequest {
            bundle_path,
            installation: target_installation,
            dry_run: false,
            backup_output_path: Some(target.path().join("backups")),
            apply_mappings: BundleApplyMappings::default(),
        },
        &cancellation,
        &mut progress,
    )
    .expect("bundle task should complete");

    let executing_messages = progress
        .events()
        .iter()
        .filter(|event| event.task == TaskKind::BundleApply && event.phase == TaskPhase::Executing)
        .map(|event| event.message.as_str())
        .collect::<Vec<_>>();

    assert!(result.written_files > 0);
    assert!(executing_messages.len() > 1);
    assert!(executing_messages.iter().any(|message| {
        message.contains("operation 1/") && message.contains("Executing bundle operation")
    }));
}

#[test]
fn unpack_bundle_task_honors_cancellation_during_execution_loop() {
    struct CancelOnFifthCheck {
        checks: Cell<usize>,
    }

    impl CancellationToken for CancelOnFifthCheck {
        fn is_cancelled(&self) -> bool {
            let next = self.checks.get() + 1;
            self.checks.set(next);
            next >= 5
        }
    }

    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let mut progress = VecTaskProgressSink::default();
    let cancellation = CancelOnFifthCheck {
        checks: Cell::new(0),
    };
    let error = unpack_bundle_task(
        UnpackBundleRequest {
            bundle_path,
            installation: target_installation.clone(),
            dry_run: false,
            backup_output_path: Some(target.path().join("backups")),
            apply_mappings: BundleApplyMappings::default(),
        },
        &cancellation,
        &mut progress,
    )
    .expect_err("bundle task should cancel during execution loop");

    assert!(matches!(error, crate::core::error::AppError::Cancelled(_)));
    assert!(progress.events().iter().any(|event| {
        event.task == TaskKind::BundleApply
            && event.phase == TaskPhase::Executing
            && event.message.contains("operation 1/")
    }));
    assert!(
        !target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
}

#[test]
fn unpack_bundle_restores_files_and_creates_backup() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path: bundle_path.clone(),
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("unpack bundle");

    assert_eq!(result.bundle_path, bundle_path);
    assert!(result.written_files > 0);
    assert!(
        result
            .backup_path
            .as_ref()
            .is_some_and(|path| path.exists())
    );
    assert!(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
    assert!(target_installation.wtf_dir.join("Config.wtf").exists());
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("SavedVariables")
            .join("Details.lua")
            .exists()
    );
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("Illidan")
            .join("Examplemage")
            .join("AddOns.txt")
            .exists()
    );
    assert!(target_installation.fonts_dir.join("FRIZQT__.ttf").exists());
    assert!(
        target_installation
            .interface_dir
            .join("SharedXML")
            .join("texture.blp")
            .exists()
    );

    let inspection = inspect_bundle(&result.bundle_path).expect("inspect bundle");
    assert_eq!(inspection.entries.addons, 2);
    assert_eq!(inspection.entries.fonts, 1);
}

#[test]
fn plan_bundle_apply_discovers_local_accounts_and_selected_accounts() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    fs::create_dir_all(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACC_A")
            .join("SavedVariables"),
    )
    .expect("account a");
    fs::create_dir_all(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACC_B")
            .join("SavedVariables"),
    )
    .expect("account b");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings {
            selected_accounts: vec!["ACC_A".to_string()],
            ..BundleApplyMappings::default()
        },
    )
    .expect("plan bundle");

    assert_eq!(plan.discovered_accounts.len(), 2);
    assert_eq!(plan.selected_target_accounts, vec!["ACC_A".to_string()]);
    assert!(plan.summary.files_to_add > 0);
    assert!(plan.operations.iter().any(|item| {
        item.group == ApplyGroup::WtfCommon && item.target_account.as_deref() == Some("ACC_A")
    }));
}

#[test]
fn plan_bundle_apply_requires_explicit_common_account_selection_for_common_only_bundle() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.resources.wtf_characters.clear();

    fs::create_dir_all(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ONLYACC")
            .join("SavedVariables"),
    )
    .expect("target account");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let error = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect_err("common-only bundle should require explicit account selection");

    assert!(
        error
            .to_string()
            .contains("common WTF resources require explicit target account selection")
    );
}

#[test]
fn keep_original_character_mode_ignores_target_identity_overrides() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.mapping.character_mode = CharacterMappingMode::KeepOriginal;

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings {
            target_account: Some("TARGETACC".to_string()),
            target_server: Some("Stormrage".to_string()),
            target_character: Some("Targetmage".to_string()),
            ..BundleApplyMappings::default()
        },
    )
    .expect("plan bundle");

    assert_eq!(plan.selected_target_accounts, vec!["ACCOUNT".to_string()]);
    assert_eq!(plan.character_mappings.len(), 1);
    assert_eq!(plan.character_mappings[0].target_account, "ACCOUNT");
    assert_eq!(plan.character_mappings[0].target_server, "Illidan");
    assert_eq!(plan.character_mappings[0].target_character, "Examplemage");
}

#[test]
fn explicit_character_mode_requires_resolved_target_identity() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.mapping.character_mode = CharacterMappingMode::Explicit;

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let error = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect_err("explicit mode should require a resolved target identity");

    assert!(
        error
            .to_string()
            .contains("explicit character mode requires a fully resolved target identity")
    );
    assert!(error.to_string().contains("--mapping-file"));
}

#[test]
fn prompt_character_mode_requires_resolved_target_identity() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.mapping.character_mode = CharacterMappingMode::Prompt;
    manifest.resources.wtf_characters[0].target_hint = Some("Map to your main".to_string());

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let error = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect_err("prompt mode should require caller-provided mappings");

    assert!(
        error
            .to_string()
            .contains("current CLI does not prompt automatically")
    );
    assert!(error.to_string().contains("Map to your main"));
}

#[test]
fn multi_character_explicit_mode_rejects_global_target_identity_overrides() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.mapping.character_mode = CharacterMappingMode::Explicit;
    manifest.resources.wtf_characters.push(CharacterResource {
        source_account: Some("ACCOUNT".to_string()),
        source_server: "Illidan".to_string(),
        source_character: "Altmage".to_string(),
        target_hint: None,
    });
    fs::create_dir_all(
        source_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("Illidan")
            .join("Altmage"),
    )
    .expect("alt character");
    fs::write(
        source_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("Illidan")
            .join("Altmage")
            .join("AddOns.txt"),
        "Altmage",
    )
    .expect("alt addons");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let error = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings {
            target_server: Some("Stormrage".to_string()),
            target_character: Some("Targetmage".to_string()),
            ..BundleApplyMappings::default()
        },
    )
    .expect_err("multi-character explicit mode should reject global target identity");

    assert!(error.to_string().contains("exactly one character"));
    assert!(error.to_string().contains("--mapping-file"));
}

#[test]
fn bundle_apply_plan_does_not_expose_execution_only_fields() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest_with_rewrite(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");

    let operations = serde_json::to_value(&plan)
        .expect("serialize plan")
        .get("operations")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .expect("operations array");

    assert!(!operations.is_empty());
    assert!(
        operations
            .iter()
            .all(|operation| operation.get("staged_path").is_none())
    );
    assert!(
        operations
            .iter()
            .all(|operation| operation.get("rewrites").is_none())
    );
    assert!(
        operations
            .iter()
            .all(|operation| operation.get("rewrite_count").is_none())
    );
    assert!(
        operations
            .iter()
            .all(|operation| operation.get("rewrite_applied").is_none())
    );
}

#[test]
fn bundle_apply_plan_uses_explicit_resource_group_order() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");
    let mut groups = Vec::new();
    for operation in plan
        .operations
        .iter()
        .filter(|operation| operation.action == ApplyAction::Add)
    {
        if groups.last().copied() != Some(operation.group) {
            groups.push(operation.group);
        }
    }

    assert_eq!(
        groups,
        vec![
            ApplyGroup::Addons,
            ApplyGroup::InterfaceAssets,
            ApplyGroup::Fonts,
            ApplyGroup::WtfCommon,
            ApplyGroup::WtfCharacters,
        ]
    );
}

#[test]
fn plan_bundle_apply_classifies_wtf_scopes_and_account_root_files() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let source_account_dir = source_installation.wtf_dir.join("Account").join("ACCOUNT");
    let source_character_dir = source_account_dir.join("Illidan").join("Examplemage");

    fs::write(
        source_account_dir.join("account-settings.wtf"),
        "account root",
    )
    .expect("account root file");
    fs::write(source_account_dir.join("config-cache.wtf"), "account cache")
        .expect("account cache file");
    fs::create_dir_all(
        source_installation
            .wtf_dir
            .join("Account")
            .join("SavedVariables"),
    )
    .expect("root saved variables");
    fs::write(
        source_installation
            .wtf_dir
            .join("Account")
            .join("SavedVariables")
            .join("RootDetails.lua"),
        "DetailsDB = {}",
    )
    .expect("root saved variable");
    fs::create_dir_all(source_character_dir.join("SavedVariables"))
        .expect("character saved variables");
    fs::write(
        source_character_dir.join("SavedVariables").join("Pawn.lua"),
        "PawnDB = {}",
    )
    .expect("character saved variable");
    fs::write(
        source_character_dir.join("config-cache.wtf"),
        "character cache",
    )
    .expect("character cache file");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let file = fs::File::open(&bundle_path).expect("bundle file");
    let mut archive = ZipArchive::new(file).expect("zip archive");
    assert!(
        archive
            .by_name("wtf/common/accounts/ACCOUNT/account-settings.wtf")
            .is_ok()
    );
    assert!(
        archive
            .by_name("wtf/common/accounts/ACCOUNT/config-cache.wtf")
            .is_ok()
    );
    assert!(
        archive
            .by_name("wtf/common/root/SavedVariables/RootDetails.lua")
            .is_ok()
    );

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");

    let scope_for = |archive_name: &str| {
        plan.operations
            .iter()
            .find(|operation| operation.archive_name == archive_name)
            .and_then(|operation| operation.wtf_scope)
    };

    assert_eq!(
        scope_for("wtf/common/Config.wtf"),
        Some(WtfScope::GlobalConfig)
    );
    assert_eq!(
        scope_for("wtf/common/root/SavedVariables/RootDetails.lua"),
        Some(WtfScope::RootSavedVariables)
    );
    assert_eq!(
        scope_for("wtf/common/accounts/ACCOUNT/account-settings.wtf"),
        Some(WtfScope::AccountRootFile)
    );
    assert_eq!(
        scope_for("wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua"),
        Some(WtfScope::AccountSavedVariables)
    );
    assert_eq!(
        scope_for("wtf/characters/ACCOUNT/Illidan/Examplemage/SavedVariables/Pawn.lua"),
        Some(WtfScope::CharacterSavedVariables)
    );
    assert_eq!(
        scope_for("wtf/characters/ACCOUNT/Illidan/Examplemage/AddOns.txt"),
        Some(WtfScope::CharacterState)
    );
    assert_eq!(
        scope_for("wtf/common/accounts/ACCOUNT/config-cache.wtf"),
        Some(WtfScope::CacheLike)
    );
    assert_eq!(
        scope_for("wtf/characters/ACCOUNT/Illidan/Examplemage/config-cache.wtf"),
        Some(WtfScope::CacheLike)
    );
}

#[test]
fn plan_bundle_apply_reports_existing_files_as_replace_in_logical_plan() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), true);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");

    assert_eq!(plan.summary.files_to_add, 0);
    assert!(plan.summary.files_to_replace > 0);
    assert_eq!(plan.summary.files_to_replace, plan.operations.len());
    assert_eq!(plan.summary.files_to_skip, 0);
    assert_eq!(
        plan.group_policies.addons.policy,
        ResourceApplyPolicy::Merge
    );
    assert!(
        plan.operations
            .iter()
            .all(|operation| operation.action == ApplyAction::Replace)
    );
}

#[test]
fn unpack_bundle_dry_run_still_skips_identical_files_after_prepare() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), true);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation,
        dry_run: true,
        backup_output_path: None,
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("dry-run identical bundle");

    assert!(result.dry_run);
    assert_eq!(result.plan_summary.files_to_add, 0);
    assert_eq!(result.plan_summary.files_to_replace, 0);
    assert!(result.plan_summary.files_to_skip > 0);
    assert_eq!(result.plan_summary.files_to_skip, result.planned_files);
    assert_eq!(result.written_files, 0);
}

#[test]
fn plan_apply_from_entries_with_reader_skips_byte_reads_for_deterministic_add_operations() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path(), false);
    let read_calls = Cell::new(0usize);

    let plan = plan_apply_from_entries_with_reader(
        std::path::Path::new("test.bundle.zip"),
        &installation,
        sample_manifest(),
        &["addons/WeakAuras/WeakAuras.toc".to_string()],
        &BundleApplyMappings::default(),
        |_archive_name| {
            read_calls.set(read_calls.get() + 1);
            Ok(b"unused".to_vec())
        },
    )
    .expect("plan apply from entries");

    assert_eq!(read_calls.get(), 0);
    assert_eq!(plan.summary.files_to_add, 1);
    assert_eq!(plan.summary.files_to_replace, 0);
    assert_eq!(plan.summary.files_to_skip, 0);
    assert_eq!(plan.operations.len(), 1);
    assert_eq!(plan.operations[0].action, ApplyAction::Add);
}

#[test]
fn plan_apply_from_entries_with_reader_keeps_existing_targets_logical_without_byte_reads() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path(), true);
    let read_calls = Cell::new(0usize);

    let plan = plan_apply_from_entries_with_reader(
        std::path::Path::new("test.bundle.zip"),
        &installation,
        sample_manifest(),
        &["addons/WeakAuras/WeakAuras.toc".to_string()],
        &BundleApplyMappings::default(),
        |_archive_name| {
            read_calls.set(read_calls.get() + 1);
            Ok(b"## Interface: 110000".to_vec())
        },
    )
    .expect("plan apply from entries");

    assert_eq!(read_calls.get(), 0);
    assert_eq!(plan.summary.files_to_add, 0);
    assert_eq!(plan.summary.files_to_replace, 1);
    assert_eq!(plan.summary.files_to_skip, 0);
    assert_eq!(plan.operations.len(), 1);
    assert_eq!(plan.operations[0].action, ApplyAction::Replace);
}

#[test]
fn prepare_bundle_apply_projects_preview_operations_into_execution_operations() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest_with_rewrite(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let prepared = prepare_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("prepare bundle");

    let plan_projection = prepared
        .plan
        .operations
        .iter()
        .map(|operation| {
            (
                operation.action,
                operation.archive_name.clone(),
                operation.destination.clone(),
            )
        })
        .collect::<Vec<_>>();
    let execution_projection = prepared
        .execution_operations
        .iter()
        .map(|operation| {
            (
                operation.action,
                operation.archive_name.clone(),
                operation.destination.clone(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(execution_projection, plan_projection);
    assert!(
        prepared
            .execution_operations
            .iter()
            .any(|operation| !operation.rewrites.is_empty())
    );

    let serialized_operations = serde_json::to_value(&prepared.plan)
        .expect("serialize prepared plan")
        .get("operations")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .expect("operations array");
    assert!(
        serialized_operations
            .iter()
            .all(|operation| operation.get("rewrites").is_none())
    );
}

#[test]
fn unpack_bundle_applies_character_mapping_and_lua_rewrite() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest_with_rewrite();
    manifest.mapping.character_mode = CharacterMappingMode::Explicit;

    fs::create_dir_all(
        source_installation
            .wtf_dir
            .join("Account")
            .join("SavedVariables"),
    )
    .expect("root saved variables");
    fs::write(
        source_installation
            .wtf_dir
            .join("Account")
            .join("SavedVariables")
            .join("RootDetails.lua"),
        r#"
DetailsDB = {
  ["profileKeys"] = {
    ["Examplemage - Illidan"] = "Default",
  },
  ["profiles"] = {
    ["Default.Illidan.Examplemage"] = {},
  },
}
"#,
    )
    .expect("root saved variable");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings {
            target_account: Some("TARGETACC".to_string()),
            target_server: Some("Stormrage".to_string()),
            target_character: Some("Targetmage".to_string()),
            selected_accounts: Vec::new(),
            all_accounts: false,
            characters: Vec::new(),
        },
    })
    .expect("unpack bundle");

    assert_eq!(result.character_mappings.len(), 1);
    assert!(result.rewritten_files >= 2);
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("TARGETACC")
            .join("SavedVariables")
            .join("Details.lua")
            .exists()
    );
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("TARGETACC")
            .join("Stormrage")
            .join("Targetmage")
            .join("SavedVariables")
            .join("Pawn.lua")
            .exists()
    );

    let common_lua = fs::read_to_string(
        target_installation
            .wtf_dir
            .join("Account")
            .join("TARGETACC")
            .join("SavedVariables")
            .join("Details.lua"),
    )
    .expect("common lua");
    assert!(common_lua.contains("Targetmage - Stormrage"));
    assert!(common_lua.contains("Default.Stormrage.Targetmage"));

    let root_common_lua = fs::read_to_string(
        target_installation
            .wtf_dir
            .join("Account")
            .join("SavedVariables")
            .join("RootDetails.lua"),
    )
    .expect("root common lua");
    assert!(root_common_lua.contains("Targetmage - Stormrage"));
    assert!(root_common_lua.contains("Default.Stormrage.Targetmage"));

    let character_lua = fs::read_to_string(
        target_installation
            .wtf_dir
            .join("Account")
            .join("TARGETACC")
            .join("Stormrage")
            .join("Targetmage")
            .join("SavedVariables")
            .join("Pawn.lua"),
    )
    .expect("character lua");
    assert!(character_lua.contains(r#""Targetmage""#));
    assert!(character_lua.contains(r#""Stormrage""#));

    let addon_lua = fs::read_to_string(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.lua"),
    )
    .expect("addon lua");
    assert!(addon_lua.contains("Examplemage - Illidan"));
    assert!(!addon_lua.contains("Targetmage - Stormrage"));
}

#[test]
fn unpack_bundle_replicates_common_wtf_to_selected_accounts() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    fs::create_dir_all(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACC_A")
            .join("SavedVariables"),
    )
    .expect("account a");
    fs::create_dir_all(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACC_B")
            .join("SavedVariables"),
    )
    .expect("account b");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings {
            selected_accounts: vec!["ACC_A".to_string(), "ACC_B".to_string()],
            ..BundleApplyMappings::default()
        },
    })
    .expect("unpack bundle");

    assert_eq!(
        result.selected_target_accounts,
        vec!["ACC_A".to_string(), "ACC_B".to_string()]
    );
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACC_A")
            .join("SavedVariables")
            .join("Details.lua")
            .exists()
    );
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACC_B")
            .join("SavedVariables")
            .join("Details.lua")
            .exists()
    );
}

#[test]
fn unpack_bundle_rolls_back_when_apply_fails() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), true);
    let bundle_path = source.path().join("bundle.zip");

    fs::write(
        source_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc"),
        "## Interface: 120000",
    )
    .expect("updated toc");
    fs::write(
        source_installation
            .addon_dir
            .join("WeakAuras")
            .join("Extra.lua"),
        "print('extra')",
    )
    .expect("extra addon file");
    fs::write(
        source_installation.wtf_dir.join("Config.wtf"),
        "SET locale zhCN",
    )
    .expect("updated config");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let original_toc = fs::read_to_string(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc"),
    )
    .expect("original toc");
    let original_config = fs::read_to_string(target_installation.wtf_dir.join("Config.wtf"))
        .expect("original config");

    let shared_xml = target_installation.interface_dir.join("SharedXML");
    fs::remove_dir_all(&shared_xml).expect("remove shared xml");
    fs::write(&shared_xml, "blocking file").expect("blocking file");

    let error = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect_err("unpack should fail");

    assert!(error.to_string().contains("rollback restored"));
    assert_eq!(
        fs::read_to_string(
            target_installation
                .addon_dir
                .join("WeakAuras")
                .join("WeakAuras.toc")
        )
        .expect("restored toc"),
        original_toc
    );
    assert_eq!(
        fs::read_to_string(target_installation.wtf_dir.join("Config.wtf"))
            .expect("restored config"),
        original_config
    );
    assert!(
        !target_installation
            .addon_dir
            .join("WeakAuras")
            .join("Extra.lua")
            .exists()
    );
    assert!(shared_xml.is_file());
    assert_eq!(
        fs::read_to_string(&shared_xml).expect("restored blocking file"),
        "blocking file"
    );
    assert!(
        !target_installation
            .interface_dir
            .join("SharedXML")
            .join("texture.blp")
            .exists()
    );
}

#[test]
fn unpack_bundle_dry_run_does_not_write_files() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: true,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("dry run");

    assert!(result.dry_run);
    assert!(result.planned_files > 0);
    assert_eq!(result.written_files, 0);
    assert!(result.backup_path.is_none());
    assert!(
        !target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
}

#[test]
fn unpack_bundle_rejects_symlink_payload_entries() {
    let temp = tempdir().expect("temp dir");
    let bundle_path = temp.path().join("symlink-payload-bundle.zip");
    let installation = create_fixture_installation(temp.path(), false);
    let manifest = toml::to_string_pretty(&sample_manifest()).expect("manifest");
    create_archive_with_raw_entries_and_symlink(
        &bundle_path,
        &[(MANIFEST_ENTRY, &manifest)],
        "addons/WeakAuras/WeakAuras.toc",
        "../outside.toc",
    );

    let error = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation,
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect_err("symlink payload should fail");

    let message = error.to_string();
    assert!(message.contains("unsupported symlink metadata"));
    assert!(message.contains("addons/WeakAuras/WeakAuras.toc"));
}

#[test]
fn unpack_bundle_rejects_non_portable_entry_paths() {
    let temp = tempdir().expect("temp dir");
    let bundle_path = temp.path().join("unsafe-path-bundle.zip");
    let installation = create_fixture_installation(temp.path(), false);
    let manifest = toml::to_string_pretty(&sample_manifest()).expect("manifest");
    create_archive_with_raw_entries(
        &bundle_path,
        &[
            (MANIFEST_ENTRY, &manifest),
            ("addons/Weak:Auras/WeakAuras.toc", "toc"),
        ],
    );

    let error = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation,
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect_err("unsafe bundle entry path should fail");

    assert!(error.to_string().contains("unsafe archive path"));
}

#[test]
fn plan_bundle_apply_rejects_case_insensitive_target_collisions_on_macos() {
    let temp = tempdir().expect("temp dir");
    let bundle_path = temp.path().join("case-collision-bundle.zip");
    let installation =
        create_fixture_installation_on_platform(temp.path(), false, HostPlatform::MacOs);
    let manifest = toml::to_string_pretty(&sample_manifest()).expect("manifest");
    create_archive_with_raw_entries(
        &bundle_path,
        &[
            (MANIFEST_ENTRY, &manifest),
            ("addons/WeakAuras/WeakAuras.toc", "toc-a"),
            ("addons/weakauras/weakauras.toc", "toc-b"),
        ],
    );

    let error = plan_bundle_apply(&bundle_path, &installation, &BundleApplyMappings::default())
        .expect_err("case-insensitive target collisions should fail on macOS");

    let message = error.to_string();
    assert!(message.contains("case-insensitive target path collisions"));
    assert!(message.contains("addons/WeakAuras/WeakAuras.toc"));
    assert!(message.contains("addons/weakauras/weakauras.toc"));
}

#[test]
fn plan_bundle_apply_allows_case_distinct_targets_on_linux() {
    let temp = tempdir().expect("temp dir");
    let bundle_path = temp.path().join("case-distinct-bundle.zip");
    let installation =
        create_fixture_installation_on_platform(temp.path(), false, HostPlatform::Linux);
    let manifest = toml::to_string_pretty(&sample_manifest()).expect("manifest");
    create_archive_with_raw_entries(
        &bundle_path,
        &[
            (MANIFEST_ENTRY, &manifest),
            ("addons/WeakAuras/WeakAuras.toc", "toc-a"),
            ("addons/weakauras/weakauras.toc", "toc-b"),
        ],
    );

    let plan = plan_bundle_apply(&bundle_path, &installation, &BundleApplyMappings::default())
        .expect("case-distinct targets should be allowed on Linux");

    assert_eq!(plan.summary.files_to_add, 2);
}

#[test]
fn preserve_policy_plans_without_writing_files() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.apply.addons = ResourceApplyPolicy::Preserve;
    manifest.apply.wtf_common = ResourceApplyPolicy::Preserve;
    manifest.apply.wtf_characters = ResourceApplyPolicy::Preserve;
    manifest.apply.fonts = ResourceApplyPolicy::Preserve;
    manifest.apply.interface_assets = ResourceApplyPolicy::Preserve;

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");
    assert!(plan.summary.files_to_preserve > 0);
    assert_eq!(plan.summary.files_to_add, 0);
    assert_eq!(plan.summary.files_to_replace, 0);
    assert_eq!(plan.summary.files_to_skip, 0);
    assert_eq!(plan.summary.files_to_preserve, plan.operations.len());
    assert!(
        plan.operations
            .iter()
            .all(|operation| operation.action == ApplyAction::Preserve)
    );

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("unpack bundle");

    assert_eq!(result.written_files, 0);
    assert_eq!(result.plan_summary.files_to_preserve, result.planned_files);
    assert!(
        !target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
    assert!(!target_installation.wtf_dir.join("Config.wtf").exists());
}

#[test]
fn share_policy_preserves_existing_target_files_and_adds_missing_files() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.apply.addons = ResourceApplyPolicy::Preserve;
    manifest.apply.wtf_common = ResourceApplyPolicy::Share;
    manifest.apply.wtf_characters = ResourceApplyPolicy::Preserve;
    manifest.apply.fonts = ResourceApplyPolicy::Preserve;
    manifest.apply.interface_assets = ResourceApplyPolicy::Preserve;

    fs::write(
        target_installation.wtf_dir.join("Config.wtf"),
        "SET locale zhCN",
    )
    .expect("existing target config");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");
    assert!(plan.operations.iter().any(|operation| {
        operation.archive_name == "wtf/common/Config.wtf"
            && operation.action == ApplyAction::Preserve
    }));
    assert!(plan.operations.iter().any(|operation| {
        operation.archive_name == "wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua"
            && operation.action == ApplyAction::Add
    }));

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("unpack bundle");

    assert_eq!(
        fs::read_to_string(target_installation.wtf_dir.join("Config.wtf")).expect("target config"),
        "SET locale zhCN"
    );
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("SavedVariables")
            .join("Details.lua")
            .exists()
    );
    assert!(result.plan_summary.files_to_preserve >= 1);
    assert!(result.written_files >= 1);
}

#[test]
fn mirror_policy_removes_existing_addon_root_before_copy() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), true);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.apply.addons = ResourceApplyPolicy::Mirror;

    fs::write(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("Stale.lua"),
        "print('stale')",
    )
    .expect("stale addon file");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");
    assert!(plan.summary.paths_to_remove >= 1);
    assert!(plan.operations.iter().any(|operation| {
        operation.action == ApplyAction::Remove
            && operation.destination == target_installation.addon_dir.join("WeakAuras")
    }));

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("unpack bundle");

    assert!(result.written_files > 0);
    assert!(
        !target_installation
            .addon_dir
            .join("WeakAuras")
            .join("Stale.lua")
            .exists()
    );
    assert!(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
}

#[test]
fn sync_policy_alias_removes_existing_addon_root_before_copy() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), true);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.apply.addons = ResourceApplyPolicy::Sync;

    fs::write(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("Stale.lua"),
        "print('stale')",
    )
    .expect("stale addon file");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");
    assert!(plan.summary.paths_to_remove >= 1);
    assert!(plan.operations.iter().any(|operation| {
        operation.action == ApplyAction::Remove
            && operation.destination == target_installation.addon_dir.join("WeakAuras")
    }));

    unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("unpack bundle");

    assert!(
        !target_installation
            .addon_dir
            .join("WeakAuras")
            .join("Stale.lua")
            .exists()
    );
    assert!(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
}
