use std::collections::{BTreeMap, BTreeSet};

use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;
use crate::core::task::NeverCancel;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::package_prep::prepare_package_from_source_input_with_provider;
use super::registry::{registry_path, select_single_tracked_package};
use super::{
    AddonProvider, AddonSourceRef, DefaultAddonProvider, PreparedAddonPackage, RelinkAddonRequest,
    RelinkedAddonPackageResult, TrackedAddonPackage, load_registry, no_tracked_packages_error,
    save_registry,
};

pub fn relink_addon(request: RelinkAddonRequest) -> AppResult<RelinkedAddonPackageResult> {
    let provider = DefaultAddonProvider::default();
    relink_addon_with_provider(&provider, request)
}

pub(crate) fn relink_addon_with_provider<P>(
    provider: &P,
    request: RelinkAddonRequest,
) -> AppResult<RelinkedAddonPackageResult>
where
    P: AddonProvider + ?Sized,
{
    let target_name = request.name.trim();
    if target_name.is_empty() {
        return Err(AppError::Validation(
            "tracked addon selector for relink must not be empty".to_string(),
        ));
    }

    let source = request.source.trim();
    if source.is_empty() {
        return Err(AppError::Validation(
            "new addon source for relink must not be empty".to_string(),
        ));
    }

    let mut registry = load_registry(&request.installation, &request.state_paths)?;
    if registry.packages.is_empty() {
        return Err(no_tracked_packages_error(
            &request.installation,
            &request.state_paths,
        ));
    }

    let (package_index, tracked_package) = select_single_tracked_package(&registry, target_name)?;
    let prepared = prepare_relink_source_with_provider(provider, &request.installation, source)?;
    ensure_relink_source_changes(&tracked_package, &prepared.source)?;
    ensure_relink_addon_directories_match(&tracked_package, &prepared)?;

    let registry_path = registry_path(&request.state_paths);
    let previous_source = tracked_package.source.clone();
    let next_source = prepared.source.clone();
    let cleared_metadata = tracked_package.metadata.is_some();

    if !request.dry_run {
        let package = registry.packages.get_mut(package_index).ok_or_else(|| {
            AppError::Validation(format!(
                "tracked addon package `{}` disappeared during relink",
                tracked_package.package_id
            ))
        })?;
        package.updated_at = relink_timestamp()?;
        package.source = next_source.clone();
        package.metadata = None;
        save_registry(&request.installation, &request.state_paths, &registry)?;
    }

    Ok(RelinkedAddonPackageResult {
        dry_run: request.dry_run,
        package_id: tracked_package.package_id,
        previous_source,
        source: next_source,
        addons: tracked_package.addons,
        registry_path,
        cleared_metadata,
    })
}

pub(crate) fn prepare_relink_source_with_provider<P>(
    provider: &P,
    installation: &DetectedFlavorInstallation,
    source: &str,
) -> AppResult<PreparedAddonPackage>
where
    P: AddonProvider + ?Sized,
{
    let cancellation = NeverCancel;
    prepare_package_from_source_input_with_provider(
        provider,
        source,
        Some(installation.flavor),
        installation.platform,
        &cancellation,
    )
}

pub(crate) fn relink_source_changed(
    tracked_package: &TrackedAddonPackage,
    next_source: &AddonSourceRef,
) -> bool {
    &tracked_package.source != next_source
}

pub(crate) fn ensure_relink_source_changes(
    tracked_package: &TrackedAddonPackage,
    next_source: &AddonSourceRef,
) -> AppResult<()> {
    if !relink_source_changed(tracked_package, next_source) {
        return Err(AppError::Validation(format!(
            "tracked addon package `{}` is already linked to `{}`",
            tracked_package.package_id,
            next_source.display_name()
        )));
    }

    Ok(())
}

pub(crate) fn ensure_relink_addon_directories_match(
    tracked_package: &TrackedAddonPackage,
    prepared: &PreparedAddonPackage,
) -> AppResult<()> {
    let tracked = addon_directory_map(
        tracked_package
            .addons
            .iter()
            .map(|addon| addon.directory_name.as_str()),
    );
    let prepared_directories = addon_directory_map(
        prepared
            .addons
            .iter()
            .map(|addon| addon.addon.directory_name.as_str()),
    );
    let tracked_keys = tracked.keys().cloned().collect::<BTreeSet<_>>();
    let prepared_keys = prepared_directories
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();

    if tracked_keys == prepared_keys {
        return Ok(());
    }

    let missing_from_source = tracked_keys
        .difference(&prepared_keys)
        .filter_map(|key| tracked.get(key).cloned())
        .collect::<Vec<_>>();
    let extra_from_source = prepared_keys
        .difference(&tracked_keys)
        .filter_map(|key| prepared_directories.get(key).cloned())
        .collect::<Vec<_>>();

    let missing = if missing_from_source.is_empty() {
        "none".to_string()
    } else {
        missing_from_source.join(", ")
    };
    let extra = if extra_from_source.is_empty() {
        "none".to_string()
    } else {
        extra_from_source.join(", ")
    };

    Err(AppError::Validation(format!(
        "new addon source `{}` is incompatible with tracked package `{}`: addon directory sets must match exactly; missing from source: {}; extra from source: {}",
        prepared.source.display_name(),
        tracked_package.package_id,
        missing,
        extra
    )))
}

fn addon_directory_map<'a>(names: impl IntoIterator<Item = &'a str>) -> BTreeMap<String, String> {
    let mut by_key = BTreeMap::new();
    for name in names {
        by_key.insert(name.trim().to_ascii_lowercase(), name.to_string());
    }
    by_key
}

pub(crate) fn relink_timestamp() -> AppResult<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AppError::Validation(error.to_string()))
}
