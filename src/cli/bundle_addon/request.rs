use std::path::PathBuf;

use crate::core::app::{
    ApplyBundleAddonLockAppRequest, PlanBundleAddonLockRequest, ResolvedInstallationValue,
};

pub(super) fn build_plan_bundle_addon_lock_request(
    bundle_path: PathBuf,
    installation: ResolvedInstallationValue,
) -> PlanBundleAddonLockRequest {
    PlanBundleAddonLockRequest {
        bundle_path,
        installation,
    }
}

pub(super) fn build_apply_bundle_addon_lock_request(
    bundle_path: PathBuf,
    installation: ResolvedInstallationValue,
    backup_output_path: Option<PathBuf>,
    replace_existing: bool,
) -> ApplyBundleAddonLockAppRequest {
    ApplyBundleAddonLockAppRequest {
        bundle_path,
        installation,
        backup_output_path,
        replace_existing,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::cli::test_support::sample_installation;

    #[test]
    fn build_plan_bundle_addon_lock_request_preserves_bundle_and_installation() {
        let request = build_plan_bundle_addon_lock_request(
            PathBuf::from("ui.bundle.zip"),
            sample_installation(),
        );

        assert_eq!(request.bundle_path, PathBuf::from("ui.bundle.zip"));
        assert_eq!(
            request.installation.flavor_root,
            PathBuf::from("C:\\Games\\World of Warcraft\\_retail_")
        );
    }

    #[test]
    fn build_apply_bundle_addon_lock_request_preserves_execution_flags() {
        let request = build_apply_bundle_addon_lock_request(
            PathBuf::from("ui.bundle.zip"),
            sample_installation(),
            Some(PathBuf::from("backups")),
            true,
        );

        assert_eq!(request.bundle_path, PathBuf::from("ui.bundle.zip"));
        assert_eq!(request.backup_output_path, Some(PathBuf::from("backups")));
        assert!(request.replace_existing);
    }
}
