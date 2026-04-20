use crate::core::app::{
    BundleApplyPlanResult, BundleApplyResult, BundleInspectionResult, CreatedBundleResult,
};

use super::shared::{
    format_bundle_characters, format_character_mapping_summary, format_discovered_accounts,
    format_selected_accounts,
};

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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::test_support::{
        sample_bundle_character_resource, sample_bundle_manifest, sample_character_mapping,
        sample_group_policies, sample_local_account,
    };
    use super::*;
    use crate::core::app::{
        ApplyPlanSummaryResult, BundleApplyPlanResult, BundleApplyResult, BundleEntryCountsResult,
        BundleInspectionResult, BundleResourcesResult, CreatedBundleResult, HelperStrategyValue,
    };

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
}
