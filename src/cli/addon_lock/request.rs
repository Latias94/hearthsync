use std::path::PathBuf;

use crate::core::app::{
    ApplyAddonLockAppRequest, DiffAddonLockRequest, InspectAddonLockRequest,
    PlanAddonLockSyncRequest, ResolvedInstallationValue, VerifyAddonLockRequest,
    WriteAddonLockRequest,
};

pub(super) fn build_inspect_addon_lock_request(
    installation: ResolvedInstallationValue,
) -> InspectAddonLockRequest {
    InspectAddonLockRequest { installation }
}

pub(super) fn build_write_addon_lock_request(
    installation: ResolvedInstallationValue,
) -> WriteAddonLockRequest {
    WriteAddonLockRequest { installation }
}

pub(super) fn build_diff_addon_lock_request(
    left_lock_path: PathBuf,
    right_lock_path: PathBuf,
) -> DiffAddonLockRequest {
    DiffAddonLockRequest {
        left_lock_path,
        right_lock_path,
    }
}

pub(super) fn build_verify_addon_lock_request(
    installation: ResolvedInstallationValue,
    lock_path: Option<PathBuf>,
) -> VerifyAddonLockRequest {
    VerifyAddonLockRequest {
        installation,
        lock_path,
    }
}

pub(super) fn build_plan_addon_lock_request(
    installation: ResolvedInstallationValue,
    lock_path: Option<PathBuf>,
) -> PlanAddonLockSyncRequest {
    PlanAddonLockSyncRequest {
        installation,
        lock_path,
    }
}

pub(super) fn build_apply_addon_lock_request(
    installation: ResolvedInstallationValue,
    lock_path: Option<PathBuf>,
    backup_output_path: Option<PathBuf>,
    replace_existing: bool,
) -> ApplyAddonLockAppRequest {
    ApplyAddonLockAppRequest {
        installation,
        lock_path,
        backup_output_path,
        replace_existing,
        source_overrides: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        build_apply_addon_lock_request, build_diff_addon_lock_request,
        build_inspect_addon_lock_request, build_plan_addon_lock_request,
        build_verify_addon_lock_request, build_write_addon_lock_request,
    };
    use crate::cli::test_support::sample_installation;

    #[test]
    fn build_inspect_and_write_addon_lock_requests_preserve_installation() {
        let inspect = build_inspect_addon_lock_request(sample_installation());
        let write = build_write_addon_lock_request(sample_installation());

        assert_eq!(
            inspect.installation.flavor_root,
            PathBuf::from("C:\\Games\\World of Warcraft\\_retail_")
        );
        assert_eq!(
            write.installation.addon_dir,
            PathBuf::from("C:\\Games\\World of Warcraft\\_retail_\\Interface\\AddOns")
        );
    }

    #[test]
    fn build_apply_addon_lock_request_sets_empty_source_overrides() {
        let request = build_apply_addon_lock_request(
            sample_installation(),
            Some(PathBuf::from("addons.lock")),
            Some(PathBuf::from("backups")),
            true,
        );

        assert_eq!(request.lock_path, Some(PathBuf::from("addons.lock")));
        assert_eq!(request.backup_output_path, Some(PathBuf::from("backups")));
        assert!(request.replace_existing);
        assert!(request.source_overrides.is_empty());
    }

    #[test]
    fn build_diff_verify_and_plan_requests_preserve_paths() {
        let diff =
            build_diff_addon_lock_request(PathBuf::from("left.lock"), PathBuf::from("right.lock"));
        let verify = build_verify_addon_lock_request(
            sample_installation(),
            Some(PathBuf::from("addons.lock")),
        );
        let plan =
            build_plan_addon_lock_request(sample_installation(), Some(PathBuf::from("plan.lock")));

        assert_eq!(diff.left_lock_path, PathBuf::from("left.lock"));
        assert_eq!(diff.right_lock_path, PathBuf::from("right.lock"));
        assert_eq!(verify.lock_path, Some(PathBuf::from("addons.lock")));
        assert_eq!(plan.lock_path, Some(PathBuf::from("plan.lock")));
    }
}
