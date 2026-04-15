use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use walkdir::WalkDir;
use zip::CompressionMethod;
use zip::ZipArchive;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupGroup {
    Addons,
    Wtf,
    Fonts,
    InterfaceAssets,
}

impl BackupGroup {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Addons => "addons",
            Self::Wtf => "wtf",
            Self::Fonts => "fonts",
            Self::InterfaceAssets => "interface_assets",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BackupRequest {
    pub installation: DetectedFlavorInstallation,
    pub output_path: Option<PathBuf>,
    pub groups: Vec<BackupGroup>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub schema_version: u32,
    pub created_at: String,
    #[serde(default)]
    pub label: Option<String>,
    pub flavor: String,
    pub flavor_root: PathBuf,
    pub groups: Vec<BackupGroup>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreatedBackup {
    pub archive_path: PathBuf,
    pub archived_files: usize,
    pub metadata: BackupMetadata,
}

#[derive(Debug, Clone, Serialize)]
pub struct RestoredBackup {
    pub archive_path: PathBuf,
    pub restored_files: usize,
    pub metadata: BackupMetadata,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupCatalog {
    pub backup_dir: PathBuf,
    pub entries: Vec<BackupCatalogEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupCatalogEntry {
    pub backup_id: String,
    pub archive_path: PathBuf,
    pub archive_size_bytes: u64,
    pub metadata: BackupMetadata,
}

#[derive(Debug, Clone)]
pub struct RestoreBackupRequest {
    pub installation: DetectedFlavorInstallation,
    pub archive_path: Option<PathBuf>,
    pub backup_id: Option<String>,
    pub backup_dir: Option<PathBuf>,
}

pub fn create_backup(request: BackupRequest) -> AppResult<CreatedBackup> {
    if request.groups.is_empty() {
        return Err(AppError::Validation(
            "backup request must include at least one group".to_string(),
        ));
    }

    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AppError::Validation(error.to_string()))?;
    let output_dir = match request.output_path {
        Some(path) => path,
        None => default_backup_dir()?,
    };

    fs::create_dir_all(&output_dir)?;

    let file_name = build_backup_file_name(
        request.installation.flavor.as_str(),
        request.label.as_deref(),
        &timestamp,
    );
    let archive_path = output_dir.join(file_name);
    let file = File::create(&archive_path)?;
    let mut zip = ZipWriter::new(file);
    let mut archived_files = 0usize;

    for group in &request.groups {
        match group {
            BackupGroup::Addons => {
                archived_files += add_directory_group(
                    &mut zip,
                    &request.installation.addon_dir,
                    Path::new("addons"),
                )?;
            }
            BackupGroup::Wtf => {
                archived_files +=
                    add_directory_group(&mut zip, &request.installation.wtf_dir, Path::new("wtf"))?;
            }
            BackupGroup::Fonts => {
                archived_files += add_directory_group(
                    &mut zip,
                    &request.installation.fonts_dir,
                    Path::new("fonts"),
                )?;
            }
            BackupGroup::InterfaceAssets => {
                archived_files +=
                    add_interface_assets_group(&mut zip, &request.installation.interface_dir)?;
            }
        }
    }

    let metadata = BackupMetadata {
        schema_version: 1,
        created_at: timestamp,
        label: request
            .label
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        flavor: request.installation.flavor.as_str().to_string(),
        flavor_root: request.installation.flavor_root.clone(),
        groups: request.groups,
    };

    zip.start_file("backup.toml", zip_file_options())?;
    zip.write_all(toml::to_string_pretty(&metadata)?.as_bytes())?;
    zip.finish()?;

    Ok(CreatedBackup {
        archive_path,
        archived_files,
        metadata,
    })
}

pub fn restore_backup(
    archive_path: &Path,
    installation: &DetectedFlavorInstallation,
) -> AppResult<RestoredBackup> {
    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;
    let metadata = read_backup_metadata(&mut archive)?;

    for group in &metadata.groups {
        clear_group_destination(*group, installation)?;
    }

    let mut restored_files = 0usize;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }

        let entry_name = entry.name().to_string();
        if entry_name == "backup.toml" {
            continue;
        }

        let Some(destination) = map_backup_entry_to_destination(&entry_name, installation)? else {
            continue;
        };

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut output = File::create(destination)?;
        std::io::copy(&mut entry, &mut output)?;
        restored_files += 1;
    }

    Ok(RestoredBackup {
        archive_path: archive_path.to_path_buf(),
        restored_files,
        metadata,
    })
}

pub fn list_backups(backup_dir: Option<&Path>) -> AppResult<BackupCatalog> {
    let backup_dir = resolve_backup_dir(backup_dir)?;
    if !backup_dir.exists() {
        return Ok(BackupCatalog {
            backup_dir,
            entries: Vec::new(),
        });
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(&backup_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_none_or(|ext| !ext.eq_ignore_ascii_case("zip"))
        {
            continue;
        }

        let metadata = read_backup_metadata_from_path(&path)?;
        let backup_id = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| {
                AppError::Validation(format!("invalid backup file name: {}", path.display()))
            })?
            .to_string();
        let archive_size_bytes = fs::metadata(&path)?.len();
        entries.push(BackupCatalogEntry {
            backup_id,
            archive_path: path,
            archive_size_bytes,
            metadata,
        });
    }

    entries.sort_by(|left, right| {
        right
            .metadata
            .created_at
            .cmp(&left.metadata.created_at)
            .then_with(|| right.archive_path.cmp(&left.archive_path))
    });

    Ok(BackupCatalog {
        backup_dir,
        entries,
    })
}

pub fn restore_backup_selection(request: RestoreBackupRequest) -> AppResult<RestoredBackup> {
    let archive_path = resolve_backup_archive(
        request.archive_path.as_deref(),
        request.backup_id.as_deref(),
        request.backup_dir.as_deref(),
    )?;
    restore_backup(&archive_path, &request.installation)
}

fn default_backup_dir() -> AppResult<PathBuf> {
    let project_dirs = ProjectDirs::from("dev", "hearthsync", "hearthsync").ok_or_else(|| {
        AppError::Validation("failed to determine platform-specific backup directory".to_string())
    })?;

    Ok(project_dirs.data_local_dir().join("backups"))
}

fn resolve_backup_dir(backup_dir: Option<&Path>) -> AppResult<PathBuf> {
    match backup_dir {
        Some(path) => Ok(path.to_path_buf()),
        None => default_backup_dir(),
    }
}

fn resolve_backup_archive(
    archive_path: Option<&Path>,
    backup_id: Option<&str>,
    backup_dir: Option<&Path>,
) -> AppResult<PathBuf> {
    match (archive_path, backup_id) {
        (Some(path), None) => Ok(path.to_path_buf()),
        (None, Some(backup_id)) => {
            let catalog = list_backups(backup_dir)?;
            let matched = catalog
                .entries
                .into_iter()
                .find(|entry| {
                    entry.backup_id == backup_id
                        || entry
                            .archive_path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| name == backup_id)
                })
                .ok_or_else(|| {
                    AppError::NotFound(format!(
                        "backup `{backup_id}` not found in {}",
                        catalog.backup_dir.display()
                    ))
                })?;
            Ok(matched.archive_path)
        }
        (Some(_), Some(_)) => Err(AppError::Validation(
            "pass either `archive_path` or `backup_id`, not both".to_string(),
        )),
        (None, None) => Err(AppError::Validation(
            "either `archive_path` or `backup_id` is required".to_string(),
        )),
    }
}

fn build_backup_file_name(flavor: &str, label: Option<&str>, timestamp: &str) -> String {
    let compact_timestamp = timestamp
        .chars()
        .filter(|char| char.is_ascii_alphanumeric())
        .collect::<String>();

    match label {
        Some(value) if !value.trim().is_empty() => {
            format!("backup-{flavor}-{value}-{compact_timestamp}.zip")
        }
        _ => format!("backup-{flavor}-{compact_timestamp}.zip"),
    }
}

fn add_directory_group(
    zip: &mut ZipWriter<File>,
    source_dir: &Path,
    archive_root: &Path,
) -> AppResult<usize> {
    if !source_dir.exists() {
        return Ok(0);
    }

    let mut archived_files = 0usize;

    for entry in WalkDir::new(source_dir).follow_links(false) {
        let entry = entry.map_err(|error| AppError::Validation(error.to_string()))?;
        let path = entry.path();
        let relative = path
            .strip_prefix(source_dir)
            .map_err(|error| AppError::Validation(error.to_string()))?;

        if relative.as_os_str().is_empty() {
            continue;
        }

        let archive_path = archive_root.join(relative);

        if entry.file_type().is_dir() {
            zip.add_directory(to_zip_path(&archive_path), zip_dir_options())?;
            continue;
        }

        write_file_to_zip(zip, path, &archive_path)?;
        archived_files += 1;
    }

    Ok(archived_files)
}

fn add_interface_assets_group(zip: &mut ZipWriter<File>, interface_dir: &Path) -> AppResult<usize> {
    if !interface_dir.exists() {
        return Ok(0);
    }

    let mut archived_files = 0usize;

    for entry in fs::read_dir(interface_dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        if name.to_string_lossy().eq_ignore_ascii_case("AddOns") {
            continue;
        }

        let archive_root = Path::new("interface").join(name);
        if path.is_dir() {
            archived_files += add_directory_group(zip, &path, &archive_root)?;
        } else if path.is_file() {
            write_file_to_zip(zip, &path, &archive_root)?;
            archived_files += 1;
        }
    }

    Ok(archived_files)
}

fn write_file_to_zip(
    zip: &mut ZipWriter<File>,
    source_path: &Path,
    archive_path: &Path,
) -> AppResult<()> {
    let mut file = File::open(source_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    zip.start_file(to_zip_path(archive_path), zip_file_options())?;
    zip.write_all(&buffer)?;
    Ok(())
}

fn to_zip_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn read_backup_metadata(archive: &mut ZipArchive<File>) -> AppResult<BackupMetadata> {
    let mut entry = archive.by_name("backup.toml")?;
    let mut content = String::new();
    entry.read_to_string(&mut content)?;
    Ok(toml::from_str(&content)?)
}

fn read_backup_metadata_from_path(path: &Path) -> AppResult<BackupMetadata> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    read_backup_metadata(&mut archive)
}

fn clear_group_destination(
    group: BackupGroup,
    installation: &DetectedFlavorInstallation,
) -> AppResult<()> {
    match group {
        BackupGroup::Addons => clear_directory(&installation.addon_dir),
        BackupGroup::Wtf => clear_directory(&installation.wtf_dir),
        BackupGroup::Fonts => clear_directory(&installation.fonts_dir),
        BackupGroup::InterfaceAssets => clear_interface_assets(&installation.interface_dir),
    }
}

fn clear_directory(path: &Path) -> AppResult<()> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    fs::create_dir_all(path)?;
    Ok(())
}

fn clear_interface_assets(interface_dir: &Path) -> AppResult<()> {
    if !interface_dir.exists() {
        fs::create_dir_all(interface_dir)?;
        return Ok(());
    }

    for entry in fs::read_dir(interface_dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.eq_ignore_ascii_case("AddOns") {
            continue;
        }

        if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

fn map_backup_entry_to_destination(
    entry_name: &str,
    installation: &DetectedFlavorInstallation,
) -> AppResult<Option<PathBuf>> {
    let segments = safe_zip_segments(entry_name)?;
    if segments.is_empty() {
        return Ok(None);
    }

    match segments.as_slice() {
        ["addons", rest @ ..] if !rest.is_empty() => {
            Ok(Some(join_segments(&installation.addon_dir, rest)))
        }
        ["wtf", rest @ ..] if !rest.is_empty() => {
            Ok(Some(join_segments(&installation.wtf_dir, rest)))
        }
        ["fonts", rest @ ..] if !rest.is_empty() => {
            Ok(Some(join_segments(&installation.fonts_dir, rest)))
        }
        ["interface", rest @ ..] if !rest.is_empty() => {
            Ok(Some(join_segments(&installation.interface_dir, rest)))
        }
        _ => Ok(None),
    }
}

fn safe_zip_segments(entry_name: &str) -> AppResult<Vec<&str>> {
    let mut segments = Vec::new();
    for segment in entry_name.split('/') {
        if segment.is_empty() {
            continue;
        }

        if segment == "." || segment == ".." || segment.contains('\\') {
            return Err(AppError::Validation(format!(
                "unsafe backup path: `{entry_name}`"
            )));
        }

        segments.push(segment);
    }

    Ok(segments)
}

fn join_segments(root: &Path, segments: &[&str]) -> PathBuf {
    let mut path = root.to_path_buf();
    for segment in segments {
        path.push(segment);
    }
    path
}

fn zip_file_options() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)
}

fn zip_dir_options() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::{Path, PathBuf};

    use tempfile::tempdir;
    use zip::ZipArchive;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::{
        BackupGroup, BackupMetadata, BackupRequest, RestoreBackupRequest, create_backup,
        list_backups, restore_backup, restore_backup_selection,
    };
    use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};

    #[test]
    fn create_backup_writes_expected_entries() {
        let temp = tempdir().expect("temp dir");
        let flavor_root = temp.path().join("_retail_");
        let interface_dir = flavor_root.join("Interface");
        let addon_dir = interface_dir.join("AddOns");
        let wtf_dir = flavor_root.join("WTF");
        let fonts_dir = flavor_root.join("Fonts");

        fs::create_dir_all(addon_dir.join("WeakAuras")).expect("addon dir");
        fs::create_dir_all(wtf_dir.join("Account")).expect("wtf dir");
        fs::create_dir_all(&fonts_dir).expect("fonts dir");
        fs::write(
            addon_dir.join("WeakAuras").join("WeakAuras.toc"),
            "## Interface: 110000",
        )
        .expect("toc");
        fs::write(wtf_dir.join("Config.wtf"), "SET locale enUS").expect("config");
        fs::write(fonts_dir.join("FRIZQT__.ttf"), "font").expect("font");

        let backup = create_backup(BackupRequest {
            installation: DetectedFlavorInstallation {
                platform: HostPlatform::Windows,
                product_root: temp.path().to_path_buf(),
                flavor_root: flavor_root.clone(),
                flavor: WowFlavor::Retail,
                interface_dir,
                addon_dir,
                wtf_dir,
                fonts_dir,
            },
            output_path: Some(temp.path().join("out")),
            groups: vec![BackupGroup::Addons, BackupGroup::Wtf, BackupGroup::Fonts],
            label: Some("smoke".to_string()),
        })
        .expect("backup");

        let file = std::fs::File::open(backup.archive_path).expect("archive");
        let mut archive = ZipArchive::new(file).expect("zip");

        assert!(archive.by_name("addons/WeakAuras/WeakAuras.toc").is_ok());
        assert!(archive.by_name("wtf/Config.wtf").is_ok());
        assert!(archive.by_name("fonts/FRIZQT__.ttf").is_ok());
        assert!(archive.by_name("backup.toml").is_ok());
    }

    #[test]
    fn restore_backup_restores_previous_state_and_removes_new_files() {
        let temp = tempdir().expect("temp dir");
        let flavor_root = temp.path().join("_retail_");
        let interface_dir = flavor_root.join("Interface");
        let addon_dir = interface_dir.join("AddOns");
        let wtf_dir = flavor_root.join("WTF");
        let fonts_dir = flavor_root.join("Fonts");
        let installation = DetectedFlavorInstallation {
            platform: HostPlatform::Windows,
            product_root: temp.path().to_path_buf(),
            flavor_root: flavor_root.clone(),
            flavor: WowFlavor::Retail,
            interface_dir,
            addon_dir: addon_dir.clone(),
            wtf_dir: wtf_dir.clone(),
            fonts_dir: fonts_dir.clone(),
        };

        fs::create_dir_all(addon_dir.join("WeakAuras")).expect("addon dir");
        fs::create_dir_all(&wtf_dir).expect("wtf dir");
        fs::create_dir_all(&fonts_dir).expect("fonts dir");
        fs::write(addon_dir.join("WeakAuras").join("WeakAuras.toc"), "before").expect("toc");
        fs::write(wtf_dir.join("Config.wtf"), "before").expect("config");

        let backup = create_backup(BackupRequest {
            installation: installation.clone(),
            output_path: Some(temp.path().join("out")),
            groups: vec![BackupGroup::Addons, BackupGroup::Wtf],
            label: Some("rollback".to_string()),
        })
        .expect("backup");

        fs::write(addon_dir.join("WeakAuras").join("WeakAuras.toc"), "after").expect("toc");
        fs::write(wtf_dir.join("Config.wtf"), "after").expect("config");
        fs::write(wtf_dir.join("New.lua"), "new").expect("new file");

        let restored = restore_backup(&backup.archive_path, &installation).expect("restore");

        assert_eq!(restored.metadata.groups.len(), 2);
        assert_eq!(
            fs::read_to_string(addon_dir.join("WeakAuras").join("WeakAuras.toc")).expect("toc"),
            "before"
        );
        assert_eq!(
            fs::read_to_string(wtf_dir.join("Config.wtf")).expect("config"),
            "before"
        );
        assert!(!wtf_dir.join("New.lua").exists());
    }

    #[test]
    fn list_backups_reads_metadata_and_sorts_newest_first() {
        let temp = tempdir().expect("temp dir");
        let backup_dir = temp.path().join("backups");
        fs::create_dir_all(&backup_dir).expect("backup dir");

        write_test_backup_archive(
            &backup_dir.join("backup-retail-old.zip"),
            BackupMetadata {
                schema_version: 1,
                created_at: "2026-04-15T10:00:00Z".to_string(),
                label: Some("old".to_string()),
                flavor: "retail".to_string(),
                flavor_root: PathBuf::from("C:/WoW/_retail_"),
                groups: vec![BackupGroup::Addons],
            },
        );
        write_test_backup_archive(
            &backup_dir.join("backup-retail-new.zip"),
            BackupMetadata {
                schema_version: 1,
                created_at: "2026-04-15T11:00:00Z".to_string(),
                label: Some("new".to_string()),
                flavor: "retail".to_string(),
                flavor_root: PathBuf::from("C:/WoW/_retail_"),
                groups: vec![BackupGroup::Wtf, BackupGroup::Fonts],
            },
        );

        let catalog = list_backups(Some(&backup_dir)).expect("list backups");

        assert_eq!(catalog.entries.len(), 2);
        assert_eq!(catalog.entries[0].backup_id, "backup-retail-new");
        assert_eq!(catalog.entries[0].metadata.label.as_deref(), Some("new"));
        assert_eq!(catalog.entries[1].backup_id, "backup-retail-old");
        assert_eq!(catalog.entries[1].metadata.label.as_deref(), Some("old"));
        assert!(
            catalog
                .entries
                .iter()
                .all(|entry| entry.archive_size_bytes > 0)
        );
    }

    #[test]
    fn restore_backup_selection_resolves_backup_by_id() {
        let temp = tempdir().expect("temp dir");
        let flavor_root = temp.path().join("_retail_");
        let interface_dir = flavor_root.join("Interface");
        let addon_dir = interface_dir.join("AddOns");
        let wtf_dir = flavor_root.join("WTF");
        let fonts_dir = flavor_root.join("Fonts");
        let installation = DetectedFlavorInstallation {
            platform: HostPlatform::Windows,
            product_root: temp.path().to_path_buf(),
            flavor_root: flavor_root.clone(),
            flavor: WowFlavor::Retail,
            interface_dir,
            addon_dir: addon_dir.clone(),
            wtf_dir: wtf_dir.clone(),
            fonts_dir,
        };

        fs::create_dir_all(addon_dir.join("WeakAuras")).expect("addon dir");
        fs::write(addon_dir.join("WeakAuras").join("WeakAuras.toc"), "before").expect("toc");

        let backup = create_backup(BackupRequest {
            installation: installation.clone(),
            output_path: Some(temp.path().join("out")),
            groups: vec![BackupGroup::Addons],
            label: Some("smoke".to_string()),
        })
        .expect("backup");

        fs::write(addon_dir.join("WeakAuras").join("WeakAuras.toc"), "after").expect("toc");
        let backup_id = backup
            .archive_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("backup id")
            .to_string();

        let restored = restore_backup_selection(RestoreBackupRequest {
            installation,
            archive_path: None,
            backup_id: Some(backup_id),
            backup_dir: Some(temp.path().join("out")),
        })
        .expect("restore by id");

        assert_eq!(restored.metadata.label.as_deref(), Some("smoke"));
        assert_eq!(
            fs::read_to_string(addon_dir.join("WeakAuras").join("WeakAuras.toc")).expect("toc"),
            "before"
        );
    }

    fn write_test_backup_archive(path: &Path, metadata: BackupMetadata) {
        let file = File::create(path).expect("archive file");
        let mut zip = ZipWriter::new(file);
        zip.start_file("backup.toml", SimpleFileOptions::default())
            .expect("start backup metadata");
        zip.write_all(
            toml::to_string_pretty(&metadata)
                .expect("serialize metadata")
                .as_bytes(),
        )
        .expect("write backup metadata");
        zip.finish().expect("finish archive");
    }
}
