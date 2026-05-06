use super::*;

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

pub(in crate::cli) fn render_addon_index_search(item: &AddonIndexSearchResult) -> String {
    let packages = if item.packages.is_empty() {
        "none".to_string()
    } else {
        item.packages
            .iter()
            .map(|package| {
                format!(
                    "{} {} => {}",
                    package.id, package.version, package.source_label
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "Index: {}\nName: {}\nQuery: {}\nPackages: {}\nMatched packages: {}\nReturned packages: {}\n{}",
        item.index_path.display(),
        item.index_name,
        item.query,
        item.package_count,
        item.matched_package_count,
        item.returned_package_count,
        packages
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
