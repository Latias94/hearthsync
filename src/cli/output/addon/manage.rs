use super::*;

pub(in crate::cli) fn render_addon_search_catalog(item: &AddonSearchCatalogResult) -> String {
    if item.results.is_empty() {
        format!("Query: {}\nNo addons found.", item.query)
    } else {
        let mut lines = vec![
            format!("Query: {}", item.query),
            format!("Found {} result(s):", item.result_count),
        ];
        for result in &item.results {
            lines.push(format!(
                "- {} | provider: {} | source: {} | downloads: {} | website: {}{}",
                result.name,
                result.provider,
                result.install_hint,
                result.download_count,
                result.website_url.as_deref().unwrap_or("none"),
                result
                    .summary
                    .as_deref()
                    .map(|summary| format!(" | summary: {summary}"))
                    .unwrap_or_default()
            ));
        }
        lines.join("\n")
    }
}

pub(in crate::cli) fn render_addon_inventory(item: &AddonInventoryResult) -> String {
    let tracked = if item.tracked_packages.is_empty() {
        "none".to_string()
    } else {
        item.tracked_packages
            .iter()
            .map(|package| {
                format!(
                    "{} => {} [{}]",
                    package.package_id,
                    package.source_label,
                    format_tracked_addon_names(&package.addons)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let untracked = format_string_list_or_none(&item.untracked_addons);

    format!(
        "Target: {}\nRegistry: {}\nTracked packages: {}\nTracked addons: {}\nTracked package details:\n{}\nUntracked addon directories: {}",
        item.target_addon_root.display(),
        item.registry_path.display(),
        item.tracked_package_count,
        item.tracked_addon_count,
        tracked,
        untracked
    )
}

pub(in crate::cli) fn render_addon_adopt(item: &AdoptedAddonPackageResult) -> String {
    let addons = format_tracked_addon_names(&item.addons);

    if item.dry_run {
        format!(
            "Dry run only.\nPlanned package: {}\nSnapshot archive: {}\nAddons: {}\nRegistry: {}",
            item.package_id,
            item.source.display_name,
            addons,
            item.registry_path.display()
        )
    } else {
        format!(
            "Adopted package: {}\nSnapshot archive: {}\nAddons: {}\nRegistry: {}",
            item.package_id,
            item.source.display_name,
            addons,
            item.registry_path.display()
        )
    }
}

pub(in crate::cli) fn render_addon_relink(item: &RelinkedAddonPackageResult) -> String {
    let addons = format_tracked_addon_names(&item.addons);
    let metadata = if item.cleared_metadata {
        "cleared"
    } else {
        "unchanged"
    };

    if item.dry_run {
        format!(
            "Dry run only.\nPackage: {}\nFrom: {}\nTo: {}\nAddons: {}\nMetadata: {}\nRegistry: {}",
            item.package_id,
            item.previous_source.display_name,
            item.source.display_name,
            addons,
            metadata,
            item.registry_path.display()
        )
    } else {
        format!(
            "Relinked package: {}\nFrom: {}\nTo: {}\nAddons: {}\nMetadata: {}\nRegistry: {}",
            item.package_id,
            item.previous_source.display_name,
            item.source.display_name,
            addons,
            metadata,
            item.registry_path.display()
        )
    }
}

pub(in crate::cli) fn render_addon_install(item: &InstalledAddonPackageResult) -> String {
    let backup = format_optional_path_or_none(item.backup_path.as_deref());
    let replaced = format_string_list_or_none(&item.replaced_addons);
    let addons = format_tracked_addon_names(&item.addons);

    if item.dry_run {
        format!(
            "Dry run only.\nSource: {}\nPackage: {}\nAddons: {}\nFiles to write: {}\nWould replace: {}\nBackup: {}",
            item.source.display_name,
            item.package_id,
            addons,
            item.files_to_write,
            replaced,
            backup
        )
    } else {
        format!(
            "Installed package: {}\nSource: {}\nAddons: {}\nWritten files: {}\nReplaced addons: {}\nRegistry: {}\nBackup: {}",
            item.package_id,
            item.source.display_name,
            addons,
            item.written_files,
            replaced,
            item.registry_path.display(),
            backup
        )
    }
}

pub(in crate::cli) fn render_addon_update(item: &UpdatedAddonPackageResult) -> String {
    let backup = format_optional_path_or_none(item.backup_path.as_deref());
    let packages = format_tracked_package_summaries(&item.updated_packages);
    let dependency_packages = format_tracked_package_summaries(&item.installed_dependency_packages);
    let ignored = format_string_list_or_none(&item.ignored_packages);

    if item.dry_run {
        format!(
            "Dry run only.\nRegistry: {}\nPackages: {}\nDependency packages: {}\nIgnored packages: {}\nFiles to write: {}\nBackup: {}",
            item.registry_path.display(),
            packages,
            dependency_packages,
            ignored,
            item.files_to_write,
            backup
        )
    } else {
        format!(
            "Updated packages: {}\nInstalled dependency packages: {}\nIgnored packages: {}\nWritten files: {}\nRegistry: {}\nBackup: {}",
            packages,
            dependency_packages,
            ignored,
            item.written_files,
            item.registry_path.display(),
            backup
        )
    }
}

pub(in crate::cli) fn render_addon_remove(item: &RemovedAddonPackageResult) -> String {
    let backup = format_optional_path_or_none(item.backup_path.as_deref());
    let packages = if item.removed_packages.is_empty() {
        "none".to_string()
    } else {
        item.removed_packages
            .iter()
            .map(|package| package.package_id.clone())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let addons = format_string_list_or_none(&item.removed_addons);

    if item.dry_run {
        format!(
            "Dry run only.\nRegistry: {}\nPackages: {}\nAddon directories: {}\nBackup: {}",
            item.registry_path.display(),
            packages,
            addons,
            backup
        )
    } else {
        format!(
            "Removed packages: {}\nRemoved addon directories: {}\nRegistry: {}\nRegistry cleaned: {}\nBackup: {}",
            packages,
            addons,
            item.registry_path.display(),
            item.registry_cleaned,
            backup
        )
    }
}
