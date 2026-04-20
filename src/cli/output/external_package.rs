use crate::core::app::{
    ExternalPackageAnalysisResult, ExternalPackageApplyPlanResult, ExternalPackageApplyResult,
};

use super::shared::{
    format_bundle_characters, format_character_mapping_summary, format_discovered_accounts,
    format_external_package_warnings, format_selected_accounts, format_string_list_or_none,
};

pub(in crate::cli) fn render_external_package_analysis(
    item: &ExternalPackageAnalysisResult,
) -> String {
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

pub(in crate::cli) fn render_external_package_plan(
    item: &ExternalPackageApplyPlanResult,
) -> String {
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

pub(in crate::cli) fn render_external_package_apply(item: &ExternalPackageApplyResult) -> String {
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::test_support::{
        sample_bundle_manifest, sample_character_mapping, sample_external_package_analysis,
        sample_group_policies, sample_local_account,
    };
    use super::{
        render_external_package_analysis, render_external_package_apply,
        render_external_package_plan,
    };
    use crate::core::app::{
        ApplyPlanSummaryResult, ExternalPackageApplyPlanResult, ExternalPackageApplyResult,
        HelperStrategyValue,
    };

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
}
