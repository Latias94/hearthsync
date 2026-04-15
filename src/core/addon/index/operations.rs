use std::collections::BTreeSet;

use super::matching::match_index_package_to_tracked_package;
use super::storage::{ensure_package_supports_flavor, find_index_package, load_addon_index};
use super::*;
use crate::core::addon::{
    AddonPackageMetadata, InstallAddonRequest, PreparedAddonPackage, TrackedAddonPackage,
    UpdatedAddonPackageResult, install_addon, list_addons, load_registry,
    prepare_package_from_source_ref_with_flavor, rollback_or_report_addon_error,
    update_prepared_packages,
};
use crate::core::backup::{BackupGroup, BackupRequest, create_backup};
use crate::core::error::{AppError, AppResult};

pub fn install_addon_from_index(
    request: AddonIndexInstallRequest,
) -> AppResult<AddonIndexInstallResult> {
    let index = load_addon_index(&request.index_path)?;
    let package = find_index_package(&index, &request.name)?.clone();
    ensure_package_supports_flavor(&package, request.installation.flavor.as_str())?;
    let install = install_addon(InstallAddonRequest {
        installation: request.installation,
        source: package.source.display_name(),
        dry_run: request.dry_run,
        backup_output_path: request.backup_output_path,
        replace_existing: request.replace_existing,
        metadata: Some(metadata_from_index_package(&index, &package)),
    })?;

    Ok(AddonIndexInstallResult {
        index_path: request.index_path,
        package,
        install,
    })
}

pub fn update_addons_from_index(
    request: AddonIndexUpdateRequest,
) -> AppResult<AddonIndexUpdateResult> {
    let index = load_addon_index(&request.index_path)?;
    let selected_packages = match &request.name {
        Some(name) => vec![find_index_package(&index, name)?.clone()],
        None => index.packages.clone(),
    };
    for package in &selected_packages {
        ensure_package_supports_flavor(package, request.installation.flavor.as_str())?;
    }
    let inventory = list_addons(&request.installation)?;
    if inventory.tracked_packages.is_empty() {
        return Err(AppError::Validation(
            "no tracked addon packages found. Use `addon index install` or `addon install` first."
                .to_string(),
        ));
    }

    let mut prepared_packages = Vec::new();
    let mut matched_packages = Vec::new();
    let mut used_package_ids = BTreeSet::new();
    for package in &selected_packages {
        let mut prepared = prepare_package_from_source_ref_with_flavor(
            &package.source,
            Some(request.installation.flavor),
        )?;
        prepared.metadata = Some(metadata_from_index_package(&index, package));
        let matched = match_index_package_to_tracked_package(
            package,
            &prepared,
            &inventory.tracked_packages,
            &used_package_ids,
        )?;
        used_package_ids.insert(matched.package_id.clone());
        prepared_packages.push(prepared);
        matched_packages.push(matched);
    }

    let files_to_write = prepared_packages
        .iter()
        .map(|package| {
            package
                .addons
                .iter()
                .map(|addon| addon.file_count)
                .sum::<usize>()
        })
        .sum::<usize>();

    let update = if request.dry_run {
        UpdatedAddonPackageResult {
            dry_run: true,
            registry_path: inventory.registry_path,
            files_to_write,
            written_files: 0,
            updated_packages: preview_updated_packages(&matched_packages, &prepared_packages),
            backup_path: None,
        }
    } else {
        let registry = load_registry(&request.installation)?;
        let registry_path = inventory.registry_path;
        let backup_path = Some(
            create_backup(BackupRequest {
                installation: request.installation.clone(),
                output_path: request.backup_output_path,
                groups: vec![BackupGroup::Addons],
                label: Some("addon-index-update".to_string()),
            })?
            .archive_path,
        );

        match update_prepared_packages(
            &request.installation,
            registry,
            matched_packages,
            prepared_packages,
        ) {
            Ok((updated_packages, written_files)) => UpdatedAddonPackageResult {
                dry_run: false,
                registry_path,
                files_to_write,
                written_files,
                updated_packages,
                backup_path,
            },
            Err(error) => {
                return rollback_or_report_addon_error(
                    error,
                    backup_path.as_deref(),
                    &request.installation,
                );
            }
        }
    };

    Ok(AddonIndexUpdateResult {
        index_path: request.index_path,
        selected_packages,
        update,
    })
}

fn preview_updated_packages(
    matched_packages: &[TrackedAddonPackage],
    prepared_packages: &[PreparedAddonPackage],
) -> Vec<TrackedAddonPackage> {
    matched_packages
        .iter()
        .zip(prepared_packages.iter())
        .map(|(matched, prepared)| TrackedAddonPackage {
            package_id: prepared.package_id.clone(),
            source: prepared.source.clone(),
            installed_at: matched.installed_at.clone(),
            updated_at: String::new(),
            addons: prepared
                .addons
                .iter()
                .map(|addon| addon.addon.clone())
                .collect(),
            metadata: prepared
                .metadata
                .clone()
                .or_else(|| matched.metadata.clone()),
        })
        .collect()
}

fn metadata_from_index_package(
    index: &AddonIndex,
    package: &AddonIndexPackage,
) -> AddonPackageMetadata {
    AddonPackageMetadata {
        index_name: Some(index.name.clone()),
        index_package_id: Some(package.id.clone()),
        package_name: Some(package.name.clone()),
        version: Some(package.version.clone()),
        source_url: package.source_url.clone(),
        website_url: package.website_url.clone(),
        source_sha256: package.sha256.clone(),
        supported_flavors: package.supported_flavors.clone(),
    }
}
