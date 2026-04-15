use super::plan::{AddonLockPlanContext, build_addon_lock_plan};
use super::source_resolution::{prepare_expected_lock_package, resolved_source_override_map};
use super::storage::now_rfc3339;
use super::verify::verify_addon_lock;
use super::*;

#[derive(Debug)]
struct PreparedAddonLockApply {
    remove_packages: Vec<TrackedAddonPackage>,
    update_current_packages: Vec<TrackedAddonPackage>,
    update_prepared_packages: Vec<PreparedAddonPackage>,
    install_prepared_packages: Vec<PreparedAddonPackage>,
    metadata_actions: Vec<MetadataOnlyAddonLockAction>,
}

impl PreparedAddonLockApply {
    fn is_empty(&self) -> bool {
        self.remove_packages.is_empty()
            && self.update_current_packages.is_empty()
            && self.install_prepared_packages.is_empty()
            && self.metadata_actions.is_empty()
    }
}

#[derive(Debug, Clone)]
struct MetadataOnlyAddonLockAction {
    current: TrackedAddonPackage,
    expected: AddonLockPackage,
}

pub fn apply_addon_lock_sync(request: AddonLockApplyRequest) -> AppResult<AddonLockApplyResult> {
    let plan = build_addon_lock_plan(
        &request.installation,
        request.lock_path.as_deref(),
        &request.source_overrides,
    )?;
    let source_overrides =
        resolved_source_override_map(&plan.result.lock_path, &request.source_overrides)?;
    let blocked_actions = plan
        .actions
        .iter()
        .filter(|action| !action.action.blocked_reasons.is_empty())
        .collect::<Vec<_>>();
    if !blocked_actions.is_empty() {
        return Err(AppError::Validation(format!(
            "cannot apply addon lock because some actions are blocked: {}",
            blocked_actions
                .iter()
                .map(|action| {
                    format!(
                        "{} ({})",
                        action.action.package_id,
                        action.action.blocked_reasons.join("; ")
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    let replace_required = plan
        .actions
        .iter()
        .filter(|action| {
            action.action.requires_replace_existing
                && matches!(
                    action.action.kind,
                    AddonLockSyncActionKind::Install | AddonLockSyncActionKind::Update
                )
        })
        .collect::<Vec<_>>();
    if !request.replace_existing && !replace_required.is_empty() {
        return Err(AppError::Validation(format!(
            "lock apply needs `--replace-existing` for packages: {}",
            replace_required
                .iter()
                .map(|action| action.action.package_id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    let prepared = prepare_addon_lock_apply(&plan, &source_overrides, &request.installation)?;
    let backup_path = if prepared.is_empty() {
        None
    } else {
        Some(
            create_backup(BackupRequest {
                installation: request.installation.clone(),
                output_path: request.backup_output_path.clone(),
                groups: vec![BackupGroup::Addons],
                label: Some("addon-lock-apply".to_string()),
            })?
            .archive_path,
        )
    };

    if let Err(error) =
        execute_prepared_addon_lock_apply(&request.installation, prepared, request.replace_existing)
    {
        return rollback_or_report_addon_error(
            error,
            backup_path.as_deref(),
            &request.installation,
        );
    }

    let verification = verify_addon_lock(&request.installation, Some(&plan.result.lock_path))?;
    Ok(AddonLockApplyResult {
        lock_path: plan.result.lock_path,
        installation_root: plan.result.installation_root,
        install_count: plan.result.install_count,
        update_count: plan.result.update_count,
        remove_count: plan.result.remove_count,
        metadata_only_count: plan.result.metadata_only_count,
        unchanged_count: plan.result.unchanged_count,
        blocked_count: plan.result.blocked_count,
        untracked_addons: verification.untracked_addons.clone(),
        actions: plan.result.actions,
        verification,
    })
}

fn metadata_from_lock_package(package: &AddonLockPackage) -> Option<AddonPackageMetadata> {
    let metadata = AddonPackageMetadata {
        index_name: package.index_name.clone(),
        index_package_id: package.index_package_id.clone(),
        package_name: package.name.clone(),
        version: package.version.clone(),
        source_url: package.source_url.clone(),
        website_url: package.website_url.clone(),
        source_sha256: package.source_sha256.clone(),
        supported_flavors: Vec::new(),
    };
    (metadata != AddonPackageMetadata::default()).then_some(metadata)
}

fn prepare_addon_lock_apply(
    plan: &AddonLockPlanContext,
    source_overrides: &BTreeMap<String, PathBuf>,
    installation: &DetectedFlavorInstallation,
) -> AppResult<PreparedAddonLockApply> {
    let mut remove_packages = Vec::new();
    let mut update_current_packages = Vec::new();
    let mut update_prepared_packages = Vec::new();
    let mut install_prepared_packages = Vec::new();
    let mut metadata_actions = Vec::new();

    for action in &plan.actions {
        match action.action.kind {
            AddonLockSyncActionKind::Remove => {
                let current = action.current.as_ref().ok_or_else(|| {
                    AppError::Validation(
                        "lock remove action is missing current package".to_string(),
                    )
                })?;
                remove_packages.push(current.clone());
            }
            AddonLockSyncActionKind::Update => {
                let current = action.current.as_ref().ok_or_else(|| {
                    AppError::Validation(
                        "lock update action is missing current package".to_string(),
                    )
                })?;
                let expected = action.expected.as_ref().ok_or_else(|| {
                    AppError::Validation(
                        "lock update action is missing expected package".to_string(),
                    )
                })?;
                let mut prepared = prepare_expected_lock_package(
                    expected,
                    source_overrides
                        .get(&action.action.comparison_key)
                        .map(PathBuf::as_path),
                    installation.flavor,
                )?;
                prepared.metadata = metadata_from_lock_package(expected);
                update_current_packages.push(current.clone());
                update_prepared_packages.push(prepared);
            }
            AddonLockSyncActionKind::Install => {
                let expected = action.expected.as_ref().ok_or_else(|| {
                    AppError::Validation(
                        "lock install action is missing expected package".to_string(),
                    )
                })?;
                let mut prepared = prepare_expected_lock_package(
                    expected,
                    source_overrides
                        .get(&action.action.comparison_key)
                        .map(PathBuf::as_path),
                    installation.flavor,
                )?;
                prepared.metadata = metadata_from_lock_package(expected);
                install_prepared_packages.push(prepared);
            }
            AddonLockSyncActionKind::MetadataOnly => {
                let current = action.current.as_ref().ok_or_else(|| {
                    AppError::Validation(
                        "lock metadata-only action is missing current package".to_string(),
                    )
                })?;
                let expected = action.expected.as_ref().ok_or_else(|| {
                    AppError::Validation(
                        "lock metadata-only action is missing expected package".to_string(),
                    )
                })?;
                metadata_actions.push(MetadataOnlyAddonLockAction {
                    current: current.clone(),
                    expected: expected.clone(),
                });
            }
        }
    }

    Ok(PreparedAddonLockApply {
        remove_packages,
        update_current_packages,
        update_prepared_packages,
        install_prepared_packages,
        metadata_actions,
    })
}

fn execute_prepared_addon_lock_apply(
    installation: &DetectedFlavorInstallation,
    prepared: PreparedAddonLockApply,
    replace_existing: bool,
) -> AppResult<()> {
    if !prepared.remove_packages.is_empty() {
        remove_selected_packages(installation, prepared.remove_packages)?;
    }

    if !prepared.update_current_packages.is_empty() {
        let registry = load_registry(installation)?;
        update_prepared_packages(
            installation,
            registry,
            prepared.update_current_packages,
            prepared.update_prepared_packages,
        )?;
    }

    for prepared_package in prepared.install_prepared_packages {
        install_prepared_package(installation, prepared_package, replace_existing)?;
    }

    if !prepared.metadata_actions.is_empty() {
        apply_metadata_only_actions(installation, prepared.metadata_actions)?;
    }

    Ok(())
}

fn apply_metadata_only_actions(
    installation: &DetectedFlavorInstallation,
    actions: Vec<MetadataOnlyAddonLockAction>,
) -> AppResult<()> {
    let mut registry = load_registry(installation)?;
    let timestamp = now_rfc3339()?;

    for action in actions {
        let package = registry
            .packages
            .iter_mut()
            .find(|candidate| **candidate == action.current)
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "tracked package disappeared before metadata apply: {}",
                    action.current.package_id
                ))
            })?;
        package.package_id = action.expected.package_id.clone();
        package.updated_at = timestamp.clone();
        package.metadata = metadata_from_lock_package(&action.expected);
    }

    save_registry(installation, &registry)
}
