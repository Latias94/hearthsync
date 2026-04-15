use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use walkdir::WalkDir;

use crate::core::addon::{
    AddonPackageMetadata, AddonRegistry, AddonSourceRef, TrackedAddon, TrackedAddonPackage,
    load_registry,
};
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddonLock {
    pub schema_version: u32,
    pub generated_at: String,
    pub packages: Vec<AddonLockPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddonLockPackage {
    pub package_id: String,
    #[serde(default)]
    pub index_name: Option<String>,
    #[serde(default)]
    pub index_package_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    pub source: AddonSourceRef,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub website_url: Option<String>,
    #[serde(default)]
    pub source_sha256: Option<String>,
    pub content_sha256: String,
    pub installed_at: String,
    pub updated_at: String,
    pub addon_directories: Vec<String>,
    pub addons: Vec<TrackedAddon>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockInspection {
    pub lock_path: PathBuf,
    pub lock: AddonLock,
    pub package_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockWriteResult {
    pub lock_path: PathBuf,
    pub package_count: usize,
    pub removed: bool,
}

pub fn inspect_addon_lock(
    installation: &DetectedFlavorInstallation,
) -> AppResult<AddonLockInspection> {
    let path = lock_path(installation);
    let content = fs::read_to_string(&path)?;
    let lock = toml::from_str::<AddonLock>(&content)?;
    validate_addon_lock(&lock)?;
    let package_count = lock.packages.len();

    Ok(AddonLockInspection {
        lock_path: path,
        lock,
        package_count,
    })
}

pub fn write_addon_lock(
    installation: &DetectedFlavorInstallation,
) -> AppResult<AddonLockWriteResult> {
    let registry = load_registry(installation)?;
    let path = lock_path(installation);
    if registry.packages.is_empty() {
        cleanup_addon_lock(&path)?;
        return Ok(AddonLockWriteResult {
            lock_path: path,
            package_count: 0,
            removed: true,
        });
    }

    let lock = build_addon_lock(installation, &registry)?;
    write_addon_lock_file(&path, &lock)?;

    Ok(AddonLockWriteResult {
        lock_path: path,
        package_count: lock.packages.len(),
        removed: false,
    })
}

pub(crate) fn sync_addon_lock_from_registry(
    installation: &DetectedFlavorInstallation,
    registry: &AddonRegistry,
) -> AppResult<()> {
    let path = lock_path(installation);
    if registry.packages.is_empty() {
        cleanup_addon_lock(&path)?;
        return Ok(());
    }

    let lock = build_addon_lock(installation, registry)?;
    write_addon_lock_file(&path, &lock)
}

pub fn lock_path(installation: &DetectedFlavorInstallation) -> PathBuf {
    installation.addon_dir.join(".hearthsync").join("lock.toml")
}

fn build_addon_lock(
    installation: &DetectedFlavorInstallation,
    registry: &AddonRegistry,
) -> AppResult<AddonLock> {
    let mut packages = registry
        .packages
        .iter()
        .map(|package| build_lock_package(installation, package))
        .collect::<AppResult<Vec<_>>>()?;
    packages.sort_by(|left, right| left.package_id.cmp(&right.package_id));

    Ok(AddonLock {
        schema_version: 1,
        generated_at: now_rfc3339()?,
        packages,
    })
}

fn build_lock_package(
    installation: &DetectedFlavorInstallation,
    package: &TrackedAddonPackage,
) -> AppResult<AddonLockPackage> {
    let metadata = package.metadata.as_ref();
    let mut addons = package.addons.clone();
    addons.sort_by(|left, right| left.directory_name.cmp(&right.directory_name));
    let addon_directories = addons
        .iter()
        .map(|addon| addon.directory_name.clone())
        .collect::<Vec<_>>();

    Ok(AddonLockPackage {
        package_id: package.package_id.clone(),
        index_name: metadata.and_then(|value| value.index_name.clone()),
        index_package_id: metadata.and_then(|value| value.index_package_id.clone()),
        name: lock_package_name(package, metadata),
        version: lock_package_version(package, metadata),
        source: package.source.clone(),
        source_url: metadata.and_then(|value| value.source_url.clone()),
        website_url: metadata.and_then(|value| value.website_url.clone()),
        source_sha256: metadata.and_then(|value| value.source_sha256.clone()),
        content_sha256: package_content_sha256(installation, package)?,
        installed_at: package.installed_at.clone(),
        updated_at: package.updated_at.clone(),
        addon_directories,
        addons,
    })
}

fn lock_package_name(
    package: &TrackedAddonPackage,
    metadata: Option<&AddonPackageMetadata>,
) -> Option<String> {
    metadata
        .and_then(|value| value.package_name.clone())
        .or_else(|| {
            package
                .addons
                .iter()
                .filter_map(|addon| addon.title.clone())
                .find(|value| !value.trim().is_empty())
        })
        .or_else(|| Some(package.package_id.clone()))
}

fn lock_package_version(
    package: &TrackedAddonPackage,
    metadata: Option<&AddonPackageMetadata>,
) -> Option<String> {
    metadata
        .and_then(|value| value.version.clone())
        .or_else(|| infer_addon_version(&package.addons))
}

fn infer_addon_version(addons: &[TrackedAddon]) -> Option<String> {
    let versions = addons
        .iter()
        .filter_map(|addon| addon.version.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>();
    if versions.len() == 1 {
        versions.iter().next().map(|value| (*value).to_string())
    } else {
        None
    }
}

fn package_content_sha256(
    installation: &DetectedFlavorInstallation,
    package: &TrackedAddonPackage,
) -> AppResult<String> {
    let mut files = Vec::new();
    for addon in &package.addons {
        let addon_path = installation.addon_dir.join(&addon.directory_name);
        if !addon_path.is_dir() {
            return Err(AppError::NotFound(format!(
                "tracked addon directory missing: {}",
                addon_path.display()
            )));
        }

        for entry in WalkDir::new(&addon_path) {
            let entry = entry.map_err(|error| AppError::Validation(error.to_string()))?;
            if !entry.file_type().is_file() {
                continue;
            }
            let relative_path = normalize_relative_path(entry.path(), &installation.addon_dir)?;
            files.push((relative_path, entry.path().to_path_buf()));
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));

    let mut hasher = Sha256::new();
    for (relative_path, path) in files {
        hash_file_entry(&mut hasher, &relative_path, &path)?;
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn hash_file_entry(hasher: &mut Sha256, relative_path: &str, path: &Path) -> AppResult<()> {
    let length = fs::metadata(path)?.len();
    hasher.update(relative_path.as_bytes());
    hasher.update([0]);
    hasher.update(length.to_le_bytes());
    hasher.update([0]);

    let mut file = File::open(path)?;
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    hasher.update([0]);

    Ok(())
}

fn normalize_relative_path(path: &Path, base: &Path) -> AppResult<String> {
    let relative = path.strip_prefix(base).map_err(|_| {
        AppError::Validation(format!(
            "path `{}` is outside addon root `{}`",
            path.display(),
            base.display()
        ))
    })?;
    Ok(relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

fn write_addon_lock_file(path: &Path, lock: &AddonLock) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, toml::to_string_pretty(lock)?)?;
    Ok(())
}

fn cleanup_addon_lock(path: &Path) -> AppResult<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }

    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if !parent.exists() {
        return Ok(());
    }

    let mut entries = fs::read_dir(parent)?;
    if entries.next().is_none() {
        fs::remove_dir(parent)?;
    }

    Ok(())
}

fn validate_addon_lock(lock: &AddonLock) -> AppResult<()> {
    if lock.schema_version != 1 {
        return Err(AppError::Validation(format!(
            "unsupported addon lock schema version: {}",
            lock.schema_version
        )));
    }
    Ok(())
}

fn now_rfc3339() -> AppResult<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AppError::Validation(error.to_string()))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;
    use std::path::Path;

    use tempfile::tempdir;
    use zip::CompressionMethod;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::{inspect_addon_lock, lock_path, write_addon_lock};
    use crate::core::addon::{
        AddonPackageMetadata, InstallAddonRequest, RemoveAddonRequest, install_addon, remove_addons,
    };
    use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};

    #[test]
    fn install_addon_writes_lock_with_metadata_and_content_hash() {
        let temp = tempdir().expect("temp dir");
        let installation = create_fixture_installation(temp.path());
        let archive_path = temp.path().join("details.zip");
        create_addon_archive(
            &archive_path,
            &[(
                "Details/Details.toc",
                "## Interface: 110000\n## Title: Details!\n## Version: 1.0.0\n",
            )],
        );

        install_addon(InstallAddonRequest {
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: Some(AddonPackageMetadata {
                index_name: Some("Fixture Index".to_string()),
                index_package_id: Some("details".to_string()),
                package_name: Some("Details".to_string()),
                version: Some("1.0.0".to_string()),
                source_url: Some("https://example.com/details.zip".to_string()),
                website_url: Some("https://example.com/details".to_string()),
                source_sha256: Some("source-hash".to_string()),
                supported_flavors: vec!["retail".to_string()],
            }),
        })
        .expect("install addon");

        let inspection = inspect_addon_lock(&installation).expect("inspect lock");
        assert_eq!(inspection.package_count, 1);
        assert_eq!(
            inspection.lock.packages[0].index_package_id.as_deref(),
            Some("details")
        );
        assert_eq!(inspection.lock.packages[0].name.as_deref(), Some("Details"));
        assert_eq!(
            inspection.lock.packages[0].version.as_deref(),
            Some("1.0.0")
        );
        assert_eq!(inspection.lock.packages[0].content_sha256.len(), 64);
        assert_eq!(
            inspection.lock.packages[0].addon_directories,
            vec!["Details"]
        );
    }

    #[test]
    fn write_addon_lock_removes_stale_lock_when_registry_is_empty() {
        let temp = tempdir().expect("temp dir");
        let installation = create_fixture_installation(temp.path());
        let path = lock_path(&installation);
        fs::create_dir_all(path.parent().expect("lock parent")).expect("lock parent");
        fs::write(&path, "stale").expect("stale lock");

        let result = write_addon_lock(&installation).expect("write lock");

        assert!(result.removed);
        assert!(!path.exists());
    }

    #[test]
    fn remove_addon_cleans_lock_file_when_last_package_is_removed() {
        let temp = tempdir().expect("temp dir");
        let installation = create_fixture_installation(temp.path());
        let archive_path = temp.path().join("details.zip");
        create_addon_archive(
            &archive_path,
            &[(
                "Details/Details.toc",
                "## Interface: 110000\n## Version: 1.0.0\n",
            )],
        );

        install_addon(InstallAddonRequest {
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install addon");
        assert!(lock_path(&installation).exists());

        remove_addons(RemoveAddonRequest {
            installation: installation.clone(),
            name: "Details".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
        })
        .expect("remove addon");

        assert!(!lock_path(&installation).exists());
    }

    fn create_fixture_installation(root: &Path) -> DetectedFlavorInstallation {
        let product_root = root.join("World of Warcraft");
        let flavor_root = product_root.join("_retail_");
        let interface_dir = flavor_root.join("Interface");
        let addon_dir = interface_dir.join("AddOns");
        let wtf_dir = flavor_root.join("WTF");
        let fonts_dir = flavor_root.join("Fonts");

        fs::create_dir_all(&addon_dir).expect("addon dir");
        fs::create_dir_all(&wtf_dir).expect("wtf dir");
        fs::create_dir_all(&fonts_dir).expect("fonts dir");

        DetectedFlavorInstallation {
            platform: HostPlatform::Windows,
            product_root,
            flavor_root,
            flavor: WowFlavor::Retail,
            interface_dir,
            addon_dir,
            wtf_dir,
            fonts_dir,
        }
    }

    fn create_addon_archive(path: &Path, entries: &[(&str, &str)]) {
        let file = fs::File::create(path).expect("archive file");
        let mut zip = ZipWriter::new(file);
        for (name, content) in entries {
            zip.start_file(
                name.replace('\\', "/"),
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .expect("start file");
            zip.write_all(content.as_bytes()).expect("write file");
        }
        zip.finish().expect("finish zip");
    }
}
