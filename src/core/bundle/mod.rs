use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tempfile::{TempDir, tempdir};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::core::addon::lock::{
    AddonLockApplyRequest, AddonLockApplyResult, AddonLockPlanResult, apply_addon_lock_sync,
    plan_addon_lock_sync, write_addon_lock,
};
use crate::core::backup::{BackupGroup, BackupRequest, create_backup, restore_backup};
use crate::core::error::{AppError, AppResult};
use crate::core::install::{DetectedFlavorInstallation, LocalWowAccount, discover_local_accounts};
use crate::core::lua_patch::{CharacterMapping, LuaRewriteOptions, rewrite_lua_file};
use crate::core::manifest::{BundleManifest, CharacterResource};

const MANIFEST_ENTRY: &str = "manifest.toml";
const ADDON_LOCK_ENTRY: &str = "metadata/addons/lock.toml";
const ADDON_INDEX_ENTRY_ROOT: &str = "metadata/addons/indexes";

#[derive(Debug, Clone)]
pub struct PackBundleRequest {
    pub installation: DetectedFlavorInstallation,
    pub manifest: BundleManifest,
    pub output_path: Option<PathBuf>,
    pub manifest_base_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreatedBundle {
    pub archive_path: PathBuf,
    pub archived_files: usize,
    pub manifest: BundleManifest,
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleAddonLockPlan {
    pub bundle_path: PathBuf,
    pub embedded_lock_entry: String,
    pub plan: AddonLockPlanResult,
}

#[derive(Debug, Clone)]
pub struct BundleAddonLockApplyRequest {
    pub bundle_path: PathBuf,
    pub installation: DetectedFlavorInstallation,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleAddonLockApply {
    pub bundle_path: PathBuf,
    pub embedded_lock_entry: String,
    pub apply: AddonLockApplyResult,
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleInspection {
    pub archive_path: PathBuf,
    pub manifest: BundleManifest,
    pub entries: BundleEntryCounts,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct BundleEntryCounts {
    pub total_files: usize,
    pub addons: usize,
    pub wtf_common: usize,
    pub wtf_characters: usize,
    pub fonts: usize,
    pub interface_assets: usize,
    pub metadata: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleApplyPlan {
    pub bundle_path: PathBuf,
    pub target_flavor_root: PathBuf,
    pub discovered_accounts: Vec<LocalWowAccount>,
    pub selected_target_accounts: Vec<String>,
    pub character_mappings: Vec<CharacterMapping>,
    pub operations: Vec<ApplyOperation>,
    pub summary: ApplyPlanSummary,
    pub helper_strategy: HelperStrategy,
    pub group_policies: ApplyGroupPolicies,
    pub manifest: BundleManifest,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyOperation {
    pub group: ApplyGroup,
    pub action: ApplyAction,
    pub archive_name: String,
    pub destination: PathBuf,
    pub target_account: Option<String>,
    pub target_server: Option<String>,
    pub target_character: Option<String>,
    pub rewrite_count: usize,
    pub rewrite_applied: bool,
    #[serde(skip_serializing)]
    pub staged_path: PathBuf,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ApplyPlanSummary {
    pub files_to_add: usize,
    pub files_to_replace: usize,
    pub files_to_skip: usize,
    pub files_to_rewrite: usize,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyAction {
    Add,
    Replace,
    Skip,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyGroup {
    Addons,
    WtfCommon,
    WtfCharacters,
    Fonts,
    InterfaceAssets,
    Metadata,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HelperStrategy {
    NativeRust,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyGroupPolicies {
    pub addons: GroupPolicy,
    pub wtf_common: GroupPolicy,
    pub wtf_characters: GroupPolicy,
    pub fonts: GroupPolicy,
    pub interface_assets: GroupPolicy,
    pub metadata: GroupPolicy,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupPolicy {
    pub mode: &'static str,
}

#[derive(Debug, Clone)]
pub struct UnpackBundleRequest {
    pub bundle_path: PathBuf,
    pub installation: DetectedFlavorInstallation,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub apply_mappings: BundleApplyMappings,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnpackedBundle {
    pub bundle_path: PathBuf,
    pub target_flavor_root: PathBuf,
    pub dry_run: bool,
    pub planned_files: usize,
    pub written_files: usize,
    pub rewritten_files: usize,
    pub backup_path: Option<PathBuf>,
    pub selected_target_accounts: Vec<String>,
    pub plan_summary: ApplyPlanSummary,
    pub character_mappings: Vec<CharacterMapping>,
    pub manifest: BundleManifest,
}

#[derive(Debug, Clone)]
struct PlannedEntry {
    archive_name: String,
    destination: PathBuf,
    rewrites: Vec<CharacterMapping>,
    group: ApplyGroup,
    target_account: Option<String>,
    target_server: Option<String>,
    target_character: Option<String>,
    staged_path: PathBuf,
}

struct PreparedBundleApply {
    plan: BundleApplyPlan,
    _stage_dir: TempDir,
}

struct ExtractedAddonLock {
    lock_path: PathBuf,
    _stage_dir: TempDir,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BundleApplyMappings {
    pub target_account: Option<String>,
    pub target_server: Option<String>,
    pub target_character: Option<String>,
    #[serde(default)]
    pub selected_accounts: Vec<String>,
    #[serde(default)]
    pub all_accounts: bool,
    #[serde(default)]
    pub characters: Vec<CharacterMappingOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterMappingOverride {
    pub source_account: Option<String>,
    pub source_server: String,
    pub source_character: String,
    pub target_account: Option<String>,
    pub target_server: String,
    pub target_character: String,
}

pub fn pack_bundle(mut request: PackBundleRequest) -> AppResult<CreatedBundle> {
    request.manifest.validate()?;

    if request.manifest.source.flavor != request.installation.flavor {
        return Err(AppError::Validation(format!(
            "manifest source flavor `{}` does not match installation flavor `{}`",
            request.manifest.source.flavor.as_str(),
            request.installation.flavor.as_str()
        )));
    }

    let timestamp = now_rfc3339()?;
    request.manifest.source.exported_at = Some(timestamp.clone());
    request.manifest.source.platform = Some(request.installation.platform);

    let archive_path = resolve_bundle_output_path(
        request.output_path.as_deref(),
        &request.manifest,
        &timestamp,
    )?;
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = File::create(&archive_path)?;
    let mut zip = ZipWriter::new(file);
    let mut archived_files = 0usize;

    for addon in &request.manifest.resources.addons {
        validate_plain_name("addon", addon)?;
        let source = request.installation.addon_dir.join(addon);
        if !source.exists() {
            return Err(AppError::NotFound(format!(
                "addon does not exist: {}",
                source.display()
            )));
        }
        archived_files += add_path_to_zip(&mut zip, &source, &Path::new("addons").join(addon))?;
    }

    if request.manifest.resources.addon_lock {
        let lock_result = write_addon_lock(&request.installation)?;
        if lock_result.removed {
            return Err(AppError::Validation(
                "cannot embed addon lock because no tracked addon packages were found".to_string(),
            ));
        }
        archived_files += add_path_to_zip(
            &mut zip,
            &lock_result.lock_path,
            Path::new(ADDON_LOCK_ENTRY),
        )?;
    }

    let addon_index_paths = resolve_addon_index_paths(
        &request.manifest.resources.addon_indexes,
        request.manifest_base_dir.as_deref(),
    )?;
    for (file_name, source_path) in addon_index_paths {
        archived_files += add_path_to_zip(
            &mut zip,
            &source_path,
            &Path::new(ADDON_INDEX_ENTRY_ROOT).join(file_name),
        )?;
    }

    if request.manifest.resources.wtf_common {
        archived_files += add_common_wtf_to_zip(&mut zip, &request.installation.wtf_dir)?;
    }

    for character in &mut request.manifest.resources.wtf_characters {
        let resolved_account = resolve_character_account(&request.installation.wtf_dir, character)?;
        character.source_account = Some(resolved_account.clone());
        archived_files += add_character_wtf_to_zip(
            &mut zip,
            &request.installation.wtf_dir,
            character,
            &resolved_account,
        )?;
    }

    zip.start_file(MANIFEST_ENTRY, zip_file_options())?;
    zip.write_all(toml::to_string_pretty(&request.manifest)?.as_bytes())?;
    archived_files += 1;

    if request.manifest.resources.fonts {
        archived_files += add_path_to_zip(
            &mut zip,
            &request.installation.fonts_dir,
            Path::new("fonts"),
        )?;
    }

    for asset in &request.manifest.resources.interface_assets {
        validate_plain_name("interface asset", asset)?;
        let source = request.installation.interface_dir.join(asset);
        if !source.exists() {
            return Err(AppError::NotFound(format!(
                "interface asset does not exist: {}",
                source.display()
            )));
        }
        archived_files += add_path_to_zip(&mut zip, &source, &Path::new("interface").join(asset))?;
    }

    zip.finish()?;

    Ok(CreatedBundle {
        archive_path,
        archived_files,
        manifest: request.manifest,
    })
}

pub fn inspect_bundle(path: &Path) -> AppResult<BundleInspection> {
    let archive_path = path.to_path_buf();
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let manifest = read_manifest_from_archive(&mut archive)?;
    manifest.validate()?;
    let entries = count_bundle_entries(&mut archive)?;

    Ok(BundleInspection {
        archive_path,
        manifest,
        entries,
    })
}

pub fn load_apply_mappings(path: &Path) -> AppResult<BundleApplyMappings> {
    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

pub fn plan_bundle_apply(
    bundle_path: &Path,
    installation: &DetectedFlavorInstallation,
    apply_mappings: &BundleApplyMappings,
) -> AppResult<BundleApplyPlan> {
    Ok(prepare_bundle_apply(bundle_path, installation, apply_mappings)?.plan)
}

pub fn plan_bundle_addon_lock(
    bundle_path: &Path,
    installation: &DetectedFlavorInstallation,
) -> AppResult<BundleAddonLockPlan> {
    let extracted = extract_embedded_addon_lock(bundle_path)?;
    let mut plan = plan_addon_lock_sync(installation, Some(&extracted.lock_path))?;
    plan.lock_path = PathBuf::from(ADDON_LOCK_ENTRY);

    Ok(BundleAddonLockPlan {
        bundle_path: bundle_path.to_path_buf(),
        embedded_lock_entry: ADDON_LOCK_ENTRY.to_string(),
        plan,
    })
}

pub fn apply_bundle_addon_lock(
    request: BundleAddonLockApplyRequest,
) -> AppResult<BundleAddonLockApply> {
    let extracted = extract_embedded_addon_lock(&request.bundle_path)?;
    let mut apply = apply_addon_lock_sync(AddonLockApplyRequest {
        installation: request.installation,
        lock_path: Some(extracted.lock_path.clone()),
        backup_output_path: request.backup_output_path,
        replace_existing: request.replace_existing,
    })?;
    apply.lock_path = PathBuf::from(ADDON_LOCK_ENTRY);
    apply.verification.lock_path = PathBuf::from(ADDON_LOCK_ENTRY);

    Ok(BundleAddonLockApply {
        bundle_path: request.bundle_path,
        embedded_lock_entry: ADDON_LOCK_ENTRY.to_string(),
        apply,
    })
}

fn prepare_bundle_apply(
    bundle_path: &Path,
    installation: &DetectedFlavorInstallation,
    apply_mappings: &BundleApplyMappings,
) -> AppResult<PreparedBundleApply> {
    let inspection = inspect_bundle(bundle_path)?;
    validate_target_compatibility(&inspection.manifest, installation)?;
    let discovered_accounts = discover_local_accounts(installation)?;
    let character_mappings = build_character_mappings(&inspection.manifest, apply_mappings)?;
    let selected_target_accounts = resolve_selected_target_accounts(
        &inspection.manifest,
        &discovered_accounts,
        &character_mappings,
        apply_mappings,
    )?;
    let stage_dir = tempdir()?;
    extract_bundle_to_stage(bundle_path, stage_dir.path())?;
    let mut planned_entries = plan_extractable_entries(
        bundle_path,
        stage_dir.path(),
        installation,
        &inspection.manifest,
        &character_mappings,
        apply_mappings,
        &selected_target_accounts,
    )?;
    prepare_operation_stage_files(&mut planned_entries, stage_dir.path())?;
    let rewrite_options = LuaRewriteOptions {
        rewrite_profile_keys: inspection.manifest.mapping.rewrite_profile_keys,
        rewrite_identity_strings: inspection.manifest.mapping.rewrite_identity_strings,
    };

    let mut operations = Vec::new();
    let mut summary = ApplyPlanSummary::default();

    for entry in &mut planned_entries {
        let rewrite_applied =
            rewrite_lua_file(&entry.staged_path, &entry.rewrites, rewrite_options)?;
        let action = if !entry.destination.exists() {
            summary.files_to_add += 1;
            ApplyAction::Add
        } else if file_contents_equal(&entry.staged_path, &entry.destination)? {
            summary.files_to_skip += 1;
            ApplyAction::Skip
        } else {
            summary.files_to_replace += 1;
            ApplyAction::Replace
        };
        if rewrite_applied {
            summary.files_to_rewrite += 1;
        }

        operations.push(ApplyOperation {
            group: entry.group,
            action,
            archive_name: entry.archive_name.clone(),
            destination: entry.destination.clone(),
            target_account: entry.target_account.clone(),
            target_server: entry.target_server.clone(),
            target_character: entry.target_character.clone(),
            rewrite_count: entry.rewrites.len(),
            rewrite_applied,
            staged_path: entry.staged_path.clone(),
        });
    }

    Ok(PreparedBundleApply {
        plan: BundleApplyPlan {
            bundle_path: bundle_path.to_path_buf(),
            target_flavor_root: installation.flavor_root.clone(),
            discovered_accounts,
            selected_target_accounts,
            character_mappings,
            operations,
            summary,
            helper_strategy: HelperStrategy::NativeRust,
            group_policies: ApplyGroupPolicies {
                addons: GroupPolicy { mode: "merge_copy" },
                wtf_common: GroupPolicy {
                    mode: "selected_accounts_merge",
                },
                wtf_characters: GroupPolicy {
                    mode: "explicit_mapping_merge",
                },
                fonts: GroupPolicy { mode: "merge_copy" },
                interface_assets: GroupPolicy { mode: "merge_copy" },
                metadata: GroupPolicy {
                    mode: "bundle_sidecar",
                },
            },
            manifest: inspection.manifest,
        },
        _stage_dir: stage_dir,
    })
}

pub fn unpack_bundle(request: UnpackBundleRequest) -> AppResult<UnpackedBundle> {
    let prepared = prepare_bundle_apply(
        &request.bundle_path,
        &request.installation,
        &request.apply_mappings,
    )?;
    let plan = prepared.plan.clone();
    if request.dry_run {
        return Ok(UnpackedBundle {
            bundle_path: request.bundle_path,
            target_flavor_root: request.installation.flavor_root,
            dry_run: true,
            planned_files: plan.operations.len(),
            written_files: 0,
            rewritten_files: 0,
            backup_path: None,
            selected_target_accounts: plan.selected_target_accounts,
            plan_summary: plan.summary,
            character_mappings: plan.character_mappings,
            manifest: plan.manifest,
        });
    }

    let backup_path = if plan.manifest.apply.create_backup {
        let groups = backup_groups_for_manifest(&plan.manifest);
        if groups.is_empty() {
            None
        } else {
            Some(
                create_backup(BackupRequest {
                    installation: request.installation.clone(),
                    output_path: request.backup_output_path,
                    groups,
                    label: Some("bundle-apply".to_string()),
                })?
                .archive_path,
            )
        }
    } else {
        None
    };

    let execution = execute_apply_operations(&plan);
    let (written_files, rewritten_files) = match execution {
        Ok(result) => result,
        Err(error) => {
            return rollback_or_report_apply_error(
                error,
                backup_path.as_deref(),
                &request.installation,
            );
        }
    };

    Ok(UnpackedBundle {
        bundle_path: request.bundle_path,
        target_flavor_root: request.installation.flavor_root,
        dry_run: false,
        planned_files: plan.operations.len(),
        written_files,
        rewritten_files,
        backup_path,
        selected_target_accounts: plan.selected_target_accounts,
        plan_summary: plan.summary,
        character_mappings: plan.character_mappings,
        manifest: plan.manifest,
    })
}

fn extract_bundle_to_stage(bundle_path: &Path, stage_root: &Path) -> AppResult<()> {
    let file = File::open(bundle_path)?;
    let mut archive = ZipArchive::new(file)?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }

        let entry_name = entry.name().to_string();
        let segments = safe_zip_segments(&entry_name)?;
        if segments.is_empty() {
            continue;
        }

        let staged_path = join_segments(stage_root, &segments);
        if let Some(parent) = staged_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = File::create(staged_path)?;
        std::io::copy(&mut entry, &mut output)?;
    }

    Ok(())
}

fn extract_embedded_addon_lock(bundle_path: &Path) -> AppResult<ExtractedAddonLock> {
    let file = File::open(bundle_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut lock_entry = archive.by_name(ADDON_LOCK_ENTRY).map_err(|_| {
        AppError::NotFound(format!(
            "bundle does not contain embedded addon lock `{ADDON_LOCK_ENTRY}`"
        ))
    })?;
    let stage_dir = tempdir()?;
    let lock_path = stage_dir.path().join("lock.toml");
    let mut output = File::create(&lock_path)?;
    std::io::copy(&mut lock_entry, &mut output)?;

    Ok(ExtractedAddonLock {
        lock_path,
        _stage_dir: stage_dir,
    })
}

fn prepare_operation_stage_files(
    planned_entries: &mut [PlannedEntry],
    stage_root: &Path,
) -> AppResult<()> {
    let operation_root = stage_root.join("__operations");
    fs::create_dir_all(&operation_root)?;

    for (index, entry) in planned_entries.iter_mut().enumerate() {
        let file_name = entry
            .staged_path
            .file_name()
            .map(|name| name.to_owned())
            .unwrap_or_else(|| format!("entry-{index}").into());
        let operation_path = operation_root.join(index.to_string()).join(file_name);
        if let Some(parent) = operation_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&entry.staged_path, &operation_path)?;
        entry.staged_path = operation_path;
    }

    Ok(())
}

fn execute_apply_operations(plan: &BundleApplyPlan) -> AppResult<(usize, usize)> {
    let mut written_files = 0usize;
    let mut rewritten_files = 0usize;

    for operation in &plan.operations {
        if operation.action == ApplyAction::Skip {
            continue;
        }

        if let Some(parent) = operation.destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&operation.staged_path, &operation.destination)?;
        written_files += 1;

        if operation.rewrite_applied {
            rewritten_files += 1;
        }
    }

    Ok((written_files, rewritten_files))
}

fn rollback_or_report_apply_error<T>(
    error: AppError,
    backup_path: Option<&Path>,
    installation: &DetectedFlavorInstallation,
) -> AppResult<T> {
    let Some(backup_path) = backup_path else {
        return Err(error);
    };

    match restore_backup(backup_path, installation) {
        Ok(restored) => Err(AppError::Validation(format!(
            "bundle apply failed and rollback restored `{}` ({} files): {error}",
            restored.archive_path.display(),
            restored.restored_files
        ))),
        Err(rollback_error) => Err(AppError::Validation(format!(
            "bundle apply failed: {error}; rollback failed: {rollback_error}"
        ))),
    }
}

fn file_contents_equal(left: &Path, right: &Path) -> AppResult<bool> {
    if !right.exists() || !left.is_file() || !right.is_file() {
        return Ok(false);
    }

    let left_metadata = fs::metadata(left)?;
    let right_metadata = fs::metadata(right)?;
    if left_metadata.len() != right_metadata.len() {
        return Ok(false);
    }

    let mut left_file = File::open(left)?;
    let mut right_file = File::open(right)?;
    let mut left_buffer = [0u8; 8192];
    let mut right_buffer = [0u8; 8192];

    loop {
        let left_read = left_file.read(&mut left_buffer)?;
        let right_read = right_file.read(&mut right_buffer)?;
        if left_read != right_read {
            return Ok(false);
        }
        if left_read == 0 {
            return Ok(true);
        }
        if left_buffer[..left_read] != right_buffer[..right_read] {
            return Ok(false);
        }
    }
}

fn add_common_wtf_to_zip(zip: &mut ZipWriter<File>, wtf_dir: &Path) -> AppResult<usize> {
    let mut archived_files = 0usize;
    let config_wtf = wtf_dir.join("Config.wtf");
    if config_wtf.exists() {
        archived_files += add_path_to_zip(zip, &config_wtf, Path::new("wtf/common/Config.wtf"))?;
    }

    let account_root = wtf_dir.join("Account");
    if !account_root.exists() {
        return Ok(archived_files);
    }

    for entry in fs::read_dir(account_root)? {
        let entry = entry?;
        let account_dir = entry.path();
        if !account_dir.is_dir() {
            continue;
        }

        let account_name = entry.file_name().to_string_lossy().to_string();
        validate_plain_name("account", &account_name)?;
        let saved_variables = account_dir.join("SavedVariables");
        if saved_variables.exists() {
            archived_files += add_path_to_zip(
                zip,
                &saved_variables,
                &Path::new("wtf/common/accounts")
                    .join(account_name)
                    .join("SavedVariables"),
            )?;
        }
    }

    Ok(archived_files)
}

fn resolve_addon_index_paths(
    addon_indexes: &[String],
    manifest_base_dir: Option<&Path>,
) -> AppResult<Vec<(String, PathBuf)>> {
    let mut resolved = Vec::new();
    let mut file_names = Vec::new();

    for addon_index in addon_indexes {
        let reference = Path::new(addon_index);
        let source_path = if reference.is_absolute() {
            reference.to_path_buf()
        } else if let Some(base_dir) = manifest_base_dir {
            base_dir.join(reference)
        } else {
            std::env::current_dir()?.join(reference)
        };

        if !source_path.is_file() {
            return Err(AppError::NotFound(format!(
                "addon index file does not exist: {}",
                source_path.display()
            )));
        }

        let file_name = source_path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "addon index file has no usable file name: {}",
                    source_path.display()
                ))
            })?
            .to_string();
        validate_plain_name("addon index file", &file_name)?;
        if file_names.iter().any(|item| item == &file_name) {
            return Err(AppError::Validation(format!(
                "duplicate addon index file name in bundle metadata: {file_name}"
            )));
        }
        file_names.push(file_name.clone());
        resolved.push((file_name, source_path));
    }

    Ok(resolved)
}

fn add_character_wtf_to_zip(
    zip: &mut ZipWriter<File>,
    wtf_dir: &Path,
    character: &CharacterResource,
    account: &str,
) -> AppResult<usize> {
    validate_plain_name("server", &character.source_server)?;
    validate_plain_name("character", &character.source_character)?;
    validate_plain_name("account", account)?;
    let character_dir = wtf_dir
        .join("Account")
        .join(account)
        .join(&character.source_server)
        .join(&character.source_character);

    if !character_dir.exists() {
        return Err(AppError::NotFound(format!(
            "character WTF directory does not exist: {}",
            character_dir.display()
        )));
    }

    add_path_to_zip(
        zip,
        &character_dir,
        &Path::new("wtf/characters")
            .join(account)
            .join(&character.source_server)
            .join(&character.source_character),
    )
}

fn resolve_character_account(wtf_dir: &Path, character: &CharacterResource) -> AppResult<String> {
    if let Some(account) = &character.source_account {
        validate_plain_name("account", account)?;
        return Ok(account.clone());
    }

    let mut matches = Vec::new();
    let account_root = wtf_dir.join("Account");
    if !account_root.exists() {
        return Err(AppError::NotFound(format!(
            "account root does not exist: {}",
            account_root.display()
        )));
    }

    for entry in fs::read_dir(account_root)? {
        let entry = entry?;
        let account_dir = entry.path();
        if !account_dir.is_dir() {
            continue;
        }

        let candidate = account_dir
            .join(&character.source_server)
            .join(&character.source_character);
        if candidate.exists() {
            matches.push(entry.file_name().to_string_lossy().to_string());
        }
    }

    match matches.as_slice() {
        [account] => Ok(account.clone()),
        [] => Err(AppError::NotFound(format!(
            "no account contains character `{}` on server `{}`",
            character.source_character, character.source_server
        ))),
        many => Err(AppError::Validation(format!(
            "character `{}` on server `{}` exists in multiple accounts: {:?}. Set source_account explicitly.",
            character.source_character, character.source_server, many
        ))),
    }
}

fn add_path_to_zip(
    zip: &mut ZipWriter<File>,
    source_path: &Path,
    archive_path: &Path,
) -> AppResult<usize> {
    if !source_path.exists() {
        return Ok(0);
    }

    if source_path.is_file() {
        write_file_to_zip(zip, source_path, archive_path)?;
        return Ok(1);
    }

    let mut archived_files = 0usize;
    for entry in WalkDir::new(source_path).follow_links(false) {
        let entry = entry.map_err(|error| AppError::Validation(error.to_string()))?;
        let path = entry.path();
        let relative = path
            .strip_prefix(source_path)
            .map_err(|error| AppError::Validation(error.to_string()))?;

        if relative.as_os_str().is_empty() || should_skip_path(relative) {
            continue;
        }

        let target_path = archive_path.join(relative);
        if entry.file_type().is_dir() {
            zip.add_directory(to_zip_path(&target_path), zip_dir_options())?;
            continue;
        }

        write_file_to_zip(zip, path, &target_path)?;
        archived_files += 1;
    }

    Ok(archived_files)
}

fn write_file_to_zip(
    zip: &mut ZipWriter<File>,
    source_path: &Path,
    archive_path: &Path,
) -> AppResult<()> {
    let mut file = File::open(source_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    zip.start_file(to_zip_path(archive_path), zip_file_options())?;
    zip.write_all(&buffer)?;
    Ok(())
}

fn read_manifest_from_archive(archive: &mut ZipArchive<File>) -> AppResult<BundleManifest> {
    let mut manifest_file = archive.by_name(MANIFEST_ENTRY)?;
    let mut content = String::new();
    manifest_file.read_to_string(&mut content)?;
    Ok(toml::from_str(&content)?)
}

fn count_bundle_entries(archive: &mut ZipArchive<File>) -> AppResult<BundleEntryCounts> {
    let mut counts = BundleEntryCounts::default();

    for index in 0..archive.len() {
        let file = archive.by_index(index)?;
        if file.is_dir() {
            continue;
        }

        counts.total_files += 1;
        let name = file.name();
        if name == MANIFEST_ENTRY || name.starts_with("metadata/") {
            counts.metadata += 1;
        } else if name.starts_with("addons/") {
            counts.addons += 1;
        } else if name.starts_with("wtf/common/") {
            counts.wtf_common += 1;
        } else if name.starts_with("wtf/characters/") {
            counts.wtf_characters += 1;
        } else if name.starts_with("fonts/") {
            counts.fonts += 1;
        } else if name.starts_with("interface/") {
            counts.interface_assets += 1;
        }
    }

    Ok(counts)
}

fn plan_extractable_entries(
    bundle_path: &Path,
    stage_root: &Path,
    installation: &DetectedFlavorInstallation,
    manifest: &BundleManifest,
    character_mappings: &[CharacterMapping],
    apply_mappings: &BundleApplyMappings,
    selected_target_accounts: &[String],
) -> AppResult<Vec<PlannedEntry>> {
    let file = File::open(bundle_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut planned_entries = Vec::new();
    let common_account_targets = resolve_common_account_targets(
        manifest,
        character_mappings,
        apply_mappings,
        selected_target_accounts,
    )?;

    for index in 0..archive.len() {
        let file = archive.by_index(index)?;
        if file.is_dir() {
            continue;
        }

        let archive_name = file.name().to_string();
        let entries = map_bundle_entry_to_destination(
            &archive_name,
            installation,
            manifest,
            character_mappings,
            &common_account_targets,
            apply_mappings.target_account.as_deref(),
            selected_target_accounts,
            stage_root,
        )?;

        planned_entries.extend(entries);
    }

    Ok(planned_entries)
}

fn map_bundle_entry_to_destination(
    archive_name: &str,
    installation: &DetectedFlavorInstallation,
    manifest: &BundleManifest,
    character_mappings: &[CharacterMapping],
    common_account_targets: &BTreeMap<String, String>,
    default_target_account: Option<&str>,
    selected_target_accounts: &[String],
    stage_root: &Path,
) -> AppResult<Vec<PlannedEntry>> {
    if archive_name == MANIFEST_ENTRY {
        return Ok(Vec::new());
    }

    let segments = safe_zip_segments(archive_name)?;
    if segments.is_empty() {
        return Ok(Vec::new());
    }
    let staged_path = join_segments(stage_root, &segments);

    match segments.as_slice() {
        ["metadata", rest @ ..] if !rest.is_empty() => Ok(vec![PlannedEntry {
            archive_name: archive_name.to_string(),
            destination: join_segments(
                &installation
                    .addon_dir
                    .join(".hearthsync")
                    .join("bundles")
                    .join(safe_file_part(&manifest.package.id)),
                rest,
            ),
            rewrites: Vec::new(),
            group: ApplyGroup::Metadata,
            target_account: None,
            target_server: None,
            target_character: None,
            staged_path,
        }]),
        ["addons", rest @ ..] if !rest.is_empty() => Ok(vec![PlannedEntry {
            archive_name: archive_name.to_string(),
            destination: join_segments(&installation.addon_dir, rest),
            rewrites: Vec::new(),
            group: ApplyGroup::Addons,
            target_account: None,
            target_server: None,
            target_character: None,
            staged_path,
        }]),
        ["wtf", "common", "Config.wtf"] => Ok(vec![PlannedEntry {
            archive_name: archive_name.to_string(),
            destination: installation.wtf_dir.join("Config.wtf"),
            rewrites: Vec::new(),
            group: ApplyGroup::WtfCommon,
            target_account: None,
            target_server: None,
            target_character: None,
            staged_path,
        }]),
        [
            "wtf",
            "common",
            "accounts",
            source_account,
            "SavedVariables",
            rest @ ..,
        ] if !rest.is_empty() => {
            let target_accounts = if !selected_target_accounts.is_empty() {
                selected_target_accounts.to_vec()
            } else {
                vec![
                    common_account_targets
                        .get(*source_account)
                        .cloned()
                        .or_else(|| default_target_account.map(|item| item.to_string()))
                        .unwrap_or_else(|| (*source_account).to_string()),
                ]
            };

            Ok(target_accounts
                .into_iter()
                .map(|target_account| PlannedEntry {
                    archive_name: archive_name.to_string(),
                    destination: installation
                        .wtf_dir
                        .join("Account")
                        .join(&target_account)
                        .join("SavedVariables")
                        .join(join_segments(Path::new(""), rest)),
                    rewrites: character_mappings
                        .iter()
                        .filter(|mapping| mapping.target_account == target_account)
                        .cloned()
                        .collect::<Vec<_>>(),
                    group: ApplyGroup::WtfCommon,
                    target_account: Some(target_account),
                    target_server: None,
                    target_character: None,
                    staged_path: staged_path.clone(),
                })
                .collect())
        }
        [
            "wtf",
            "characters",
            source_account,
            server,
            character,
            rest @ ..,
        ] if !rest.is_empty() => {
            let mapping =
                find_character_mapping(character_mappings, source_account, server, character)
                    .cloned()
                    .unwrap_or_else(|| CharacterMapping {
                        source_account: Some((*source_account).to_string()),
                        source_server: (*server).to_string(),
                        source_character: (*character).to_string(),
                        target_account: (*source_account).to_string(),
                        target_server: (*server).to_string(),
                        target_character: (*character).to_string(),
                    });

            Ok(vec![PlannedEntry {
                archive_name: archive_name.to_string(),
                destination: installation
                    .wtf_dir
                    .join("Account")
                    .join(&mapping.target_account)
                    .join(&mapping.target_server)
                    .join(&mapping.target_character)
                    .join(join_segments(Path::new(""), rest)),
                rewrites: vec![mapping.clone()],
                group: ApplyGroup::WtfCharacters,
                target_account: Some(mapping.target_account),
                target_server: Some(mapping.target_server),
                target_character: Some(mapping.target_character),
                staged_path,
            }])
        }
        ["fonts", rest @ ..] if !rest.is_empty() => Ok(vec![PlannedEntry {
            archive_name: archive_name.to_string(),
            destination: join_segments(&installation.fonts_dir, rest),
            rewrites: Vec::new(),
            group: ApplyGroup::Fonts,
            target_account: None,
            target_server: None,
            target_character: None,
            staged_path,
        }]),
        ["interface", rest @ ..] if !rest.is_empty() => Ok(vec![PlannedEntry {
            archive_name: archive_name.to_string(),
            destination: join_segments(&installation.interface_dir, rest),
            rewrites: Vec::new(),
            group: ApplyGroup::InterfaceAssets,
            target_account: None,
            target_server: None,
            target_character: None,
            staged_path,
        }]),
        _ => Ok(Vec::new()),
    }
}

fn build_character_mappings(
    manifest: &BundleManifest,
    apply_mappings: &BundleApplyMappings,
) -> AppResult<Vec<CharacterMapping>> {
    let single_character_bundle = manifest.resources.wtf_characters.len() == 1;
    let mut mappings = Vec::new();

    for resource in &manifest.resources.wtf_characters {
        let source_account = resource.source_account.clone();
        let override_mapping = resolve_mapping_override(resource, &apply_mappings.characters)?;
        let target_account = override_mapping
            .and_then(|item| item.target_account.clone())
            .or_else(|| apply_mappings.target_account.clone())
            .or_else(|| source_account.clone())
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "target account is required for `{}/{}`
because the source account is unknown",
                    resource.source_server, resource.source_character
                ))
            })?;

        let target_server = override_mapping
            .map(|item| item.target_server.clone())
            .or_else(|| {
                if single_character_bundle {
                    apply_mappings.target_server.clone()
                } else {
                    None
                }
            })
            .unwrap_or_else(|| resource.source_server.clone());

        let target_character = override_mapping
            .map(|item| item.target_character.clone())
            .or_else(|| {
                if single_character_bundle {
                    apply_mappings.target_character.clone()
                } else {
                    None
                }
            })
            .unwrap_or_else(|| resource.source_character.clone());

        validate_plain_name("target account", &target_account)?;
        validate_plain_name("target server", &target_server)?;
        validate_plain_name("target character", &target_character)?;

        mappings.push(CharacterMapping {
            source_account,
            source_server: resource.source_server.clone(),
            source_character: resource.source_character.clone(),
            target_account,
            target_server,
            target_character,
        });
    }

    Ok(mappings)
}

fn resolve_selected_target_accounts(
    manifest: &BundleManifest,
    discovered_accounts: &[LocalWowAccount],
    character_mappings: &[CharacterMapping],
    apply_mappings: &BundleApplyMappings,
) -> AppResult<Vec<String>> {
    if !manifest.resources.wtf_common {
        return Ok(Vec::new());
    }

    if apply_mappings.all_accounts {
        return Ok(discovered_accounts
            .iter()
            .map(|account| account.account_name.clone())
            .collect());
    }

    if !apply_mappings.selected_accounts.is_empty() {
        let mut selected = apply_mappings.selected_accounts.clone();
        selected.sort();
        selected.dedup();
        for account in &selected {
            validate_plain_name("selected account", account)?;
        }
        return Ok(selected);
    }

    if let Some(target_account) = &apply_mappings.target_account {
        validate_plain_name("target account", target_account)?;
        return Ok(vec![target_account.clone()]);
    }

    if discovered_accounts.len() == 1 {
        return Ok(vec![discovered_accounts[0].account_name.clone()]);
    }

    let mut mapped_accounts = character_mappings
        .iter()
        .map(|mapping| mapping.target_account.clone())
        .collect::<Vec<_>>();
    mapped_accounts.sort();
    mapped_accounts.dedup();

    if mapped_accounts.len() == 1 {
        return Ok(mapped_accounts);
    }

    if discovered_accounts.is_empty() {
        let mut source_accounts = manifest
            .resources
            .wtf_characters
            .iter()
            .filter_map(|character| character.source_account.clone())
            .collect::<Vec<_>>();
        source_accounts.sort();
        source_accounts.dedup();
        if source_accounts.len() == 1 {
            return Ok(source_accounts);
        }
        return Ok(Vec::new());
    }

    Err(AppError::Validation(
        "common WTF resources require explicit target account selection. Use `--select-account`, `--all-accounts`, or `--target-account`.".to_string(),
    ))
}

fn resolve_mapping_override<'a>(
    resource: &CharacterResource,
    overrides: &'a [CharacterMappingOverride],
) -> AppResult<Option<&'a CharacterMappingOverride>> {
    let mut matches = overrides
        .iter()
        .filter(|item| {
            item.source_server == resource.source_server
                && item.source_character == resource.source_character
                && match (&resource.source_account, &item.source_account) {
                    (Some(resource_account), Some(mapping_account)) => {
                        resource_account == mapping_account
                    }
                    (Some(_), None) => true,
                    (None, Some(_)) => false,
                    (None, None) => true,
                }
        })
        .collect::<Vec<_>>();

    match matches.len() {
        0 => Ok(None),
        1 => Ok(matches.pop()),
        _ => Err(AppError::Validation(format!(
            "multiple mapping overrides matched `{}/{}`",
            resource.source_server, resource.source_character
        ))),
    }
}

fn resolve_common_account_targets(
    manifest: &BundleManifest,
    character_mappings: &[CharacterMapping],
    apply_mappings: &BundleApplyMappings,
    selected_target_accounts: &[String],
) -> AppResult<BTreeMap<String, String>> {
    let mut targets = BTreeMap::new();

    if !manifest.resources.wtf_common {
        return Ok(targets);
    }

    if !selected_target_accounts.is_empty() {
        return Ok(targets);
    }

    for mapping in character_mappings {
        let Some(source_account) = &mapping.source_account else {
            continue;
        };

        match targets.get(source_account) {
            Some(existing) if existing != &mapping.target_account => {
                return Err(AppError::Validation(format!(
                    "source account `{source_account}` maps to multiple target accounts (`{existing}` and `{}`), which is unsafe for common WTF resources",
                    mapping.target_account
                )));
            }
            Some(_) => {}
            None => {
                targets.insert(source_account.clone(), mapping.target_account.clone());
            }
        }
    }

    if let Some(default_target_account) = &apply_mappings.target_account {
        validate_plain_name("target account", default_target_account)?;
        for override_mapping in &apply_mappings.characters {
            if let Some(source_account) = &override_mapping.source_account {
                let target_account = override_mapping
                    .target_account
                    .clone()
                    .unwrap_or_else(|| default_target_account.clone());
                match targets.get(source_account) {
                    Some(existing) if existing != &target_account => {
                        return Err(AppError::Validation(format!(
                            "source account `{source_account}` maps to multiple target accounts (`{existing}` and `{target_account}`)"
                        )));
                    }
                    Some(_) => {}
                    None => {
                        targets.insert(source_account.clone(), target_account);
                    }
                }
            }
        }
    }

    Ok(targets)
}

fn find_character_mapping<'a>(
    mappings: &'a [CharacterMapping],
    source_account: &str,
    source_server: &str,
    source_character: &str,
) -> Option<&'a CharacterMapping> {
    mappings.iter().find(|mapping| {
        mapping.source_account.as_deref() == Some(source_account)
            && mapping.source_server == source_server
            && mapping.source_character == source_character
    })
}

fn validate_target_compatibility(
    manifest: &BundleManifest,
    installation: &DetectedFlavorInstallation,
) -> AppResult<()> {
    if !manifest.source.supported_targets.is_empty()
        && !manifest
            .source
            .supported_targets
            .contains(&installation.flavor)
    {
        return Err(AppError::Validation(format!(
            "bundle does not support target flavor `{}`",
            installation.flavor.as_str()
        )));
    }

    if let Some(source_platform) = manifest.source.platform {
        if source_platform != installation.platform && !manifest.mapping.allow_cross_platform {
            return Err(AppError::Validation(
                "bundle was exported on another platform, but allow_cross_platform is false"
                    .to_string(),
            ));
        }
    }

    Ok(())
}

fn backup_groups_for_manifest(manifest: &BundleManifest) -> Vec<BackupGroup> {
    let mut groups = Vec::new();

    if !manifest.resources.addons.is_empty()
        || manifest.resources.addon_lock
        || !manifest.resources.addon_indexes.is_empty()
    {
        groups.push(BackupGroup::Addons);
    }
    if manifest.resources.wtf_common || !manifest.resources.wtf_characters.is_empty() {
        groups.push(BackupGroup::Wtf);
    }
    if manifest.resources.fonts {
        groups.push(BackupGroup::Fonts);
    }
    if !manifest.resources.interface_assets.is_empty() {
        groups.push(BackupGroup::InterfaceAssets);
    }

    groups
}

fn resolve_bundle_output_path(
    output_path: Option<&Path>,
    manifest: &BundleManifest,
    timestamp: &str,
) -> AppResult<PathBuf> {
    let file_name = format!(
        "bundle-{}-{}.zip",
        safe_file_part(&manifest.package.id),
        compact_timestamp(timestamp)
    );

    match output_path {
        Some(path) if path.extension().is_some_and(|extension| extension == "zip") => {
            Ok(path.to_path_buf())
        }
        Some(path) => Ok(path.join(file_name)),
        None => Ok(std::env::current_dir()?.join(file_name)),
    }
}

fn validate_plain_name(kind: &str, value: &str) -> AppResult<()> {
    if value.trim().is_empty()
        || value.contains('/')
        || value.contains('\\')
        || value == "."
        || value == ".."
    {
        return Err(AppError::Validation(format!(
            "invalid {kind} name: `{value}`"
        )));
    }

    Ok(())
}

fn safe_zip_segments(archive_name: &str) -> AppResult<Vec<&str>> {
    let mut segments = Vec::new();
    for segment in archive_name.split('/') {
        if segment.is_empty() {
            continue;
        }

        if segment == "." || segment == ".." || segment.contains('\\') {
            return Err(AppError::Validation(format!(
                "unsafe archive path: `{archive_name}`"
            )));
        }

        segments.push(segment);
    }

    Ok(segments)
}

fn join_segments(root: &Path, segments: &[&str]) -> PathBuf {
    let mut path = root.to_path_buf();
    for segment in segments {
        path.push(segment);
    }
    path
}

fn should_skip_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.eq_ignore_ascii_case(".DS_Store")
                || name.eq_ignore_ascii_case("Thumbs.db")
                || name.eq_ignore_ascii_case("desktop.ini")
        })
}

fn now_rfc3339() -> AppResult<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AppError::Validation(error.to_string()))
}

fn compact_timestamp(timestamp: &str) -> String {
    timestamp
        .chars()
        .filter(|char| char.is_ascii_alphanumeric())
        .collect::<String>()
}

fn safe_file_part(value: &str) -> String {
    let mut output = value
        .chars()
        .map(|char| {
            if char.is_ascii_alphanumeric() || char == '-' || char == '_' {
                char
            } else {
                '-'
            }
        })
        .collect::<String>();

    while output.contains("--") {
        output = output.replace("--", "-");
    }

    output.trim_matches('-').to_string()
}

fn to_zip_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn zip_file_options() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)
}

fn zip_dir_options() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;

    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipArchive, ZipWriter};

    use super::{
        BundleAddonLockApplyRequest, BundleApplyMappings, PackBundleRequest, UnpackBundleRequest,
        apply_bundle_addon_lock, inspect_bundle, pack_bundle, plan_bundle_addon_lock,
        plan_bundle_apply, unpack_bundle,
    };
    use crate::core::addon::{InstallAddonRequest, install_addon};
    use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};
    use crate::core::manifest::{
        ApplyDefaults, BundleManifest, BundleResources, CharacterMappingMode, CharacterResource,
        MappingRules, PackageMetadata, SourceInstallation,
    };

    #[test]
    fn pack_bundle_writes_normalized_layout() {
        let temp = tempdir().expect("temp dir");
        let installation = create_fixture_installation(temp.path(), true);
        let bundle_path = temp.path().join("bundle.zip");

        let bundle = pack_bundle(PackBundleRequest {
            installation,
            manifest: sample_manifest(),
            output_path: Some(bundle_path.clone()),
            manifest_base_dir: None,
        })
        .expect("pack bundle");

        assert_eq!(bundle.archive_path, bundle_path);

        let file = fs::File::open(bundle.archive_path).expect("bundle file");
        let mut archive = ZipArchive::new(file).expect("zip archive");

        assert!(archive.by_name("manifest.toml").is_ok());
        assert!(archive.by_name("addons/WeakAuras/WeakAuras.toc").is_ok());
        assert!(archive.by_name("wtf/common/Config.wtf").is_ok());
        assert!(
            archive
                .by_name("wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua")
                .is_ok()
        );
        assert!(
            archive
                .by_name("wtf/characters/ACCOUNT/Illidan/Examplemage/AddOns.txt")
                .is_ok()
        );
        assert!(archive.by_name("fonts/FRIZQT__.ttf").is_ok());
        assert!(archive.by_name("interface/SharedXML/texture.blp").is_ok());
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
        assert_eq!(inspection.entries.addons, 1);
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
            item.group == super::ApplyGroup::WtfCommon
                && item.target_account.as_deref() == Some("ACC_A")
        }));
    }

    #[test]
    fn plan_bundle_apply_skips_identical_files() {
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
        assert_eq!(plan.summary.files_to_replace, 0);
        assert!(plan.summary.files_to_skip > 0);
        assert_eq!(plan.summary.files_to_skip, plan.operations.len());
        assert!(
            plan.operations
                .iter()
                .all(|operation| operation.action == super::ApplyAction::Skip)
        );
    }

    #[test]
    fn unpack_bundle_applies_character_mapping_and_lua_rewrite() {
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

        let character_lua = fs::read_to_string(
            target_installation
                .wtf_dir
                .join("Account")
                .join("TARGETACC")
                .join("Stormrage")
                .join("Targetmage")
                .join("Pawn.lua"),
        )
        .expect("character lua");
        assert!(character_lua.contains(r#""Targetmage""#));
        assert!(character_lua.contains(r#""Stormrage""#));
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
    fn pack_bundle_embeds_addon_lock_and_indexes_as_sidecar_metadata() {
        let source = tempdir().expect("source temp dir");
        let target = tempdir().expect("target temp dir");
        let source_installation = create_fixture_installation(source.path(), false);
        let target_installation = create_fixture_installation(target.path(), false);
        let bundle_path = source.path().join("bundle.zip");
        let archive_path = source.path().join("WeakAuras.zip");
        let index_path = source.path().join("addon-index.toml");

        create_addon_archive(
            &archive_path,
            &[(
                "WeakAuras/WeakAuras.toc",
                "## Interface: 110000\n## Version: 1.0.0\n",
            )],
        );
        install_addon(InstallAddonRequest {
            installation: source_installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(source.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install tracked addon");
        fs::write(
            &index_path,
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "weakauras"
name = "WeakAuras"
version = "1.0.0"
source = { kind = "local_archive", path = "WeakAuras.zip" }
"#,
        )
        .expect("index");

        let mut manifest = sample_manifest();
        manifest.resources.addons = Vec::new();
        manifest.resources.wtf_common = false;
        manifest.resources.wtf_characters = Vec::new();
        manifest.resources.fonts = false;
        manifest.resources.interface_assets = Vec::new();
        manifest.resources.addon_lock = true;
        manifest.resources.addon_indexes = vec!["addon-index.toml".to_string()];
        manifest.mapping.character_mode = CharacterMappingMode::KeepOriginal;

        let bundle = pack_bundle(PackBundleRequest {
            installation: source_installation,
            manifest,
            output_path: Some(bundle_path.clone()),
            manifest_base_dir: Some(source.path().to_path_buf()),
        })
        .expect("pack bundle");

        let file = fs::File::open(&bundle.archive_path).expect("bundle file");
        let mut archive = ZipArchive::new(file).expect("zip archive");
        assert!(archive.by_name("metadata/addons/lock.toml").is_ok());
        assert!(
            archive
                .by_name("metadata/addons/indexes/addon-index.toml")
                .is_ok()
        );

        let inspection = inspect_bundle(&bundle.archive_path).expect("inspect bundle");
        assert_eq!(inspection.entries.metadata, 3);

        unpack_bundle(UnpackBundleRequest {
            bundle_path,
            installation: target_installation.clone(),
            dry_run: false,
            backup_output_path: Some(target.path().join("backups")),
            apply_mappings: BundleApplyMappings::default(),
        })
        .expect("unpack bundle");

        let sidecar_root = target_installation
            .addon_dir
            .join(".hearthsync")
            .join("bundles")
            .join("test-ui");
        assert!(sidecar_root.join("addons").join("lock.toml").exists());
        assert!(
            sidecar_root
                .join("addons")
                .join("indexes")
                .join("addon-index.toml")
                .exists()
        );

        let addon_plan =
            plan_bundle_addon_lock(&bundle.archive_path, &target_installation).expect("addon plan");
        assert_eq!(addon_plan.plan.install_count, 1);
        assert_eq!(addon_plan.plan.update_count, 0);
        assert_eq!(addon_plan.plan.remove_count, 0);

        let addon_apply = apply_bundle_addon_lock(BundleAddonLockApplyRequest {
            bundle_path: bundle.archive_path,
            installation: target_installation.clone(),
            backup_output_path: Some(target.path().join("addon-backups")),
            replace_existing: false,
        })
        .expect("addon apply");
        assert!(addon_apply.apply.verification.matches);
        assert!(
            target_installation
                .addon_dir
                .join("WeakAuras")
                .join("WeakAuras.toc")
                .exists()
        );
    }

    fn create_fixture_installation(
        root: &std::path::Path,
        with_content: bool,
    ) -> DetectedFlavorInstallation {
        let product_root = root.join("World of Warcraft");
        let flavor_root = product_root.join("_retail_");
        let interface_dir = flavor_root.join("Interface");
        let addon_dir = interface_dir.join("AddOns");
        let wtf_dir = flavor_root.join("WTF");
        let fonts_dir = flavor_root.join("Fonts");

        fs::create_dir_all(&addon_dir).expect("addon root");
        fs::create_dir_all(&wtf_dir).expect("wtf root");
        fs::create_dir_all(&fonts_dir).expect("fonts root");

        if with_content {
            fs::create_dir_all(addon_dir.join("WeakAuras")).expect("addon dir");
            fs::write(
                addon_dir.join("WeakAuras").join("WeakAuras.toc"),
                "## Interface: 110000",
            )
            .expect("toc");

            fs::write(wtf_dir.join("Config.wtf"), "SET locale enUS").expect("config");
            fs::create_dir_all(
                wtf_dir
                    .join("Account")
                    .join("ACCOUNT")
                    .join("SavedVariables"),
            )
            .expect("saved variables");
            fs::write(
                wtf_dir
                    .join("Account")
                    .join("ACCOUNT")
                    .join("SavedVariables")
                    .join("Details.lua"),
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
            .expect("saved variable");
            fs::create_dir_all(
                wtf_dir
                    .join("Account")
                    .join("ACCOUNT")
                    .join("Illidan")
                    .join("Examplemage"),
            )
            .expect("character");
            fs::write(
                wtf_dir
                    .join("Account")
                    .join("ACCOUNT")
                    .join("Illidan")
                    .join("Examplemage")
                    .join("AddOns.txt"),
                "WeakAuras: enabled",
            )
            .expect("addons state");
            fs::write(
                wtf_dir
                    .join("Account")
                    .join("ACCOUNT")
                    .join("Illidan")
                    .join("Examplemage")
                    .join("Pawn.lua"),
                r#"
PawnOptions = {
  ["LastPlayerFullName"] = "Examplemage",
  ["LastRealm"] = "Illidan",
}
"#,
            )
            .expect("character lua");

            fs::write(fonts_dir.join("FRIZQT__.ttf"), "font").expect("font");
            fs::create_dir_all(interface_dir.join("SharedXML")).expect("asset dir");
            fs::write(
                interface_dir.join("SharedXML").join("texture.blp"),
                "texture",
            )
            .expect("asset");
        }

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

    fn create_addon_archive(path: &std::path::Path, entries: &[(&str, &str)]) {
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

    fn sample_manifest() -> BundleManifest {
        BundleManifest {
            schema_version: 1,
            package: PackageMetadata {
                id: "test-ui".to_string(),
                name: "Test UI".to_string(),
                created_by: "test".to_string(),
                description: None,
            },
            source: SourceInstallation {
                flavor: WowFlavor::Retail,
                platform: None,
                exported_at: None,
                supported_targets: vec![WowFlavor::Retail],
            },
            resources: BundleResources {
                addons: vec!["WeakAuras".to_string()],
                wtf_common: true,
                wtf_characters: vec![CharacterResource {
                    source_account: Some("ACCOUNT".to_string()),
                    source_server: "Illidan".to_string(),
                    source_character: "Examplemage".to_string(),
                    target_hint: None,
                }],
                fonts: true,
                interface_assets: vec!["SharedXML".to_string()],
                addon_lock: false,
                addon_indexes: Vec::new(),
            },
            mapping: MappingRules {
                character_mode: CharacterMappingMode::Explicit,
                rewrite_profile_keys: false,
                rewrite_identity_strings: false,
                allow_cross_platform: true,
            },
            apply: ApplyDefaults {
                create_backup: true,
                replace_addons: false,
                replace_fonts: false,
                merge_wtf: true,
            },
        }
    }

    fn sample_manifest_with_rewrite() -> BundleManifest {
        let mut manifest = sample_manifest();
        manifest.mapping.rewrite_profile_keys = true;
        manifest.mapping.rewrite_identity_strings = true;
        manifest
    }
}
