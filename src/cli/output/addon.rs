use crate::core::app::{
    AddonCachePurgeResult, AddonCacheRepairResult, AddonIndexAttachPackageStatusResult,
    AddonIndexAttachResult, AddonIndexInspectionResult, AddonIndexInspectionWarningCodeResult,
    AddonIndexInspectionWarningSeverityResult, AddonIndexInstallResult,
    AddonIndexPackageSuggestionStatusResult, AddonIndexRelinkResult, AddonIndexScaffoldResult,
    AddonIndexSuggestionResult, AddonIndexTrackedMatchStrategyResult, AddonIndexUpdateResult,
    AddonIndexValidationResult, AddonInventoryResult, AddonSearchCatalogResult,
    AdoptedAddonPackageResult, InstalledAddonPackageResult, RelinkedAddonPackageResult,
    RemovedAddonPackageResult, TrackedAddonPackageResult, TrackedAddonResult,
    UpdatedAddonPackageResult,
};

use super::shared::{format_optional_path_or_none, format_string_list_or_none};

pub(in crate::cli) fn render_addon_index_inspection(item: &AddonIndexInspectionResult) -> String {
    let packages_without_exact_hints =
        format_string_list_or_none(&item.identity_hint_coverage.packages_without_exact_hints);
    let warnings = if item.warnings.is_empty() {
        "Warnings: none".to_string()
    } else {
        let mut lines = vec![format!("Warnings: {}", item.warning_count)];
        for warning in &item.warnings {
            lines.push(format!(
                "- {} {} [{}]: {}",
                format_addon_index_warning_severity(&warning.severity),
                format_addon_index_warning_code(&warning.code),
                warning.package_id,
                warning.message
            ));
        }
        lines.join("\n")
    };
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
        "Index: {}\nName: {}\nPackages: {}\nExact identity hints: {}/{}\nBoth exact hints: {}\nmatch_package_ids hints: {}\naddon_directories hints: {}\nMissing match_package_ids: {}\nMissing addon_directories: {}\nPackages without exact identity hints: {}\nBlocking warnings: {}\nAdvisory warnings: {}\n{}\n{}",
        item.index_path.display(),
        item.name,
        item.package_count,
        item.identity_hint_coverage
            .package_count_with_any_exact_hints,
        item.package_count,
        item.identity_hint_coverage
            .package_count_with_both_exact_hints,
        item.identity_hint_coverage
            .package_count_with_match_package_ids,
        item.identity_hint_coverage
            .package_count_with_addon_directories,
        item.identity_hint_coverage
            .package_count_without_match_package_ids,
        item.identity_hint_coverage
            .package_count_without_addon_directories,
        packages_without_exact_hints,
        item.blocking_warning_count,
        item.advisory_warning_count,
        warnings,
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

pub(in crate::cli) fn render_addon_index_relink(item: &AddonIndexRelinkResult) -> String {
    let addons = item
        .addons
        .iter()
        .map(|addon| addon.directory_name.clone())
        .collect::<Vec<_>>()
        .join(", ");

    if item.dry_run {
        format!(
            "Dry run only.\nIndex: {}\nIndex package: {} {}\nTracked package: {}\nFrom: {}\nTo: {}\nAddons: {}\nSource changed: {}\nMetadata changed: {}\nRegistry: {}",
            item.index_path.display(),
            item.package.id,
            item.package.version,
            item.tracked_package_id,
            item.previous_source.display_name,
            item.source.display_name,
            addons,
            item.source_changed,
            item.metadata_changed,
            item.registry_path.display()
        )
    } else {
        format!(
            "Relinked index package: {} {}\nIndex: {}\nTracked package: {}\nFrom: {}\nTo: {}\nAddons: {}\nSource changed: {}\nMetadata changed: {}\nRegistry: {}",
            item.package.id,
            item.package.version,
            item.index_path.display(),
            item.tracked_package_id,
            item.previous_source.display_name,
            item.source.display_name,
            addons,
            item.source_changed,
            item.metadata_changed,
            item.registry_path.display()
        )
    }
}

pub(in crate::cli) fn render_addon_index_validation(item: &AddonIndexValidationResult) -> String {
    let warnings = if item.warnings.is_empty() {
        "Warnings: none".to_string()
    } else {
        let mut lines = vec![format!("Warnings: {}", item.warning_count)];
        for warning in &item.warnings {
            lines.push(format!(
                "- {} {} [{}]: {}",
                format_addon_index_warning_severity(&warning.severity),
                format_addon_index_warning_code(&warning.code),
                warning.package_id,
                warning.message
            ));
        }
        lines.join("\n")
    };
    let status = if item.valid { "valid" } else { "invalid" };

    format!(
        "Status: {}\nValid: {}\nIndex: {}\nName: {}\nPackages: {}\nExact identity hints: {}/{}\nBoth exact hints: {}\nmatch_package_ids hints: {}\naddon_directories hints: {}\nBlocking warnings: {}\nAdvisory warnings: {}\n{}",
        status,
        item.valid,
        item.index_path.display(),
        item.name,
        item.package_count,
        item.identity_hint_coverage
            .package_count_with_any_exact_hints,
        item.package_count,
        item.identity_hint_coverage
            .package_count_with_both_exact_hints,
        item.identity_hint_coverage
            .package_count_with_match_package_ids,
        item.identity_hint_coverage
            .package_count_with_addon_directories,
        item.blocking_warning_count,
        item.advisory_warning_count,
        warnings
    )
}

pub(in crate::cli) fn render_addon_index_suggestion(item: &AddonIndexSuggestionResult) -> String {
    let packages = if item.packages.is_empty() {
        "none".to_string()
    } else {
        item.packages
            .iter()
            .map(|package| {
                let mut lines = vec![format!(
                    "- {} ({})",
                    package.package_id,
                    format_addon_index_suggestion_status(&package.status)
                )];
                if let Some(matched_package_id) = &package.matched_tracked_package_id {
                    let strategy = package
                        .match_strategy
                        .as_ref()
                        .map(format_addon_index_match_strategy)
                        .unwrap_or("unknown");
                    lines.push(format!(
                        "  matched tracked package: {} ({})",
                        matched_package_id, strategy
                    ));
                }
                lines.push(format!(
                    "  match_package_ids to add: {}",
                    format_string_list_or_none(&package.match_package_ids_to_add)
                ));
                lines.push(format!(
                    "  addon_directories to add: {}",
                    format_string_list_or_none(&package.addon_directories_to_add)
                ));
                lines.push(format!("  note: {}", package.message));
                lines.join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "Index: {}\nName: {}\nIndex packages: {}\nConsidered packages: {}\nSuggested packages: {}\nComplete packages: {}\nNo local match packages: {}\nAmbiguous match packages: {}\nSkipped unsupported flavor packages: {}\n{}",
        item.index_path.display(),
        item.index_name,
        item.index_package_count,
        item.considered_package_count,
        item.suggested_package_count,
        item.complete_package_count,
        item.no_match_package_count,
        item.ambiguous_match_package_count,
        item.skipped_unsupported_flavor_package_count,
        packages
    )
}

pub(in crate::cli) fn render_addon_index_attach(item: &AddonIndexAttachResult) -> String {
    let status = if item.partial_apply {
        "partially_attached"
    } else if item.applied {
        "attached"
    } else if !item.ready {
        "blocked"
    } else if item.change_package_count == 0 {
        "already_attached"
    } else if item.dry_run {
        "dry_run"
    } else {
        "ready"
    };
    let packages = if item.packages.is_empty() {
        "none".to_string()
    } else {
        item.packages
            .iter()
            .map(|package| {
                let mut lines = vec![format!(
                    "- {} {} ({})",
                    package.package.id,
                    package.package.version,
                    format_addon_index_attach_status(&package.status)
                )];
                if let Some(tracked_package_id) = &package.matched_tracked_package_id {
                    let strategy = package
                        .match_strategy
                        .as_ref()
                        .map(format_addon_index_match_strategy)
                        .unwrap_or("unknown");
                    lines.push(format!(
                        "  tracked package: {} ({})",
                        tracked_package_id, strategy
                    ));
                }
                if let Some(previous_source) = &package.previous_source {
                    lines.push(format!("  from: {}", previous_source.display_name));
                }
                if let Some(source) = &package.source {
                    lines.push(format!("  to: {}", source.display_name));
                }
                lines.push(format!("  source changed: {}", package.source_changed));
                lines.push(format!("  metadata changed: {}", package.metadata_changed));
                lines.push(format!("  note: {}", package.message));
                lines.join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "Status: {}\nDry run: {}\nReady: {}\nApplied: {}\nPartial apply: {}\nIndex: {}\nName: {}\nIndex packages: {}\nConsidered packages: {}\nPlanned changes: {}\nAttached packages: {}\nAlready attached packages: {}\nBlocked packages: {}\nSkipped unsupported flavor packages: {}\nRegistry: {}\n{}",
        status,
        item.dry_run,
        item.ready,
        item.applied,
        item.partial_apply,
        item.index_path.display(),
        item.index_name,
        item.index_package_count,
        item.considered_package_count,
        item.change_package_count,
        item.attached_package_count,
        item.already_attached_package_count,
        item.blocked_package_count,
        item.skipped_unsupported_flavor_package_count,
        item.registry_path.display(),
        packages
    )
}

pub(in crate::cli) fn render_addon_index_scaffold(item: &AddonIndexScaffoldResult) -> String {
    format!(
        "Wrote addon index scaffold: {}\nName: {}\nPackages: {}\nUsed existing metadata: {}\nInferred names: {}\nInferred versions: {}\nPlaceholder versions: {}\nPackage ids: {}",
        item.index_path.display(),
        item.index_name,
        item.package_count,
        item.used_metadata_package_count,
        item.inferred_name_package_count,
        item.inferred_version_package_count,
        item.placeholder_version_package_count,
        format_string_list_or_none(&item.package_ids)
    )
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
    let dependency_packages =
        format_tracked_package_summaries(&item.update.installed_dependency_packages);
    let ignored = format_string_list_or_none(&item.update.ignored_packages);

    if item.update.dry_run {
        format!(
            "Dry run only.\nIndex: {}\nPackages: {}\nDependency packages: {}\nIgnored packages: {}\nFiles to write: {}\nBackup: {}",
            item.index_path.display(),
            packages,
            dependency_packages,
            ignored,
            item.update.files_to_write,
            backup
        )
    } else {
        format!(
            "Updated index packages: {}\nInstalled dependency packages: {}\nIgnored packages: {}\nIndex: {}\nWritten files: {}\nBackup: {}",
            packages,
            dependency_packages,
            ignored,
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

pub(in crate::cli) fn render_addon_cache_purge(item: &AddonCachePurgeResult) -> String {
    if !item.configured {
        return "Addon download cache is not configured.".to_string();
    }

    format!(
        "Purged addon cache: {}\nRemoved files: {}\nRemoved directories: {}\nReclaimed bytes: {}",
        item.cache_dir
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "none".to_string()),
        item.removed_file_count,
        item.removed_directory_count,
        item.reclaimed_bytes
    )
}

pub(in crate::cli) fn render_addon_cache_repair(item: &AddonCacheRepairResult) -> String {
    if !item.configured {
        return "Addon download cache is not configured.".to_string();
    }

    format!(
        "Repaired addon cache: {}\nScanned metadata entries: {}\nRepaired entries: {}\nInvalid metadata: {}\nMissing archives: {}\nMismatched archives: {}\nOrphan archives: {}\nPartial downloads: {}\nRemote verified entries: {}\nRemote refreshed entries: {}\nRemote check failures: {}\nExpired freshness entries: {}\nRemoved files: {}\nRemoved directories: {}\nReclaimed bytes: {}",
        item.cache_dir
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "none".to_string()),
        item.scanned_metadata_count,
        item.repaired_entry_count,
        item.invalid_metadata_count,
        item.missing_archive_count,
        item.mismatched_archive_count,
        item.orphan_archive_count,
        item.partial_download_count,
        item.remote_verified_entry_count,
        item.remote_refreshed_entry_count,
        item.remote_check_failed_count,
        item.expired_freshness_entry_count,
        item.removed_file_count,
        item.removed_directory_count,
        item.reclaimed_bytes
    )
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

fn format_addon_index_warning_code(code: &AddonIndexInspectionWarningCodeResult) -> &'static str {
    match code {
        AddonIndexInspectionWarningCodeResult::MissingMatchPackageIds => {
            "missing_match_package_ids"
        }
        AddonIndexInspectionWarningCodeResult::MissingAddonDirectories => {
            "missing_addon_directories"
        }
        AddonIndexInspectionWarningCodeResult::MissingExactIdentityHints => {
            "missing_exact_identity_hints"
        }
    }
}

fn format_addon_index_warning_severity(
    severity: &AddonIndexInspectionWarningSeverityResult,
) -> &'static str {
    match severity {
        AddonIndexInspectionWarningSeverityResult::Blocking => "blocking",
        AddonIndexInspectionWarningSeverityResult::Advisory => "advisory",
    }
}

fn format_addon_index_match_strategy(
    strategy: &AddonIndexTrackedMatchStrategyResult,
) -> &'static str {
    match strategy {
        AddonIndexTrackedMatchStrategyResult::StoredIndexPackageId => "stored_index_package_id",
        AddonIndexTrackedMatchStrategyResult::ExactPackageId => "exact_package_id",
        AddonIndexTrackedMatchStrategyResult::CuratedMatchPackageId => "curated_match_package_id",
        AddonIndexTrackedMatchStrategyResult::SourceIdentity => "source_identity",
        AddonIndexTrackedMatchStrategyResult::SourceFamilyIdentity => "source_family_identity",
        AddonIndexTrackedMatchStrategyResult::DisplayName => "display_name",
        AddonIndexTrackedMatchStrategyResult::AddonDirectories => "addon_directories",
        AddonIndexTrackedMatchStrategyResult::AddonDirectoryOverlap => "addon_directory_overlap",
    }
}

fn format_addon_index_suggestion_status(
    status: &AddonIndexPackageSuggestionStatusResult,
) -> &'static str {
    match status {
        AddonIndexPackageSuggestionStatusResult::Suggested => "suggested",
        AddonIndexPackageSuggestionStatusResult::Complete => "complete",
        AddonIndexPackageSuggestionStatusResult::NoLocalMatch => "no_local_match",
        AddonIndexPackageSuggestionStatusResult::AmbiguousLocalMatch => "ambiguous_local_match",
    }
}

fn format_addon_index_attach_status(status: &AddonIndexAttachPackageStatusResult) -> &'static str {
    match status {
        AddonIndexAttachPackageStatusResult::WouldAttach => "would_attach",
        AddonIndexAttachPackageStatusResult::Attached => "attached",
        AddonIndexAttachPackageStatusResult::AlreadyAttached => "already_attached",
        AddonIndexAttachPackageStatusResult::NoLocalMatch => "no_local_match",
        AddonIndexAttachPackageStatusResult::AmbiguousLocalMatch => "ambiguous_local_match",
        AddonIndexAttachPackageStatusResult::AddonDirectoryMismatch => "addon_directory_mismatch",
        AddonIndexAttachPackageStatusResult::PrepareFailed => "prepare_failed",
        AddonIndexAttachPackageStatusResult::SkippedUnsupportedFlavor => {
            "skipped_unsupported_flavor"
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::test_support::{
        sample_index_package, sample_source, sample_tracked_addon, sample_tracked_package,
    };
    use super::{
        render_addon_adopt, render_addon_cache_purge, render_addon_cache_repair,
        render_addon_index_attach, render_addon_index_inspection, render_addon_index_install,
        render_addon_index_relink, render_addon_index_scaffold, render_addon_index_suggestion,
        render_addon_index_update, render_addon_index_validation, render_addon_install,
        render_addon_inventory, render_addon_relink, render_addon_remove,
        render_addon_search_catalog, render_addon_update,
    };
    use crate::core::app::{
        AddonCachePurgeResult, AddonCacheRepairResult, AddonIndexAttachPackageResult,
        AddonIndexAttachPackageStatusResult, AddonIndexAttachResult,
        AddonIndexIdentityHintCoverageResult, AddonIndexInspectionResult,
        AddonIndexInspectionWarningCodeResult, AddonIndexInspectionWarningResult,
        AddonIndexInspectionWarningSeverityResult, AddonIndexInstallResult,
        AddonIndexPackageSuggestionResult, AddonIndexPackageSuggestionStatusResult,
        AddonIndexRelinkResult, AddonIndexScaffoldResult, AddonIndexSuggestionResult,
        AddonIndexTrackedMatchStrategyResult, AddonIndexUpdateResult, AddonIndexValidationResult,
        AddonInventoryResult, AddonSearchCatalogResult, AddonSearchResult,
        AdoptedAddonPackageResult, InstalledAddonPackageResult, RelinkedAddonPackageResult,
        RemovedAddonPackageResult, UpdatedAddonPackageResult,
    };

    #[test]
    fn render_addon_index_inspection_lists_packages() {
        let rendered = render_addon_index_inspection(&AddonIndexInspectionResult {
            index_path: PathBuf::from("addons.toml"),
            name: "Curated".to_string(),
            description: Some("test".to_string()),
            package_count: 2,
            identity_hint_coverage: AddonIndexIdentityHintCoverageResult {
                package_count_with_both_exact_hints: 0,
                package_count_with_any_exact_hints: 1,
                package_count_with_match_package_ids: 1,
                package_count_with_addon_directories: 0,
                package_count_without_match_package_ids: 1,
                package_count_without_addon_directories: 2,
                package_count_without_exact_hints: 1,
                packages_without_match_package_ids: vec!["weakauras".to_string()],
                packages_without_addon_directories: vec![
                    "details".to_string(),
                    "weakauras".to_string(),
                ],
                packages_without_exact_hints: vec!["weakauras".to_string()],
            },
            warning_count: 2,
            blocking_warning_count: 1,
            advisory_warning_count: 1,
            warnings: vec![
                AddonIndexInspectionWarningResult {
                    code: AddonIndexInspectionWarningCodeResult::MissingAddonDirectories,
                    severity: AddonIndexInspectionWarningSeverityResult::Advisory,
                    package_id: "details".to_string(),
                    message: "package `details` does not declare addon_directories".to_string(),
                },
                AddonIndexInspectionWarningResult {
                    code: AddonIndexInspectionWarningCodeResult::MissingExactIdentityHints,
                    severity: AddonIndexInspectionWarningSeverityResult::Blocking,
                    package_id: "weakauras".to_string(),
                    message: "package `weakauras` does not declare exact identity hints"
                        .to_string(),
                },
            ],
            packages: vec![
                sample_index_package("details", "2.0.0"),
                sample_index_package("weakauras", "5.18.2"),
            ],
        });

        assert!(rendered.contains("Index: addons.toml"));
        assert!(rendered.contains("Name: Curated"));
        assert!(rendered.contains("Packages: 2"));
        assert!(rendered.contains("Exact identity hints: 1/2"));
        assert!(rendered.contains("Both exact hints: 0"));
        assert!(rendered.contains("match_package_ids hints: 1"));
        assert!(rendered.contains("addon_directories hints: 0"));
        assert!(rendered.contains("Missing match_package_ids: 1"));
        assert!(rendered.contains("Missing addon_directories: 2"));
        assert!(rendered.contains("Packages without exact identity hints: weakauras"));
        assert!(rendered.contains("Blocking warnings: 1"));
        assert!(rendered.contains("Advisory warnings: 1"));
        assert!(rendered.contains("Warnings: 2"));
        assert!(rendered.contains("advisory missing_addon_directories [details]"));
        assert!(rendered.contains("blocking missing_exact_identity_hints [weakauras]"));
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
    fn render_addon_index_relink_reports_source_and_metadata_changes() {
        let rendered = render_addon_index_relink(&AddonIndexRelinkResult {
            index_path: PathBuf::from("addons.toml"),
            package: sample_index_package("details", "2.0.0"),
            dry_run: true,
            tracked_package_id: "details-local".to_string(),
            previous_source: sample_source(),
            previous_source_label: "local.zip".to_string(),
            source: crate::core::app::AddonSourceResult {
                kind: crate::core::app::AddonSourceKindResult::HttpArchive,
                display_name: "https://example.invalid/details.zip".to_string(),
                dependency_resolution_capability:
                    crate::core::app::AddonDependencyResolutionCapabilityValue::Unsupported,
                local_archive_path: None,
                url: Some("https://example.invalid/details.zip".to_string()),
                mod_id: None,
                file_id: None,
                owner: None,
                repo: None,
                tag: None,
                asset_name: None,
            },
            source_label: "https://example.invalid/details.zip".to_string(),
            addon_count: 1,
            addons: vec![sample_tracked_addon("Details")],
            metadata: crate::core::app::AddonPackageMetadataValue {
                index_name: Some("Fixture Index".to_string()),
                index_package_id: Some("details".to_string()),
                package_name: Some("Details".to_string()),
                version: Some("2.0.0".to_string()),
                source_url: Some("https://example.invalid/details.zip".to_string()),
                website_url: None,
                source_sha256: None,
                supported_flavors: vec!["retail".to_string()],
            },
            registry_path: PathBuf::from("addon-registry.json"),
            source_changed: true,
            metadata_changed: true,
        });

        assert!(rendered.contains("Dry run only."));
        assert!(rendered.contains("Index package: details 2.0.0"));
        assert!(rendered.contains("Tracked package: details-local"));
        assert!(rendered.contains("From: local.zip"));
        assert!(rendered.contains("To: https://example.invalid/details.zip"));
        assert!(rendered.contains("Source changed: true"));
        assert!(rendered.contains("Metadata changed: true"));
    }

    #[test]
    fn render_addon_index_validation_reports_invalid_result() {
        let rendered = render_addon_index_validation(&AddonIndexValidationResult {
            index_path: PathBuf::from("addons.toml"),
            name: "Curated".to_string(),
            package_count: 2,
            identity_hint_coverage: AddonIndexIdentityHintCoverageResult {
                package_count_with_both_exact_hints: 0,
                package_count_with_any_exact_hints: 1,
                package_count_with_match_package_ids: 1,
                package_count_with_addon_directories: 0,
                package_count_without_match_package_ids: 1,
                package_count_without_addon_directories: 2,
                package_count_without_exact_hints: 1,
                packages_without_match_package_ids: vec!["weakauras".to_string()],
                packages_without_addon_directories: vec![
                    "details".to_string(),
                    "weakauras".to_string(),
                ],
                packages_without_exact_hints: vec!["weakauras".to_string()],
            },
            valid: false,
            warning_count: 2,
            blocking_warning_count: 1,
            advisory_warning_count: 1,
            warnings: vec![
                AddonIndexInspectionWarningResult {
                    code: AddonIndexInspectionWarningCodeResult::MissingAddonDirectories,
                    severity: AddonIndexInspectionWarningSeverityResult::Advisory,
                    package_id: "details".to_string(),
                    message: "package `details` does not declare addon_directories".to_string(),
                },
                AddonIndexInspectionWarningResult {
                    code: AddonIndexInspectionWarningCodeResult::MissingExactIdentityHints,
                    severity: AddonIndexInspectionWarningSeverityResult::Blocking,
                    package_id: "weakauras".to_string(),
                    message: "package `weakauras` does not declare exact identity hints"
                        .to_string(),
                },
            ],
        });

        assert!(rendered.contains("Status: invalid"));
        assert!(rendered.contains("Valid: false"));
        assert!(rendered.contains("Index: addons.toml"));
        assert!(rendered.contains("Both exact hints: 0"));
        assert!(rendered.contains("Blocking warnings: 1"));
        assert!(rendered.contains("Advisory warnings: 1"));
        assert!(rendered.contains("Warnings: 2"));
        assert!(rendered.contains("blocking missing_exact_identity_hints [weakauras]"));
    }

    #[test]
    fn render_addon_index_validation_reports_valid_status_with_advisory_warnings() {
        let rendered = render_addon_index_validation(&AddonIndexValidationResult {
            index_path: PathBuf::from("addons.toml"),
            name: "Curated".to_string(),
            package_count: 1,
            identity_hint_coverage: AddonIndexIdentityHintCoverageResult {
                package_count_with_both_exact_hints: 0,
                package_count_with_any_exact_hints: 1,
                package_count_with_match_package_ids: 1,
                package_count_with_addon_directories: 0,
                package_count_without_match_package_ids: 0,
                package_count_without_addon_directories: 1,
                package_count_without_exact_hints: 0,
                packages_without_match_package_ids: Vec::new(),
                packages_without_addon_directories: vec!["details".to_string()],
                packages_without_exact_hints: Vec::new(),
            },
            valid: true,
            warning_count: 1,
            blocking_warning_count: 0,
            advisory_warning_count: 1,
            warnings: vec![AddonIndexInspectionWarningResult {
                code: AddonIndexInspectionWarningCodeResult::MissingAddonDirectories,
                severity: AddonIndexInspectionWarningSeverityResult::Advisory,
                package_id: "details".to_string(),
                message: "package `details` does not declare addon_directories".to_string(),
            }],
        });

        assert!(rendered.contains("Status: valid"));
        assert!(rendered.contains("Valid: true"));
        assert!(rendered.contains("Blocking warnings: 0"));
        assert!(rendered.contains("Advisory warnings: 1"));
        assert!(rendered.contains("advisory missing_addon_directories [details]"));
    }

    #[test]
    fn render_addon_index_suggestion_reports_match_strategies_and_hint_additions() {
        let rendered = render_addon_index_suggestion(&AddonIndexSuggestionResult {
            index_path: PathBuf::from("addons.toml"),
            index_name: "Curated".to_string(),
            index_package_count: 3,
            considered_package_count: 2,
            suggested_package_count: 1,
            complete_package_count: 0,
            no_match_package_count: 1,
            ambiguous_match_package_count: 0,
            skipped_unsupported_flavor_package_count: 1,
            packages: vec![
                AddonIndexPackageSuggestionResult {
                    package_id: "curated-plater".to_string(),
                    package_name: "Curated Plater".to_string(),
                    current_match_package_ids: Vec::new(),
                    current_addon_directories: Vec::new(),
                    status: AddonIndexPackageSuggestionStatusResult::Suggested,
                    matched_tracked_package_id: Some("plater".to_string()),
                    match_strategy: Some(
                        AddonIndexTrackedMatchStrategyResult::SourceFamilyIdentity,
                    ),
                    matched_addon_directories: vec!["Plater".to_string()],
                    match_package_ids_to_add: vec!["plater".to_string()],
                    addon_directories_to_add: vec!["Plater".to_string()],
                    message: "matched tracked package `plater` by source family identity"
                        .to_string(),
                },
                AddonIndexPackageSuggestionResult {
                    package_id: "weakauras".to_string(),
                    package_name: "WeakAuras".to_string(),
                    current_match_package_ids: Vec::new(),
                    current_addon_directories: Vec::new(),
                    status: AddonIndexPackageSuggestionStatusResult::NoLocalMatch,
                    matched_tracked_package_id: None,
                    match_strategy: None,
                    matched_addon_directories: Vec::new(),
                    match_package_ids_to_add: Vec::new(),
                    addon_directories_to_add: Vec::new(),
                    message: "no tracked addon package from the current registry matched this index package"
                        .to_string(),
                },
            ],
        });

        assert!(rendered.contains("Index: addons.toml"));
        assert!(rendered.contains("Name: Curated"));
        assert!(rendered.contains("Index packages: 3"));
        assert!(rendered.contains("Considered packages: 2"));
        assert!(rendered.contains("Suggested packages: 1"));
        assert!(rendered.contains("No local match packages: 1"));
        assert!(rendered.contains("Skipped unsupported flavor packages: 1"));
        assert!(rendered.contains("- curated-plater (suggested)"));
        assert!(rendered.contains("matched tracked package: plater (source_family_identity)"));
        assert!(rendered.contains("match_package_ids to add: plater"));
        assert!(rendered.contains("addon_directories to add: Plater"));
        assert!(rendered.contains("- weakauras (no_local_match)"));
    }

    #[test]
    fn render_addon_index_attach_reports_blocked_and_planned_packages() {
        let rendered = render_addon_index_attach(&AddonIndexAttachResult {
            index_path: PathBuf::from("addons.toml"),
            index_name: "Curated".to_string(),
            dry_run: true,
            ready: false,
            applied: false,
            partial_apply: false,
            registry_path: PathBuf::from("addon-registry.json"),
            index_package_count: 3,
            considered_package_count: 2,
            change_package_count: 1,
            attached_package_count: 0,
            already_attached_package_count: 0,
            blocked_package_count: 1,
            skipped_unsupported_flavor_package_count: 1,
            packages: vec![
                AddonIndexAttachPackageResult {
                    package: sample_index_package("curated-plater", "2.0.0"),
                    status: AddonIndexAttachPackageStatusResult::WouldAttach,
                    matched_tracked_package_id: Some("plater".to_string()),
                    match_strategy: Some(
                        AddonIndexTrackedMatchStrategyResult::SourceFamilyIdentity,
                    ),
                    previous_source: Some(sample_source()),
                    previous_source_label: Some("local.zip".to_string()),
                    source: Some(crate::core::app::AddonSourceResult {
                        kind: crate::core::app::AddonSourceKindResult::GitHubRelease,
                        display_name: "github:foo/plater".to_string(),
                        dependency_resolution_capability:
                            crate::core::app::AddonDependencyResolutionCapabilityValue::Unsupported,
                        local_archive_path: None,
                        url: None,
                        mod_id: None,
                        file_id: None,
                        owner: Some("foo".to_string()),
                        repo: Some("plater".to_string()),
                        tag: None,
                        asset_name: None,
                    }),
                    source_label: Some("github:foo/plater".to_string()),
                    source_changed: true,
                    metadata_changed: true,
                    message:
                        "matched tracked package `plater` by source family identity; would attach curated source and metadata"
                            .to_string(),
                },
                AddonIndexAttachPackageResult {
                    package: sample_index_package("weakauras", "5.18.2"),
                    status: AddonIndexAttachPackageStatusResult::NoLocalMatch,
                    matched_tracked_package_id: None,
                    match_strategy: None,
                    previous_source: None,
                    previous_source_label: None,
                    source: None,
                    source_label: None,
                    source_changed: false,
                    metadata_changed: false,
                    message:
                        "no tracked addon package from the current registry matched this index package"
                            .to_string(),
                },
            ],
        });

        assert!(rendered.contains("Status: blocked"));
        assert!(rendered.contains("Dry run: true"));
        assert!(rendered.contains("Ready: false"));
        assert!(rendered.contains("Applied: false"));
        assert!(rendered.contains("Partial apply: false"));
        assert!(rendered.contains("Planned changes: 1"));
        assert!(rendered.contains("Blocked packages: 1"));
        assert!(rendered.contains("Skipped unsupported flavor packages: 1"));
        assert!(rendered.contains("- curated-plater 2.0.0 (would_attach)"));
        assert!(rendered.contains("tracked package: plater (source_family_identity)"));
        assert!(rendered.contains("from: local.zip"));
        assert!(rendered.contains("to: github:foo/plater"));
        assert!(rendered.contains("source changed: true"));
        assert!(rendered.contains("metadata changed: true"));
        assert!(rendered.contains("- weakauras 5.18.2 (no_local_match)"));
    }

    #[test]
    fn render_addon_index_scaffold_reports_summary_counts() {
        let rendered = render_addon_index_scaffold(&AddonIndexScaffoldResult {
            index_path: PathBuf::from("addons.toml"),
            index_name: "Guild UI".to_string(),
            package_count: 2,
            used_metadata_package_count: 1,
            inferred_name_package_count: 1,
            inferred_version_package_count: 2,
            placeholder_version_package_count: 1,
            package_ids: vec!["plater".to_string(), "weakauras".to_string()],
        });

        assert!(rendered.contains("Wrote addon index scaffold: addons.toml"));
        assert!(rendered.contains("Name: Guild UI"));
        assert!(rendered.contains("Packages: 2"));
        assert!(rendered.contains("Used existing metadata: 1"));
        assert!(rendered.contains("Inferred names: 1"));
        assert!(rendered.contains("Inferred versions: 2"));
        assert!(rendered.contains("Placeholder versions: 1"));
        assert!(rendered.contains("Package ids: plater, weakauras"));
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
                installed_dependency_package_count: 1,
                installed_dependency_packages: vec![sample_tracked_package("sharedmedia")],
                ignored_package_count: 1,
                ignored_packages: vec!["plater".to_string()],
                backup_path: None,
            },
        });

        assert!(rendered.contains("Updated index packages: details 2.0.0, weakauras 5.18.2"));
        assert!(
            rendered.contains(
                "Installed dependency packages: sharedmedia [WeakAuras, WeakAurasOptions]"
            )
        );
        assert!(rendered.contains("Ignored packages: plater"));
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
    fn render_addon_adopt_reports_snapshot_archive() {
        let rendered = render_addon_adopt(&AdoptedAddonPackageResult {
            dry_run: false,
            source: sample_source(),
            source_label: "local.zip".to_string(),
            package_id: "guild-ui".to_string(),
            addon_count: 2,
            addons: vec![
                sample_tracked_addon("WeakAuras"),
                sample_tracked_addon("SharedMedia"),
            ],
            registry_path: PathBuf::from("app-data/wow/test-install/retail/addons/addons.toml"),
        });

        assert!(rendered.contains("Adopted package: guild-ui"));
        assert!(rendered.contains("Snapshot archive: local.zip"));
        assert!(rendered.contains("Addons: WeakAuras, SharedMedia"));
        assert!(rendered.contains("Registry: app-data/wow/test-install/retail/addons/addons.toml"));
    }

    #[test]
    fn render_addon_relink_reports_source_transition() {
        let rendered = render_addon_relink(&RelinkedAddonPackageResult {
            dry_run: true,
            package_id: "plater".to_string(),
            previous_source: sample_source(),
            previous_source_label: "local.zip".to_string(),
            source: crate::core::app::AddonSourceResult {
                kind: crate::core::app::AddonSourceKindResult::GitHubRelease,
                display_name: "github:foo/plater".to_string(),
                dependency_resolution_capability:
                    crate::core::app::AddonDependencyResolutionCapabilityValue::Unsupported,
                local_archive_path: None,
                url: None,
                mod_id: None,
                file_id: None,
                owner: Some("foo".to_string()),
                repo: Some("plater".to_string()),
                tag: None,
                asset_name: None,
            },
            source_label: "github:foo/plater".to_string(),
            addon_count: 1,
            addons: vec![sample_tracked_addon("Plater")],
            registry_path: PathBuf::from("addon-registry.json"),
            cleared_metadata: true,
        });

        assert!(rendered.contains("Dry run only."));
        assert!(rendered.contains("Package: plater"));
        assert!(rendered.contains("From: local.zip"));
        assert!(rendered.contains("To: github:foo/plater"));
        assert!(rendered.contains("Metadata: cleared"));
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
            installed_dependency_package_count: 1,
            installed_dependency_packages: vec![sample_tracked_package("sharedmedia")],
            ignored_package_count: 1,
            ignored_packages: vec!["details".to_string()],
            backup_path: None,
        });

        assert!(rendered.contains("Updated packages: weakauras [WeakAuras, WeakAurasOptions]"));
        assert!(
            rendered.contains(
                "Installed dependency packages: sharedmedia [WeakAuras, WeakAurasOptions]"
            )
        );
        assert!(rendered.contains("Ignored packages: details"));
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

    #[test]
    fn render_addon_cache_purge_reports_configured_summary() {
        let rendered = render_addon_cache_purge(&AddonCachePurgeResult {
            configured: true,
            cache_dir: Some(PathBuf::from("cache/addons")),
            removed_file_count: 3,
            removed_directory_count: 2,
            reclaimed_bytes: 2048,
        });

        assert!(rendered.contains("Purged addon cache: cache/addons"));
        assert!(rendered.contains("Removed files: 3"));
        assert!(rendered.contains("Removed directories: 2"));
        assert!(rendered.contains("Reclaimed bytes: 2048"));
    }

    #[test]
    fn render_addon_cache_repair_reports_not_configured() {
        let rendered = render_addon_cache_repair(&AddonCacheRepairResult {
            configured: false,
            cache_dir: None,
            scanned_metadata_count: 0,
            repaired_entry_count: 0,
            invalid_metadata_count: 0,
            missing_archive_count: 0,
            mismatched_archive_count: 0,
            orphan_archive_count: 0,
            partial_download_count: 0,
            remote_verified_entry_count: 0,
            remote_refreshed_entry_count: 0,
            remote_check_failed_count: 0,
            expired_freshness_entry_count: 0,
            removed_file_count: 0,
            removed_directory_count: 0,
            reclaimed_bytes: 0,
        });

        assert_eq!(rendered, "Addon download cache is not configured.");
    }

    #[test]
    fn render_addon_cache_repair_reports_remote_validation_summary() {
        let rendered = render_addon_cache_repair(&AddonCacheRepairResult {
            configured: true,
            cache_dir: Some(PathBuf::from("cache/addons")),
            scanned_metadata_count: 5,
            repaired_entry_count: 2,
            invalid_metadata_count: 1,
            missing_archive_count: 0,
            mismatched_archive_count: 1,
            orphan_archive_count: 0,
            partial_download_count: 1,
            remote_verified_entry_count: 3,
            remote_refreshed_entry_count: 1,
            remote_check_failed_count: 2,
            expired_freshness_entry_count: 1,
            removed_file_count: 4,
            removed_directory_count: 2,
            reclaimed_bytes: 4096,
        });

        assert!(rendered.contains("Repaired addon cache: cache/addons"));
        assert!(rendered.contains("Remote verified entries: 3"));
        assert!(rendered.contains("Remote refreshed entries: 1"));
        assert!(rendered.contains("Remote check failures: 2"));
        assert!(rendered.contains("Expired freshness entries: 1"));
    }
}
