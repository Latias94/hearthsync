use std::path::PathBuf;

use crate::core::app::{
    AttachAddonIndexAppRequest, InspectAddonIndexRequest, InstallAddonIndexAppRequest,
    RelinkAddonIndexAppRequest, ResolvedInstallationValue, ScaffoldAddonIndexRequest,
    SuggestAddonIndexRequest, UpdateAddonIndexAppRequest,
};

pub(super) fn build_inspect_addon_index_request(index_path: PathBuf) -> InspectAddonIndexRequest {
    InspectAddonIndexRequest { index_path }
}

pub(super) fn build_install_addon_index_request(
    installation: ResolvedInstallationValue,
    index_path: PathBuf,
    name: String,
    dry_run: bool,
    backup_output_path: Option<PathBuf>,
    replace_existing: bool,
) -> InstallAddonIndexAppRequest {
    InstallAddonIndexAppRequest {
        installation,
        index_path,
        name,
        dry_run,
        backup_output_path,
        replace_existing,
    }
}

pub(super) fn build_suggest_addon_index_request(
    installation: ResolvedInstallationValue,
    index_path: PathBuf,
    name: Option<String>,
) -> SuggestAddonIndexRequest {
    SuggestAddonIndexRequest {
        installation,
        index_path,
        name,
    }
}

pub(super) fn build_attach_addon_index_request(
    installation: ResolvedInstallationValue,
    index_path: PathBuf,
    name: Option<String>,
    dry_run: bool,
    apply_ready_only: bool,
) -> AttachAddonIndexAppRequest {
    AttachAddonIndexAppRequest {
        installation,
        index_path,
        name,
        dry_run,
        apply_ready_only,
    }
}

pub(super) fn build_scaffold_addon_index_request(
    installation: ResolvedInstallationValue,
    index_path: PathBuf,
    index_name: String,
    description: Option<String>,
    name: Option<String>,
    overwrite: bool,
) -> ScaffoldAddonIndexRequest {
    ScaffoldAddonIndexRequest {
        installation,
        index_path,
        index_name,
        description,
        name,
        overwrite,
    }
}

pub(super) fn build_update_addon_index_request(
    installation: ResolvedInstallationValue,
    index_path: PathBuf,
    name: Option<String>,
    dry_run: bool,
    backup_output_path: Option<PathBuf>,
) -> UpdateAddonIndexAppRequest {
    UpdateAddonIndexAppRequest {
        installation,
        index_path,
        name,
        dry_run,
        backup_output_path,
    }
}

pub(super) fn build_relink_addon_index_request(
    installation: ResolvedInstallationValue,
    index_path: PathBuf,
    name: String,
    target: Option<String>,
    dry_run: bool,
) -> RelinkAddonIndexAppRequest {
    RelinkAddonIndexAppRequest {
        installation,
        index_path,
        name,
        target,
        dry_run,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        build_attach_addon_index_request, build_inspect_addon_index_request,
        build_install_addon_index_request, build_relink_addon_index_request,
        build_scaffold_addon_index_request, build_suggest_addon_index_request,
        build_update_addon_index_request,
    };
    use crate::cli::test_support::sample_installation;

    #[test]
    fn build_inspect_addon_index_request_preserves_index_path() {
        let request = build_inspect_addon_index_request(PathBuf::from("addons.index.toml"));

        assert_eq!(request.index_path, PathBuf::from("addons.index.toml"));
    }

    #[test]
    fn build_install_and_update_addon_index_requests_preserve_options() {
        let install = build_install_addon_index_request(
            sample_installation(),
            PathBuf::from("addons.index.toml"),
            "WeakAuras".to_string(),
            true,
            Some(PathBuf::from("backups")),
            true,
        );
        let update = build_update_addon_index_request(
            sample_installation(),
            PathBuf::from("addons.index.toml"),
            Some("WeakAuras".to_string()),
            false,
            None,
        );

        assert_eq!(install.index_path, PathBuf::from("addons.index.toml"));
        assert_eq!(install.name, "WeakAuras");
        assert!(install.dry_run);
        assert_eq!(install.backup_output_path, Some(PathBuf::from("backups")));
        assert!(install.replace_existing);

        assert_eq!(update.index_path, PathBuf::from("addons.index.toml"));
        assert_eq!(update.name.as_deref(), Some("WeakAuras"));
        assert!(!update.dry_run);
        assert!(update.backup_output_path.is_none());
    }

    #[test]
    fn build_suggest_addon_index_request_preserves_options() {
        let request = build_suggest_addon_index_request(
            sample_installation(),
            PathBuf::from("addons.index.toml"),
            Some("WeakAuras".to_string()),
        );

        assert_eq!(request.index_path, PathBuf::from("addons.index.toml"));
        assert_eq!(request.name.as_deref(), Some("WeakAuras"));
    }

    #[test]
    fn build_attach_addon_index_request_preserves_options() {
        let request = build_attach_addon_index_request(
            sample_installation(),
            PathBuf::from("addons.index.toml"),
            Some("WeakAuras".to_string()),
            true,
            true,
        );

        assert_eq!(request.index_path, PathBuf::from("addons.index.toml"));
        assert_eq!(request.name.as_deref(), Some("WeakAuras"));
        assert!(request.dry_run);
        assert!(request.apply_ready_only);
    }

    #[test]
    fn build_scaffold_addon_index_request_preserves_options() {
        let request = build_scaffold_addon_index_request(
            sample_installation(),
            PathBuf::from("addons.index.toml"),
            "Guild UI".to_string(),
            Some("Initial scaffold".to_string()),
            Some("WeakAuras".to_string()),
            true,
        );

        assert_eq!(request.index_path, PathBuf::from("addons.index.toml"));
        assert_eq!(request.index_name, "Guild UI");
        assert_eq!(request.description.as_deref(), Some("Initial scaffold"));
        assert_eq!(request.name.as_deref(), Some("WeakAuras"));
        assert!(request.overwrite);
    }

    #[test]
    fn build_relink_addon_index_request_preserves_options() {
        let request = build_relink_addon_index_request(
            sample_installation(),
            PathBuf::from("addons.index.toml"),
            "curated-plater".to_string(),
            Some("Plater".to_string()),
            true,
        );

        assert_eq!(request.index_path, PathBuf::from("addons.index.toml"));
        assert_eq!(request.name, "curated-plater");
        assert_eq!(request.target.as_deref(), Some("Plater"));
        assert!(request.dry_run);
    }
}
