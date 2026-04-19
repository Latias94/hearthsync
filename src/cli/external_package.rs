use super::bundle_apply::{format_character_mappings, resolve_apply_mappings};
use super::output::render;
use super::{ExternalPackageBundleOptions, ExternalPackageCommands};
use crate::core::app::{
    AnalyzeExternalPackageAppRequest, ApplyExternalPackageAppRequest, BundleApplyDefaultsValue,
    CreateExternalPackageBundleAppRequest, ExternalPackageSummaryResult,
    ExternalPackageWarningCategoryValue, ExternalPackageWarningCodeValue,
    ExternalPackageWarningResult, HearthSyncApp, PlanExternalPackageApplyAppRequest,
    ResolveInstallationRequest, ResourceApplyPolicyValue,
};
use crate::core::error::AppResult;

pub(super) fn handle_external_package_command(
    json: bool,
    command: ExternalPackageCommands,
) -> AppResult<()> {
    let app = HearthSyncApp::new();

    match command {
        ExternalPackageCommands::Inspect { source } => {
            let analysis = app.analyze_external_package(AnalyzeExternalPackageAppRequest {
                source_path: source,
            })?;
            render(json, &analysis, |item| {
                let warnings = format_external_package_warnings(&item.warnings, &item.summary);
                let characters = if item.resources.wtf_characters.is_empty() {
                    "none".to_string()
                } else {
                    item.resources
                        .wtf_characters
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
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                format!(
                    "Source: {}\nDetected kind: {:?}\nPackage id: {}\nPackage name: {}\nFiles: {}\nNormalized files: {}\nIgnored files: {}\nAddOns: {}\nWTF common: {}\nWTF characters: {}\nFonts: {}\nInterface assets: {}\nCharacters: {}\nWarnings: {}",
                    item.source_path.display(),
                    item.source_kind,
                    item.package_id,
                    item.package_name,
                    item.summary.total_files,
                    item.summary.normalized_files,
                    item.summary.ignored_files,
                    if item.resources.addons.is_empty() {
                        "none".to_string()
                    } else {
                        item.resources.addons.join(", ")
                    },
                    if item.resources.wtf_common {
                        "yes"
                    } else {
                        "no"
                    },
                    item.resources.wtf_characters.len(),
                    if item.resources.fonts { "yes" } else { "no" },
                    if item.resources.interface_assets.is_empty() {
                        "none".to_string()
                    } else {
                        item.resources.interface_assets.join(", ")
                    },
                    characters,
                    warnings
                )
            })?;
        }
        ExternalPackageCommands::Plan {
            bundle_options,
            install,
            flavor,
            mapping_file,
            target_account,
            target_server,
            target_character,
            selected_accounts,
            all_accounts,
        } => {
            let installation = app.resolve_installation(ResolveInstallationRequest {
                path: install,
                flavor: flavor.map(Into::into),
            })?;
            let apply_mappings = resolve_apply_mappings(
                mapping_file.as_deref(),
                target_account,
                target_server,
                target_character,
                selected_accounts,
                all_accounts,
            )?;
            let plan = app.plan_external_package_apply(PlanExternalPackageApplyAppRequest {
                external_package: build_external_package_bundle_request(bundle_options),
                installation,
                apply_mappings,
            })?;
            render(json, &plan, |item| {
                let accounts = if item.discovered_accounts.is_empty() {
                    "none".to_string()
                } else {
                    item.discovered_accounts
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
                };
                let selected_accounts = if item.selected_target_accounts.is_empty() {
                    "none".to_string()
                } else {
                    item.selected_target_accounts.join(", ")
                };
                format!(
                    "External package: {}\nTarget: {}\nDiscovered accounts: {}\nSelected accounts: {}\nWarnings: {}\nPlanned remove: {}\nPlanned add: {}\nPlanned replace: {}\nPlanned skip: {}\nPlanned preserve: {}\nCharacter mappings: {}",
                    item.analysis.source_path.display(),
                    item.target_flavor_root.display(),
                    accounts,
                    selected_accounts,
                    format_external_package_warnings(
                        &item.analysis.warnings,
                        &item.analysis.summary,
                    ),
                    item.summary.paths_to_remove,
                    item.summary.files_to_add,
                    item.summary.files_to_replace,
                    item.summary.files_to_skip,
                    item.summary.files_to_preserve,
                    if item.character_mappings.is_empty() {
                        "none".to_string()
                    } else {
                        format_character_mappings(&item.character_mappings)
                    }
                )
            })?;
        }
        ExternalPackageCommands::Apply {
            bundle_options,
            install,
            flavor,
            dry_run,
            backup_output,
            mapping_file,
            target_account,
            target_server,
            target_character,
            selected_accounts,
            all_accounts,
        } => {
            let installation = app.resolve_installation(ResolveInstallationRequest {
                path: install,
                flavor: flavor.map(Into::into),
            })?;
            let apply_mappings = resolve_apply_mappings(
                mapping_file.as_deref(),
                target_account,
                target_server,
                target_character,
                selected_accounts,
                all_accounts,
            )?;
            let result = app.apply_external_package(ApplyExternalPackageAppRequest {
                external_package: build_external_package_bundle_request(bundle_options),
                installation,
                dry_run,
                backup_output_path: backup_output,
                apply_mappings,
            })?;
            render(json, &result, |item| {
                let backup = item
                    .backup_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "none".to_string());
                let selected_accounts = if item.selected_target_accounts.is_empty() {
                    "none".to_string()
                } else {
                    item.selected_target_accounts.join(", ")
                };
                let mapping_summary = if item.character_mappings.is_empty() {
                    "none".to_string()
                } else {
                    format_character_mappings(&item.character_mappings)
                };
                if item.dry_run {
                    format!(
                        "Dry run only.\nExternal package: {}\nTarget: {}\nWarnings: {}\nPlanned files: {}\nSelected accounts: {}\nPlanned remove: {}\nPlanned add: {}\nPlanned replace: {}\nPlanned skip: {}\nPlanned preserve: {}\nCharacter mappings: {}\nBackup: {}",
                        item.analysis.source_path.display(),
                        item.target_flavor_root.display(),
                        format_external_package_warnings(
                            &item.analysis.warnings,
                            &item.analysis.summary,
                        ),
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
            })?;
        }
    }

    Ok(())
}

fn build_external_package_bundle_request(
    options: ExternalPackageBundleOptions,
) -> CreateExternalPackageBundleAppRequest {
    let apply_defaults = build_external_package_apply_defaults(&options);

    CreateExternalPackageBundleAppRequest {
        source_path: options.source,
        source_flavor: options.source_flavor.into(),
        source_platform: options.source_platform.map(Into::into),
        supported_targets: if options.supported_targets.is_empty() {
            Vec::new()
        } else {
            options
                .supported_targets
                .into_iter()
                .map(Into::into)
                .collect()
        },
        output_path: None,
        package_id: options.package_id,
        package_name: options.package_name,
        created_by: options.created_by,
        description: options.description,
        apply_defaults,
    }
}

fn build_external_package_apply_defaults(
    options: &ExternalPackageBundleOptions,
) -> Option<BundleApplyDefaultsValue> {
    let has_override = options.no_backup
        || options.addons_policy.is_some()
        || options.wtf_common_policy.is_some()
        || options.wtf_characters_policy.is_some()
        || options.fonts_policy.is_some()
        || options.interface_assets_policy.is_some();

    if !has_override {
        return None;
    }

    let mut defaults = BundleApplyDefaultsValue::author_package_defaults();
    defaults.create_backup = !options.no_backup;
    if let Some(policy) = options.addons_policy {
        defaults.addons = ResourceApplyPolicyValue::from(
            crate::core::manifest::ResourceApplyPolicy::from(policy),
        );
    }
    if let Some(policy) = options.wtf_common_policy {
        defaults.wtf_common = ResourceApplyPolicyValue::from(
            crate::core::manifest::ResourceApplyPolicy::from(policy),
        );
    }
    if let Some(policy) = options.wtf_characters_policy {
        defaults.wtf_characters = ResourceApplyPolicyValue::from(
            crate::core::manifest::ResourceApplyPolicy::from(policy),
        );
    }
    if let Some(policy) = options.fonts_policy {
        defaults.fonts = ResourceApplyPolicyValue::from(
            crate::core::manifest::ResourceApplyPolicy::from(policy),
        );
    }
    if let Some(policy) = options.interface_assets_policy {
        defaults.interface_assets = ResourceApplyPolicyValue::from(
            crate::core::manifest::ResourceApplyPolicy::from(policy),
        );
    }

    Some(defaults)
}

fn format_external_package_warnings(
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::cli::{ApplyPolicyArg, FlavorArg, PlatformArg};
    use crate::core::app::{
        ExternalPackageWarningCategoryValue, ExternalPackageWarningCodeValue, HostPlatformValue,
        ResourceApplyPolicyValue, WowFlavorValue,
    };

    #[test]
    fn build_external_package_bundle_request_maps_metadata_and_policy_overrides() {
        let request = build_external_package_bundle_request(ExternalPackageBundleOptions {
            source: PathBuf::from("C:\\temp\\author-ui.zip"),
            source_flavor: FlavorArg::Retail,
            source_platform: Some(PlatformArg::Windows),
            supported_targets: vec![FlavorArg::Retail, FlavorArg::Classic],
            package_id: Some("author-ui".to_string()),
            package_name: Some("Author UI".to_string()),
            created_by: Some("newbeebox-import".to_string()),
            description: Some("normalized import".to_string()),
            no_backup: true,
            addons_policy: Some(ApplyPolicyArg::Mirror),
            wtf_common_policy: Some(ApplyPolicyArg::Share),
            wtf_characters_policy: Some(ApplyPolicyArg::Sync),
            fonts_policy: Some(ApplyPolicyArg::Preserve),
            interface_assets_policy: Some(ApplyPolicyArg::ReplaceSelected),
        });

        assert_eq!(
            request.source_path,
            PathBuf::from("C:\\temp\\author-ui.zip")
        );
        assert_eq!(request.source_flavor, WowFlavorValue::Retail);
        assert_eq!(request.source_platform, Some(HostPlatformValue::Windows));
        assert_eq!(
            request.supported_targets,
            vec![WowFlavorValue::Retail, WowFlavorValue::Classic]
        );
        assert_eq!(request.package_id.as_deref(), Some("author-ui"));
        assert_eq!(request.package_name.as_deref(), Some("Author UI"));
        assert_eq!(request.created_by.as_deref(), Some("newbeebox-import"));
        assert_eq!(request.description.as_deref(), Some("normalized import"));

        let apply_defaults = request
            .apply_defaults
            .expect("expected explicit apply defaults");
        assert!(!apply_defaults.create_backup);
        assert_eq!(apply_defaults.addons, ResourceApplyPolicyValue::Mirror);
        assert_eq!(apply_defaults.wtf_common, ResourceApplyPolicyValue::Share);
        assert_eq!(
            apply_defaults.wtf_characters,
            ResourceApplyPolicyValue::Sync
        );
        assert_eq!(apply_defaults.fonts, ResourceApplyPolicyValue::Preserve);
        assert_eq!(
            apply_defaults.interface_assets,
            ResourceApplyPolicyValue::ReplaceSelected
        );
    }

    #[test]
    fn build_external_package_bundle_request_skips_apply_defaults_without_overrides() {
        let request = build_external_package_bundle_request(ExternalPackageBundleOptions {
            source: PathBuf::from("C:\\temp\\author-ui.zip"),
            source_flavor: FlavorArg::Retail,
            source_platform: None,
            supported_targets: Vec::new(),
            package_id: None,
            package_name: None,
            created_by: None,
            description: None,
            no_backup: false,
            addons_policy: None,
            wtf_common_policy: None,
            wtf_characters_policy: None,
            fonts_policy: None,
            interface_assets_policy: None,
        });

        assert!(request.apply_defaults.is_none());
    }

    #[test]
    fn build_external_package_bundle_request_partial_overrides_inherit_author_package_defaults() {
        let request = build_external_package_bundle_request(ExternalPackageBundleOptions {
            source: PathBuf::from("C:\\temp\\author-ui.zip"),
            source_flavor: FlavorArg::Retail,
            source_platform: None,
            supported_targets: Vec::new(),
            package_id: None,
            package_name: None,
            created_by: None,
            description: None,
            no_backup: true,
            addons_policy: None,
            wtf_common_policy: None,
            wtf_characters_policy: None,
            fonts_policy: Some(ApplyPolicyArg::Preserve),
            interface_assets_policy: None,
        });

        let apply_defaults = request
            .apply_defaults
            .expect("expected explicit apply defaults");

        assert!(!apply_defaults.create_backup);
        assert_eq!(apply_defaults.addons, ResourceApplyPolicyValue::Mirror);
        assert_eq!(apply_defaults.wtf_common, ResourceApplyPolicyValue::Share);
        assert_eq!(
            apply_defaults.wtf_characters,
            ResourceApplyPolicyValue::ReplaceSelected
        );
        assert_eq!(apply_defaults.fonts, ResourceApplyPolicyValue::Preserve);
        assert_eq!(
            apply_defaults.interface_assets,
            ResourceApplyPolicyValue::Mirror
        );
    }

    #[test]
    fn format_external_package_warnings_renders_groups_and_details() {
        let warnings = vec![
            ExternalPackageWarningResult {
                category: ExternalPackageWarningCategoryValue::Addon,
                code: ExternalPackageWarningCodeValue::AddonRootNotDetected,
                source_path: "AuthorUI/Interface/AddOns/BrokenAddon/README.txt".to_string(),
                message: "ignored addon entry".to_string(),
            },
            ExternalPackageWarningResult {
                category: ExternalPackageWarningCategoryValue::Wtf,
                code: ExternalPackageWarningCodeValue::UnsupportedWtfRootSavedVariables,
                source_path: "AuthorUI/WTF/Account/SavedVariables/Broken.lua".to_string(),
                message: "unsupported wtf entry".to_string(),
            },
        ];
        let summary = ExternalPackageSummaryResult {
            warning_count: 2,
            addon_warning_count: 1,
            wtf_warning_count: 1,
            warning_groups: vec![
                crate::core::app::ExternalPackageWarningGroupResult {
                    category: ExternalPackageWarningCategoryValue::Addon,
                    code: ExternalPackageWarningCodeValue::AddonRootNotDetected,
                    count: 1,
                },
                crate::core::app::ExternalPackageWarningGroupResult {
                    category: ExternalPackageWarningCategoryValue::Wtf,
                    code: ExternalPackageWarningCodeValue::UnsupportedWtfRootSavedVariables,
                    count: 1,
                },
            ],
            total_files: 0,
            normalized_files: 0,
            ignored_files: 0,
            addons: 0,
            wtf_common: 0,
            wtf_characters: 0,
            fonts: 0,
            interface_assets: 0,
        };

        let rendered = format_external_package_warnings(&warnings, &summary);

        assert!(rendered.contains("2 (addon: 1, wtf: 1; groups: ["));
        assert!(rendered.contains("addon/addon_root_not_detected=1"));
        assert!(rendered.contains("wtf/unsupported_wtf_root_savedvariables=1"));
        assert!(rendered.contains(
            "addon/addon_root_not_detected: AuthorUI/Interface/AddOns/BrokenAddon/README.txt"
        ));
        assert!(rendered.contains(
            "wtf/unsupported_wtf_root_savedvariables: AuthorUI/WTF/Account/SavedVariables/Broken.lua"
        ));
    }

    #[test]
    fn format_external_package_warnings_returns_none_for_empty_warnings() {
        let warnings: [ExternalPackageWarningResult; 0] = [];
        let rendered = format_external_package_warnings(
            &warnings,
            &ExternalPackageSummaryResult {
                total_files: 0,
                normalized_files: 0,
                ignored_files: 0,
                addons: 0,
                wtf_common: 0,
                wtf_characters: 0,
                fonts: 0,
                interface_assets: 0,
                warning_count: 0,
                addon_warning_count: 0,
                wtf_warning_count: 0,
                warning_groups: Vec::new(),
            },
        );

        assert_eq!(rendered, "none");
    }
}
