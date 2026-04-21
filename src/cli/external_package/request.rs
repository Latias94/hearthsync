use std::path::PathBuf;

use crate::cli::ExternalPackageBundleOptions;
use crate::core::app::{
    AnalyzeExternalPackageAppRequest, ApplyExternalPackageAppRequest, BundleApplyDefaultsValue,
    BundleApplyMappingsValue, CreateExternalPackageBundleAppRequest,
    PlanExternalPackageApplyAppRequest, ResolvedInstallationValue, ResourceApplyPolicyValue,
};

pub(in crate::cli) fn build_analyze_external_package_request(
    source_path: PathBuf,
) -> AnalyzeExternalPackageAppRequest {
    AnalyzeExternalPackageAppRequest { source_path }
}

pub(in crate::cli) fn build_external_package_bundle_request(
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

pub(in crate::cli) fn build_plan_external_package_request(
    external_package: CreateExternalPackageBundleAppRequest,
    installation: ResolvedInstallationValue,
    apply_mappings: BundleApplyMappingsValue,
) -> PlanExternalPackageApplyAppRequest {
    PlanExternalPackageApplyAppRequest {
        external_package,
        installation,
        apply_mappings,
    }
}

pub(in crate::cli) fn build_apply_external_package_request(
    external_package: CreateExternalPackageBundleAppRequest,
    installation: ResolvedInstallationValue,
    dry_run: bool,
    backup_output_path: Option<PathBuf>,
    apply_mappings: BundleApplyMappingsValue,
) -> ApplyExternalPackageAppRequest {
    ApplyExternalPackageAppRequest {
        external_package,
        installation,
        dry_run,
        backup_output_path,
        apply_mappings,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        ExternalPackageBundleOptions, build_analyze_external_package_request,
        build_apply_external_package_request, build_external_package_bundle_request,
        build_plan_external_package_request,
    };
    use crate::cli::test_support::sample_installation;
    use crate::cli::{ApplyPolicyArg, FlavorArg, PlatformArg};
    use crate::core::app::{
        BundleApplyMappingsValue, HostPlatformValue, ResourceApplyPolicyValue, WowFlavorValue,
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
    fn build_analyze_external_package_request_preserves_source_path() {
        let request = build_analyze_external_package_request(PathBuf::from("C:\\temp\\author-ui"));

        assert_eq!(request.source_path, PathBuf::from("C:\\temp\\author-ui"));
    }

    #[test]
    fn build_plan_and_apply_external_package_requests_preserve_execution_fields() {
        let external_package =
            build_external_package_bundle_request(ExternalPackageBundleOptions {
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
        let plan = build_plan_external_package_request(
            external_package.clone(),
            sample_installation(),
            BundleApplyMappingsValue::default(),
        );
        let apply = build_apply_external_package_request(
            external_package,
            sample_installation(),
            true,
            Some(PathBuf::from("backups")),
            BundleApplyMappingsValue::default(),
        );

        assert_eq!(
            plan.external_package.source_path,
            PathBuf::from("C:\\temp\\author-ui.zip")
        );
        assert_eq!(
            plan.installation.flavor_root,
            PathBuf::from("C:\\Games\\World of Warcraft\\_retail_")
        );
        assert!(plan.apply_mappings.selected_accounts.is_empty());

        assert!(apply.dry_run);
        assert_eq!(apply.backup_output_path, Some(PathBuf::from("backups")));
        assert_eq!(
            apply.installation.addon_dir,
            PathBuf::from("C:\\Games\\World of Warcraft\\_retail_\\Interface\\AddOns")
        );
    }
}
