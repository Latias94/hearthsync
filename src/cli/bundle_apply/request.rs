use std::path::PathBuf;

use crate::core::app::{
    ApplyBundleAppRequest, BundleApplyMappingsValue, PlanBundleApplyRequest,
    ResolvedInstallationValue,
};

pub(super) fn build_plan_bundle_apply_request(
    bundle_path: PathBuf,
    installation: ResolvedInstallationValue,
    apply_mappings: BundleApplyMappingsValue,
) -> PlanBundleApplyRequest {
    PlanBundleApplyRequest {
        bundle_path,
        installation,
        apply_mappings,
    }
}

pub(super) fn build_apply_bundle_request(
    bundle_path: PathBuf,
    installation: ResolvedInstallationValue,
    dry_run: bool,
    backup_output_path: Option<PathBuf>,
    apply_mappings: BundleApplyMappingsValue,
) -> ApplyBundleAppRequest {
    ApplyBundleAppRequest {
        bundle_path,
        installation,
        dry_run,
        backup_output_path,
        apply_mappings,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{build_apply_bundle_request, build_plan_bundle_apply_request};
    use crate::cli::test_support::sample_installation;
    use crate::core::app::BundleApplyMappingsValue;

    #[test]
    fn build_plan_bundle_apply_request_preserves_fields() {
        let request = build_plan_bundle_apply_request(
            PathBuf::from("bundle.zip"),
            sample_installation(),
            BundleApplyMappingsValue::default(),
        );

        assert_eq!(request.bundle_path, PathBuf::from("bundle.zip"));
        assert_eq!(
            request.installation.flavor_root,
            PathBuf::from("C:\\Games\\World of Warcraft\\_retail_")
        );
    }

    #[test]
    fn build_apply_bundle_request_preserves_execution_options() {
        let request = build_apply_bundle_request(
            PathBuf::from("bundle.zip"),
            sample_installation(),
            true,
            Some(PathBuf::from("backups")),
            BundleApplyMappingsValue::default(),
        );

        assert_eq!(request.bundle_path, PathBuf::from("bundle.zip"));
        assert!(request.dry_run);
        assert_eq!(request.backup_output_path, Some(PathBuf::from("backups")));
    }
}
