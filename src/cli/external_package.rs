use super::app_support::{resolve_cli_installation, stable_services};
use super::bundle_apply::resolve_apply_mappings;
use super::output::{
    render, render_external_package_analysis, render_external_package_apply,
    render_external_package_plan,
};
use super::{ExternalPackageBundleOptions, ExternalPackageCommands};
use crate::core::app::{
    AnalyzeExternalPackageAppRequest, ApplyExternalPackageAppRequest, BundleApplyDefaultsValue,
    CreateExternalPackageBundleAppRequest, PlanExternalPackageApplyAppRequest,
    ResourceApplyPolicyValue,
};
use crate::core::error::AppResult;

pub(super) fn handle_external_package_command(
    json: bool,
    command: ExternalPackageCommands,
) -> AppResult<()> {
    let app = stable_services();

    match command {
        ExternalPackageCommands::Inspect { source } => {
            let analysis = app.analyze_external_package(AnalyzeExternalPackageAppRequest {
                source_path: source,
            })?;
            render(json, &analysis, render_external_package_analysis)?;
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
            let installation = resolve_cli_installation(&app, install, flavor)?;
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
            render(json, &plan, render_external_package_plan)?;
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
            let installation = resolve_cli_installation(&app, install, flavor)?;
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
            render(json, &result, render_external_package_apply)?;
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
        defaults.addons = ResourceApplyPolicyValue::from_domain(
            crate::core::manifest::ResourceApplyPolicy::from(policy),
        );
    }
    if let Some(policy) = options.wtf_common_policy {
        defaults.wtf_common = ResourceApplyPolicyValue::from_domain(
            crate::core::manifest::ResourceApplyPolicy::from(policy),
        );
    }
    if let Some(policy) = options.wtf_characters_policy {
        defaults.wtf_characters = ResourceApplyPolicyValue::from_domain(
            crate::core::manifest::ResourceApplyPolicy::from(policy),
        );
    }
    if let Some(policy) = options.fonts_policy {
        defaults.fonts = ResourceApplyPolicyValue::from_domain(
            crate::core::manifest::ResourceApplyPolicy::from(policy),
        );
    }
    if let Some(policy) = options.interface_assets_policy {
        defaults.interface_assets = ResourceApplyPolicyValue::from_domain(
            crate::core::manifest::ResourceApplyPolicy::from(policy),
        );
    }

    Some(defaults)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::cli::output::format_external_package_warnings;
    use crate::cli::{ApplyPolicyArg, FlavorArg, PlatformArg};
    use crate::core::app::{
        ExternalPackageSummaryResult, ExternalPackageWarningCategoryValue,
        ExternalPackageWarningCodeValue, ExternalPackageWarningResult, HostPlatformValue,
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
