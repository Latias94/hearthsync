use std::path::PathBuf;

use crate::core::app::{
    InstallAddonAppRequest, ListAddonsRequest, RemoveAddonAppRequest, ResolvedInstallationValue,
    SearchAddonsRequest, UpdateAddonAppRequest,
};

pub(super) fn build_search_addons_request(
    installation: ResolvedInstallationValue,
    query: String,
    limit: usize,
) -> SearchAddonsRequest {
    SearchAddonsRequest {
        installation,
        query,
        limit,
    }
}

pub(super) fn build_list_addons_request(
    installation: ResolvedInstallationValue,
) -> ListAddonsRequest {
    ListAddonsRequest { installation }
}

pub(super) fn build_install_addon_request(
    installation: ResolvedInstallationValue,
    source: String,
    dry_run: bool,
    backup_output_path: Option<PathBuf>,
    replace_existing: bool,
) -> InstallAddonAppRequest {
    InstallAddonAppRequest {
        installation,
        source,
        dry_run,
        backup_output_path,
        replace_existing,
        metadata: None,
    }
}

pub(super) fn build_update_addons_request(
    installation: ResolvedInstallationValue,
    name: Option<String>,
    dry_run: bool,
    backup_output_path: Option<PathBuf>,
) -> UpdateAddonAppRequest {
    UpdateAddonAppRequest {
        installation,
        name,
        dry_run,
        backup_output_path,
    }
}

pub(super) fn build_remove_addons_request(
    installation: ResolvedInstallationValue,
    name: String,
    dry_run: bool,
    backup_output_path: Option<PathBuf>,
) -> RemoveAddonAppRequest {
    RemoveAddonAppRequest {
        installation,
        name,
        dry_run,
        backup_output_path,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        build_install_addon_request, build_remove_addons_request, build_update_addons_request,
    };
    use crate::cli::test_support::sample_installation;

    #[test]
    fn build_install_addon_request_sets_optional_metadata_to_none() {
        let request = build_install_addon_request(
            sample_installation(),
            "curseforge:weakauras".to_string(),
            true,
            Some(PathBuf::from("backups")),
            true,
        );

        assert_eq!(request.source, "curseforge:weakauras");
        assert!(request.dry_run);
        assert_eq!(request.backup_output_path, Some(PathBuf::from("backups")));
        assert!(request.replace_existing);
        assert!(request.metadata.is_none());
    }

    #[test]
    fn build_update_and_remove_requests_preserve_flags() {
        let update = build_update_addons_request(
            sample_installation(),
            Some("WeakAuras".to_string()),
            true,
            Some(PathBuf::from("backups")),
        );
        let remove = build_remove_addons_request(
            sample_installation(),
            "WeakAuras".to_string(),
            false,
            None,
        );

        assert_eq!(update.name.as_deref(), Some("WeakAuras"));
        assert!(update.dry_run);
        assert_eq!(update.backup_output_path, Some(PathBuf::from("backups")));

        assert_eq!(remove.name, "WeakAuras");
        assert!(!remove.dry_run);
        assert!(remove.backup_output_path.is_none());
    }
}
