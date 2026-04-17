use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use tempfile::{TempDir, tempdir};
use walkdir::WalkDir;
use zip::ZipArchive;

use super::wtf_scope::{classify_account_wtf_scope, classify_character_wtf_scope};
use super::*;
use crate::core::addon_layout::discover_addon_roots_from_entry_segments;
use crate::core::archive_io::copy_reader_to_path;
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};
use crate::core::manifest::{
    ApplyDefaults, BundleManifest, BundleResources, CharacterMappingMode, CharacterResource,
    MappingRules, PackageMetadata, ResourceApplyPolicy, SourceInstallation,
};
use crate::core::task::{
    CancellationToken, NeverCancel, NoopProgressSink, TaskKind, TaskPhase, TaskProgressSink,
    emit_task_progress, ensure_task_not_cancelled,
};

#[derive(Debug, Clone)]
pub struct AnalyzeExternalPackageRequest {
    pub source_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct CreateExternalPackageBundleRequest {
    pub source_path: PathBuf,
    pub source_flavor: WowFlavor,
    pub source_platform: Option<HostPlatform>,
    pub supported_targets: Vec<WowFlavor>,
    pub output_path: Option<PathBuf>,
    pub package_id: Option<String>,
    pub package_name: Option<String>,
    pub created_by: Option<String>,
    pub description: Option<String>,
    pub apply_defaults: Option<ApplyDefaults>,
}

#[derive(Debug, Clone)]
pub struct PlanExternalPackageApplyRequest {
    pub external_package: CreateExternalPackageBundleRequest,
    pub installation: DetectedFlavorInstallation,
    pub apply_mappings: BundleApplyMappings,
}

#[derive(Debug, Clone)]
pub struct ApplyExternalPackageRequest {
    pub external_package: CreateExternalPackageBundleRequest,
    pub installation: DetectedFlavorInstallation,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub apply_mappings: BundleApplyMappings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalPackageSourceKind {
    Directory,
    ZipArchive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalPackageWarningCategory {
    Addon,
    Wtf,
}

impl ExternalPackageWarningCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Addon => "addon",
            Self::Wtf => "wtf",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalPackageWarningCode {
    AddonRootNotDetected,
    UnsupportedWtfLayout,
    #[serde(rename = "unsupported_wtf_root_savedvariables")]
    UnsupportedWtfRootSavedVariables,
    WtfAccountPathWithoutFile,
    #[serde(rename = "wtf_savedvariables_path_without_file")]
    WtfSavedVariablesPathWithoutFile,
    UnsupportedWtfNestedAccountLayout,
}

impl ExternalPackageWarningCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AddonRootNotDetected => "addon_root_not_detected",
            Self::UnsupportedWtfLayout => "unsupported_wtf_layout",
            Self::UnsupportedWtfRootSavedVariables => "unsupported_wtf_root_savedvariables",
            Self::WtfAccountPathWithoutFile => "wtf_account_path_without_file",
            Self::WtfSavedVariablesPathWithoutFile => "wtf_savedvariables_path_without_file",
            Self::UnsupportedWtfNestedAccountLayout => "unsupported_wtf_nested_account_layout",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ExternalPackageWarning {
    pub category: ExternalPackageWarningCategory,
    pub code: ExternalPackageWarningCode,
    pub source_path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ExternalPackageWarningGroup {
    pub category: ExternalPackageWarningCategory,
    pub code: ExternalPackageWarningCode,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageAnalysis {
    pub source_path: PathBuf,
    pub source_kind: ExternalPackageSourceKind,
    pub package_id: String,
    pub package_name: String,
    pub entries: Vec<ExternalPackageEntry>,
    pub resources: BundleResources,
    pub summary: ExternalPackageSummary,
    pub warnings: Vec<ExternalPackageWarning>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageEntry {
    pub source_path: String,
    pub normalized_path: String,
    pub group: ApplyGroup,
    pub wtf_scope: Option<WtfScope>,
    pub source_account: Option<String>,
    pub source_server: Option<String>,
    pub source_character: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ExternalPackageSummary {
    pub total_files: usize,
    pub normalized_files: usize,
    pub ignored_files: usize,
    pub addons: usize,
    pub wtf_common: usize,
    pub wtf_characters: usize,
    pub fonts: usize,
    pub interface_assets: usize,
    pub warning_count: usize,
    pub addon_warning_count: usize,
    pub wtf_warning_count: usize,
    pub warning_groups: Vec<ExternalPackageWarningGroup>,
}

#[derive(Debug)]
pub struct PreparedExternalPackageBundle {
    pub analysis: ExternalPackageAnalysis,
    pub manifest: BundleManifest,
    pub bundle: CreatedBundle,
    _stage_dir: TempDir,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageApplyPlan {
    pub analysis: ExternalPackageAnalysis,
    pub target_flavor_root: PathBuf,
    pub discovered_accounts: Vec<crate::core::install::LocalWowAccount>,
    pub selected_target_accounts: Vec<String>,
    pub character_mappings: Vec<crate::core::lua_patch::CharacterMapping>,
    pub operations: Vec<ApplyOperation>,
    pub summary: ApplyPlanSummary,
    pub helper_strategy: HelperStrategy,
    pub group_policies: ApplyGroupPolicies,
    pub manifest: BundleManifest,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppliedExternalPackage {
    pub analysis: ExternalPackageAnalysis,
    pub target_flavor_root: PathBuf,
    pub dry_run: bool,
    pub planned_files: usize,
    pub written_files: usize,
    pub rewritten_files: usize,
    pub backup_path: Option<PathBuf>,
    pub selected_target_accounts: Vec<String>,
    pub plan_summary: ApplyPlanSummary,
    pub character_mappings: Vec<crate::core::lua_patch::CharacterMapping>,
    pub manifest: BundleManifest,
}

#[derive(Debug, Clone)]
struct SourceEntry {
    source_path: String,
    segments: Vec<String>,
}

#[derive(Debug)]
struct PreparedExternalPackageApply {
    analysis: ExternalPackageAnalysis,
    prepared_apply: PreparedBundleApply,
}

pub fn analyze_external_package(
    request: AnalyzeExternalPackageRequest,
) -> AppResult<ExternalPackageAnalysis> {
    let source_path = request.source_path;
    if !source_path.exists() {
        return Err(AppError::NotFound(format!(
            "external package source does not exist: {}",
            source_path.display()
        )));
    }

    let source_kind = detect_source_kind(&source_path)?;
    let source_entries = collect_source_entries(&source_path, source_kind)?;
    let addon_roots = discover_addon_roots_from_entry_segments(
        source_entries.iter().map(|entry| entry.segments.as_slice()),
    );
    let mut entries = Vec::new();
    let mut warnings = Vec::new();

    for source_entry in &source_entries {
        if let Some(addon_entry) = classify_addon_entry(source_entry, &addon_roots) {
            entries.push(addon_entry);
            continue;
        }

        match classify_non_addon_entry(source_entry) {
            ClassifiedExternalEntry::Recognized(entry) => entries.push(entry),
            ClassifiedExternalEntry::Ignored => {}
            ClassifiedExternalEntry::Warn(message) => warnings.push(message),
        }
    }

    entries.sort_by(|left, right| {
        left.normalized_path
            .cmp(&right.normalized_path)
            .then_with(|| left.source_path.cmp(&right.source_path))
    });
    warnings.sort();
    warnings.dedup();

    let summary = build_summary(source_entries.len(), &entries, &warnings);
    let resources = build_resources(&entries);

    Ok(ExternalPackageAnalysis {
        package_id: package_id_from_source_path(&source_path),
        package_name: package_name_from_source_path(&source_path),
        source_path,
        source_kind,
        entries,
        resources,
        summary,
        warnings,
    })
}

pub fn analyze_external_package_task<TCancel, TProgress>(
    request: AnalyzeExternalPackageRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<ExternalPackageAnalysis>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    emit_task_progress(
        progress,
        TaskKind::ExternalPackageAnalyze,
        TaskPhase::Preparing,
        format!(
            "Inspecting external package source `{}`",
            request.source_path.display()
        ),
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::ExternalPackageAnalyze,
        TaskPhase::Preparing,
    )?;
    emit_task_progress(
        progress,
        TaskKind::ExternalPackageAnalyze,
        TaskPhase::Planning,
        "Classifying external package resources and warnings",
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::ExternalPackageAnalyze,
        TaskPhase::Planning,
    )?;

    let analysis = analyze_external_package(request)?;
    emit_task_progress(
        progress,
        TaskKind::ExternalPackageAnalyze,
        TaskPhase::Completed,
        format!(
            "External package analysis completed with {} normalized file(s) and {} warning(s)",
            analysis.summary.normalized_files, analysis.summary.warning_count
        ),
    );
    Ok(analysis)
}

pub fn create_external_package_bundle(
    request: CreateExternalPackageBundleRequest,
) -> AppResult<PreparedExternalPackageBundle> {
    let (analysis, manifest) = prepare_external_package_artifacts(&request)?;

    let stage_dir = tempdir()?;
    let staged_installation = create_staging_installation(
        stage_dir.path(),
        request.source_flavor,
        request
            .source_platform
            .unwrap_or_else(HostPlatform::current),
    )?;
    materialize_analysis_to_installation(&analysis, &staged_installation)?;

    let output_path = request
        .output_path
        .clone()
        .or_else(|| Some(stage_dir.path().join("external-package.bundle.zip")));
    let bundle = pack_bundle(PackBundleRequest {
        installation: staged_installation,
        manifest: manifest.clone(),
        output_path,
        manifest_base_dir: None,
    })?;

    Ok(PreparedExternalPackageBundle {
        analysis,
        manifest,
        bundle,
        _stage_dir: stage_dir,
    })
}

pub fn plan_external_package_apply(
    request: PlanExternalPackageApplyRequest,
) -> AppResult<ExternalPackageApplyPlan> {
    let prepared = prepare_external_package_apply(
        request.external_package,
        &request.installation,
        &request.apply_mappings,
    )?;
    Ok(project_external_package_plan(
        prepared.analysis,
        prepared.prepared_apply.plan,
    ))
}

pub fn plan_external_package_apply_task<TCancel, TProgress>(
    request: PlanExternalPackageApplyRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<ExternalPackageApplyPlan>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    emit_task_progress(
        progress,
        TaskKind::ExternalPackagePlan,
        TaskPhase::Preparing,
        format!(
            "Normalizing external package `{}` for planning",
            request.external_package.source_path.display()
        ),
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::ExternalPackagePlan,
        TaskPhase::Preparing,
    )?;
    emit_task_progress(
        progress,
        TaskKind::ExternalPackagePlan,
        TaskPhase::Planning,
        "Building apply plan for normalized external package",
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::ExternalPackagePlan,
        TaskPhase::Planning,
    )?;

    let plan = plan_external_package_apply(request)?;
    emit_task_progress(
        progress,
        TaskKind::ExternalPackagePlan,
        TaskPhase::Completed,
        format!(
            "External package plan completed with {} operation(s)",
            plan.operations.len()
        ),
    );
    Ok(plan)
}

pub fn apply_external_package(
    request: ApplyExternalPackageRequest,
) -> AppResult<AppliedExternalPackage> {
    let cancellation = NeverCancel;
    let mut progress = NoopProgressSink;
    apply_external_package_task(request, &cancellation, &mut progress)
}

pub fn apply_external_package_task<TCancel, TProgress>(
    request: ApplyExternalPackageRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<AppliedExternalPackage>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    emit_task_progress(
        progress,
        TaskKind::ExternalPackageApply,
        TaskPhase::Preparing,
        format!(
            "Normalizing external package `{}` for direct apply",
            request.external_package.source_path.display()
        ),
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::ExternalPackageApply,
        TaskPhase::Preparing,
    )?;

    let prepared = prepare_external_package_apply(
        request.external_package,
        &request.installation,
        &request.apply_mappings,
    )?;
    let result = super::apply::execute_prepared_apply_with_context(
        prepared.prepared_apply,
        request.installation,
        request.dry_run,
        request.backup_output_path,
        cancellation,
        progress,
        super::apply::BundleApplyTaskContext::ExternalPackageApply,
    )?;

    Ok(project_applied_external_package(prepared.analysis, result))
}

fn prepare_external_package_artifacts(
    request: &CreateExternalPackageBundleRequest,
) -> AppResult<(ExternalPackageAnalysis, BundleManifest)> {
    let analysis = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: request.source_path.clone(),
    })?;
    validate_unique_normalized_paths(&analysis)?;

    let manifest = build_external_manifest(&analysis, request)?;
    manifest.validate()?;

    Ok((analysis, manifest))
}

fn prepare_external_package_apply(
    external_package: CreateExternalPackageBundleRequest,
    installation: &DetectedFlavorInstallation,
    apply_mappings: &BundleApplyMappings,
) -> AppResult<PreparedExternalPackageApply> {
    let (analysis, manifest) = prepare_external_package_artifacts(&external_package)?;
    let entry_source_map = build_external_package_entry_source_map(&analysis)?;
    let entry_names = entry_source_map.keys().cloned().collect::<Vec<_>>();
    let source_path = analysis.source_path.clone();
    let source_kind = analysis.source_kind;

    let prepared_apply = match source_kind {
        ExternalPackageSourceKind::Directory => super::planner::prepare_apply_from_entries(
            &source_path,
            installation,
            manifest,
            &entry_names,
            apply_mappings,
            PreparedApplySource::ExternalPackage {
                source_path: source_path.clone(),
                source_kind,
            },
            |normalized_path| {
                let source_entry =
                    lookup_external_package_source_path(&entry_source_map, normalized_path)?;
                let resolved_path = resolve_zip_style_path(&source_path, source_entry)?;
                Ok(fs::read(resolved_path)?)
            },
            |normalized_path| {
                Ok(Some(
                    lookup_external_package_source_path(&entry_source_map, normalized_path)?
                        .to_string(),
                ))
            },
        )?,
        ExternalPackageSourceKind::ZipArchive => {
            let file = File::open(&source_path)?;
            let mut archive = ZipArchive::new(file)?;
            super::planner::prepare_apply_from_entries(
                &source_path,
                installation,
                manifest,
                &entry_names,
                apply_mappings,
                PreparedApplySource::ExternalPackage {
                    source_path: source_path.clone(),
                    source_kind,
                },
                |normalized_path| {
                    let source_entry =
                        lookup_external_package_source_path(&entry_source_map, normalized_path)?;
                    read_bundle_entry_bytes_from_archive(&mut archive, source_entry)
                },
                |normalized_path| {
                    Ok(Some(
                        lookup_external_package_source_path(&entry_source_map, normalized_path)?
                            .to_string(),
                    ))
                },
            )?
        }
    };

    Ok(PreparedExternalPackageApply {
        analysis,
        prepared_apply,
    })
}

fn detect_source_kind(path: &Path) -> AppResult<ExternalPackageSourceKind> {
    if path.is_dir() {
        return Ok(ExternalPackageSourceKind::Directory);
    }

    let file = File::open(path)?;
    ZipArchive::new(file).map_err(|error| {
        AppError::Validation(format!(
            "external package source is not a valid zip archive: {} ({error})",
            path.display()
        ))
    })?;
    Ok(ExternalPackageSourceKind::ZipArchive)
}

fn collect_source_entries(
    source_path: &Path,
    source_kind: ExternalPackageSourceKind,
) -> AppResult<Vec<SourceEntry>> {
    let mut entries = match source_kind {
        ExternalPackageSourceKind::Directory => collect_directory_entries(source_path)?,
        ExternalPackageSourceKind::ZipArchive => collect_zip_entries(source_path)?,
    };
    entries.sort_by(|left, right| left.source_path.cmp(&right.source_path));
    Ok(entries)
}

fn collect_directory_entries(root: &Path) -> AppResult<Vec<SourceEntry>> {
    let mut entries = Vec::new();

    for entry in WalkDir::new(root) {
        let entry = entry.map_err(|error| AppError::Validation(error.to_string()))?;
        if entry.file_type().is_dir() {
            continue;
        }

        if should_skip_path(entry.path()) {
            continue;
        }

        let relative_path = entry.path().strip_prefix(root).map_err(|_| {
            AppError::Validation(format!(
                "failed to derive relative path for external package entry: {}",
                entry.path().display()
            ))
        })?;
        let segments = safe_relative_segments(relative_path)?;
        if should_ignore_source_segments(&segments) {
            continue;
        }

        entries.push(SourceEntry {
            source_path: to_zip_path(relative_path),
            segments,
        });
    }

    Ok(entries)
}

fn collect_zip_entries(path: &Path) -> AppResult<Vec<SourceEntry>> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut entries = Vec::new();

    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }

        let entry_name = entry.name().to_string();
        let segments = safe_zip_segments(&entry_name)?
            .into_iter()
            .map(|segment| segment.to_string())
            .collect::<Vec<_>>();
        if should_ignore_source_segments(&segments) {
            continue;
        }

        if Path::new(&entry_name)
            .file_name()
            .is_some_and(|name| should_skip_path(Path::new(name)))
        {
            continue;
        }

        entries.push(SourceEntry {
            source_path: entry_name,
            segments,
        });
    }

    Ok(entries)
}

fn safe_relative_segments(relative_path: &Path) -> AppResult<Vec<String>> {
    let mut segments = Vec::new();

    for component in relative_path.components() {
        let Component::Normal(segment) = component else {
            return Err(AppError::Validation(format!(
                "unsafe directory entry path: {}",
                relative_path.display()
            )));
        };
        let segment = segment.to_string_lossy().to_string();
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(AppError::Validation(format!(
                "unsafe directory entry path: {}",
                relative_path.display()
            )));
        }
        segments.push(segment);
    }

    Ok(segments)
}

fn should_ignore_source_segments(segments: &[String]) -> bool {
    segments
        .iter()
        .any(|segment| segment.eq_ignore_ascii_case("__MACOSX"))
}

fn classify_addon_entry(
    source_entry: &SourceEntry,
    addon_roots: &[Vec<String>],
) -> Option<ExternalPackageEntry> {
    let root = addon_roots
        .iter()
        .find(|root| starts_with_segments(&source_entry.segments, root))?;
    let addon_name = root.last()?.clone();
    let relative = &source_entry.segments[root.len()..];
    let normalized_path = join_normalized_segments("addons", &addon_name, relative);

    Some(ExternalPackageEntry {
        source_path: source_entry.source_path.clone(),
        normalized_path,
        group: ApplyGroup::Addons,
        wtf_scope: None,
        source_account: None,
        source_server: None,
        source_character: None,
    })
}

enum ClassifiedExternalEntry {
    Recognized(ExternalPackageEntry),
    Ignored,
    Warn(ExternalPackageWarning),
}

fn classify_non_addon_entry(source_entry: &SourceEntry) -> ClassifiedExternalEntry {
    if let Some(entry) = classify_wtf_entry(source_entry) {
        return entry;
    }

    if let Some(entry) = classify_fonts_entry(source_entry) {
        return ClassifiedExternalEntry::Recognized(entry);
    }

    if let Some(entry) = classify_interface_entry(source_entry) {
        return ClassifiedExternalEntry::Recognized(entry);
    }

    if find_segment_index(&source_entry.segments, "AddOns").is_some() {
        return ClassifiedExternalEntry::Warn(build_external_package_warning(
            ExternalPackageWarningCategory::Addon,
            ExternalPackageWarningCode::AddonRootNotDetected,
            &source_entry.source_path,
            format!(
                "entry is under `AddOns` but no addon root was detected from a `.toc` file: {}",
                source_entry.source_path
            ),
        ));
    }

    ClassifiedExternalEntry::Ignored
}

fn classify_wtf_entry(source_entry: &SourceEntry) -> Option<ClassifiedExternalEntry> {
    let wtf_index = find_segment_index(&source_entry.segments, "WTF")?;
    let suffix = &source_entry.segments[wtf_index..];
    if suffix.len() == 2 && suffix[1].eq_ignore_ascii_case("Config.wtf") {
        return Some(ClassifiedExternalEntry::Recognized(ExternalPackageEntry {
            source_path: source_entry.source_path.clone(),
            normalized_path: "wtf/common/Config.wtf".to_string(),
            group: ApplyGroup::WtfCommon,
            wtf_scope: Some(WtfScope::GlobalConfig),
            source_account: None,
            source_server: None,
            source_character: None,
        }));
    }

    if suffix.len() < 4 || !suffix[1].eq_ignore_ascii_case("Account") {
        return Some(ClassifiedExternalEntry::Warn(
            build_external_package_warning(
                ExternalPackageWarningCategory::Wtf,
                ExternalPackageWarningCode::UnsupportedWtfLayout,
                &source_entry.source_path,
                format!(
                    "WTF path does not match a supported account or character layout: {}",
                    source_entry.source_path
                ),
            ),
        ));
    }

    let account = &suffix[2];
    if account.eq_ignore_ascii_case("SavedVariables") {
        let rest = &suffix[3..];
        if rest.is_empty() {
            return Some(ClassifiedExternalEntry::Warn(
                build_external_package_warning(
                    ExternalPackageWarningCategory::Wtf,
                    ExternalPackageWarningCode::WtfSavedVariablesPathWithoutFile,
                    &source_entry.source_path,
                    format!(
                        "root-level `WTF/Account/SavedVariables` entry does not point to a file: {}",
                        source_entry.source_path
                    ),
                ),
            ));
        }

        return Some(ClassifiedExternalEntry::Recognized(ExternalPackageEntry {
            source_path: source_entry.source_path.clone(),
            normalized_path: join_exact_normalized_segments(
                &["wtf", "common", "root", "SavedVariables"],
                rest,
            ),
            group: ApplyGroup::WtfCommon,
            wtf_scope: Some(WtfScope::RootSavedVariables),
            source_account: None,
            source_server: None,
            source_character: None,
        }));
    }

    let rest = &suffix[3..];
    if rest.is_empty() {
        return Some(ClassifiedExternalEntry::Warn(
            build_external_package_warning(
                ExternalPackageWarningCategory::Wtf,
                ExternalPackageWarningCode::WtfAccountPathWithoutFile,
                &source_entry.source_path,
                format!(
                    "WTF account entry does not point to a file path: {}",
                    source_entry.source_path
                ),
            ),
        ));
    }

    if rest[0].eq_ignore_ascii_case("SavedVariables") {
        if rest.len() < 2 {
            return Some(ClassifiedExternalEntry::Warn(
                build_external_package_warning(
                    ExternalPackageWarningCategory::Wtf,
                    ExternalPackageWarningCode::WtfSavedVariablesPathWithoutFile,
                    &source_entry.source_path,
                    format!(
                        "WTF account `SavedVariables` entry does not point to a file: {}",
                        source_entry.source_path
                    ),
                ),
            ));
        }

        return Some(ClassifiedExternalEntry::Recognized(ExternalPackageEntry {
            source_path: source_entry.source_path.clone(),
            normalized_path: join_exact_normalized_segments(
                &["wtf", "common", "accounts", account],
                rest,
            ),
            group: ApplyGroup::WtfCommon,
            wtf_scope: Some(WtfScope::AccountSavedVariables),
            source_account: Some(account.clone()),
            source_server: None,
            source_character: None,
        }));
    }

    if rest.len() >= 3 {
        let server = &rest[0];
        let character = &rest[1];
        let character_relative = &rest[2..];
        return Some(ClassifiedExternalEntry::Recognized(ExternalPackageEntry {
            source_path: source_entry.source_path.clone(),
            normalized_path: join_exact_normalized_segments(
                &["wtf", "characters", account, server, character],
                character_relative,
            ),
            group: ApplyGroup::WtfCharacters,
            wtf_scope: Some(classify_character_wtf_scope(character_relative)),
            source_account: Some(account.clone()),
            source_server: Some(server.clone()),
            source_character: Some(character.clone()),
        }));
    }

    if rest.len() == 1 {
        return Some(ClassifiedExternalEntry::Recognized(ExternalPackageEntry {
            source_path: source_entry.source_path.clone(),
            normalized_path: join_exact_normalized_segments(
                &["wtf", "common", "accounts", account],
                rest,
            ),
            group: ApplyGroup::WtfCommon,
            wtf_scope: Some(classify_account_wtf_scope(rest)),
            source_account: Some(account.clone()),
            source_server: None,
            source_character: None,
        }));
    }

    Some(ClassifiedExternalEntry::Warn(
        build_external_package_warning(
            ExternalPackageWarningCategory::Wtf,
            ExternalPackageWarningCode::UnsupportedWtfNestedAccountLayout,
            &source_entry.source_path,
            format!(
                "WTF path is nested under an account but does not match a supported file layout: {}",
                source_entry.source_path
            ),
        ),
    ))
}

fn classify_fonts_entry(source_entry: &SourceEntry) -> Option<ExternalPackageEntry> {
    let fonts_index = find_segment_index(&source_entry.segments, "Fonts")?;
    let rest = &source_entry.segments[fonts_index + 1..];
    if rest.is_empty() {
        return None;
    }

    Some(ExternalPackageEntry {
        source_path: source_entry.source_path.clone(),
        normalized_path: join_exact_normalized_segments(&["fonts"], rest),
        group: ApplyGroup::Fonts,
        wtf_scope: None,
        source_account: None,
        source_server: None,
        source_character: None,
    })
}

fn classify_interface_entry(source_entry: &SourceEntry) -> Option<ExternalPackageEntry> {
    let interface_index = find_segment_index(&source_entry.segments, "Interface")?;
    let rest = &source_entry.segments[interface_index + 1..];
    if rest.is_empty() {
        return None;
    }
    if rest[0].eq_ignore_ascii_case("AddOns") {
        return None;
    }

    Some(ExternalPackageEntry {
        source_path: source_entry.source_path.clone(),
        normalized_path: join_exact_normalized_segments(&["interface"], rest),
        group: ApplyGroup::InterfaceAssets,
        wtf_scope: None,
        source_account: None,
        source_server: None,
        source_character: None,
    })
}

fn build_external_package_warning(
    category: ExternalPackageWarningCategory,
    code: ExternalPackageWarningCode,
    source_path: &str,
    message: String,
) -> ExternalPackageWarning {
    ExternalPackageWarning {
        category,
        code,
        source_path: source_path.to_string(),
        message,
    }
}

fn build_summary(
    total_files: usize,
    entries: &[ExternalPackageEntry],
    warnings: &[ExternalPackageWarning],
) -> ExternalPackageSummary {
    let mut warning_groups = BTreeMap::new();
    for warning in warnings {
        *warning_groups
            .entry((warning.category, warning.code))
            .or_insert(0usize) += 1;
    }

    let mut summary = ExternalPackageSummary {
        total_files,
        normalized_files: entries.len(),
        ignored_files: total_files.saturating_sub(entries.len()),
        warning_count: warnings.len(),
        addon_warning_count: warnings
            .iter()
            .filter(|warning| warning.category == ExternalPackageWarningCategory::Addon)
            .count(),
        wtf_warning_count: warnings
            .iter()
            .filter(|warning| warning.category == ExternalPackageWarningCategory::Wtf)
            .count(),
        warning_groups: warning_groups
            .into_iter()
            .map(|((category, code), count)| ExternalPackageWarningGroup {
                category,
                code,
                count,
            })
            .collect(),
        ..ExternalPackageSummary::default()
    };

    for entry in entries {
        match entry.group {
            ApplyGroup::Addons => summary.addons += 1,
            ApplyGroup::WtfCommon => summary.wtf_common += 1,
            ApplyGroup::WtfCharacters => summary.wtf_characters += 1,
            ApplyGroup::Fonts => summary.fonts += 1,
            ApplyGroup::InterfaceAssets => summary.interface_assets += 1,
            ApplyGroup::Metadata => {}
        }
    }

    summary
}

fn build_resources(entries: &[ExternalPackageEntry]) -> BundleResources {
    let mut addons = BTreeSet::new();
    let mut characters = BTreeSet::new();
    let mut interface_assets = BTreeSet::new();
    let mut wtf_common = false;
    let mut fonts = false;

    for entry in entries {
        match entry.group {
            ApplyGroup::Addons => {
                if let Some(addon_name) = normalized_path_tail(&entry.normalized_path, "addons") {
                    addons.insert(addon_name.to_string());
                }
            }
            ApplyGroup::WtfCommon => {
                wtf_common = true;
            }
            ApplyGroup::WtfCharacters => {
                if let (Some(source_account), Some(source_server), Some(source_character)) = (
                    entry.source_account.as_deref(),
                    entry.source_server.as_deref(),
                    entry.source_character.as_deref(),
                ) {
                    characters.insert((
                        source_account.to_string(),
                        source_server.to_string(),
                        source_character.to_string(),
                    ));
                }
            }
            ApplyGroup::Fonts => {
                fonts = true;
            }
            ApplyGroup::InterfaceAssets => {
                if let Some(asset_name) = normalized_path_tail(&entry.normalized_path, "interface")
                {
                    interface_assets.insert(asset_name.to_string());
                }
            }
            ApplyGroup::Metadata => {}
        }
    }

    BundleResources {
        addons: addons.into_iter().collect(),
        wtf_common,
        wtf_characters: characters
            .into_iter()
            .map(
                |(source_account, source_server, source_character)| CharacterResource {
                    source_account: Some(source_account),
                    source_server,
                    source_character,
                    target_hint: None,
                },
            )
            .collect(),
        fonts,
        interface_assets: interface_assets.into_iter().collect(),
        addon_lock: false,
        addon_indexes: Vec::new(),
    }
}

fn validate_unique_normalized_paths(analysis: &ExternalPackageAnalysis) -> AppResult<()> {
    let mut seen = BTreeSet::new();
    let mut case_insensitive_seen = BTreeMap::new();
    for entry in &analysis.entries {
        if !seen.insert(entry.normalized_path.clone()) {
            return Err(AppError::Validation(format!(
                "external package normalizes multiple files onto the same target path: {}",
                entry.normalized_path
            )));
        }

        let folded = entry.normalized_path.to_lowercase();
        if let Some(previous) = case_insensitive_seen.insert(folded, entry.normalized_path.clone())
            && previous != entry.normalized_path
        {
            return Err(AppError::Validation(format!(
                "external package contains case-insensitive target path collisions: `{previous}` and `{}` would map to the same path on Windows/default macOS targets",
                entry.normalized_path
            )));
        }
    }

    Ok(())
}

fn build_external_package_entry_source_map(
    analysis: &ExternalPackageAnalysis,
) -> AppResult<BTreeMap<String, String>> {
    let mut entry_source_map = BTreeMap::new();
    for entry in &analysis.entries {
        if entry_source_map
            .insert(entry.normalized_path.clone(), entry.source_path.clone())
            .is_some()
        {
            return Err(AppError::Validation(format!(
                "external package normalizes multiple files onto the same target path: {}",
                entry.normalized_path
            )));
        }
    }

    Ok(entry_source_map)
}

fn lookup_external_package_source_path<'a>(
    entry_source_map: &'a BTreeMap<String, String>,
    normalized_path: &str,
) -> AppResult<&'a str> {
    entry_source_map
        .get(normalized_path)
        .map(String::as_str)
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "normalized external package entry is missing: {normalized_path}"
            ))
        })
}

fn build_external_manifest(
    analysis: &ExternalPackageAnalysis,
    request: &CreateExternalPackageBundleRequest,
) -> AppResult<BundleManifest> {
    let package_id = request
        .package_id
        .clone()
        .unwrap_or_else(|| analysis.package_id.clone());
    let package_name = request
        .package_name
        .clone()
        .unwrap_or_else(|| analysis.package_name.clone());
    let created_by = request
        .created_by
        .clone()
        .unwrap_or_else(|| "external-package".to_string());
    let description = request.description.clone().or_else(|| {
        Some(format!(
            "Normalized import bundle created from external package `{}`.",
            analysis.source_path.display()
        ))
    });
    let supported_targets = if request.supported_targets.is_empty() {
        vec![request.source_flavor]
    } else {
        request.supported_targets.clone()
    };
    let character_mode = if analysis.resources.wtf_characters.is_empty() {
        CharacterMappingMode::KeepOriginal
    } else {
        CharacterMappingMode::Prompt
    };

    Ok(BundleManifest {
        schema_version: 1,
        package: PackageMetadata {
            id: package_id,
            name: package_name,
            created_by,
            description,
        },
        source: SourceInstallation {
            flavor: request.source_flavor,
            platform: request.source_platform,
            exported_at: None,
            supported_targets,
        },
        resources: analysis.resources.clone(),
        mapping: MappingRules {
            character_mode,
            rewrite_profile_keys: true,
            rewrite_identity_strings: true,
            allow_cross_platform: true,
        },
        apply: request
            .apply_defaults
            .clone()
            .unwrap_or_else(default_external_apply_defaults),
    })
}

fn default_external_apply_defaults() -> ApplyDefaults {
    ApplyDefaults {
        create_backup: true,
        addons: ResourceApplyPolicy::Merge,
        wtf_common: ResourceApplyPolicy::Merge,
        wtf_characters: ResourceApplyPolicy::Merge,
        fonts: ResourceApplyPolicy::Merge,
        interface_assets: ResourceApplyPolicy::Merge,
    }
}

fn create_staging_installation(
    stage_root: &Path,
    flavor: WowFlavor,
    platform: HostPlatform,
) -> AppResult<DetectedFlavorInstallation> {
    let product_root = stage_root.join("World of Warcraft");
    let flavor_root = product_root.join(flavor.folder_name());
    let interface_dir = flavor_root.join("Interface");
    let addon_dir = interface_dir.join("AddOns");
    let wtf_dir = flavor_root.join("WTF");
    let fonts_dir = flavor_root.join("Fonts");

    fs::create_dir_all(&addon_dir)?;
    fs::create_dir_all(&wtf_dir)?;
    fs::create_dir_all(&fonts_dir)?;

    Ok(DetectedFlavorInstallation {
        platform,
        product_root,
        flavor_root,
        flavor,
        interface_dir,
        addon_dir,
        wtf_dir,
        fonts_dir,
    })
}

fn materialize_analysis_to_installation(
    analysis: &ExternalPackageAnalysis,
    installation: &DetectedFlavorInstallation,
) -> AppResult<()> {
    match analysis.source_kind {
        ExternalPackageSourceKind::Directory => {
            for entry in &analysis.entries {
                let source_path =
                    resolve_directory_source_entry_path(&analysis.source_path, entry)?;
                let destination = destination_path_for_normalized_entry(entry, installation)?;
                copy_file_to_destination(&source_path, &destination)?;
            }
        }
        ExternalPackageSourceKind::ZipArchive => {
            let file = File::open(&analysis.source_path)?;
            let mut archive = ZipArchive::new(file)?;
            for entry in &analysis.entries {
                let destination = destination_path_for_normalized_entry(entry, installation)?;
                write_zip_entry_to_destination(&mut archive, &entry.source_path, &destination)?;
            }
        }
    }

    Ok(())
}

fn destination_path_for_normalized_entry(
    entry: &ExternalPackageEntry,
    installation: &DetectedFlavorInstallation,
) -> AppResult<PathBuf> {
    let segments = safe_zip_segments(&entry.normalized_path)?;
    let destination = match segments.as_slice() {
        ["addons", rest @ ..] if !rest.is_empty() => join_segments(&installation.addon_dir, rest),
        ["wtf", "common", "Config.wtf"] => installation.wtf_dir.join("Config.wtf"),
        ["wtf", "common", "root", "SavedVariables", rest @ ..] if !rest.is_empty() => installation
            .wtf_dir
            .join("Account")
            .join("SavedVariables")
            .join(join_segments(Path::new(""), rest)),
        ["wtf", "common", "accounts", account, rest @ ..] if !rest.is_empty() => installation
            .wtf_dir
            .join("Account")
            .join(account)
            .join(join_segments(Path::new(""), rest)),
        ["wtf", "characters", account, server, character, rest @ ..] if !rest.is_empty() => {
            installation
                .wtf_dir
                .join("Account")
                .join(account)
                .join(server)
                .join(character)
                .join(join_segments(Path::new(""), rest))
        }
        ["fonts", rest @ ..] if !rest.is_empty() => join_segments(&installation.fonts_dir, rest),
        ["interface", rest @ ..] if !rest.is_empty() => {
            join_segments(&installation.interface_dir, rest)
        }
        _ => {
            return Err(AppError::Validation(format!(
                "unsupported normalized external package path: {}",
                entry.normalized_path
            )));
        }
    };

    Ok(destination)
}

fn copy_file_to_destination(source_path: &Path, destination: &Path) -> AppResult<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source_path, destination)?;
    Ok(())
}

fn write_zip_entry_to_destination(
    archive: &mut ZipArchive<File>,
    source_entry_name: &str,
    destination: &Path,
) -> AppResult<()> {
    let mut entry = archive.by_name(source_entry_name).map_err(|_| {
        AppError::NotFound(format!(
            "external package entry is missing during normalization: {source_entry_name}"
        ))
    })?;
    copy_reader_to_path(&mut entry, destination)
}

fn project_external_package_plan(
    analysis: ExternalPackageAnalysis,
    plan: BundleApplyPlan,
) -> ExternalPackageApplyPlan {
    ExternalPackageApplyPlan {
        analysis,
        target_flavor_root: plan.target_flavor_root,
        discovered_accounts: plan.discovered_accounts,
        selected_target_accounts: plan.selected_target_accounts,
        character_mappings: plan.character_mappings,
        operations: plan.operations,
        summary: plan.summary,
        helper_strategy: plan.helper_strategy,
        group_policies: plan.group_policies,
        manifest: plan.manifest,
    }
}

fn project_applied_external_package(
    analysis: ExternalPackageAnalysis,
    result: UnpackedBundle,
) -> AppliedExternalPackage {
    AppliedExternalPackage {
        analysis,
        target_flavor_root: result.target_flavor_root,
        dry_run: result.dry_run,
        planned_files: result.planned_files,
        written_files: result.written_files,
        rewritten_files: result.rewritten_files,
        backup_path: result.backup_path,
        selected_target_accounts: result.selected_target_accounts,
        plan_summary: result.plan_summary,
        character_mappings: result.character_mappings,
        manifest: result.manifest,
    }
}

fn resolve_directory_source_entry_path(
    root: &Path,
    entry: &ExternalPackageEntry,
) -> AppResult<PathBuf> {
    let segments = safe_zip_segments(&entry.source_path)?;
    Ok(join_segments(root, &segments))
}

fn package_id_from_source_path(path: &Path) -> String {
    let candidate = package_name_from_source_path(path);
    let normalized = safe_file_part(&candidate);
    if normalized.is_empty() {
        "external-package".to_string()
    } else {
        normalized
    }
}

fn package_name_from_source_path(path: &Path) -> String {
    path.file_stem()
        .or_else(|| path.file_name())
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("external-package")
        .to_string()
}

fn starts_with_segments(segments: &[String], prefix: &[String]) -> bool {
    prefix.len() <= segments.len()
        && prefix
            .iter()
            .zip(segments.iter())
            .all(|(left, right)| left == right)
}

fn join_normalized_segments(root: &str, name: &str, rest: &[String]) -> String {
    let mut segments = vec![root.to_string(), name.to_string()];
    segments.extend(rest.iter().cloned());
    segments.join("/")
}

fn join_exact_normalized_segments(prefix: &[&str], rest: &[String]) -> String {
    let mut segments = prefix
        .iter()
        .map(|segment| (*segment).to_string())
        .collect::<Vec<_>>();
    segments.extend(rest.iter().cloned());
    segments.join("/")
}

fn normalized_path_tail<'a>(normalized_path: &'a str, root: &str) -> Option<&'a str> {
    normalized_path
        .strip_prefix(root)
        .and_then(|value| value.strip_prefix('/'))
        .and_then(|value| value.split('/').next())
}

fn find_segment_index(segments: &[String], needle: &str) -> Option<usize> {
    segments
        .iter()
        .position(|segment| segment.eq_ignore_ascii_case(needle))
}
