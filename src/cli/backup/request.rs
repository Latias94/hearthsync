use std::path::PathBuf;

use crate::core::app::{
    BackupGroupValue, CreateBackupAppRequest, ListBackupsRequest, ResolvedInstallationValue,
    RestoreBackupAppRequest,
};

pub(super) fn build_create_backup_request(
    installation: ResolvedInstallationValue,
    output_path: Option<PathBuf>,
) -> CreateBackupAppRequest {
    CreateBackupAppRequest {
        installation,
        output_path,
        groups: vec![
            BackupGroupValue::Addons,
            BackupGroupValue::Wtf,
            BackupGroupValue::Fonts,
            BackupGroupValue::InterfaceAssets,
        ],
        label: None,
    }
}

pub(super) fn build_list_backups_request(backup_dir: Option<PathBuf>) -> ListBackupsRequest {
    ListBackupsRequest { backup_dir }
}

pub(super) fn build_restore_backup_request(
    installation: ResolvedInstallationValue,
    archive_path: Option<PathBuf>,
    backup_id: Option<String>,
    backup_dir: Option<PathBuf>,
) -> RestoreBackupAppRequest {
    RestoreBackupAppRequest {
        installation,
        archive_path,
        backup_id,
        backup_dir,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::cli::test_support::sample_installation;

    #[test]
    fn build_create_backup_request_sets_default_groups() {
        let request =
            build_create_backup_request(sample_installation(), Some(PathBuf::from("backups")));

        assert_eq!(request.output_path, Some(PathBuf::from("backups")));
        assert_eq!(
            request.groups,
            vec![
                BackupGroupValue::Addons,
                BackupGroupValue::Wtf,
                BackupGroupValue::Fonts,
                BackupGroupValue::InterfaceAssets
            ]
        );
        assert!(request.label.is_none());
    }

    #[test]
    fn build_list_and_restore_backup_requests_preserve_selection() {
        let list = build_list_backups_request(Some(PathBuf::from("backups")));
        let restore = build_restore_backup_request(
            sample_installation(),
            Some(PathBuf::from("backup.zip")),
            Some("backup-123".to_string()),
            Some(PathBuf::from("backups")),
        );

        assert_eq!(list.backup_dir, Some(PathBuf::from("backups")));
        assert_eq!(restore.archive_path, Some(PathBuf::from("backup.zip")));
        assert_eq!(restore.backup_id.as_deref(), Some("backup-123"));
        assert_eq!(restore.backup_dir, Some(PathBuf::from("backups")));
    }
}
