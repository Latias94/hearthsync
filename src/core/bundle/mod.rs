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
    preview: ApplyOperation,
    rewrites: Vec<CharacterMapping>,
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
    execution_plan: BundleExecutionPlan,
}

struct BundleExecutionPlan {
    operations: Vec<PreparedApplyOperation>,
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
            let operation = ApplyOperation {
                group: cleanup.group,
                wtf_scope: None,
                action: ApplyAction::Remove,
                archive_name: format!("[cleanup] {}", cleanup.destination.display()),
                destination: cleanup.destination,
                target_account: cleanup.target_account,
                target_server: cleanup.target_server,
                target_character: cleanup.target_character,
                rewrite_count: 0,
                rewrite_applied: false,
            };
            summary.paths_to_remove += 1;
            execution_operations.push(PreparedApplyOperation {
                preview: operation,
                rewrites: Vec::new(),
            });
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

            let operation = ApplyOperation {
                group: entry.group,
                wtf_scope: entry.wtf_scope,
                action,
                archive_name: entry.archive_name.clone(),
                destination: entry.destination.clone(),
                target_account: entry.target_account.clone(),
                target_server: entry.target_server.clone(),
                target_character: entry.target_character.clone(),
                rewrite_count: entry.rewrites.len(),
                rewrite_applied,
            };
            execution_operations.push(PreparedApplyOperation {
                preview: operation,
                rewrites: entry.rewrites.clone(),
            });
        }

        execution_operations.sort_by(|left, right| {
            apply_action_order(left.preview.action)
                .cmp(&apply_action_order(right.preview.action))
                .then_with(|| {
                    apply_group_order(left.preview.group)
                        .cmp(&apply_group_order(right.preview.group))
                })
                .then_with(|| left.preview.destination.cmp(&right.preview.destination))
                .then_with(|| left.preview.archive_name.cmp(&right.preview.archive_name))
        });
        let operations = execution_operations
            .iter()
            .map(|operation| operation.preview.clone())
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
            execution_plan: BundleExecutionPlan {
                operations: execution_operations,
            },
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
        execution_plan,
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
    .execute(&plan, &execution_plan)?;

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
        execution_plan: &BundleExecutionPlan,
    ) -> AppResult<BundleExecution> {
        let backup_path = self.create_backup(plan)?;

        match execute_apply_operations(&plan.bundle_path, execution_plan, &plan.manifest) {
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

fn build_cleanup_operations(
    planned_entries: &[PlannedEntry],
    manifest: &BundleManifest,
    installation: &DetectedFlavorInstallation,
) -> AppResult<Vec<PlannedCleanup>> {
    let mut cleanup_roots = BTreeMap::<PathBuf, PlannedCleanup>::new();

    for entry in planned_entries {
        let policy = resource_policy_for_group(manifest, entry.group);
        if !policy_requires_cleanup(policy) {
            continue;
        }

        let Some(destination) = cleanup_scope_for_entry(entry, installation)? else {
            continue;
        };
        if !destination.exists() {
            continue;
        }

        cleanup_roots
            .entry(destination.clone())
            .or_insert_with(|| PlannedCleanup {
                group: entry.group,
                destination,
                target_account: entry.target_account.clone(),
                target_server: entry.target_server.clone(),
                target_character: entry.target_character.clone(),
            });
    }

    Ok(cleanup_roots.into_values().collect())
}

fn cleanup_scope_for_entry(
    entry: &PlannedEntry,
    installation: &DetectedFlavorInstallation,
) -> AppResult<Option<PathBuf>> {
    match entry.group {
        ApplyGroup::Addons => {
            let relative = entry
                .destination
                .strip_prefix(&installation.addon_dir)
                .map_err(|error| AppError::Validation(error.to_string()))?;
            let mut components = relative.components();
            let Some(component) = components.next() else {
                return Ok(None);
            };
            Ok(Some(installation.addon_dir.join(component.as_os_str())))
        }
        ApplyGroup::WtfCommon => match entry.wtf_scope.unwrap_or(WtfScope::Unknown) {
            WtfScope::GlobalConfig => Ok(Some(installation.wtf_dir.join("Config.wtf"))),
            WtfScope::AccountSavedVariables => {
                let target_account = entry.target_account.as_ref().ok_or_else(|| {
                    AppError::Validation(
                        "wtf common cleanup root requires a target account".to_string(),
                    )
                })?;
                Ok(Some(
                    installation
                        .wtf_dir
                        .join("Account")
                        .join(target_account)
                        .join("SavedVariables"),
                ))
            }
            WtfScope::AccountRootFile | WtfScope::CacheLike | WtfScope::Unknown => {
                Ok(Some(entry.destination.clone()))
            }
            WtfScope::CharacterSavedVariables | WtfScope::CharacterState => {
                Err(AppError::Validation(
                    "character WTF scope cannot be used for common WTF cleanup".to_string(),
                ))
            }
        },
        ApplyGroup::WtfCharacters => {
            let target_account = entry.target_account.as_ref().ok_or_else(|| {
                AppError::Validation(
                    "wtf character cleanup root requires a target account".to_string(),
                )
            })?;
            let target_server = entry.target_server.as_ref().ok_or_else(|| {
                AppError::Validation(
                    "wtf character cleanup root requires a target server".to_string(),
                )
            })?;
            let target_character = entry.target_character.as_ref().ok_or_else(|| {
                AppError::Validation(
                    "wtf character cleanup root requires a target character".to_string(),
                )
            })?;
            Ok(Some(
                installation
                    .wtf_dir
                    .join("Account")
                    .join(target_account)
                    .join(target_server)
                    .join(target_character),
            ))
        }
        ApplyGroup::Fonts => Ok(Some(installation.fonts_dir.clone())),
        ApplyGroup::InterfaceAssets => {
            let relative = entry
                .destination
                .strip_prefix(&installation.interface_dir)
                .map_err(|error| AppError::Validation(error.to_string()))?;
            let mut components = relative.components();
            let Some(component) = components.next() else {
                return Ok(None);
            };
            Ok(Some(installation.interface_dir.join(component.as_os_str())))
        }
        ApplyGroup::Metadata => Ok(None),
    }
}

fn resource_policy_for_group(manifest: &BundleManifest, group: ApplyGroup) -> ResourceApplyPolicy {
    match group {
        ApplyGroup::Addons => manifest.apply.addons,
        ApplyGroup::WtfCommon => manifest.apply.wtf_common,
        ApplyGroup::WtfCharacters => manifest.apply.wtf_characters,
        ApplyGroup::Fonts => manifest.apply.fonts,
        ApplyGroup::InterfaceAssets => manifest.apply.interface_assets,
        ApplyGroup::Metadata => ResourceApplyPolicy::Merge,
    }
}

fn policy_requires_cleanup(policy: ResourceApplyPolicy) -> bool {
    matches!(
        policy,
        ResourceApplyPolicy::Sync
            | ResourceApplyPolicy::Mirror
            | ResourceApplyPolicy::ReplaceSelected
    )
}

fn apply_action_order(action: ApplyAction) -> u8 {
    match action {
        ApplyAction::Remove => 0,
        ApplyAction::Add => 1,
        ApplyAction::Replace => 2,
        ApplyAction::Skip => 3,
        ApplyAction::Preserve => 4,
    }
}

fn apply_group_order(group: ApplyGroup) -> u8 {
    match group {
        ApplyGroup::Addons => 0,
        ApplyGroup::InterfaceAssets => 1,
        ApplyGroup::Fonts => 2,
        ApplyGroup::WtfCommon => 3,
        ApplyGroup::WtfCharacters => 4,
        ApplyGroup::Metadata => 5,
    }
}

fn execute_apply_operations(
    bundle_path: &Path,
    execution_plan: &BundleExecutionPlan,
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

    for (operation_index, operation) in execution_plan.operations.iter().enumerate() {
        if matches!(
            operation.preview.action,
            ApplyAction::Skip | ApplyAction::Preserve
        ) {
            continue;
        }

        if operation.preview.action == ApplyAction::Remove {
            remove_target_path(&operation.preview.destination)?;
            continue;
        }

        if let Some(parent) = operation.preview.destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let source_path = if operation.preview.rewrite_applied {
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
                &operation.preview.archive_name,
                &mut archive,
                rewrite_stage.path(),
            )?
        };
        fs::copy(source_path, &operation.preview.destination)?;
        written_files += 1;

        if operation.preview.rewrite_applied {
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
        &operation.preview.archive_name,
        archive,
        rewrite_stage_root,
    )?;
    rewrite_lua_file(&rewrite_path, &operation.rewrites, rewrite_options)?;
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

fn plan_extractable_entries(
    entry_names: &[String],
    installation: &DetectedFlavorInstallation,
    manifest: &BundleManifest,
    character_mappings: &[CharacterMapping],
    apply_mappings: &BundleApplyMappings,
    selected_target_accounts: &[String],
) -> AppResult<Vec<PlannedEntry>> {
    let mut planned_entries = Vec::new();
    let common_account_targets = resolve_common_account_targets(
        manifest,
        character_mappings,
        apply_mappings,
        selected_target_accounts,
    )?;

    for archive_name in entry_names {
        let entries = map_bundle_entry_to_destination(
            archive_name,
            installation,
            manifest,
            character_mappings,
            &common_account_targets,
            apply_mappings.target_account.as_deref(),
            selected_target_accounts,
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
) -> AppResult<Vec<PlannedEntry>> {
    if archive_name == MANIFEST_ENTRY {
        return Ok(Vec::new());
    }

    let segments = safe_zip_segments(archive_name)?;
    if segments.is_empty() {
        return Ok(Vec::new());
    }

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
            wtf_scope: None,
            target_account: None,
            target_server: None,
            target_character: None,
        }]),
        ["addons", rest @ ..] if !rest.is_empty() => Ok(vec![PlannedEntry {
            archive_name: archive_name.to_string(),
            destination: join_segments(&installation.addon_dir, rest),
            rewrites: Vec::new(),
            group: ApplyGroup::Addons,
            wtf_scope: None,
            target_account: None,
            target_server: None,
            target_character: None,
        }]),
        ["wtf", "common", "Config.wtf"] => Ok(vec![PlannedEntry {
            archive_name: archive_name.to_string(),
            destination: installation.wtf_dir.join("Config.wtf"),
            rewrites: Vec::new(),
            group: ApplyGroup::WtfCommon,
            wtf_scope: Some(WtfScope::GlobalConfig),
            target_account: None,
            target_server: None,
            target_character: None,
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
                    wtf_scope: Some(WtfScope::AccountSavedVariables),
                    target_account: Some(target_account),
                    target_server: None,
                    target_character: None,
                })
                .collect())
        }
        ["wtf", "common", "accounts", source_account, rest @ ..] if !rest.is_empty() => {
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
                        .join(join_segments(Path::new(""), rest)),
                    rewrites: character_mappings
                        .iter()
                        .filter(|mapping| mapping.target_account == target_account)
                        .cloned()
                        .collect::<Vec<_>>(),
                    group: ApplyGroup::WtfCommon,
                    wtf_scope: Some(classify_account_wtf_scope(rest)),
                    target_account: Some(target_account),
                    target_server: None,
                    target_character: None,
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
                wtf_scope: Some(classify_character_wtf_scope(rest)),
                target_account: Some(mapping.target_account),
                target_server: Some(mapping.target_server),
                target_character: Some(mapping.target_character),
            }])
        }
        ["fonts", rest @ ..] if !rest.is_empty() => Ok(vec![PlannedEntry {
            archive_name: archive_name.to_string(),
            destination: join_segments(&installation.fonts_dir, rest),
            rewrites: Vec::new(),
            group: ApplyGroup::Fonts,
            wtf_scope: None,
            target_account: None,
            target_server: None,
            target_character: None,
        }]),
        ["interface", rest @ ..] if !rest.is_empty() => Ok(vec![PlannedEntry {
            archive_name: archive_name.to_string(),
            destination: join_segments(&installation.interface_dir, rest),
            rewrites: Vec::new(),
            group: ApplyGroup::InterfaceAssets,
            wtf_scope: None,
            target_account: None,
            target_server: None,
            target_character: None,
        }]),
        _ => Ok(Vec::new()),
    }
}

fn classify_account_wtf_scope(relative_segments: &[&str]) -> WtfScope {
    if relative_segments.is_empty() {
        return WtfScope::Unknown;
    }

    if is_saved_variables_segment(relative_segments[0]) {
        WtfScope::AccountSavedVariables
    } else if relative_segments
        .last()
        .is_some_and(|name| is_cache_like_wtf_file_name(name))
    {
        WtfScope::CacheLike
    } else {
        WtfScope::AccountRootFile
    }
}

fn classify_character_wtf_scope(relative_segments: &[&str]) -> WtfScope {
    if relative_segments.is_empty() {
        return WtfScope::Unknown;
    }

    if is_saved_variables_segment(relative_segments[0]) {
        WtfScope::CharacterSavedVariables
    } else if relative_segments
        .last()
        .is_some_and(|name| is_cache_like_wtf_file_name(name))
    {
        WtfScope::CacheLike
    } else {
        WtfScope::CharacterState
    }
}

fn is_saved_variables_segment(segment: &str) -> bool {
    segment.eq_ignore_ascii_case("SavedVariables")
}

fn is_cache_like_wtf_file_name(file_name: &str) -> bool {
    let file_name = file_name.to_ascii_lowercase();
    matches!(
        file_name.as_str(),
        "bindings-cache.wtf" | "chat-cache.txt" | "config-cache.wtf" | "macros-cache.txt"
    ) || file_name.ends_with("-cache.wtf")
        || file_name.ends_with("-cache.txt")
        || file_name.ends_with("-cache.old")
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
    use crate::core::addon::lock::plan_addon_lock_sync;
    use crate::core::addon::{InstallAddonRequest, install_addon};
    use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};
    use crate::core::manifest::{
        ApplyDefaults, BundleManifest, BundleResources, CharacterMappingMode, CharacterResource,
        MappingRules, PackageMetadata, ResourceApplyPolicy, SourceInstallation,
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
            .filter(|operation| operation.action == super::ApplyAction::Add)
        {
            if groups.last().copied() != Some(operation.group) {
                groups.push(operation.group);
            }
        }

        assert_eq!(
            groups,
            vec![
                super::ApplyGroup::Addons,
                super::ApplyGroup::InterfaceAssets,
                super::ApplyGroup::Fonts,
                super::ApplyGroup::WtfCommon,
                super::ApplyGroup::WtfCharacters,
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
            Some(super::WtfScope::GlobalConfig)
        );
        assert_eq!(
            scope_for("wtf/common/accounts/ACCOUNT/account-settings.wtf"),
            Some(super::WtfScope::AccountRootFile)
        );
        assert_eq!(
            scope_for("wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua"),
            Some(super::WtfScope::AccountSavedVariables)
        );
        assert_eq!(
            scope_for("wtf/characters/ACCOUNT/Illidan/Examplemage/SavedVariables/Pawn.lua"),
            Some(super::WtfScope::CharacterSavedVariables)
        );
        assert_eq!(
            scope_for("wtf/characters/ACCOUNT/Illidan/Examplemage/AddOns.txt"),
            Some(super::WtfScope::CharacterState)
        );
        assert_eq!(
            scope_for("wtf/common/accounts/ACCOUNT/config-cache.wtf"),
            Some(super::WtfScope::CacheLike)
        );
        assert_eq!(
            scope_for("wtf/characters/ACCOUNT/Illidan/Examplemage/config-cache.wtf"),
            Some(super::WtfScope::CacheLike)
        );
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
        assert_eq!(
            plan.group_policies.addons.policy,
            ResourceApplyPolicy::Merge
        );
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
        let mut manifest = sample_manifest_with_rewrite();
        manifest.mapping.character_mode = CharacterMappingMode::Explicit;

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
                .all(|operation| operation.action == super::ApplyAction::Preserve)
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
                && operation.action == super::ApplyAction::Preserve
        }));
        assert!(plan.operations.iter().any(|operation| {
            operation.archive_name == "wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua"
                && operation.action == super::ApplyAction::Add
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
            fs::read_to_string(target_installation.wtf_dir.join("Config.wtf"))
                .expect("target config"),
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
            operation.action == super::ApplyAction::Remove
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
            operation.action == super::ApplyAction::Remove
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
        manifest.apply.addons = ResourceApplyPolicy::Mirror;

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
        assert!(archive.by_name("metadata/addons/sources.toml").is_ok());
        assert!(
            archive
                .by_name("metadata/addons/sources/addons-weakauras.zip")
                .is_ok()
        );
        assert!(
            archive
                .by_name("metadata/addons/indexes/addon-index.toml")
                .is_ok()
        );

        let inspection = inspect_bundle(&bundle.archive_path).expect("inspect bundle");
        assert_eq!(inspection.entries.metadata, 5);
        assert_eq!(
            inspection.manifest.apply.addons,
            ResourceApplyPolicy::Mirror
        );
        fs::remove_file(&archive_path).expect("remove original addon source");

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

        let sidecar_plan = plan_addon_lock_sync(
            &target_installation,
            Some(&sidecar_root.join("addons").join("lock.toml")),
        )
        .expect("sidecar addon plan");
        assert_eq!(sidecar_plan.install_count, 1);
        assert_eq!(sidecar_plan.blocked_count, 0);

        let addon_plan =
            plan_bundle_addon_lock(&bundle.archive_path, &target_installation).expect("addon plan");
        assert_eq!(addon_plan.plan.install_count, 1);
        assert_eq!(addon_plan.plan.update_count, 0);
        assert_eq!(addon_plan.plan.remove_count, 0);
        assert_eq!(addon_plan.plan.blocked_count, 0);

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
                character_mode: CharacterMappingMode::KeepOriginal,
                rewrite_profile_keys: false,
                rewrite_identity_strings: false,
                allow_cross_platform: true,
            },
            apply: ApplyDefaults {
                create_backup: true,
                addons: ResourceApplyPolicy::Merge,
                wtf_common: ResourceApplyPolicy::Merge,
                wtf_characters: ResourceApplyPolicy::Merge,
                fonts: ResourceApplyPolicy::Merge,
                interface_assets: ResourceApplyPolicy::Merge,
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
