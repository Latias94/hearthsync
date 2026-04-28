use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Seek, Write};
use std::path::Path;

use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::core::archive_path::{platform_path_collision_key, safe_zip_segments};
use crate::core::error::{AppError, AppResult};
use crate::core::install::HostPlatform;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PortableArchivePathIssueKind {
    ExactCollision,
    CaseInsensitiveCollision,
    ExactPrefixConflict,
    CaseInsensitivePrefixConflict,
}

#[derive(Debug, Clone)]
pub(crate) struct PortableArchivePathIssue {
    pub(crate) previous: String,
    pub(crate) current: String,
    pub(crate) kind: PortableArchivePathIssueKind,
}

#[derive(Debug, Clone)]
struct RegisteredArchivePath {
    archive_path: String,
    is_directory: bool,
}

#[derive(Debug, Default)]
pub(crate) struct PortableArchivePathSet {
    seen_paths: BTreeMap<String, RegisteredArchivePath>,
    descendants_by_ancestor: BTreeMap<String, RegisteredArchivePath>,
}

impl PortableArchivePathSet {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn register(
        &mut self,
        archive_path: &str,
        is_directory: bool,
    ) -> Result<(), PortableArchivePathIssue> {
        let path = Path::new(archive_path);
        let current = RegisteredArchivePath {
            archive_path: archive_path.to_string(),
            is_directory,
        };

        for ancestor in proper_ancestors(path) {
            let ancestor_key = platform_path_collision_key(ancestor, HostPlatform::Windows);
            let Some(previous) = self.seen_paths.get(&ancestor_key) else {
                continue;
            };
            if previous.is_directory {
                continue;
            }

            let kind = if path.starts_with(Path::new(&previous.archive_path)) {
                PortableArchivePathIssueKind::ExactPrefixConflict
            } else {
                PortableArchivePathIssueKind::CaseInsensitivePrefixConflict
            };
            return Err(PortableArchivePathIssue {
                previous: previous.archive_path.clone(),
                current: current.archive_path,
                kind,
            });
        }

        let key = platform_path_collision_key(path, HostPlatform::Windows);
        if !is_directory && let Some(descendant) = self.descendants_by_ancestor.get(&key) {
            let kind = if Path::new(&descendant.archive_path).starts_with(path) {
                PortableArchivePathIssueKind::ExactPrefixConflict
            } else {
                PortableArchivePathIssueKind::CaseInsensitivePrefixConflict
            };
            return Err(PortableArchivePathIssue {
                previous: current.archive_path,
                current: descendant.archive_path.clone(),
                kind,
            });
        }

        if let Some(previous) = self.seen_paths.get(&key) {
            let kind = if previous.archive_path == archive_path {
                PortableArchivePathIssueKind::ExactCollision
            } else {
                PortableArchivePathIssueKind::CaseInsensitiveCollision
            };
            return Err(PortableArchivePathIssue {
                previous: previous.archive_path.clone(),
                current: current.archive_path,
                kind,
            });
        }

        self.seen_paths.insert(key, current.clone());
        for ancestor in proper_ancestors(path) {
            let ancestor_key = platform_path_collision_key(ancestor, HostPlatform::Windows);
            self.descendants_by_ancestor
                .entry(ancestor_key)
                .or_insert_with(|| current.clone());
        }

        Ok(())
    }
}

pub(crate) fn portable_archive_path_issue_error(
    operation: &str,
    issue: PortableArchivePathIssue,
) -> AppError {
    match issue.kind {
        PortableArchivePathIssueKind::ExactCollision => AppError::Validation(format!(
            "{operation} would emit multiple archive entries onto the same path: `{}` and `{}`",
            issue.previous, issue.current
        )),
        PortableArchivePathIssueKind::CaseInsensitiveCollision => AppError::Validation(format!(
            "{operation} would emit case-insensitive archive path collisions: `{}` and `{}` would map to the same path on Windows/default macOS targets",
            issue.previous, issue.current
        )),
        PortableArchivePathIssueKind::ExactPrefixConflict => AppError::Validation(format!(
            "{operation} would emit conflicting file and directory archive paths: `{}` and `{}`",
            issue.previous, issue.current
        )),
        PortableArchivePathIssueKind::CaseInsensitivePrefixConflict => {
            AppError::Validation(format!(
                "{operation} would emit case-insensitive file and directory archive path conflicts: `{}` and `{}` would create file/directory collisions on Windows/default macOS targets",
                issue.previous, issue.current
            ))
        }
    }
}

pub(crate) fn start_file_to_zip<W>(
    zip: &mut ZipWriter<W>,
    archive_path: &str,
    options: SimpleFileOptions,
) -> AppResult<()>
where
    W: Write + Seek,
{
    validate_zip_archive_path(archive_path)?;
    zip.start_file(archive_path, options)?;
    Ok(())
}

pub(crate) fn add_directory_to_zip<W>(
    zip: &mut ZipWriter<W>,
    archive_path: &str,
    options: SimpleFileOptions,
) -> AppResult<()>
where
    W: Write + Seek,
{
    validate_zip_archive_path(archive_path)?;
    zip.add_directory(archive_path, options)?;
    Ok(())
}

pub(crate) fn stream_file_to_zip<W>(
    zip: &mut ZipWriter<W>,
    source_path: &Path,
    archive_path: &str,
    options: SimpleFileOptions,
) -> AppResult<()>
where
    W: Write + Seek,
{
    let mut file = File::open(source_path)?;
    start_file_to_zip(zip, archive_path, options)?;
    std::io::copy(&mut file, zip)?;
    Ok(())
}

pub(crate) fn copy_reader_to_path<R>(reader: &mut R, destination: &Path) -> AppResult<()>
where
    R: Read,
{
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut output = File::create(destination)?;
    std::io::copy(reader, &mut output)?;
    Ok(())
}

pub(crate) fn reject_unsupported_symlink_metadata(
    entry_kind: &str,
    entry_path: &str,
    is_symlink: bool,
) -> AppResult<()> {
    if is_symlink {
        return Err(AppError::Validation(format!(
            "{entry_kind} uses unsupported symlink metadata: {entry_path}"
        )));
    }

    Ok(())
}

pub(crate) fn validate_zip_archive_entry(
    entry_kind: &str,
    entry_path: &str,
    is_symlink: bool,
    is_dir: bool,
) -> AppResult<()> {
    validated_zip_file_entry_segments(entry_kind, entry_path, is_symlink, is_dir)?;
    Ok(())
}

pub(crate) fn validated_zip_file_entry_segments(
    entry_kind: &str,
    entry_path: &str,
    is_symlink: bool,
    is_dir: bool,
) -> AppResult<Option<Vec<String>>> {
    reject_unsupported_symlink_metadata(entry_kind, entry_path, is_symlink)?;

    if is_dir {
        return Ok(None);
    }

    Ok(Some(
        safe_zip_segments(entry_path)?
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
    ))
}

fn validate_zip_archive_path(archive_path: &str) -> AppResult<()> {
    safe_zip_segments(archive_path)?;
    Ok(())
}

fn proper_ancestors(path: &Path) -> impl Iterator<Item = &Path> {
    path.ancestors()
        .skip(1)
        .take_while(|ancestor| !ancestor.as_os_str().is_empty())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::{
        PortableArchivePathIssue, PortableArchivePathIssueKind, PortableArchivePathSet,
        add_directory_to_zip, portable_archive_path_issue_error,
        reject_unsupported_symlink_metadata, start_file_to_zip, validate_zip_archive_entry,
        validated_zip_file_entry_segments,
    };

    #[test]
    fn start_file_to_zip_rejects_non_portable_archive_paths() {
        let cursor = Cursor::new(Vec::<u8>::new());
        let mut zip = ZipWriter::new(cursor);

        let error = start_file_to_zip(
            &mut zip,
            "addons/CON/Config.lua",
            SimpleFileOptions::default(),
        )
        .expect_err("non-portable archive path should fail");

        assert!(error.to_string().contains("unsafe archive path"));
    }

    #[test]
    fn add_directory_to_zip_rejects_non_portable_archive_paths() {
        let cursor = Cursor::new(Vec::<u8>::new());
        let mut zip = ZipWriter::new(cursor);

        let error = add_directory_to_zip(&mut zip, "addons/CON", SimpleFileOptions::default())
            .expect_err("non-portable archive directory should fail");

        assert!(error.to_string().contains("unsafe archive path"));
    }

    #[test]
    fn portable_archive_path_set_rejects_case_insensitive_collisions() {
        let mut paths = PortableArchivePathSet::new();
        paths
            .register("addons/WeakAuras/Config.lua", false)
            .expect("first path should register");

        let issue = paths
            .register("addons/weakauras/config.lua", false)
            .expect_err("case-insensitive collision should fail");

        assert_eq!(
            issue.kind,
            PortableArchivePathIssueKind::CaseInsensitiveCollision
        );
        assert_eq!(issue.previous, "addons/WeakAuras/Config.lua");
        assert_eq!(issue.current, "addons/weakauras/config.lua");
    }

    #[test]
    fn portable_archive_path_set_rejects_case_insensitive_prefix_conflicts() {
        let mut paths = PortableArchivePathSet::new();
        paths
            .register("addons/WeakAuras", false)
            .expect("first path should register");

        let issue = paths
            .register("addons/weakauras/Config.lua", false)
            .expect_err("case-insensitive prefix conflict should fail");

        assert_eq!(
            issue.kind,
            PortableArchivePathIssueKind::CaseInsensitivePrefixConflict
        );
        assert_eq!(issue.previous, "addons/WeakAuras");
        assert_eq!(issue.current, "addons/weakauras/Config.lua");
    }

    #[test]
    fn portable_archive_path_set_allows_directories_as_ancestors() {
        let mut paths = PortableArchivePathSet::new();
        paths
            .register("addons/.hearthsync", true)
            .expect("directory path should register");
        paths
            .register("addons/.hearthsync/addons.toml", false)
            .expect("file below directory should be allowed");
    }

    #[test]
    fn portable_archive_path_issue_error_formats_operation_context() {
        let error = portable_archive_path_issue_error(
            "bundle creation",
            PortableArchivePathIssue {
                previous: "metadata/addons/indexes/addon-index.toml".to_string(),
                current: "metadata/addons/indexes/ADDON-INDEX.toml".to_string(),
                kind: PortableArchivePathIssueKind::CaseInsensitiveCollision,
            },
        );

        let message = error.to_string();
        assert!(message.contains("bundle creation"));
        assert!(message.contains("case-insensitive archive path collisions"));
        assert!(message.contains("metadata/addons/indexes/addon-index.toml"));
        assert!(message.contains("metadata/addons/indexes/ADDON-INDEX.toml"));
    }

    #[test]
    fn reject_unsupported_symlink_metadata_reports_context_and_path() {
        let error =
            reject_unsupported_symlink_metadata("bundle archive entry", "addons/WeakAuras", true)
                .expect_err("symlink metadata should fail");

        let message = error.to_string();
        assert!(message.contains("bundle archive entry"));
        assert!(message.contains("unsupported symlink metadata"));
        assert!(message.contains("addons/WeakAuras"));
    }

    #[test]
    fn reject_unsupported_symlink_metadata_allows_regular_entries() {
        reject_unsupported_symlink_metadata("backup archive entry", "addons/WeakAuras", false)
            .expect("regular entry should pass");
    }

    #[test]
    fn validate_zip_archive_entry_reports_symlink_context() {
        let error = validate_zip_archive_entry(
            "bundle archive entry",
            "addons/WeakAuras/WeakAuras.lua",
            true,
            false,
        )
        .expect_err("symlink metadata should fail");

        let message = error.to_string();
        assert!(message.contains("bundle archive entry"));
        assert!(message.contains("unsupported symlink metadata"));
        assert!(message.contains("addons/WeakAuras/WeakAuras.lua"));
    }

    #[test]
    fn validate_zip_archive_entry_rejects_non_portable_file_paths() {
        let error = validate_zip_archive_entry(
            "external package zip entry",
            "addons/Weak:Auras/WeakAuras.toc",
            false,
            false,
        )
        .expect_err("non-portable archive path should fail");

        assert!(error.to_string().contains("unsafe archive path"));
    }

    #[test]
    fn validate_zip_archive_entry_allows_directory_entries() {
        validate_zip_archive_entry("backup archive entry", "addons/WeakAuras/", false, true)
            .expect("directory entry should skip file-segment validation");
    }

    #[test]
    fn validated_zip_file_entry_segments_returns_owned_file_segments() {
        assert_eq!(
            validated_zip_file_entry_segments(
                "addon archive entry",
                "WeakAuras/WeakAuras.toc",
                false,
                false,
            )
            .expect("valid file entry")
            .expect("file entry segments"),
            vec!["WeakAuras".to_string(), "WeakAuras.toc".to_string()]
        );
    }

    #[test]
    fn validated_zip_file_entry_segments_skips_directory_entries() {
        assert_eq!(
            validated_zip_file_entry_segments(
                "bundle archive entry",
                "addons/WeakAuras/",
                false,
                true,
            )
            .expect("directory entry should pass"),
            None
        );
    }
}
