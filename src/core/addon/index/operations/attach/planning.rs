use super::result::{
    ReadyAttachResultRequest, addon_directory_mismatch_attach_result,
    already_attached_attach_result, ambiguous_local_match_attach_result,
    no_local_match_attach_result, prepare_failed_attach_result, ready_attach_result,
    skipped_unsupported_flavor_attach_result,
};
use super::*;

pub(super) fn prepare_index_attach_with_provider<P>(
    provider: &P,
    request: AddonIndexAttachRequest,
    cancellation: &dyn CancellationToken,
    progress: &mut impl TaskProgressSink,
) -> AppResult<IndexAttachPlan>
where
    P: AddonProvider + ?Sized,
{
    let index = load_addon_index(&request.index_path)?;
    let inventory = list_addons(&request.installation, &request.state_paths)?;
    if inventory.tracked_packages.is_empty() {
        return Err(no_tracked_packages_error(
            &request.installation,
            &request.state_paths,
        ));
    }

    let registry = load_registry(&request.installation, &request.state_paths)?;
    let flavor = request.installation.flavor.as_str();
    let selected_packages = match request.name.as_deref() {
        Some(name) => vec![find_index_package(&index, name)?.clone()],
        None => index.packages.clone(),
    };
    let mut packages = Vec::new();
    let mut changes = Vec::new();
    let mut used_package_ids = BTreeSet::new();
    let mut considered_package_count = 0usize;
    let mut skipped_unsupported_flavor_package_count = 0usize;

    for package in selected_packages {
        if let Err(error) = ensure_package_supports_flavor(&package, flavor) {
            if request.name.is_some() {
                return Err(error);
            }
            skipped_unsupported_flavor_package_count += 1;
            packages.push(skipped_unsupported_flavor_attach_result(
                package,
                error.to_string(),
            ));
            continue;
        }

        considered_package_count += 1;
        let package_for_matching =
            resolved_index_package_for_matching(&request.index_path, &package);
        let explained = match explain_preflight_match_index_package_to_tracked_package(
            &package_for_matching,
            &inventory.tracked_packages,
            &used_package_ids,
        ) {
            Ok(Some(matched)) => matched,
            Ok(None) => {
                packages.push(no_local_match_attach_result(package));
                continue;
            }
            Err(error) => {
                packages.push(ambiguous_local_match_attach_result(
                    package,
                    error.to_string(),
                ));
                continue;
            }
        };
        used_package_ids.insert(package_id_usage_key(&explained.package.package_id));
        let tracked_package = explained.package;
        let match_strategy = explained.strategy;

        let tracked_package_index = registry
            .packages
            .iter()
            .position(|candidate| *candidate == tracked_package)
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "tracked addon package `{}` disappeared before addon index attach could plan",
                    tracked_package.package_id
                ))
            })?;
        let resolved_source =
            match resolve_index_package_source(&request.index_path, &package.source) {
                Ok(source) => source,
                Err(error) => {
                    packages.push(prepare_failed_attach_result(
                        package,
                        Some(tracked_package.package_id.as_str()),
                        Some(match_strategy.clone()),
                        Some(tracked_package.source.clone()),
                        None,
                        error.to_string(),
                    ));
                    continue;
                }
            };
        let prepared = match prepare_package_from_source_ref_task_with_provider(
            provider,
            PreparePackageFromSourceRefTaskRequest::new(
                &resolved_source,
                PreparePackageTaskContext::new(
                    Some(request.installation.flavor),
                    request.installation.platform,
                    cancellation,
                    TaskKind::AddonIndexAttach,
                    TaskPhase::Preparing,
                ),
            ),
            progress,
        ) {
            Ok(prepared) => prepared,
            Err(error) => {
                packages.push(prepare_failed_attach_result(
                    package,
                    Some(tracked_package.package_id.as_str()),
                    Some(match_strategy.clone()),
                    Some(tracked_package.source.clone()),
                    Some(resolved_source),
                    error.to_string(),
                ));
                continue;
            }
        };
        if let Err(error) = ensure_relink_addon_directories_match(&tracked_package, &prepared) {
            packages.push(addon_directory_mismatch_attach_result(
                package,
                &tracked_package.package_id,
                match_strategy.clone(),
                tracked_package.source.clone(),
                prepared.source.clone(),
                error.to_string(),
            ));
            continue;
        }

        let metadata = metadata_from_index_package(&index, &package);
        let source_changed = relink_source_changed(&tracked_package, &prepared.source);
        let metadata_changed = tracked_package.metadata.as_ref() != Some(&metadata);
        if !source_changed && !metadata_changed {
            packages.push(already_attached_attach_result(
                package,
                &tracked_package.package_id,
                match_strategy,
                tracked_package.source.clone(),
            ));
            continue;
        }

        let package_result_index = packages.len();
        packages.push(ready_attach_result(ReadyAttachResultRequest {
            package: package.clone(),
            tracked_package_id: &tracked_package.package_id,
            match_strategy: match_strategy.clone(),
            previous_source: tracked_package.source.clone(),
            source: prepared.source.clone(),
            source_changed,
            metadata_changed,
            applied: false,
        }));
        changes.push(IndexAttachChange {
            package_result_index,
            package,
            tracked_package_index,
            tracked_package,
            next_source: prepared.source,
            metadata,
            match_strategy,
            source_changed,
            metadata_changed,
        });
    }

    Ok(IndexAttachPlan {
        installation: request.installation,
        state_paths: request.state_paths,
        index_path: request.index_path,
        index_name: index.name,
        dry_run: request.dry_run,
        apply_ready_only: request.apply_ready_only,
        registry_path: inventory.registry_path,
        index_package_count: index.packages.len(),
        considered_package_count,
        skipped_unsupported_flavor_package_count,
        packages,
        changes,
    })
}
