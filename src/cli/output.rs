use serde::Serialize;

use crate::core::app::{
    AddonIndexInspectionResult, AddonIndexInstallResult, AddonIndexUpdateResult,
    AddonLockApplyResult, AddonLockDiffResult, AddonLockInspectionResult,
    AddonLockPackageDiffResult, AddonLockPackageSnapshotResult, AddonLockPlanResult,
    AddonLockVerifyResult, AddonLockWriteResult, BundleApplyPlanResult, BundleApplyResult,
    BundleCharacterResourceResult, BundleInspectionResult, CharacterMappingResult,
    CreatedBundleResult, ExternalPackageAnalysisResult, ExternalPackageApplyPlanResult,
    ExternalPackageApplyResult, ExternalPackageSummaryResult, ExternalPackageWarningCategoryValue,
    ExternalPackageWarningCodeValue, ExternalPackageWarningResult, LocalWowAccountResult,
};
use crate::core::error::AppResult;

pub(super) fn render_addon_index_inspection(item: &AddonIndexInspectionResult) -> String {
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

pub(super) fn render_addon_index_install(item: &AddonIndexInstallResult) -> String {
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

pub(super) fn render_addon_index_update(item: &AddonIndexUpdateResult) -> String {
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

pub(super) fn render_bundle_archive_created(item: &CreatedBundleResult) -> String {
    format!(
        "Created bundle: {}\nArchived files: {}\nPackage: {}",
        item.archive_path.display(),
        item.archived_files,
        item.manifest.package.name
    )
}

pub(super) fn render_bundle_archive_inspection(item: &BundleInspectionResult) -> String {
    format!(
        "Bundle: {}\nPackage: {}\nSource flavor: {}\nFiles: {}\nAddOns: {}\nWTF common: {}\nWTF characters: {}\nFonts: {}\nInterface assets: {}\nCharacters: {}",
        item.archive_path.display(),
        item.package.name,
        item.source.flavor.as_str(),
        item.entries.total_files,
        item.entries.addons,
        item.entries.wtf_common,
        item.entries.wtf_characters,
        item.entries.fonts,
        item.entries.interface_assets,
        format_bundle_characters(&item.resources.wtf_characters)
    )
}

pub(super) fn render_bundle_apply_plan(item: &BundleApplyPlanResult) -> String {
    format!(
        "Bundle: {}\nTarget: {}\nDiscovered accounts: {}\nSelected accounts: {}\nPlanned remove: {}\nPlanned add: {}\nPlanned replace: {}\nPlanned skip: {}\nPlanned preserve: {}\nCharacter mappings: {}",
        item.bundle_path.display(),
        item.target_flavor_root.display(),
        format_discovered_accounts(&item.discovered_accounts),
        format_selected_accounts(&item.selected_target_accounts),
        item.summary.paths_to_remove,
        item.summary.files_to_add,
        item.summary.files_to_replace,
        item.summary.files_to_skip,
        item.summary.files_to_preserve,
        format_character_mapping_summary(&item.character_mappings)
    )
}

pub(super) fn render_bundle_apply(item: &BundleApplyResult) -> String {
    let backup = item
        .backup_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "none".to_string());
    let selected_accounts = format_selected_accounts(&item.selected_target_accounts);
    let mapping_summary = format_character_mapping_summary(&item.character_mappings);

    if item.dry_run {
        format!(
            "Dry run only.\nBundle: {}\nTarget: {}\nPlanned files: {}\nSelected accounts: {}\nPlanned remove: {}\nPlanned add: {}\nPlanned replace: {}\nPlanned skip: {}\nPlanned preserve: {}\nCharacter mappings: {}\nBackup: {}",
            item.bundle_path.display(),
            item.target_flavor_root.display(),
            item.planned_files,
            selected_accounts,
            item.plan_summary.paths_to_remove,
            item.plan_summary.files_to_add,
            item.plan_summary.files_to_replace,
            item.plan_summary.files_to_skip,
            item.plan_summary.files_to_preserve,
            mapping_summary,
            backup
        )
    } else {
        format!(
            "Unpacked bundle: {}\nTarget: {}\nWritten files: {}\nRewritten files: {}\nSelected accounts: {}\nCharacter mappings: {}\nBackup: {}",
            item.bundle_path.display(),
            item.target_flavor_root.display(),
            item.written_files,
            item.rewritten_files,
            selected_accounts,
            mapping_summary,
            backup
        )
    }
}

pub(super) fn render_external_package_analysis(item: &ExternalPackageAnalysisResult) -> String {
    let warnings = format_external_package_warnings(&item.warnings, &item.summary);

    format!(
        "Source: {}\nDetected kind: {:?}\nPackage id: {}\nPackage name: {}\nFiles: {}\nNormalized files: {}\nIgnored files: {}\nAddOns: {}\nWTF common: {}\nWTF characters: {}\nFonts: {}\nInterface assets: {}\nCharacters: {}\nWarnings: {}",
        item.source_path.display(),
        item.source_kind,
        item.package_id,
        item.package_name,
        item.summary.total_files,
        item.summary.normalized_files,
        item.summary.ignored_files,
        format_string_list_or_none(&item.resources.addons),
        if item.resources.wtf_common {
            "yes"
        } else {
            "no"
        },
        item.resources.wtf_characters.len(),
        if item.resources.fonts { "yes" } else { "no" },
        format_string_list_or_none(&item.resources.interface_assets),
        format_bundle_characters(&item.resources.wtf_characters),
        warnings
    )
}

pub(super) fn render_external_package_plan(item: &ExternalPackageApplyPlanResult) -> String {
    format!(
        "External package: {}\nTarget: {}\nDiscovered accounts: {}\nSelected accounts: {}\nWarnings: {}\nPlanned remove: {}\nPlanned add: {}\nPlanned replace: {}\nPlanned skip: {}\nPlanned preserve: {}\nCharacter mappings: {}",
        item.analysis.source_path.display(),
        item.target_flavor_root.display(),
        format_discovered_accounts(&item.discovered_accounts),
        format_selected_accounts(&item.selected_target_accounts),
        format_external_package_warnings(&item.analysis.warnings, &item.analysis.summary),
        item.summary.paths_to_remove,
        item.summary.files_to_add,
        item.summary.files_to_replace,
        item.summary.files_to_skip,
        item.summary.files_to_preserve,
        format_character_mapping_summary(&item.character_mappings)
    )
}

pub(super) fn render_external_package_apply(item: &ExternalPackageApplyResult) -> String {
    let backup = item
        .backup_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "none".to_string());
    let selected_accounts = format_selected_accounts(&item.selected_target_accounts);
    let mapping_summary = format_character_mapping_summary(&item.character_mappings);

    if item.dry_run {
        format!(
            "Dry run only.\nExternal package: {}\nTarget: {}\nWarnings: {}\nPlanned files: {}\nSelected accounts: {}\nPlanned remove: {}\nPlanned add: {}\nPlanned replace: {}\nPlanned skip: {}\nPlanned preserve: {}\nCharacter mappings: {}\nBackup: {}",
            item.analysis.source_path.display(),
            item.target_flavor_root.display(),
            format_external_package_warnings(&item.analysis.warnings, &item.analysis.summary),
            item.planned_files,
            selected_accounts,
            item.plan_summary.paths_to_remove,
            item.plan_summary.files_to_add,
            item.plan_summary.files_to_replace,
            item.plan_summary.files_to_skip,
            item.plan_summary.files_to_preserve,
            mapping_summary,
            backup
        )
    } else {
        format!(
            "Applied external package: {}\nTarget: {}\nWritten files: {}\nRewritten files: {}\nSelected accounts: {}\nCharacter mappings: {}\nBackup: {}",
            item.analysis.source_path.display(),
            item.target_flavor_root.display(),
            item.written_files,
            item.rewritten_files,
            selected_accounts,
            mapping_summary,
            backup
        )
    }
}

pub(super) fn render_addon_lock_inspection(item: &AddonLockInspectionResult) -> String {
    let packages = item
        .packages
        .iter()
        .map(|package| {
            format!(
                "{} {} => {} ({})",
                package.package_id,
                package.version.as_deref().unwrap_or("unknown"),
                package.addon_directories.join(", "),
                package.content_sha256
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "Lock: {}\nGenerated: {}\nPackages: {}\n{}",
        item.lock_path.display(),
        item.generated_at,
        item.package_count,
        if packages.is_empty() {
            "none".to_string()
        } else {
            packages
        }
    )
}

pub(super) fn render_addon_lock_write(item: &AddonLockWriteResult) -> String {
    if item.removed {
        format!(
            "Removed addon lock: {}\nTracked packages: 0",
            item.lock_path.display()
        )
    } else {
        format!(
            "Wrote addon lock: {}\nTracked packages: {}",
            item.lock_path.display(),
            item.package_count
        )
    }
}

pub(super) fn render_addon_lock_diff(item: &AddonLockDiffResult) -> String {
    let mut lines = vec![
        format!("Left: {}", item.left_label),
        format!("Right: {}", item.right_label),
        format!(
            "Summary: {} changed, {} added, {} removed, {} unchanged",
            item.changed_packages.len(),
            item.added_packages.len(),
            item.removed_packages.len(),
            item.unchanged_packages
        ),
    ];

    if item.identical {
        lines.push("Result: identical".to_string());
        return lines.join("\n");
    }

    push_changed_packages(&mut lines, "Changed packages:", &item.changed_packages);
    push_snapshot_packages(&mut lines, "Added packages:", &item.added_packages);
    push_snapshot_packages(&mut lines, "Removed packages:", &item.removed_packages);

    lines.join("\n")
}

pub(super) fn render_addon_lock_verify(item: &AddonLockVerifyResult) -> String {
    let mut lines = vec![
        format!("Lock: {}", item.lock_path.display()),
        format!("Installation: {}", item.installation_root.display()),
        format!(
            "Summary: {} changed, {} added, {} removed, {} unchanged",
            item.diff.changed_packages.len(),
            item.diff.added_packages.len(),
            item.diff.removed_packages.len(),
            item.diff.unchanged_packages
        ),
    ];

    if item.matches {
        lines.push("Result: verified".to_string());
        return lines.join("\n");
    }

    lines.push("Result: drift detected".to_string());

    if !item.missing_addon_directories.is_empty() {
        lines.push("Missing tracked addon directories:".to_string());
        for issue in &item.missing_addon_directories {
            lines.push(format!(
                "- {} => {}",
                issue.package_id,
                issue.missing_addon_directories.join(", ")
            ));
        }
    }

    if !item.untracked_addons.is_empty() {
        lines.push(format!(
            "Untracked addon directories: {}",
            item.untracked_addons.join(", ")
        ));
    }

    push_changed_packages(&mut lines, "Changed packages:", &item.diff.changed_packages);
    push_snapshot_packages(
        &mut lines,
        "Unexpected tracked packages:",
        &item.diff.added_packages,
    );
    push_snapshot_packages(
        &mut lines,
        "Missing expected packages:",
        &item.diff.removed_packages,
    );

    lines.join("\n")
}

pub(super) fn render_addon_lock_apply_summary(
    mut lines: Vec<String>,
    item: &AddonLockApplyResult,
) -> String {
    if !item.untracked_addons.is_empty() {
        lines.push(format!(
            "Untracked addon directories remain: {}",
            item.untracked_addons.join(", ")
        ));
    }

    lines.push(format_addon_lock_verification_summary(&item.verification));
    lines.join("\n")
}

pub(super) fn render_addon_lock_plan_summary(header: &str, item: &AddonLockPlanResult) -> String {
    let mut lines = vec![
        header.to_string(),
        format!("Embedded/lock path: {}", item.lock_path.display()),
        format!("Installation: {}", item.installation_root.display()),
        format!(
            "Summary: {} install, {} update, {} remove, {} metadata-only, {} unchanged, {} blocked",
            item.install_count,
            item.update_count,
            item.remove_count,
            item.metadata_only_count,
            item.unchanged_count,
            item.blocked_count
        ),
    ];

    if !item.untracked_addons.is_empty() {
        lines.push(format!(
            "Untracked addon directories: {}",
            item.untracked_addons.join(", ")
        ));
    }

    if item.actions.is_empty() {
        lines.push("No sync actions required.".to_string());
        return lines.join("\n");
    }

    lines.push("Actions:".to_string());
    for action in &item.actions {
        let reason = if action.reasons.is_empty() {
            "no details".to_string()
        } else {
            action.reasons.join("; ")
        };
        let mut suffix = String::new();
        if action.requires_replace_existing {
            suffix.push_str(" | requires --replace-existing");
        }
        if !action.blocked_reasons.is_empty() {
            suffix.push_str(&format!(
                " | blocked: {}",
                action.blocked_reasons.join("; ")
            ));
        }
        lines.push(format!(
            "- {:?}: {} ({}){}",
            action.kind, action.package_id, reason, suffix
        ));
    }

    lines.join("\n")
}

fn push_changed_packages(
    lines: &mut Vec<String>,
    heading: &str,
    packages: &[AddonLockPackageDiffResult],
) {
    if packages.is_empty() {
        return;
    }

    lines.push(heading.to_string());
    for package in packages {
        let changed_fields = package
            .changes
            .iter()
            .map(|change| change.field.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!(
            "- {} ({})",
            addon_lock_package_label(package.left.name.as_deref(), &package.left.package_id),
            changed_fields
        ));
    }
}

fn push_snapshot_packages(
    lines: &mut Vec<String>,
    heading: &str,
    packages: &[AddonLockPackageSnapshotResult],
) {
    if packages.is_empty() {
        return;
    }

    lines.push(heading.to_string());
    for package in packages {
        lines.push(format!(
            "- {}",
            addon_lock_package_label(package.name.as_deref(), &package.package_id)
        ));
    }
}

fn addon_lock_package_label(name: Option<&str>, package_id: &str) -> String {
    name.unwrap_or(package_id).to_string()
}

fn format_bundle_characters(resources: &[BundleCharacterResourceResult]) -> String {
    let characters = resources
        .iter()
        .map(|character| {
            format!(
                "{}/{}/{}",
                character
                    .source_account
                    .as_deref()
                    .unwrap_or("<unknown-account>"),
                character.source_server,
                character.source_character
            )
        })
        .collect::<Vec<_>>();

    if characters.is_empty() {
        "none".to_string()
    } else {
        characters.join(", ")
    }
}

fn format_discovered_accounts(accounts: &[LocalWowAccountResult]) -> String {
    if accounts.is_empty() {
        "none".to_string()
    } else {
        accounts
            .iter()
            .map(|account| {
                format!(
                    "{}({} chars)",
                    account.account_name,
                    account.characters.len()
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn format_string_list_or_none(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_string()
    } else {
        items.join(", ")
    }
}

fn format_selected_accounts(accounts: &[String]) -> String {
    format_string_list_or_none(accounts)
}

fn format_character_mapping_summary(mappings: &[CharacterMappingResult]) -> String {
    if mappings.is_empty() {
        "none".to_string()
    } else {
        format_character_mappings(mappings)
    }
}

pub(super) fn format_character_mappings(mappings: &[CharacterMappingResult]) -> String {
    mappings
        .iter()
        .map(|mapping| {
            format!(
                "{}/{}/{} -> {}/{}/{}",
                mapping
                    .source_account
                    .as_deref()
                    .unwrap_or("<unknown-account>"),
                mapping.source_server,
                mapping.source_character,
                mapping.target_account,
                mapping.target_server,
                mapping.target_character
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn format_external_package_warnings(
    warnings: &[ExternalPackageWarningResult],
    summary: &ExternalPackageSummaryResult,
) -> String {
    if warnings.is_empty() {
        return "none".to_string();
    }

    let groups = summary
        .warning_groups
        .iter()
        .map(|group| {
            format!(
                "{}/{}={}",
                format_warning_category(group.category),
                format_warning_code(group.code),
                group.count
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    let details = warnings
        .iter()
        .map(|warning| {
            format!(
                "{}/{}: {}",
                format_warning_category(warning.category),
                format_warning_code(warning.code),
                warning.source_path
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");

    format!(
        "{} (addon: {}, wtf: {}; groups: [{}]) [{}]",
        summary.warning_count,
        summary.addon_warning_count,
        summary.wtf_warning_count,
        groups,
        details
    )
}

fn format_warning_category(category: ExternalPackageWarningCategoryValue) -> &'static str {
    match category {
        ExternalPackageWarningCategoryValue::Addon => "addon",
        ExternalPackageWarningCategoryValue::Wtf => "wtf",
    }
}

fn format_warning_code(code: ExternalPackageWarningCodeValue) -> &'static str {
    match code {
        ExternalPackageWarningCodeValue::AddonRootNotDetected => "addon_root_not_detected",
        ExternalPackageWarningCodeValue::UnsupportedWtfLayout => "unsupported_wtf_layout",
        ExternalPackageWarningCodeValue::UnsupportedWtfRootSavedVariables => {
            "unsupported_wtf_root_savedvariables"
        }
        ExternalPackageWarningCodeValue::WtfAccountPathWithoutFile => {
            "wtf_account_path_without_file"
        }
        ExternalPackageWarningCodeValue::WtfSavedVariablesPathWithoutFile => {
            "wtf_savedvariables_path_without_file"
        }
        ExternalPackageWarningCodeValue::UnsupportedWtfNestedAccountLayout => {
            "unsupported_wtf_nested_account_layout"
        }
    }
}

fn format_addon_lock_verification_summary(item: &AddonLockVerifyResult) -> String {
    if item.matches {
        "Verification: matches".to_string()
    } else {
        format!(
            "Verification: drift remains ({} changed, {} added, {} removed)",
            item.diff.changed_packages.len(),
            item.diff.added_packages.len(),
            item.diff.removed_packages.len()
        )
    }
}

pub(super) fn render<T, F>(json: bool, value: &T, text_renderer: F) -> AppResult<()>
where
    T: Serialize,
    F: FnOnce(&T) -> String,
{
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{}", text_renderer(value));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::core::app::{
        AddonIndexInspectionResult, AddonIndexInstallResult, AddonIndexPackageResult,
        AddonIndexUpdateResult, AddonLockFieldChangeResult, AddonLockPackageDirectoryIssueResult,
        AddonSourceKindResult, AddonSourceResult, ApplyGroupPoliciesResult, ApplyPlanSummaryResult,
        BundleApplyDefaultsValue, BundleApplyPlanResult, BundleApplyResult,
        BundleCharacterResourceValue, BundleEntryCountsResult, BundleInspectionResult,
        BundleManifestValue, BundleMappingRulesValue, BundlePackageValue, BundleResourcesResult,
        BundleResourcesValue, BundleSourceValue, CharacterMappingModeValue, CharacterMappingResult,
        CreatedBundleResult, ExternalPackageAnalysisResult, ExternalPackageApplyPlanResult,
        ExternalPackageApplyResult, ExternalPackageEntryResult, ExternalPackageSummaryResult,
        ExternalPackageWarningCategoryValue, ExternalPackageWarningCodeValue,
        ExternalPackageWarningGroupResult, ExternalPackageWarningResult, GroupPolicyResult,
        HelperStrategyValue, HostPlatformValue, InstalledAddonPackageResult, LocalWowAccountResult,
        LocalWowCharacterResult, ResourceApplyPolicyValue, TrackedAddonResult,
        UpdatedAddonPackageResult, WowFlavorValue,
    };
    use crate::core::bundle::ExternalPackageSourceKind;

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
    fn render_bundle_archive_created_reports_archive_summary() {
        let rendered = render_bundle_archive_created(&CreatedBundleResult {
            archive_path: PathBuf::from("AuthorUI.zip"),
            archived_files: 42,
            manifest: sample_bundle_manifest(),
        });

        assert!(rendered.contains("Created bundle: AuthorUI.zip"));
        assert!(rendered.contains("Archived files: 42"));
        assert!(rendered.contains("Package: Author UI"));
    }

    #[test]
    fn render_bundle_archive_inspection_lists_bundle_characters() {
        let rendered = render_bundle_archive_inspection(&BundleInspectionResult {
            archive_path: PathBuf::from("AuthorUI.zip"),
            package: sample_bundle_manifest().package,
            source: sample_bundle_manifest().source,
            resources: BundleResourcesResult {
                addons: vec!["WeakAuras".to_string()],
                addon_count: 1,
                wtf_common: true,
                wtf_character_count: 1,
                wtf_characters: vec![sample_bundle_character_resource()],
                fonts: true,
                interface_assets: vec!["Interface/SharedXML".to_string()],
                interface_asset_count: 1,
                addon_lock: true,
                addon_indexes: vec!["addons.toml".to_string()],
            },
            entries: BundleEntryCountsResult {
                total_files: 64,
                addons: 20,
                wtf_common: 10,
                wtf_characters: 30,
                fonts: 2,
                interface_assets: 1,
                metadata: 1,
            },
        });

        assert!(rendered.contains("Bundle: AuthorUI.zip"));
        assert!(rendered.contains("Package: Author UI"));
        assert!(rendered.contains("Source flavor: retail"));
        assert!(rendered.contains("Files: 64"));
        assert!(rendered.contains("Characters: AccountA/Aegwynn/Hero"));
    }

    #[test]
    fn render_bundle_apply_plan_reports_accounts_and_mappings() {
        let rendered = render_bundle_apply_plan(&BundleApplyPlanResult {
            bundle_path: PathBuf::from("AuthorUI.zip"),
            target_flavor_root: PathBuf::from("World of Warcraft/_retail_"),
            discovered_accounts: vec![
                sample_local_account("AccountA"),
                sample_local_account("AccountB"),
            ],
            selected_target_accounts: vec!["AccountA".to_string()],
            character_mappings: vec![sample_character_mapping()],
            operations: Vec::new(),
            summary: ApplyPlanSummaryResult {
                files_to_add: 10,
                files_to_replace: 4,
                files_to_skip: 3,
                paths_to_remove: 2,
                files_to_preserve: 5,
            },
            helper_strategy: HelperStrategyValue::NativeRust,
            group_policies: sample_group_policies(),
            manifest: sample_bundle_manifest(),
        });

        assert!(rendered.contains("Bundle: AuthorUI.zip"));
        assert!(rendered.contains("Target: World of Warcraft/_retail_"));
        assert!(rendered.contains("Discovered accounts: AccountA(1 chars), AccountB(1 chars)"));
        assert!(rendered.contains("Selected accounts: AccountA"));
        assert!(rendered.contains("Planned remove: 2"));
        assert!(
            rendered.contains(
                "Character mappings: AccountA/Aegwynn/Hero -> TargetAccount/Illidan/Main"
            )
        );
    }

    #[test]
    fn render_bundle_apply_reports_written_files_and_backup() {
        let rendered = render_bundle_apply(&BundleApplyResult {
            bundle_path: PathBuf::from("AuthorUI.zip"),
            target_flavor_root: PathBuf::from("World of Warcraft/_retail_"),
            dry_run: false,
            planned_files: 0,
            written_files: 27,
            rewritten_files: 6,
            backup_path: Some(PathBuf::from("backup.zip")),
            selected_target_accounts: vec!["TargetAccount".to_string()],
            plan_summary: ApplyPlanSummaryResult {
                files_to_add: 0,
                files_to_replace: 0,
                files_to_skip: 0,
                paths_to_remove: 0,
                files_to_preserve: 0,
            },
            character_mappings: vec![sample_character_mapping()],
            manifest: sample_bundle_manifest(),
        });

        assert!(rendered.contains("Unpacked bundle: AuthorUI.zip"));
        assert!(rendered.contains("Written files: 27"));
        assert!(rendered.contains("Rewritten files: 6"));
        assert!(rendered.contains("Selected accounts: TargetAccount"));
        assert!(rendered.contains("Backup: backup.zip"));
    }

    #[test]
    fn render_external_package_analysis_reports_resources_and_warnings() {
        let rendered = render_external_package_analysis(&sample_external_package_analysis());

        assert!(rendered.contains("Source: C:\\temp\\author-ui.zip"));
        assert!(rendered.contains("Detected kind: ZipArchive"));
        assert!(rendered.contains("AddOns: WeakAuras"));
        assert!(rendered.contains("Characters: AccountA/Aegwynn/Hero"));
        assert!(rendered.contains("Warnings: 1 (addon: 1, wtf: 0; groups: ["));
    }

    #[test]
    fn render_external_package_plan_reports_accounts_and_mappings() {
        let rendered = render_external_package_plan(&ExternalPackageApplyPlanResult {
            analysis: sample_external_package_analysis(),
            target_flavor_root: PathBuf::from("World of Warcraft/_retail_"),
            discovered_accounts: vec![sample_local_account("AccountA")],
            selected_target_accounts: vec!["TargetAccount".to_string()],
            character_mappings: vec![sample_character_mapping()],
            operations: Vec::new(),
            summary: ApplyPlanSummaryResult {
                files_to_add: 8,
                files_to_replace: 4,
                files_to_skip: 2,
                paths_to_remove: 1,
                files_to_preserve: 3,
            },
            helper_strategy: HelperStrategyValue::NativeRust,
            group_policies: sample_group_policies(),
            manifest: sample_bundle_manifest(),
        });

        assert!(rendered.contains("External package: C:\\temp\\author-ui.zip"));
        assert!(rendered.contains("Discovered accounts: AccountA(1 chars)"));
        assert!(rendered.contains("Selected accounts: TargetAccount"));
        assert!(rendered.contains("Planned replace: 4"));
        assert!(
            rendered.contains(
                "Character mappings: AccountA/Aegwynn/Hero -> TargetAccount/Illidan/Main"
            )
        );
    }

    #[test]
    fn render_external_package_apply_reports_written_files() {
        let rendered = render_external_package_apply(&ExternalPackageApplyResult {
            analysis: sample_external_package_analysis(),
            target_flavor_root: PathBuf::from("World of Warcraft/_retail_"),
            dry_run: false,
            planned_files: 0,
            written_files: 19,
            rewritten_files: 5,
            backup_path: Some(PathBuf::from("backup.zip")),
            selected_target_accounts: vec!["TargetAccount".to_string()],
            plan_summary: ApplyPlanSummaryResult {
                files_to_add: 0,
                files_to_replace: 0,
                files_to_skip: 0,
                paths_to_remove: 0,
                files_to_preserve: 0,
            },
            character_mappings: vec![sample_character_mapping()],
            manifest: sample_bundle_manifest(),
        });

        assert!(rendered.contains("Applied external package: C:\\temp\\author-ui.zip"));
        assert!(rendered.contains("Written files: 19"));
        assert!(rendered.contains("Rewritten files: 5"));
        assert!(rendered.contains("Backup: backup.zip"));
    }

    #[test]
    fn render_addon_lock_diff_groups_changed_added_and_removed_packages() {
        let rendered = render_addon_lock_diff(&AddonLockDiffResult {
            left_label: "left.lock".to_string(),
            right_label: "right.lock".to_string(),
            left_package_count: 1,
            right_package_count: 2,
            identical: false,
            unchanged_packages: 3,
            added_package_count: 1,
            removed_package_count: 1,
            changed_package_count: 1,
            added_packages: vec![sample_snapshot("new-package", Some("New Package"))],
            removed_packages: vec![sample_snapshot("old-package", None)],
            changed_packages: vec![AddonLockPackageDiffResult {
                comparison_key: "shared".to_string(),
                left: sample_snapshot("details", Some("Details")),
                right: sample_snapshot("details", Some("Details")),
                changes: vec![AddonLockFieldChangeResult {
                    field: "version".to_string(),
                    left: Some("1.0.0".to_string()),
                    right: Some("2.0.0".to_string()),
                }],
            }],
        });

        assert!(rendered.contains("Summary: 1 changed, 1 added, 1 removed, 3 unchanged"));
        assert!(rendered.contains("Changed packages:"));
        assert!(rendered.contains("- Details (version)"));
        assert!(rendered.contains("Added packages:"));
        assert!(rendered.contains("- New Package"));
        assert!(rendered.contains("Removed packages:"));
        assert!(rendered.contains("- old-package"));
    }

    #[test]
    fn render_addon_lock_verify_includes_missing_and_untracked_sections() {
        let rendered = render_addon_lock_verify(&AddonLockVerifyResult {
            lock_path: PathBuf::from("addon.lock"),
            installation_root: PathBuf::from("World of Warcraft/_retail_"),
            tracked_package_count: 2,
            untracked_addon_count: 1,
            untracked_addons: vec!["LooseAddon".to_string()],
            missing_package_count: 1,
            missing_addon_directories: vec![AddonLockPackageDirectoryIssueResult {
                comparison_key: "pkg".to_string(),
                package_id: "weakauras".to_string(),
                missing_addon_directories: vec!["WeakAuras".to_string()],
            }],
            diff: AddonLockDiffResult {
                left_label: "lock".to_string(),
                right_label: "install".to_string(),
                left_package_count: 2,
                right_package_count: 2,
                identical: false,
                unchanged_packages: 0,
                added_package_count: 1,
                removed_package_count: 1,
                changed_package_count: 1,
                added_packages: vec![sample_snapshot("extra", None)],
                removed_packages: vec![sample_snapshot("missing", Some("Missing Package"))],
                changed_packages: vec![AddonLockPackageDiffResult {
                    comparison_key: "details".to_string(),
                    left: sample_snapshot("details", Some("Details")),
                    right: sample_snapshot("details", Some("Details")),
                    changes: vec![AddonLockFieldChangeResult {
                        field: "content_sha256".to_string(),
                        left: Some("old".to_string()),
                        right: Some("new".to_string()),
                    }],
                }],
            },
            matches: false,
        });

        assert!(rendered.contains("Result: drift detected"));
        assert!(rendered.contains("Missing tracked addon directories:"));
        assert!(rendered.contains("- weakauras => WeakAuras"));
        assert!(rendered.contains("Untracked addon directories: LooseAddon"));
        assert!(rendered.contains("Unexpected tracked packages:"));
        assert!(rendered.contains("- extra"));
        assert!(rendered.contains("Missing expected packages:"));
        assert!(rendered.contains("- Missing Package"));
    }

    fn sample_snapshot(package_id: &str, name: Option<&str>) -> AddonLockPackageSnapshotResult {
        AddonLockPackageSnapshotResult {
            comparison_key: package_id.to_string(),
            package_id: package_id.to_string(),
            index_name: None,
            index_package_id: None,
            name: name.map(ToString::to_string),
            version: Some("1.0.0".to_string()),
            source: sample_source(),
            source_label: "local.zip".to_string(),
            source_url: None,
            website_url: None,
            source_sha256: None,
            content_sha256: Some("sha256".to_string()),
            addon_directories: vec!["AddonDir".to_string()],
        }
    }

    fn sample_index_package(package_id: &str, version: &str) -> AddonIndexPackageResult {
        AddonIndexPackageResult {
            id: package_id.to_string(),
            name: package_id.to_string(),
            version: version.to_string(),
            source: sample_source(),
            source_label: "local.zip".to_string(),
            source_url: None,
            website_url: None,
            sha256: None,
            addon_directories: vec![package_id.to_string()],
            supported_flavors: vec!["retail".to_string()],
        }
    }

    fn sample_tracked_addon(directory_name: &str) -> TrackedAddonResult {
        TrackedAddonResult {
            directory_name: directory_name.to_string(),
            toc_file: Some(format!("{directory_name}.toc")),
            title: Some(directory_name.to_string()),
            version: Some("1.0.0".to_string()),
        }
    }

    fn sample_bundle_manifest() -> BundleManifestValue {
        BundleManifestValue {
            schema_version: 1,
            package: BundlePackageValue {
                id: "author-ui".to_string(),
                name: "Author UI".to_string(),
                created_by: "tester".to_string(),
                description: None,
            },
            source: BundleSourceValue {
                flavor: WowFlavorValue::Retail,
                platform: Some(HostPlatformValue::Windows),
                exported_at: Some("2026-04-19T12:00:00Z".to_string()),
                supported_targets: vec![WowFlavorValue::Retail, WowFlavorValue::Classic],
            },
            resources: BundleResourcesValue {
                addons: vec!["WeakAuras".to_string()],
                wtf_common: true,
                wtf_characters: vec![sample_bundle_character_resource()],
                fonts: true,
                interface_assets: vec!["Interface/SharedXML".to_string()],
                addon_lock: true,
                addon_indexes: vec!["addons.toml".to_string()],
            },
            mapping: BundleMappingRulesValue {
                character_mode: CharacterMappingModeValue::Explicit,
                rewrite_profile_keys: true,
                rewrite_identity_strings: true,
                allow_cross_platform: true,
            },
            apply: BundleApplyDefaultsValue::author_package_defaults(),
        }
    }

    fn sample_bundle_character_resource() -> BundleCharacterResourceValue {
        BundleCharacterResourceValue {
            source_account: Some("AccountA".to_string()),
            source_server: "Aegwynn".to_string(),
            source_character: "Hero".to_string(),
            target_hint: None,
        }
    }

    fn sample_local_account(account_name: &str) -> LocalWowAccountResult {
        LocalWowAccountResult {
            account_name: account_name.to_string(),
            account_dir: PathBuf::from(format!("WTF/Account/{account_name}")),
            saved_variables_dir: PathBuf::from(format!(
                "WTF/Account/{account_name}/SavedVariables"
            )),
            characters: vec![LocalWowCharacterResult {
                server: "Illidan".to_string(),
                character: "Main".to_string(),
                character_dir: PathBuf::from(format!("WTF/Account/{account_name}/Illidan/Main")),
            }],
        }
    }

    fn sample_character_mapping() -> CharacterMappingResult {
        CharacterMappingResult {
            source_account: Some("AccountA".to_string()),
            source_server: "Aegwynn".to_string(),
            source_character: "Hero".to_string(),
            target_account: "TargetAccount".to_string(),
            target_server: "Illidan".to_string(),
            target_character: "Main".to_string(),
        }
    }

    fn sample_group_policies() -> ApplyGroupPoliciesResult {
        ApplyGroupPoliciesResult {
            addons: GroupPolicyResult {
                policy: ResourceApplyPolicyValue::Mirror,
            },
            wtf_common: GroupPolicyResult {
                policy: ResourceApplyPolicyValue::Share,
            },
            wtf_characters: GroupPolicyResult {
                policy: ResourceApplyPolicyValue::ReplaceSelected,
            },
            fonts: GroupPolicyResult {
                policy: ResourceApplyPolicyValue::Preserve,
            },
            interface_assets: GroupPolicyResult {
                policy: ResourceApplyPolicyValue::Mirror,
            },
            metadata: GroupPolicyResult {
                policy: ResourceApplyPolicyValue::Preserve,
            },
        }
    }

    fn sample_external_package_analysis() -> ExternalPackageAnalysisResult {
        ExternalPackageAnalysisResult {
            source_path: PathBuf::from("C:\\temp\\author-ui.zip"),
            source_kind: ExternalPackageSourceKind::ZipArchive,
            package_id: "author-ui".to_string(),
            package_name: "Author UI".to_string(),
            entry_count: 0,
            entries: Vec::<ExternalPackageEntryResult>::new(),
            resources: BundleResourcesResult {
                addons: vec!["WeakAuras".to_string()],
                addon_count: 1,
                wtf_common: true,
                wtf_character_count: 1,
                wtf_characters: vec![sample_bundle_character_resource()],
                fonts: true,
                interface_assets: vec!["Interface/SharedXML".to_string()],
                interface_asset_count: 1,
                addon_lock: false,
                addon_indexes: Vec::new(),
            },
            summary: ExternalPackageSummaryResult {
                total_files: 12,
                normalized_files: 10,
                ignored_files: 2,
                addons: 1,
                wtf_common: 1,
                wtf_characters: 1,
                fonts: 1,
                interface_assets: 1,
                warning_count: 1,
                addon_warning_count: 1,
                wtf_warning_count: 0,
                warning_groups: vec![ExternalPackageWarningGroupResult {
                    category: ExternalPackageWarningCategoryValue::Addon,
                    code: ExternalPackageWarningCodeValue::AddonRootNotDetected,
                    count: 1,
                }],
            },
            warnings: vec![ExternalPackageWarningResult {
                category: ExternalPackageWarningCategoryValue::Addon,
                code: ExternalPackageWarningCodeValue::AddonRootNotDetected,
                source_path: "AuthorUI/README.txt".to_string(),
                message: "ignored addon entry".to_string(),
            }],
        }
    }

    fn sample_source() -> AddonSourceResult {
        AddonSourceResult {
            kind: AddonSourceKindResult::LocalArchive,
            display_name: "local.zip".to_string(),
            local_archive_path: Some(PathBuf::from("local.zip")),
            url: None,
            mod_id: None,
            file_id: None,
            owner: None,
            repo: None,
            tag: None,
            asset_name: None,
        }
    }
}
