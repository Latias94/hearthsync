use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tempfile::{TempDir, tempdir};

mod analysis;
mod classify;
mod manifest;
mod materialize;
mod source;

use super::*;
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};
use crate::core::manifest::{ApplyDefaults, BundleManifest, BundleResources};
use crate::core::task::{
    CancellationToken, NeverCancel, NoopProgressSink, TaskKind, TaskPhase, TaskProgressSink,
    emit_task_progress, ensure_task_not_cancelled,
};
use analysis::build_analysis;
use classify::classify_source_entries;
pub(crate) use manifest::author_package_apply_defaults;
use manifest::build_external_manifest;
use materialize::{create_staging_installation, materialize_analysis_to_installation};
use source::{collect_source_entries, detect_source_kind};

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
    let (entries, warnings) = classify_source_entries(&source_entries);

    Ok(build_analysis(
        source_path,
        source_kind,
        source_entries.len(),
        entries,
        warnings,
    ))
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
    let (analysis, manifest) = prepare_external_package_artifacts(&request.external_package)?;
    let entry_source_map = build_external_package_entry_source_map(&analysis)?;
    let source_path = analysis.source_path.clone();
    let source = PreparedApplySource::ExternalPackage {
        source_path: source_path.clone(),
        source_kind: analysis.source_kind,
        entry_source_map,
    };
    let plan = super::planner::plan_apply_from_source(
        &source_path,
        &request.installation,
        manifest,
        &request.apply_mappings,
        &source,
    )?;

    Ok(project_external_package_plan(analysis, plan))
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

    let manifest = build_external_manifest(&analysis, request);
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
    let source_path = analysis.source_path.clone();
    let prepared_apply = super::planner::prepare_apply_from_source(
        &source_path,
        installation,
        manifest,
        apply_mappings,
        PreparedApplySource::ExternalPackage {
            source_path: source_path.clone(),
            source_kind: analysis.source_kind,
            entry_source_map,
        },
    )?;

    Ok(PreparedExternalPackageApply {
        analysis,
        prepared_apply,
    })
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
