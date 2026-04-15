mod apply_policy;
mod entry_plan;
#[cfg(test)]
mod tests;

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

use self::apply_policy::{
    apply_action_order, apply_group_order, build_cleanup_operations, cleanup_scope_for_entry,
    resource_policy_for_group,
};
use self::entry_plan::plan_extractable_entries;
use crate::core::addon::lock::{
    AddonLock, AddonLockApplyRequest, AddonLockApplyResult, AddonLockPackage, AddonLockPlanResult,
    AddonLockSourceOverride, addon_lock_package_comparison_key, apply_addon_lock_sync,
    plan_addon_lock_sync_with_source_overrides, write_addon_lock,
};
use crate::core::backup::{BackupGroup, BackupRequest, create_backup, restore_backup};
use crate::core::error::{AppError, AppResult};
use crate::core::install::{DetectedFlavorInstallation, LocalWowAccount, discover_local_accounts};
use crate::core::lua_patch::{
    CharacterMapping, LuaRewriteOptions, preview_lua_bytes_rewrite, rewrite_lua_file,
};
use crate::core::manifest::{
    BundleManifest, CharacterMappingMode, CharacterResource, ResourceApplyPolicy,
};

const MANIFEST_ENTRY: &str = "manifest.toml";
const ADDON_LOCK_ENTRY: &str = "metadata/addons/lock.toml";
const ADDON_INDEX_ENTRY_ROOT: &str = "metadata/addons/indexes";
const ADDON_SOURCE_INDEX_ENTRY: &str = "metadata/addons/sources.toml";
const ADDON_SOURCE_ENTRY_ROOT: &str = "metadata/addons/sources";

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
    pub wtf_scope: Option<WtfScope>,
    pub action: ApplyAction,
    pub archive_name: String,
    pub destination: PathBuf,
    pub target_account: Option<String>,
    pub target_server: Option<String>,
    pub target_character: Option<String>,
    pub rewrite_count: usize,
    pub rewrite_applied: bool,
}

#[derive(Debug, Clone)]
struct PreparedApplyOperation {
    group: ApplyGroup,
    wtf_scope: Option<WtfScope>,
    action: ApplyAction,
    archive_name: String,
    destination: PathBuf,
    target_account: Option<String>,
    target_server: Option<String>,
    target_character: Option<String>,
    rewrite_applied: bool,
    rewrites: Vec<CharacterMapping>,
}

impl PreparedApplyOperation {
    fn from_cleanup(cleanup: PlannedCleanup) -> Self {
        Self {
            group: cleanup.group,
            wtf_scope: None,
            action: ApplyAction::Remove,
            archive_name: format!("[cleanup] {}", cleanup.destination.display()),
            destination: cleanup.destination,
            target_account: cleanup.target_account,
            target_server: cleanup.target_server,
            target_character: cleanup.target_character,
            rewrite_applied: false,
            rewrites: Vec::new(),
        }
    }

    fn from_entry(entry: &PlannedEntry, action: ApplyAction, rewrite_applied: bool) -> Self {
        Self {
            group: entry.group,
            wtf_scope: entry.wtf_scope,
            action,
            archive_name: entry.archive_name.clone(),
            destination: entry.destination.clone(),
            target_account: entry.target_account.clone(),
            target_server: entry.target_server.clone(),
            target_character: entry.target_character.clone(),
            rewrite_applied,
            rewrites: entry.rewrites.clone(),
        }
    }

    fn preview(&self) -> ApplyOperation {
        ApplyOperation {
            group: self.group,
            wtf_scope: self.wtf_scope,
            action: self.action,
            archive_name: self.archive_name.clone(),
            destination: self.destination.clone(),
            target_account: self.target_account.clone(),
            target_server: self.target_server.clone(),
            target_character: self.target_character.clone(),
            rewrite_count: self.rewrites.len(),
            rewrite_applied: self.rewrite_applied,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ApplyPlanSummary {
    pub files_to_add: usize,
    pub files_to_replace: usize,
    pub files_to_skip: usize,
    pub paths_to_remove: usize,
    pub files_to_preserve: usize,
    pub files_to_rewrite: usize,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyAction {
    Remove,
    Add,
    Replace,
    Skip,
    Preserve,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WtfScope {
    GlobalConfig,
    AccountRootFile,
    AccountSavedVariables,
    CharacterSavedVariables,
    CharacterState,
    CacheLike,
    Unknown,
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
    pub policy: ResourceApplyPolicy,
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
    wtf_scope: Option<WtfScope>,
    target_account: Option<String>,
    target_server: Option<String>,
    target_character: Option<String>,
}

#[derive(Debug, Clone)]
struct PlannedCleanup {
    group: ApplyGroup,
    destination: PathBuf,
    target_account: Option<String>,
    target_server: Option<String>,
    target_character: Option<String>,
}

struct PreparedBundleApply {
    plan: BundleApplyPlan,
    execution_operations: Vec<PreparedApplyOperation>,
}

struct BundleReader<'a> {
    bundle_path: &'a Path,
}

struct BundleReadModel {
    inspection: BundleInspection,
    entry_names: Vec<String>,
}

struct BundlePlanner<'a> {
    bundle_path: &'a Path,
    installation: &'a DetectedFlavorInstallation,
    apply_mappings: &'a BundleApplyMappings,
}

struct BundleExecution {
    backup_path: Option<PathBuf>,
    written_files: usize,
    rewritten_files: usize,
}

struct BundleExecutor<'a> {
    installation: &'a DetectedFlavorInstallation,
    backup_output_path: Option<PathBuf>,
}

struct ExtractedAddonLock {
    lock_path: PathBuf,
    source_overrides: Vec<AddonLockSourceOverride>,
    _stage_dir: TempDir,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BundleAddonSourceIndex {
    schema_version: u32,
    sources: Vec<BundleAddonSourceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BundleAddonSourceEntry {
    comparison_key: String,
    package_id: String,
    path: String,
    content_sha256: String,
    addon_directories: Vec<String>,
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
        let lock = read_generated_addon_lock(&lock_result.lock_path)?;
        let source_index =
            add_bundle_addon_sources_to_zip(&mut zip, &request.installation, &lock.packages)?;
        archived_files += source_index.sources.len();
        archived_files += write_toml_to_zip(&mut zip, ADDON_SOURCE_INDEX_ENTRY, &source_index)?;
        archived_files += write_toml_to_zip(&mut zip, ADDON_LOCK_ENTRY, &lock)?;
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
    let mut plan = plan_addon_lock_sync_with_source_overrides(
        installation,
        Some(&extracted.lock_path),
        &extracted.source_overrides,
    )?;
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
        source_overrides: extracted.source_overrides,
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
    BundlePlanner {
        bundle_path,
        installation,
        apply_mappings,
    }
    .prepare()
}

impl<'a> BundleReader<'a> {
    fn new(bundle_path: &'a Path) -> Self {
        Self { bundle_path }
    }

    fn inspect(&self) -> AppResult<BundleInspection> {
        inspect_bundle(self.bundle_path)
    }

    fn read_for_apply(&self) -> AppResult<BundleReadModel> {
        let inspection = self.inspect()?;

        Ok(BundleReadModel {
            inspection,
            entry_names: collect_bundle_entry_names(self.bundle_path)?,
        })
    }
}

impl<'a> BundlePlanner<'a> {
    fn prepare(&self) -> AppResult<PreparedBundleApply> {
        let read_model = BundleReader::new(self.bundle_path).read_for_apply()?;
        validate_target_compatibility(&read_model.inspection.manifest, self.installation)?;
        let discovered_accounts = discover_local_accounts(self.installation)?;
        let character_mappings =
            build_character_mappings(&read_model.inspection.manifest, self.apply_mappings)?;
        let selected_target_accounts = resolve_selected_target_accounts(
            &read_model.inspection.manifest,
            &discovered_accounts,
            &character_mappings,
            self.apply_mappings,
        )?;
        self.plan(
            read_model,
            discovered_accounts,
            character_mappings,
            selected_target_accounts,
        )
    }

    fn plan(
        &self,
        read_model: BundleReadModel,
        discovered_accounts: Vec<LocalWowAccount>,
        character_mappings: Vec<CharacterMapping>,
        selected_target_accounts: Vec<String>,
    ) -> AppResult<PreparedBundleApply> {
        let inspection = read_model.inspection;
        let planned_entries = plan_extractable_entries(
            &read_model.entry_names,
            self.installation,
            &inspection.manifest,
            &character_mappings,
            self.apply_mappings,
            &selected_target_accounts,
        )?;
        let file = File::open(self.bundle_path)?;
        let mut archive = ZipArchive::new(file)?;
        let rewrite_options = LuaRewriteOptions {
            rewrite_profile_keys: inspection.manifest.mapping.rewrite_profile_keys,
            rewrite_identity_strings: inspection.manifest.mapping.rewrite_identity_strings,
        };

        let cleanup_operations =
            build_cleanup_operations(&planned_entries, &inspection.manifest, self.installation)?;
        let cleanup_roots = cleanup_operations
            .iter()
            .map(|operation| operation.destination.clone())
            .collect::<Vec<_>>();
        let mut execution_operations = Vec::new();
        let mut summary = ApplyPlanSummary::default();
        for cleanup in cleanup_operations {
            summary.paths_to_remove += 1;
            execution_operations.push(PreparedApplyOperation::from_cleanup(cleanup));
        }

        for entry in &planned_entries {
            let policy = resource_policy_for_group(&inspection.manifest, entry.group);
            let preserve = policy == ResourceApplyPolicy::Preserve;
            let share = policy == ResourceApplyPolicy::Share;
            let cleanup_root = cleanup_scope_for_entry(entry, self.installation)?;
            let will_cleanup = cleanup_root
                .as_ref()
                .is_some_and(|root| cleanup_roots.iter().any(|candidate| candidate == root));
            let source_bytes =
                read_bundle_entry_bytes_from_archive(&mut archive, &entry.archive_name)?;
            let rewritten_bytes = if preserve {
                None
            } else {
                preview_lua_bytes_rewrite(
                    Path::new(&entry.archive_name),
                    &source_bytes,
                    &entry.rewrites,
                    rewrite_options,
                )?
            };
            let rewrite_applied = rewritten_bytes.is_some();
            let action = if preserve {
                summary.files_to_preserve += 1;
                ApplyAction::Preserve
            } else if share && entry.destination.exists() {
                summary.files_to_preserve += 1;
                ApplyAction::Preserve
            } else if will_cleanup {
                summary.files_to_add += 1;
                ApplyAction::Add
            } else if !entry.destination.exists() {
                summary.files_to_add += 1;
                ApplyAction::Add
            } else if rewritten_bytes.as_deref().map_or_else(
                || file_contents_equal_to_bytes(&source_bytes, &entry.destination),
                |bytes| file_contents_equal_to_bytes(bytes, &entry.destination),
            )? {
                summary.files_to_skip += 1;
                ApplyAction::Skip
            } else {
                summary.files_to_replace += 1;
                ApplyAction::Replace
            };
            if rewrite_applied {
                summary.files_to_rewrite += 1;
            }

            execution_operations.push(PreparedApplyOperation::from_entry(
                entry,
                action,
                rewrite_applied,
            ));
        }

        execution_operations.sort_by(|left, right| {
            apply_action_order(left.action)
                .cmp(&apply_action_order(right.action))
                .then_with(|| apply_group_order(left.group).cmp(&apply_group_order(right.group)))
                .then_with(|| left.destination.cmp(&right.destination))
                .then_with(|| left.archive_name.cmp(&right.archive_name))
        });
        let operations = execution_operations
            .iter()
            .map(PreparedApplyOperation::preview)
            .collect::<Vec<_>>();

        Ok(PreparedBundleApply {
            plan: BundleApplyPlan {
                bundle_path: self.bundle_path.to_path_buf(),
                target_flavor_root: self.installation.flavor_root.clone(),
                discovered_accounts,
                selected_target_accounts,
                character_mappings,
                operations,
                summary,
                helper_strategy: HelperStrategy::NativeRust,
                group_policies: ApplyGroupPolicies {
                    addons: GroupPolicy {
                        policy: inspection.manifest.apply.addons,
                    },
                    wtf_common: GroupPolicy {
                        policy: inspection.manifest.apply.wtf_common,
                    },
                    wtf_characters: GroupPolicy {
                        policy: inspection.manifest.apply.wtf_characters,
                    },
                    fonts: GroupPolicy {
                        policy: inspection.manifest.apply.fonts,
                    },
                    interface_assets: GroupPolicy {
                        policy: inspection.manifest.apply.interface_assets,
                    },
                    metadata: GroupPolicy {
                        policy: ResourceApplyPolicy::Merge,
                    },
                },
                manifest: inspection.manifest,
            },
            execution_operations,
        })
    }
}

pub fn unpack_bundle(request: UnpackBundleRequest) -> AppResult<UnpackedBundle> {
    let prepared = prepare_bundle_apply(
        &request.bundle_path,
        &request.installation,
        &request.apply_mappings,
    )?;
    let PreparedBundleApply {
        plan,
        execution_operations,
    } = prepared;
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

    let execution = BundleExecutor {
        installation: &request.installation,
        backup_output_path: request.backup_output_path.clone(),
    }
    .execute(&plan, &execution_operations)?;

    Ok(UnpackedBundle {
        bundle_path: request.bundle_path,
        target_flavor_root: request.installation.flavor_root,
        dry_run: false,
        planned_files: plan.operations.len(),
        written_files: execution.written_files,
        rewritten_files: execution.rewritten_files,
        backup_path: execution.backup_path,
        selected_target_accounts: plan.selected_target_accounts,
        plan_summary: plan.summary,
        character_mappings: plan.character_mappings,
        manifest: plan.manifest,
    })
}

impl<'a> BundleExecutor<'a> {
    fn execute(
        &self,
        plan: &BundleApplyPlan,
        execution_operations: &[PreparedApplyOperation],
    ) -> AppResult<BundleExecution> {
        let backup_path = self.create_backup(plan)?;

        match execute_apply_operations(&plan.bundle_path, execution_operations, &plan.manifest) {
            Ok((written_files, rewritten_files)) => Ok(BundleExecution {
                backup_path,
                written_files,
                rewritten_files,
            }),
            Err(error) => {
                rollback_or_report_apply_error(error, backup_path.as_deref(), self.installation)
            }
        }
    }

    fn create_backup(&self, plan: &BundleApplyPlan) -> AppResult<Option<PathBuf>> {
        if !plan.manifest.apply.create_backup {
            return Ok(None);
        }

        let groups = backup_groups_for_manifest(&plan.manifest);
        if groups.is_empty() {
            Ok(None)
        } else {
            Ok(Some(
                create_backup(BackupRequest {
                    installation: self.installation.clone(),
                    output_path: self.backup_output_path.clone(),
                    groups,
                    label: Some("bundle-apply".to_string()),
                })?
                .archive_path,
            ))
        }
    }
}

fn collect_bundle_entry_names(bundle_path: &Path) -> AppResult<Vec<String>> {
    let file = File::open(bundle_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut entry_names = Vec::new();

    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }
        entry_names.push(entry.name().to_string());
    }

    Ok(entry_names)
}

fn read_bundle_entry_bytes_from_archive(
    archive: &mut ZipArchive<File>,
    archive_name: &str,
) -> AppResult<Vec<u8>> {
    let mut entry = archive
        .by_name(archive_name)
        .map_err(|_| AppError::NotFound(format!("bundle entry is missing: {archive_name}")))?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn extract_archive_entry_to_path(
    archive: &mut ZipArchive<File>,
    archive_name: &str,
    destination: &Path,
) -> AppResult<()> {
    let segments = safe_zip_segments(archive_name)?;
    if segments.is_empty() {
        return Err(AppError::Validation(format!(
            "bundle entry cannot be materialized because its path is empty: {archive_name}"
        )));
    }
    let mut entry = archive
        .by_name(archive_name)
        .map_err(|_| AppError::NotFound(format!("bundle entry is missing: {archive_name}")))?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut output = File::create(destination)?;
    std::io::copy(&mut entry, &mut output)?;
    Ok(())
}

fn extract_embedded_addon_lock(bundle_path: &Path) -> AppResult<ExtractedAddonLock> {
    let file = File::open(bundle_path)?;
    let mut archive = ZipArchive::new(file)?;
    let stage_dir = tempdir()?;
    let lock_path = stage_dir.path().join("lock.toml");
    {
        let mut lock_entry = archive.by_name(ADDON_LOCK_ENTRY).map_err(|_| {
            AppError::NotFound(format!(
                "bundle does not contain embedded addon lock `{ADDON_LOCK_ENTRY}`"
            ))
        })?;
        let mut output = File::create(&lock_path)?;
        std::io::copy(&mut lock_entry, &mut output)?;
    }

    let source_overrides = extract_bundle_addon_source_overrides(&mut archive, stage_dir.path())?;

    Ok(ExtractedAddonLock {
        lock_path,
        source_overrides,
        _stage_dir: stage_dir,
    })
}

fn extract_bundle_addon_source_overrides(
    archive: &mut ZipArchive<File>,
    stage_root: &Path,
) -> AppResult<Vec<AddonLockSourceOverride>> {
    let source_index = match archive.by_name(ADDON_SOURCE_INDEX_ENTRY) {
        Ok(mut entry) => {
            let mut content = String::new();
            entry.read_to_string(&mut content)?;
            toml::from_str::<BundleAddonSourceIndex>(&content)?
        }
        Err(zip::result::ZipError::FileNotFound) => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };

    if source_index.schema_version != 1 {
        return Err(AppError::Validation(format!(
            "unsupported bundle addon source index schema version: {}",
            source_index.schema_version
        )));
    }

    let mut source_overrides = Vec::new();
    for source in source_index.sources {
        let segments = safe_zip_segments(&source.path)?;
        if segments.first().copied() != Some("sources") || segments.len() < 2 {
            return Err(AppError::Validation(format!(
                "bundle addon source path must be under `sources/`: {}",
                source.path
            )));
        }

        let archive_entry_name = format!("metadata/addons/{}", segments.join("/"));
        let mut source_entry = archive.by_name(&archive_entry_name).map_err(|_| {
            AppError::NotFound(format!(
                "bundle addon source archive is missing: {archive_entry_name}"
            ))
        })?;
        let extracted_path = join_segments(stage_root, &segments);
        if let Some(parent) = extracted_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = File::create(&extracted_path)?;
        std::io::copy(&mut source_entry, &mut output)?;

        source_overrides.push(AddonLockSourceOverride {
            comparison_key: source.comparison_key,
            archive_path: extracted_path,
        });
    }

    Ok(source_overrides)
}

fn execute_apply_operations(
    bundle_path: &Path,
    execution_operations: &[PreparedApplyOperation],
    manifest: &BundleManifest,
) -> AppResult<(usize, usize)> {
    let mut written_files = 0usize;
    let mut rewritten_files = 0usize;
    let rewrite_stage = tempdir()?;
    let file = File::open(bundle_path)?;
    let mut archive = ZipArchive::new(file)?;
    let rewrite_options = LuaRewriteOptions {
        rewrite_profile_keys: manifest.mapping.rewrite_profile_keys,
        rewrite_identity_strings: manifest.mapping.rewrite_identity_strings,
    };

    for (operation_index, operation) in execution_operations.iter().enumerate() {
        if matches!(operation.action, ApplyAction::Skip | ApplyAction::Preserve) {
            continue;
        }

        if operation.action == ApplyAction::Remove {
            remove_target_path(&operation.destination)?;
            continue;
        }

        if let Some(parent) = operation.destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let source_path = if operation.rewrite_applied {
            materialize_rewritten_operation(
                operation_index,
                operation,
                &mut archive,
                rewrite_stage.path(),
                rewrite_options,
            )?
        } else {
            materialize_archive_operation(
                operation_index,
                &operation.archive_name,
                &mut archive,
                rewrite_stage.path(),
            )?
        };
        fs::copy(source_path, &operation.destination)?;
        written_files += 1;

        if operation.rewrite_applied {
            rewritten_files += 1;
        }
    }

    Ok((written_files, rewritten_files))
}

fn materialize_rewritten_operation(
    operation_index: usize,
    operation: &PreparedApplyOperation,
    archive: &mut ZipArchive<File>,
    rewrite_stage_root: &Path,
    rewrite_options: LuaRewriteOptions,
) -> AppResult<PathBuf> {
    let rewrite_path = materialize_archive_operation(
        operation_index,
        &operation.archive_name,
        archive,
        rewrite_stage_root,
    )?;
    rewrite_lua_file(
        Path::new(&operation.archive_name),
        &rewrite_path,
        &operation.rewrites,
        rewrite_options,
    )?;
    Ok(rewrite_path)
}

fn materialize_archive_operation(
    operation_index: usize,
    archive_name: &str,
    archive: &mut ZipArchive<File>,
    stage_root: &Path,
) -> AppResult<PathBuf> {
    let file_name = Path::new(archive_name)
        .file_name()
        .map(|name| name.to_owned())
        .unwrap_or_else(|| format!("operation-{operation_index}").into());
    let stage_path = stage_root.join(operation_index.to_string()).join(file_name);
    extract_archive_entry_to_path(archive, archive_name, &stage_path)?;
    Ok(stage_path)
}

fn remove_target_path(path: &Path) -> AppResult<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
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

fn file_contents_equal_to_bytes(bytes: &[u8], right: &Path) -> AppResult<bool> {
    if !right.exists() || !right.is_file() {
        return Ok(false);
    }

    let right_metadata = fs::metadata(right)?;
    if right_metadata.len() != bytes.len() as u64 {
        return Ok(false);
    }

    let mut right_file = File::open(right)?;
    let mut right_buffer = [0u8; 8192];
    let mut offset = 0usize;

    loop {
        let right_read = right_file.read(&mut right_buffer)?;
        if right_read == 0 {
            return Ok(offset == bytes.len());
        }
        if offset + right_read > bytes.len() {
            return Ok(false);
        }
        if bytes[offset..offset + right_read] != right_buffer[..right_read] {
            return Ok(false);
        }
        offset += right_read;
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
        for account_entry in fs::read_dir(&account_dir)? {
            let account_entry = account_entry?;
            let account_path = account_entry.path();
            if !account_path.is_file() {
                continue;
            }

            let file_name = account_entry.file_name().to_string_lossy().to_string();
            validate_plain_name("account WTF file", &file_name)?;
            archived_files += add_path_to_zip(
                zip,
                &account_path,
                &Path::new("wtf/common/accounts")
                    .join(&account_name)
                    .join(file_name),
            )?;
        }

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

fn read_generated_addon_lock(path: &Path) -> AppResult<AddonLock> {
    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

fn add_bundle_addon_sources_to_zip(
    zip: &mut ZipWriter<File>,
    installation: &DetectedFlavorInstallation,
    packages: &[AddonLockPackage],
) -> AppResult<BundleAddonSourceIndex> {
    let source_stage = tempdir()?;
    let mut entries = Vec::new();
    let mut used_file_names = Vec::new();
    let mut packages = packages.iter().collect::<Vec<_>>();
    packages.sort_by(|left, right| {
        addon_lock_package_comparison_key(left).cmp(&addon_lock_package_comparison_key(right))
    });

    for (index, package) in packages.into_iter().enumerate() {
        let comparison_key = addon_lock_package_comparison_key(package);
        let file_name = unique_bundle_source_archive_name(
            &comparison_key,
            &package.package_id,
            index,
            &mut used_file_names,
        );
        let source_archive_path = source_stage.path().join(&file_name);
        write_addon_package_source_archive(&source_archive_path, installation, package)?;
        let relative_source_path = format!("sources/{file_name}");
        let bundle_entry_path = Path::new(ADDON_SOURCE_ENTRY_ROOT).join(&file_name);
        add_path_to_zip(zip, &source_archive_path, &bundle_entry_path)?;

        entries.push(BundleAddonSourceEntry {
            comparison_key,
            package_id: package.package_id.clone(),
            path: relative_source_path,
            content_sha256: package.content_sha256.clone(),
            addon_directories: package.addon_directories.clone(),
        });
    }

    Ok(BundleAddonSourceIndex {
        schema_version: 1,
        sources: entries,
    })
}

fn unique_bundle_source_archive_name(
    comparison_key: &str,
    package_id: &str,
    index: usize,
    used_file_names: &mut Vec<String>,
) -> String {
    let mut base = safe_file_part(comparison_key);
    if base.is_empty() {
        base = safe_file_part(package_id);
    }
    if base.is_empty() {
        base = format!("package-{index}");
    }

    let mut candidate = format!("{base}.zip");
    let mut suffix = 2usize;
    while used_file_names.iter().any(|item| item == &candidate) {
        candidate = format!("{base}-{suffix}.zip");
        suffix += 1;
    }
    used_file_names.push(candidate.clone());
    candidate
}

fn write_addon_package_source_archive(
    archive_path: &Path,
    installation: &DetectedFlavorInstallation,
    package: &AddonLockPackage,
) -> AppResult<()> {
    let file = File::create(archive_path)?;
    let mut zip = ZipWriter::new(file);
    let mut archived_files = 0usize;

    for addon_directory in &package.addon_directories {
        validate_plain_name("addon", addon_directory)?;
        let source = installation.addon_dir.join(addon_directory);
        if !source.is_dir() {
            return Err(AppError::NotFound(format!(
                "tracked addon directory does not exist: {}",
                source.display()
            )));
        }
        archived_files += add_path_to_zip(&mut zip, &source, Path::new(addon_directory))?;
    }

    zip.finish()?;
    if archived_files == 0 {
        return Err(AppError::Validation(format!(
            "tracked package `{}` does not contain any addon files",
            package.package_id
        )));
    }

    Ok(())
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

fn write_toml_to_zip<T: Serialize>(
    zip: &mut ZipWriter<File>,
    archive_path: &str,
    value: &T,
) -> AppResult<usize> {
    zip.start_file(archive_path, zip_file_options())?;
    zip.write_all(toml::to_string_pretty(value)?.as_bytes())?;
    Ok(1)
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

fn build_character_mappings(
    manifest: &BundleManifest,
    apply_mappings: &BundleApplyMappings,
) -> AppResult<Vec<CharacterMapping>> {
    let single_character_bundle = manifest.resources.wtf_characters.len() == 1;
    validate_character_mapping_inputs(
        manifest.mapping.character_mode,
        apply_mappings,
        single_character_bundle,
    )?;
    let mut mappings = Vec::new();

    for resource in &manifest.resources.wtf_characters {
        let source_account = resource.source_account.clone();
        let mapping = match manifest.mapping.character_mode {
            CharacterMappingMode::KeepOriginal => {
                build_keep_original_character_mapping(resource, source_account)?
            }
            CharacterMappingMode::Explicit | CharacterMappingMode::Prompt => {
                build_resolved_character_mapping(
                    manifest.mapping.character_mode,
                    resource,
                    source_account,
                    apply_mappings,
                    single_character_bundle,
                )?
            }
        };

        mappings.push(mapping);
    }

    Ok(mappings)
}

fn validate_character_mapping_inputs(
    character_mode: CharacterMappingMode,
    apply_mappings: &BundleApplyMappings,
    single_character_bundle: bool,
) -> AppResult<()> {
    if matches!(
        character_mode,
        CharacterMappingMode::Explicit | CharacterMappingMode::Prompt
    ) && !single_character_bundle
        && (apply_mappings.target_server.is_some() || apply_mappings.target_character.is_some())
    {
        return Err(AppError::Validation(
            "global target_server/target_character overrides are only supported when the bundle contains exactly one character; use `--mapping-file` for multi-character explicit mappings.".to_string(),
        ));
    }

    Ok(())
}

fn build_keep_original_character_mapping(
    resource: &CharacterResource,
    source_account: Option<String>,
) -> AppResult<CharacterMapping> {
    let target_account = source_account.clone().ok_or_else(|| {
        AppError::Validation(format!(
            "source account is required for keep_original character mapping on `{}/{}`",
            resource.source_server, resource.source_character
        ))
    })?;
    validate_plain_name("target account", &target_account)?;
    validate_plain_name("target server", &resource.source_server)?;
    validate_plain_name("target character", &resource.source_character)?;

    Ok(CharacterMapping {
        source_account,
        source_server: resource.source_server.clone(),
        source_character: resource.source_character.clone(),
        target_account,
        target_server: resource.source_server.clone(),
        target_character: resource.source_character.clone(),
    })
}

fn build_resolved_character_mapping(
    character_mode: CharacterMappingMode,
    resource: &CharacterResource,
    source_account: Option<String>,
    apply_mappings: &BundleApplyMappings,
    single_character_bundle: bool,
) -> AppResult<CharacterMapping> {
    let override_mapping = resolve_mapping_override(resource, &apply_mappings.characters)?;
    let target_account = override_mapping
        .and_then(|item| item.target_account.clone())
        .or_else(|| apply_mappings.target_account.clone());
    let target_server = override_mapping
        .map(|item| item.target_server.clone())
        .or_else(|| {
            if single_character_bundle {
                apply_mappings.target_server.clone()
            } else {
                None
            }
        });
    let target_character = override_mapping
        .map(|item| item.target_character.clone())
        .or_else(|| {
            if single_character_bundle {
                apply_mappings.target_character.clone()
            } else {
                None
            }
        });

    let mut missing_fields = Vec::new();
    if target_account.is_none() {
        missing_fields.push("target_account");
    }
    if target_server.is_none() {
        missing_fields.push("target_server");
    }
    if target_character.is_none() {
        missing_fields.push("target_character");
    }

    if !missing_fields.is_empty() {
        return Err(AppError::Validation(
            format_character_mapping_resolution_error(
                character_mode,
                resource,
                single_character_bundle,
                &missing_fields,
            ),
        ));
    }

    let target_account = target_account.expect("validated target account");
    let target_server = target_server.expect("validated target server");
    let target_character = target_character.expect("validated target character");

    validate_plain_name("target account", &target_account)?;
    validate_plain_name("target server", &target_server)?;
    validate_plain_name("target character", &target_character)?;

    Ok(CharacterMapping {
        source_account,
        source_server: resource.source_server.clone(),
        source_character: resource.source_character.clone(),
        target_account,
        target_server,
        target_character,
    })
}

fn format_character_mapping_resolution_error(
    character_mode: CharacterMappingMode,
    resource: &CharacterResource,
    single_character_bundle: bool,
    missing_fields: &[&str],
) -> String {
    let mode_message = match character_mode {
        CharacterMappingMode::KeepOriginal => "keep_original should not require target identity",
        CharacterMappingMode::Explicit => {
            "explicit character mode requires a fully resolved target identity"
        }
        CharacterMappingMode::Prompt => {
            "prompt character mode requires caller-provided target identity because the current CLI does not prompt automatically"
        }
    };
    let resolution = if single_character_bundle {
        "Provide `--target-account`, `--target-server`, and `--target-character`, or use `--mapping-file`."
    } else {
        "Provide per-character mappings with `--mapping-file`."
    };
    let hint = resource
        .target_hint
        .as_deref()
        .map(|hint| format!(" Hint: {hint}."))
        .unwrap_or_default();

    format!(
        "{mode_message} for `{}/{}` (missing: {}). {resolution}{hint}",
        resource.source_server,
        resource.source_character,
        missing_fields.join(", "),
    )
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

    if manifest.mapping.character_mode != CharacterMappingMode::KeepOriginal
        && let Some(target_account) = &apply_mappings.target_account
    {
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
