mod addon_lock;
mod apply_policy;
mod archive_io;
mod entry_plan;
mod execution;
mod packing;
mod shared;
mod target_resolution;
#[cfg(test)]
mod tests;

use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use self::addon_lock::ExtractedAddonLock;
pub use self::addon_lock::{apply_bundle_addon_lock, plan_bundle_addon_lock};
use self::apply_policy::{
    apply_action_order, apply_group_order, build_cleanup_operations, cleanup_scope_for_entry,
    resource_policy_for_group,
};
use self::archive_io::{
    add_bundle_addon_sources_to_zip, add_character_wtf_to_zip, add_common_wtf_to_zip,
    add_path_to_zip, collect_bundle_entry_names, count_bundle_entries, extract_embedded_addon_lock,
    read_bundle_entry_bytes_from_archive, read_generated_addon_lock, read_manifest_from_archive,
    resolve_addon_index_paths, resolve_character_account, write_toml_to_zip,
};
use self::entry_plan::plan_extractable_entries;
use self::execution::{
    execute_apply_operations, file_contents_equal_to_bytes, rollback_or_report_apply_error,
};
pub use self::packing::{inspect_bundle, load_apply_mappings, pack_bundle};
use self::shared::{
    BundleAddonSourceEntry, BundleAddonSourceIndex, join_segments, safe_file_part,
    safe_zip_segments, should_skip_path, to_zip_path, validate_plain_name, zip_dir_options,
    zip_file_options,
};
use self::target_resolution::{
    build_character_mappings, resolve_selected_target_accounts, validate_target_compatibility,
};
use crate::core::addon::lock::{
    AddonLock, AddonLockApplyResult, AddonLockPackage, AddonLockPlanResult,
    AddonLockSourceOverride, addon_lock_package_comparison_key, write_addon_lock,
};
use crate::core::backup::{BackupGroup, BackupRequest, create_backup, restore_backup};
use crate::core::error::{AppError, AppResult};
use crate::core::install::{DetectedFlavorInstallation, LocalWowAccount, discover_local_accounts};
use crate::core::lua_patch::{CharacterMapping, LuaRewriteOptions, preview_lua_bytes_rewrite};
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

pub fn plan_bundle_apply(
    bundle_path: &Path,
    installation: &DetectedFlavorInstallation,
    apply_mappings: &BundleApplyMappings,
) -> AppResult<BundleApplyPlan> {
    Ok(prepare_bundle_apply(bundle_path, installation, apply_mappings)?.plan)
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
