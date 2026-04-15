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

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockPackageSnapshot {
    pub comparison_key: String,
    pub package_id: String,
    pub index_name: Option<String>,
    pub index_package_id: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub source: AddonSourceRef,
    pub source_url: Option<String>,
    pub website_url: Option<String>,
    pub source_sha256: Option<String>,
    pub content_sha256: Option<String>,
    pub addon_directories: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockFieldChange {
    pub field: String,
    pub left: Option<String>,
    pub right: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockPackageDiff {
    pub comparison_key: String,
    pub left: AddonLockPackageSnapshot,
    pub right: AddonLockPackageSnapshot,
    pub changes: Vec<AddonLockFieldChange>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockDiffResult {
    pub left_label: String,
    pub right_label: String,
    pub left_package_count: usize,
    pub right_package_count: usize,
    pub identical: bool,
    pub unchanged_packages: usize,
    pub added_packages: Vec<AddonLockPackageSnapshot>,
    pub removed_packages: Vec<AddonLockPackageSnapshot>,
    pub changed_packages: Vec<AddonLockPackageDiff>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockVerifyResult {
    pub lock_path: PathBuf,
    pub installation_root: PathBuf,
    pub tracked_package_count: usize,
    pub untracked_addons: Vec<String>,
    pub missing_addon_directories: Vec<AddonLockPackageDirectoryIssue>,
    pub diff: AddonLockDiffResult,
    pub matches: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockPackageDirectoryIssue {
    pub comparison_key: String,
    pub package_id: String,
    pub missing_addon_directories: Vec<String>,
}

pub fn inspect_addon_lock(
    installation: &DetectedFlavorInstallation,
) -> AppResult<AddonLockInspection> {
    let path = lock_path(installation);
    let lock = read_addon_lock(&path)?;
    let package_count = lock.packages.len();

    Ok(AddonLockInspection {
        lock_path: path,
        lock,
        package_count,
    })
}

pub fn diff_addon_locks(left_path: &Path, right_path: &Path) -> AppResult<AddonLockDiffResult> {
    let left = read_addon_lock(left_path)?;
    let right = read_addon_lock(right_path)?;
    compare_lock_snapshots(
        &left_label(left_path),
        &lock_snapshots(&left)?,
        &left_label(right_path),
        &lock_snapshots(&right)?,
    )
}

pub fn verify_addon_lock(
    installation: &DetectedFlavorInstallation,
    expected_lock_path: Option<&Path>,
) -> AppResult<AddonLockVerifyResult> {
    let lock_path = expected_lock_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| lock_path(installation));
    let expected = read_addon_lock(&lock_path)?;
    let inventory = crate::core::addon::list_addons(installation)?;

    let mut current_snapshots = Vec::new();
    let mut missing_addon_directories = Vec::new();
    for package in &inventory.tracked_packages {
        let (snapshot, missing) = snapshot_from_tracked_package(installation, package);
        if !missing.is_empty() {
            missing_addon_directories.push(AddonLockPackageDirectoryIssue {
                comparison_key: snapshot.comparison_key.clone(),
                package_id: snapshot.package_id.clone(),
                missing_addon_directories: missing,
            });
        }
        current_snapshots.push(snapshot);
    }

    let diff = compare_lock_snapshots(
        &lock_path.display().to_string(),
        &lock_snapshots(&expected)?,
        &installation.flavor_root.display().to_string(),
        &current_snapshots,
    )?;
    let matches = diff.identical
        && inventory.untracked_addons.is_empty()
        && missing_addon_directories.is_empty();

    Ok(AddonLockVerifyResult {
        lock_path,
        installation_root: installation.flavor_root.clone(),
        tracked_package_count: inventory.tracked_packages.len(),
        untracked_addons: inventory.untracked_addons,
        missing_addon_directories,
        diff,
        matches,
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

fn read_addon_lock(path: &Path) -> AppResult<AddonLock> {
    let content = fs::read_to_string(path)?;
    let lock = toml::from_str::<AddonLock>(&content)?;
    validate_addon_lock(&lock)?;
    Ok(lock)
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
    let (content_sha256, missing_addon_directories) =
        package_content_sha256_with_missing(installation, package)?;
    if !missing_addon_directories.is_empty() {
        return Err(AppError::NotFound(format!(
            "tracked addon directories missing for package `{}`: {}",
            package.package_id,
            missing_addon_directories.join(", ")
        )));
    }
    Ok(content_sha256.unwrap_or_default())
}

fn package_content_sha256_with_missing(
    installation: &DetectedFlavorInstallation,
    package: &TrackedAddonPackage,
) -> AppResult<(Option<String>, Vec<String>)> {
    let mut files = Vec::new();
    let mut missing_addon_directories = Vec::new();
    for addon in &package.addons {
        let addon_path = installation.addon_dir.join(&addon.directory_name);
        if !addon_path.is_dir() {
            missing_addon_directories.push(addon.directory_name.clone());
            continue;
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

    if !missing_addon_directories.is_empty() {
        return Ok((None, missing_addon_directories));
    }

    let mut hasher = Sha256::new();
    for (relative_path, path) in files {
        hash_file_entry(&mut hasher, &relative_path, &path)?;
    }

    Ok((
        Some(format!("{:x}", hasher.finalize())),
        missing_addon_directories,
    ))
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

    let mut comparison_keys = BTreeSet::new();
    for package in &lock.packages {
        let comparison_key = comparison_key(
            &package.package_id,
            package.index_name.as_deref(),
            package.index_package_id.as_deref(),
        );
        if !comparison_keys.insert(comparison_key.clone()) {
            return Err(AppError::Validation(format!(
                "duplicate addon lock package comparison key: {comparison_key}"
            )));
        }
    }
    Ok(())
}

fn now_rfc3339() -> AppResult<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AppError::Validation(error.to_string()))
}

fn compare_lock_snapshots(
    left_label: &str,
    left_packages: &[AddonLockPackageSnapshot],
    right_label: &str,
    right_packages: &[AddonLockPackageSnapshot],
) -> AppResult<AddonLockDiffResult> {
    let left_map = snapshot_map(left_packages)?;
    let right_map = snapshot_map(right_packages)?;

    let mut all_keys = left_map.keys().cloned().collect::<Vec<_>>();
    for key in right_map.keys() {
        if !left_map.contains_key(key) {
            all_keys.push(key.clone());
        }
    }
    all_keys.sort();

    let mut unchanged_packages = 0usize;
    let mut added_packages = Vec::new();
    let mut removed_packages = Vec::new();
    let mut changed_packages = Vec::new();

    for key in all_keys {
        match (left_map.get(&key), right_map.get(&key)) {
            (Some(left), Some(right)) => {
                let changes = diff_snapshot_fields(left, right);
                if changes.is_empty() {
                    unchanged_packages += 1;
                } else {
                    changed_packages.push(AddonLockPackageDiff {
                        comparison_key: key,
                        left: (*left).clone(),
                        right: (*right).clone(),
                        changes,
                    });
                }
            }
            (Some(left), None) => removed_packages.push((*left).clone()),
            (None, Some(right)) => added_packages.push((*right).clone()),
            (None, None) => {}
        }
    }

    Ok(AddonLockDiffResult {
        left_label: left_label.to_string(),
        right_label: right_label.to_string(),
        left_package_count: left_packages.len(),
        right_package_count: right_packages.len(),
        identical: added_packages.is_empty()
            && removed_packages.is_empty()
            && changed_packages.is_empty(),
        unchanged_packages,
        added_packages,
        removed_packages,
        changed_packages,
    })
}

fn snapshot_map<'a>(
    packages: &'a [AddonLockPackageSnapshot],
) -> AppResult<std::collections::BTreeMap<String, &'a AddonLockPackageSnapshot>> {
    let mut map = std::collections::BTreeMap::new();
    for package in packages {
        if map
            .insert(package.comparison_key.clone(), package)
            .is_some()
        {
            return Err(AppError::Validation(format!(
                "duplicate addon lock snapshot comparison key: {}",
                package.comparison_key
            )));
        }
    }
    Ok(map)
}

fn lock_snapshots(lock: &AddonLock) -> AppResult<Vec<AddonLockPackageSnapshot>> {
    let mut packages = lock
        .packages
        .iter()
        .map(snapshot_from_lock_package)
        .collect::<Vec<_>>();
    packages.sort_by(|left, right| left.comparison_key.cmp(&right.comparison_key));
    if has_duplicate_snapshot_keys(&packages) {
        return Err(AppError::Validation(
            "duplicate addon lock package comparison keys".to_string(),
        ));
    }
    Ok(packages)
}

fn snapshot_from_lock_package(package: &AddonLockPackage) -> AddonLockPackageSnapshot {
    AddonLockPackageSnapshot {
        comparison_key: comparison_key(
            &package.package_id,
            package.index_name.as_deref(),
            package.index_package_id.as_deref(),
        ),
        package_id: package.package_id.clone(),
        index_name: package.index_name.clone(),
        index_package_id: package.index_package_id.clone(),
        name: package.name.clone(),
        version: package.version.clone(),
        source: package.source.clone(),
        source_url: package.source_url.clone(),
        website_url: package.website_url.clone(),
        source_sha256: package.source_sha256.clone(),
        content_sha256: Some(package.content_sha256.clone()),
        addon_directories: package.addon_directories.clone(),
    }
}

fn snapshot_from_tracked_package(
    installation: &DetectedFlavorInstallation,
    package: &TrackedAddonPackage,
) -> (AddonLockPackageSnapshot, Vec<String>) {
    let metadata = package.metadata.as_ref();
    let (content_sha256, missing_addon_directories) =
        package_content_sha256_with_missing(installation, package).unwrap_or_else(|_| {
            (
                None,
                package
                    .addons
                    .iter()
                    .map(|addon| addon.directory_name.clone())
                    .collect(),
            )
        });

    let mut addon_directories = package
        .addons
        .iter()
        .map(|addon| addon.directory_name.clone())
        .collect::<Vec<_>>();
    addon_directories.sort();

    (
        AddonLockPackageSnapshot {
            comparison_key: comparison_key(
                &package.package_id,
                metadata.and_then(|value| value.index_name.as_deref()),
                metadata.and_then(|value| value.index_package_id.as_deref()),
            ),
            package_id: package.package_id.clone(),
            index_name: metadata.and_then(|value| value.index_name.clone()),
            index_package_id: metadata.and_then(|value| value.index_package_id.clone()),
            name: lock_package_name(package, metadata),
            version: lock_package_version(package, metadata),
            source: package.source.clone(),
            source_url: metadata.and_then(|value| value.source_url.clone()),
            website_url: metadata.and_then(|value| value.website_url.clone()),
            source_sha256: metadata.and_then(|value| value.source_sha256.clone()),
            content_sha256,
            addon_directories,
        },
        missing_addon_directories,
    )
}

fn diff_snapshot_fields(
    left: &AddonLockPackageSnapshot,
    right: &AddonLockPackageSnapshot,
) -> Vec<AddonLockFieldChange> {
    let mut changes = Vec::new();
    push_change(
        &mut changes,
        "package_id",
        Some(left.package_id.clone()),
        Some(right.package_id.clone()),
    );
    push_change(&mut changes, "name", left.name.clone(), right.name.clone());
    push_change(
        &mut changes,
        "version",
        left.version.clone(),
        right.version.clone(),
    );
    push_change(
        &mut changes,
        "source",
        Some(left.source.display_name()),
        Some(right.source.display_name()),
    );
    push_change(
        &mut changes,
        "source_url",
        left.source_url.clone(),
        right.source_url.clone(),
    );
    push_change(
        &mut changes,
        "website_url",
        left.website_url.clone(),
        right.website_url.clone(),
    );
    push_change(
        &mut changes,
        "source_sha256",
        left.source_sha256.clone(),
        right.source_sha256.clone(),
    );
    push_change(
        &mut changes,
        "content_sha256",
        left.content_sha256.clone(),
        right.content_sha256.clone(),
    );
    push_change(
        &mut changes,
        "addon_directories",
        Some(left.addon_directories.join(", ")),
        Some(right.addon_directories.join(", ")),
    );
    changes
}

fn push_change(
    changes: &mut Vec<AddonLockFieldChange>,
    field: &str,
    left: Option<String>,
    right: Option<String>,
) {
    if left != right {
        changes.push(AddonLockFieldChange {
            field: field.to_string(),
            left,
            right,
        });
    }
}

fn has_duplicate_snapshot_keys(packages: &[AddonLockPackageSnapshot]) -> bool {
    let mut keys = BTreeSet::new();
    packages
        .iter()
        .any(|package| !keys.insert(package.comparison_key.clone()))
}

fn comparison_key(
    package_id: &str,
    index_name: Option<&str>,
    index_package_id: Option<&str>,
) -> String {
    let index_name = index_name.map(str::trim).filter(|value| !value.is_empty());
    let index_package_id = index_package_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    match (index_name, index_package_id) {
        (Some(index_name), Some(index_package_id)) => {
            format!("index:{index_name}:{index_package_id}")
        }
        (None, Some(index_package_id)) => format!("index:{index_package_id}"),
        _ => format!("package:{package_id}"),
    }
}

fn left_label(path: &Path) -> String {
    path.display().to_string()
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

    use super::{
        diff_addon_locks, inspect_addon_lock, lock_path, verify_addon_lock, write_addon_lock,
    };
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

    #[test]
    fn diff_addon_locks_reports_changed_added_and_removed_packages() {
        let temp = tempdir().expect("temp dir");
        let left_path = temp.path().join("left.toml");
        let right_path = temp.path().join("right.toml");

        fs::write(
            &left_path,
            r#"
schema_version = 1
generated_at = "2026-04-15T00:00:00Z"

[[packages]]
package_id = "details"
index_name = "Raid"
index_package_id = "details"
name = "Details"
version = "1.0.0"
source = { kind = "local_archive", path = "C:\\details.zip" }
content_sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
installed_at = "2026-04-15T00:00:00Z"
updated_at = "2026-04-15T00:00:00Z"
addon_directories = ["Details"]
addons = []

[[packages]]
package_id = "omen"
name = "Omen"
source = { kind = "local_archive", path = "C:\\omen.zip" }
content_sha256 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
installed_at = "2026-04-15T00:00:00Z"
updated_at = "2026-04-15T00:00:00Z"
addon_directories = ["Omen"]
addons = []
"#,
        )
        .expect("left lock");

        fs::write(
            &right_path,
            r#"
schema_version = 1
generated_at = "2026-04-16T00:00:00Z"

[[packages]]
package_id = "details-v2"
index_name = "Raid"
index_package_id = "details"
name = "Details"
version = "2.0.0"
source = { kind = "local_archive", path = "C:\\details-v2.zip" }
content_sha256 = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
installed_at = "2026-04-16T00:00:00Z"
updated_at = "2026-04-16T00:00:00Z"
addon_directories = ["Details"]
addons = []

[[packages]]
package_id = "bigwigs"
name = "BigWigs"
source = { kind = "local_archive", path = "C:\\bigwigs.zip" }
content_sha256 = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
installed_at = "2026-04-16T00:00:00Z"
updated_at = "2026-04-16T00:00:00Z"
addon_directories = ["BigWigs"]
addons = []
"#,
        )
        .expect("right lock");

        let diff = diff_addon_locks(&left_path, &right_path).expect("diff locks");

        assert!(!diff.identical);
        assert_eq!(diff.changed_packages.len(), 1);
        assert_eq!(diff.added_packages.len(), 1);
        assert_eq!(diff.removed_packages.len(), 1);
        assert!(
            diff.changed_packages[0]
                .changes
                .iter()
                .any(|change| change.field == "version")
        );
    }

    #[test]
    fn verify_addon_lock_reports_drift_and_untracked_addons() {
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

        fs::write(
            installation.addon_dir.join("Details").join("Details.toc"),
            "## Interface: 110000\n## Version: 2.0.0\n",
        )
        .expect("mutate toc");
        fs::create_dir_all(installation.addon_dir.join("BigWigs")).expect("untracked addon dir");
        fs::write(
            installation.addon_dir.join("BigWigs").join("BigWigs.toc"),
            "## Interface: 110000\n## Version: 1.0.0\n",
        )
        .expect("untracked addon toc");

        let verification = verify_addon_lock(&installation, None).expect("verify lock");

        assert!(!verification.matches);
        assert_eq!(verification.diff.changed_packages.len(), 1);
        assert_eq!(verification.untracked_addons, vec!["BigWigs"]);
        assert!(
            verification.diff.changed_packages[0]
                .changes
                .iter()
                .any(|change| change.field == "content_sha256")
        );
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
