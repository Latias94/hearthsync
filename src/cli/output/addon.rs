use crate::core::app::{
    AddonIndexInspectionResult, AddonIndexInstallResult, AddonIndexUpdateResult,
    AddonInventoryResult, AddonSearchCatalogResult, InstalledAddonPackageResult,
    RemovedAddonPackageResult, TrackedAddonPackageResult, TrackedAddonResult,
    UpdatedAddonPackageResult,
};

use super::shared::{format_optional_path_or_none, format_string_list_or_none};

pub(in crate::cli) fn render_addon_index_inspection(item: &AddonIndexInspectionResult) -> String {
    let packages = item
        .packages
        .iter()
        .map(|package| {
            format!(
                "{} {} => {}",
                package.id, package.version, package.source_label
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "Index: {}\nName: {}\nPackages: {}\n{}",
        item.index_path.display(),
        item.name,
        item.package_count,
        if packages.is_empty() {
            "none".to_string()
        } else {
            packages
        }
    )
}

pub(in crate::cli) fn render_addon_index_install(item: &AddonIndexInstallResult) -> String {
    let backup = item
        .install
        .backup_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "none".to_string());
    let addons = item
        .install
        .addons
        .iter()
        .map(|addon| addon.directory_name.clone())
        .collect::<Vec<_>>()
        .join(", ");

    if item.install.dry_run {
        format!(
            "Dry run only.\nIndex: {}\nPackage: {} {}\nAddons: {}\nFiles to write: {}\nBackup: {}",
            item.index_path.display(),
            item.package.id,
            item.package.version,
            addons,
            item.install.files_to_write,
            backup
        )
    } else {
        format!(
            "Installed index package: {} {}\nIndex: {}\nAddons: {}\nWritten files: {}\nBackup: {}",
            item.package.id,
            item.package.version,
            item.index_path.display(),
            addons,
            item.install.written_files,
            backup
        )
    }
}

pub(in crate::cli) fn render_addon_index_update(item: &AddonIndexUpdateResult) -> String {
    let backup = item
        .update
        .backup_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "none".to_string());
    let packages = item
        .selected_packages
        .iter()
        .map(|package| format!("{} {}", package.id, package.version))
        .collect::<Vec<_>>()
        .join(", ");

    if item.update.dry_run {
        format!(
            "Dry run only.\nIndex: {}\nPackages: {}\nFiles to write: {}\nBackup: {}",
            item.index_path.display(),
            packages,
            item.update.files_to_write,
            backup
        )
    } else {
        format!(
            "Updated index packages: {}\nIndex: {}\nWritten files: {}\nBackup: {}",
            packages,
            item.index_path.display(),
            item.update.written_files,
            backup
        )
    }
}

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

    if item.dry_run {
        format!(
            "Dry run only.\nRegistry: {}\nPackages: {}\nFiles to write: {}\nBackup: {}",
            item.registry_path.display(),
            packages,
            item.files_to_write,
            backup
        )
    } else {
        format!(
            "Updated packages: {}\nWritten files: {}\nRegistry: {}\nBackup: {}",
            packages,
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

fn format_tracked_addon_names(addons: &[TrackedAddonResult]) -> String {
    addons
        .iter()
        .map(|addon| addon.directory_name.clone())
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_tracked_package_summaries(packages: &[TrackedAddonPackageResult]) -> String {
    if packages.is_empty() {
        "none".to_string()
    } else {
        packages
            .iter()
            .map(|package| {
                format!(
                    "{} [{}]",
                    package.package_id,
                    format_tracked_addon_names(&package.addons)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::test_support::{
        sample_index_package, sample_source, sample_tracked_addon, sample_tracked_package,
    };
    use super::{
        render_addon_index_inspection, render_addon_index_install, render_addon_index_update,
        render_addon_install, render_addon_inventory, render_addon_remove,
        render_addon_search_catalog, render_addon_update,
    };
    use crate::core::app::{
        AddonIndexInspectionResult, AddonIndexInstallResult, AddonIndexUpdateResult,
        AddonInventoryResult, AddonSearchCatalogResult, AddonSearchResult,
        InstalledAddonPackageResult, RemovedAddonPackageResult, UpdatedAddonPackageResult,
    };

    #[test]
    fn render_addon_index_inspection_lists_packages() {
        let rendered = render_addon_index_inspection(&AddonIndexInspectionResult {
            index_path: PathBuf::from("addons.toml"),
            name: "Curated".to_string(),
            description: Some("test".to_string()),
            package_count: 2,
            packages: vec![
                sample_index_package("details", "2.0.0"),
                sample_index_package("weakauras", "5.18.2"),
            ],
        });

        assert!(rendered.contains("Index: addons.toml"));
        assert!(rendered.contains("Name: Curated"));
        assert!(rendered.contains("Packages: 2"));
        assert!(rendered.contains("details 2.0.0 => local.zip"));
        assert!(rendered.contains("weakauras 5.18.2 => local.zip"));
    }

    #[test]
    fn render_addon_index_install_reports_dry_run_summary() {
        let rendered = render_addon_index_install(&AddonIndexInstallResult {
            index_path: PathBuf::from("addons.toml"),
            package: sample_index_package("details", "2.0.0"),
            install: InstalledAddonPackageResult {
                dry_run: true,
                source: sample_source(),
                source_label: "local.zip".to_string(),
                package_id: "details".to_string(),
                addon_count: 2,
                addons: vec![
                    sample_tracked_addon("Details"),
                    sample_tracked_addon("Details_Streamer"),
                ],
                files_to_write: 18,
                written_files: 0,
                replaced_addon_count: 0,
                replaced_addons: Vec::new(),
                registry_path: PathBuf::from("addon-registry.json"),
                backup_path: Some(PathBuf::from("backup.zip")),
            },
        });

        assert!(rendered.contains("Dry run only."));
        assert!(rendered.contains("Package: details 2.0.0"));
        assert!(rendered.contains("Addons: Details, Details_Streamer"));
        assert!(rendered.contains("Files to write: 18"));
        assert!(rendered.contains("Backup: backup.zip"));
    }

    #[test]
    fn render_addon_index_update_reports_written_files() {
        let rendered = render_addon_index_update(&AddonIndexUpdateResult {
            index_path: PathBuf::from("addons.toml"),
            selected_package_count: 2,
            selected_packages: vec![
                sample_index_package("details", "2.0.0"),
                sample_index_package("weakauras", "5.18.2"),
            ],
            update: UpdatedAddonPackageResult {
                dry_run: false,
                registry_path: PathBuf::from("addon-registry.json"),
                files_to_write: 0,
                written_files: 24,
                updated_package_count: 2,
                updated_packages: Vec::new(),
                backup_path: None,
            },
        });

        assert!(rendered.contains("Updated index packages: details 2.0.0, weakauras 5.18.2"));
        assert!(rendered.contains("Index: addons.toml"));
        assert!(rendered.contains("Written files: 24"));
        assert!(rendered.contains("Backup: none"));
    }

    #[test]
    fn render_addon_search_catalog_lists_results() {
        let rendered = render_addon_search_catalog(&AddonSearchCatalogResult {
            query: "weakauras".to_string(),
            result_count: 1,
            results: vec![AddonSearchResult {
                provider: "curseforge".to_string(),
                name: "WeakAuras".to_string(),
                summary: Some("Aura tracking".to_string()),
                source: sample_source(),
                source_label: "curseforge:123".to_string(),
                install_hint: "curseforge:weakauras".to_string(),
                website_url: Some("https://example.com".to_string()),
                provider_project_id: Some(123),
                provider_file_id: Some(456),
                download_count: 999,
            }],
        });

        assert!(rendered.contains("Query: weakauras"));
        assert!(rendered.contains("Found 1 result(s):"));
        assert!(rendered.contains("WeakAuras | provider: curseforge"));
        assert!(rendered.contains("summary: Aura tracking"));
    }

    #[test]
    fn render_addon_inventory_reports_tracked_and_untracked_addons() {
        let rendered = render_addon_inventory(&AddonInventoryResult {
            target_addon_root: PathBuf::from("Interface/AddOns"),
            registry_path: PathBuf::from("addons.toml"),
            tracked_package_count: 1,
            tracked_addon_count: 2,
            tracked_packages: vec![sample_tracked_package("weakauras")],
            untracked_addons: vec!["LooseAddon".to_string()],
        });

        assert!(rendered.contains("Tracked packages: 1"));
        assert!(rendered.contains("Tracked addons: 2"));
        assert!(rendered.contains("weakauras => local.zip [WeakAuras, WeakAurasOptions]"));
        assert!(rendered.contains("Untracked addon directories: LooseAddon"));
    }

    #[test]
    fn render_addon_install_reports_written_files() {
        let rendered = render_addon_install(&InstalledAddonPackageResult {
            dry_run: false,
            source: sample_source(),
            source_label: "local.zip".to_string(),
            package_id: "weakauras".to_string(),
            addon_count: 2,
            addons: vec![
                sample_tracked_addon("WeakAuras"),
                sample_tracked_addon("WeakAurasOptions"),
            ],
            files_to_write: 0,
            written_files: 20,
            replaced_addon_count: 1,
            replaced_addons: vec!["OldWeakAuras".to_string()],
            registry_path: PathBuf::from("addons.toml"),
            backup_path: Some(PathBuf::from("backup.zip")),
        });

        assert!(rendered.contains("Installed package: weakauras"));
        assert!(rendered.contains("Addons: WeakAuras, WeakAurasOptions"));
        assert!(rendered.contains("Replaced addons: OldWeakAuras"));
        assert!(rendered.contains("Backup: backup.zip"));
    }

    #[test]
    fn render_addon_update_reports_package_summaries() {
        let rendered = render_addon_update(&UpdatedAddonPackageResult {
            dry_run: false,
            registry_path: PathBuf::from("addons.toml"),
            files_to_write: 0,
            written_files: 12,
            updated_package_count: 1,
            updated_packages: vec![sample_tracked_package("weakauras")],
            backup_path: None,
        });

        assert!(rendered.contains("Updated packages: weakauras [WeakAuras, WeakAurasOptions]"));
        assert!(rendered.contains("Written files: 12"));
        assert!(rendered.contains("Backup: none"));
    }

    #[test]
    fn render_addon_remove_reports_registry_cleanup() {
        let rendered = render_addon_remove(&RemovedAddonPackageResult {
            dry_run: false,
            registry_path: PathBuf::from("addons.toml"),
            removed_package_count: 1,
            removed_packages: vec![sample_tracked_package("weakauras")],
            removed_addon_count: 2,
            removed_addons: vec!["WeakAuras".to_string(), "WeakAurasOptions".to_string()],
            registry_cleaned: true,
            backup_path: None,
        });

        assert!(rendered.contains("Removed packages: weakauras"));
        assert!(rendered.contains("Removed addon directories: WeakAuras, WeakAurasOptions"));
        assert!(rendered.contains("Registry cleaned: true"));
    }
}
