use std::collections::BTreeSet;

use crate::core::error::AppResult;
use crate::core::install::DetectedFlavorInstallation;
use crate::core::task::{CancellationToken, TaskKind, TaskPhase, TaskProgressSink};

use super::policy::AddonUpdatePolicySnapshot;
use super::provider::{
    AddonDependencyResolutionStrategy, AddonSourceResolutionPolicy, ResolveAddonDependenciesRequest,
};
use super::{
    AddonProvider, AddonProviderContext, AddonRegistry, AddonSourceRef,
    PreparePackageFromSourceRefTaskRequest, PreparePackageTaskContext, PreparedAddonPackage,
    TrackedAddonPackage, load_registry, prepare_package_from_source_ref_task_with_provider,
    select_tracked_packages,
};
use crate::core::error::AppError;

pub(crate) fn preview_installed_dependency_packages(
    prepared_packages: &[PreparedAddonPackage],
) -> Vec<TrackedAddonPackage> {
    prepared_packages
        .iter()
        .map(|package| TrackedAddonPackage {
            package_id: package.package_id.clone(),
            source: package.source.clone(),
            installed_at: String::new(),
            updated_at: String::new(),
            addons: package
                .addons
                .iter()
                .map(|addon| addon.addon.clone())
                .collect(),
            metadata: package.metadata.clone(),
        })
        .collect()
}

#[derive(Clone, Copy)]
pub(crate) struct MissingDependencyCollectionRequest<'a> {
    pub(crate) source: &'a AddonSourceRef,
    pub(crate) resolution_policy: AddonSourceResolutionPolicy,
    pub(crate) installation: &'a DetectedFlavorInstallation,
    pub(crate) registry: &'a AddonRegistry,
    pub(crate) selected_packages: &'a [TrackedAddonPackage],
    pub(crate) task_kind: TaskKind,
}

pub(crate) struct MissingDependencyCollectionState<'a> {
    pub(crate) prepared_packages: &'a mut Vec<PreparedAddonPackage>,
    pub(crate) planned_keys: &'a mut BTreeSet<String>,
}

pub(crate) fn collect_missing_dependency_prepared_packages<P>(
    provider: &P,
    request: MissingDependencyCollectionRequest<'_>,
    state: &mut MissingDependencyCollectionState<'_>,
    cancellation: &dyn CancellationToken,
    progress: &mut impl TaskProgressSink,
) -> AppResult<()>
where
    P: AddonProvider + ?Sized,
{
    let source = request.source;
    let expected_strategy = validate_dependency_resolution_support(provider, source)?;

    let dependencies = provider.resolve_addon_dependencies(ResolveAddonDependenciesRequest {
        source,
        context: AddonProviderContext::new(Some(request.installation.flavor), Some(cancellation))
            .with_resolution_policy(request.resolution_policy),
    })?;

    if dependencies.strategy != expected_strategy {
        return Err(AppError::Validation(format!(
            "addon provider dependency capability mismatch for `{}`: expected `{:?}`, resolved `{:?}`",
            source.display_name(),
            expected_strategy,
            dependencies.strategy,
        )));
    }

    let dependency_sources = match dependencies.strategy {
        AddonDependencyResolutionStrategy::MissingRequiredOnly => dependencies.dependencies,
    };

    for dependency_source in dependency_sources {
        let dependency_key = dependency_identity_key(&dependency_source);
        if !state.planned_keys.insert(dependency_key) {
            continue;
        }
        if source_satisfies_dependency(source, &dependency_source)
            || request
                .selected_packages
                .iter()
                .any(|package| source_satisfies_dependency(&package.source, &dependency_source))
            || request
                .registry
                .packages
                .iter()
                .any(|package| source_satisfies_dependency(&package.source, &dependency_source))
        {
            continue;
        }

        collect_missing_dependency_prepared_packages(
            provider,
            MissingDependencyCollectionRequest {
                source: &dependency_source,
                ..request
            },
            state,
            cancellation,
            progress,
        )?;

        let prepared_dependency = prepare_package_from_source_ref_task_with_provider(
            provider,
            PreparePackageFromSourceRefTaskRequest::new(
                &dependency_source,
                PreparePackageTaskContext::new(
                    Some(request.installation.flavor),
                    request.installation.platform,
                    cancellation,
                    request.task_kind,
                    TaskPhase::Preparing,
                ),
            )
            .with_resolution_policy(request.resolution_policy),
            progress,
        )?;
        state.prepared_packages.push(prepared_dependency);
    }

    Ok(())
}

pub(crate) fn validate_addon_update_dependency_policy_support<P>(
    provider: &P,
    installation: &DetectedFlavorInstallation,
    state_paths: &super::AddonStatePaths,
    name: Option<&str>,
) -> AppResult<()>
where
    P: AddonProvider + ?Sized,
{
    let registry = load_registry(installation, state_paths)?;
    if registry.packages.is_empty() {
        return Ok(());
    }

    let policies = AddonUpdatePolicySnapshot::load(installation, state_paths)?;
    let mut selected_packages = select_tracked_packages(&registry, name)?;
    if name.is_none() {
        selected_packages.retain(|package| !policies.is_ignored(package));
    }

    for package in &selected_packages {
        let package_policy = policies.provider_update_policy(provider, package)?;
        if !package_policy.install_dependencies {
            continue;
        }
        let _ = validate_dependency_resolution_support(provider, &package_policy.effective_source)?;
    }

    Ok(())
}

pub(crate) fn validate_dependency_resolution_support<P>(
    provider: &P,
    source: &AddonSourceRef,
) -> AppResult<AddonDependencyResolutionStrategy>
where
    P: AddonProvider + ?Sized,
{
    provider
        .dependency_resolution_capability(source)
        .supported_strategy()
        .ok_or_else(|| unsupported_dependency_installation_error(source))
}

fn dependency_identity_key(source: &AddonSourceRef) -> String {
    match source {
        AddonSourceRef::CurseForgeMod { mod_id, .. } => format!("curseforge:{mod_id}"),
        AddonSourceRef::WagoAddon { project_id, .. } => format!("wago:{project_id}"),
        _ => source.display_name(),
    }
}

fn dependency_source_kind_label(source: &AddonSourceRef) -> &'static str {
    match source {
        AddonSourceRef::LocalArchive { .. } => "local_archive",
        AddonSourceRef::HttpArchive { .. } => "http_archive",
        AddonSourceRef::CurseForgeMod { .. } => "curseforge_mod",
        AddonSourceRef::GitHubRelease { .. } => "github_release",
        AddonSourceRef::WagoAddon { .. } => "wago_addon",
    }
}

fn unsupported_dependency_installation_error(source: &AddonSourceRef) -> AppError {
    AppError::Validation(format!(
        "addon dependency installation is not supported for source `{}` ({})",
        source.display_name(),
        dependency_source_kind_label(source),
    ))
}

fn source_satisfies_dependency(candidate: &AddonSourceRef, dependency: &AddonSourceRef) -> bool {
    match (candidate, dependency) {
        (
            AddonSourceRef::CurseForgeMod {
                mod_id: candidate_mod_id,
                ..
            },
            AddonSourceRef::CurseForgeMod {
                mod_id: dependency_mod_id,
                ..
            },
        ) => candidate_mod_id == dependency_mod_id,
        (
            AddonSourceRef::WagoAddon {
                project_id: candidate_project_id,
                ..
            },
            AddonSourceRef::WagoAddon {
                project_id: dependency_project_id,
                ..
            },
        ) => candidate_project_id.eq_ignore_ascii_case(dependency_project_id),
        _ => candidate == dependency,
    }
}
