use std::fs::File;
use std::io::{Read, Write};

use tempfile::tempdir;
use walkdir::WalkDir;
use zip::{ZipArchive, ZipWriter};

use super::*;

pub(super) fn collect_bundle_entry_names(bundle_path: &Path) -> AppResult<Vec<String>> {
    let file = File::open(bundle_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut entry_names = Vec::new();

    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }
        entry_names.push(entry.name().to_string());
    }

    Ok(entry_names)
}

pub(super) fn read_bundle_entry_bytes_from_archive(
    archive: &mut ZipArchive<File>,
    archive_name: &str,
) -> AppResult<Vec<u8>> {
    let mut entry = archive
        .by_name(archive_name)
        .map_err(|_| AppError::NotFound(format!("bundle entry is missing: {archive_name}")))?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes)?;
    Ok(bytes)
}

pub(super) fn extract_archive_entry_to_path(
    archive: &mut ZipArchive<File>,
    archive_name: &str,
    destination: &Path,
) -> AppResult<()> {
    let segments = safe_zip_segments(archive_name)?;
    if segments.is_empty() {
        return Err(AppError::Validation(format!(
            "bundle entry cannot be materialized because its path is empty: {archive_name}"
        )));
    }
    let mut entry = archive
        .by_name(archive_name)
        .map_err(|_| AppError::NotFound(format!("bundle entry is missing: {archive_name}")))?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut output = File::create(destination)?;
    std::io::copy(&mut entry, &mut output)?;
    Ok(())
}

pub(super) fn extract_embedded_addon_lock(bundle_path: &Path) -> AppResult<ExtractedAddonLock> {
    let file = File::open(bundle_path)?;
    let mut archive = ZipArchive::new(file)?;
    let stage_dir = tempdir()?;
    let lock_path = stage_dir.path().join("lock.toml");
    {
        let mut lock_entry = archive.by_name(ADDON_LOCK_ENTRY).map_err(|_| {
            AppError::NotFound(format!(
                "bundle does not contain embedded addon lock `{ADDON_LOCK_ENTRY}`"
            ))
        })?;
        let mut output = File::create(&lock_path)?;
        std::io::copy(&mut lock_entry, &mut output)?;
    }

    let source_overrides = extract_bundle_addon_source_overrides(&mut archive, stage_dir.path())?;

    Ok(ExtractedAddonLock {
        lock_path,
        source_overrides,
        _stage_dir: stage_dir,
    })
}

fn extract_bundle_addon_source_overrides(
    archive: &mut ZipArchive<File>,
    stage_root: &Path,
) -> AppResult<Vec<AddonLockSourceOverride>> {
    let source_index = match archive.by_name(ADDON_SOURCE_INDEX_ENTRY) {
        Ok(mut entry) => {
            let mut content = String::new();
            entry.read_to_string(&mut content)?;
            toml::from_str::<BundleAddonSourceIndex>(&content)?
        }
        Err(zip::result::ZipError::FileNotFound) => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };

    if source_index.schema_version != 1 {
        return Err(AppError::Validation(format!(
            "unsupported bundle addon source index schema version: {}",
            source_index.schema_version
        )));
    }

    let mut source_overrides = Vec::new();
    for source in source_index.sources {
        let segments = safe_zip_segments(&source.path)?;
        if segments.first().copied() != Some("sources") || segments.len() < 2 {
            return Err(AppError::Validation(format!(
                "bundle addon source path must be under `sources/`: {}",
                source.path
            )));
        }

        let archive_entry_name = format!("metadata/addons/{}", segments.join("/"));
        let mut source_entry = archive.by_name(&archive_entry_name).map_err(|_| {
            AppError::NotFound(format!(
                "bundle addon source archive is missing: {archive_entry_name}"
            ))
        })?;
        let extracted_path = join_segments(stage_root, &segments);
        if let Some(parent) = extracted_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = File::create(&extracted_path)?;
        std::io::copy(&mut source_entry, &mut output)?;

        source_overrides.push(AddonLockSourceOverride {
            comparison_key: source.comparison_key,
            archive_path: extracted_path,
        });
    }

    Ok(source_overrides)
}

pub(super) fn add_common_wtf_to_zip(zip: &mut ZipWriter<File>, wtf_dir: &Path) -> AppResult<usize> {
    let mut archived_files = 0usize;
    let config_wtf = wtf_dir.join("Config.wtf");
    if config_wtf.exists() {
        archived_files += add_path_to_zip(zip, &config_wtf, Path::new("wtf/common/Config.wtf"))?;
    }

    let account_root = wtf_dir.join("Account");
    if !account_root.exists() {
        return Ok(archived_files);
    }

    for entry in fs::read_dir(account_root)? {
        let entry = entry?;
        let account_dir = entry.path();
        if !account_dir.is_dir() {
            continue;
        }

        let account_name = entry.file_name().to_string_lossy().to_string();
        validate_plain_name("account", &account_name)?;
        for account_entry in fs::read_dir(&account_dir)? {
            let account_entry = account_entry?;
            let account_path = account_entry.path();
            if !account_path.is_file() {
                continue;
            }

            let file_name = account_entry.file_name().to_string_lossy().to_string();
            validate_plain_name("account WTF file", &file_name)?;
            archived_files += add_path_to_zip(
                zip,
                &account_path,
                &Path::new("wtf/common/accounts")
                    .join(&account_name)
                    .join(file_name),
            )?;
        }

        let saved_variables = account_dir.join("SavedVariables");
        if saved_variables.exists() {
            archived_files += add_path_to_zip(
                zip,
                &saved_variables,
                &Path::new("wtf/common/accounts")
                    .join(account_name)
                    .join("SavedVariables"),
            )?;
        }
    }

    Ok(archived_files)
}

pub(super) fn resolve_addon_index_paths(
    addon_indexes: &[String],
    manifest_base_dir: Option<&Path>,
) -> AppResult<Vec<(String, PathBuf)>> {
    let mut resolved = Vec::new();
    let mut file_names = Vec::new();

    for addon_index in addon_indexes {
        let reference = Path::new(addon_index);
        let source_path = if reference.is_absolute() {
            reference.to_path_buf()
        } else if let Some(base_dir) = manifest_base_dir {
            base_dir.join(reference)
        } else {
            std::env::current_dir()?.join(reference)
        };

        if !source_path.is_file() {
            return Err(AppError::NotFound(format!(
                "addon index file does not exist: {}",
                source_path.display()
            )));
        }

        let file_name = source_path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "addon index file has no usable file name: {}",
                    source_path.display()
                ))
            })?
            .to_string();
        validate_plain_name("addon index file", &file_name)?;
        if file_names.iter().any(|item| item == &file_name) {
            return Err(AppError::Validation(format!(
                "duplicate addon index file name in bundle metadata: {file_name}"
            )));
        }
        file_names.push(file_name.clone());
        resolved.push((file_name, source_path));
    }

    Ok(resolved)
}

pub(super) fn read_generated_addon_lock(path: &Path) -> AppResult<AddonLock> {
    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

pub(super) fn add_bundle_addon_sources_to_zip(
    zip: &mut ZipWriter<File>,
    installation: &DetectedFlavorInstallation,
    packages: &[AddonLockPackage],
) -> AppResult<BundleAddonSourceIndex> {
    let source_stage = tempdir()?;
    let mut entries = Vec::new();
    let mut used_file_names = Vec::new();
    let mut packages = packages.iter().collect::<Vec<_>>();
    packages.sort_by(|left, right| {
        addon_lock_package_comparison_key(left).cmp(&addon_lock_package_comparison_key(right))
    });

    for (index, package) in packages.into_iter().enumerate() {
        let comparison_key = addon_lock_package_comparison_key(package);
        let file_name = unique_bundle_source_archive_name(
            &comparison_key,
            &package.package_id,
            index,
            &mut used_file_names,
        );
        let source_archive_path = source_stage.path().join(&file_name);
        write_addon_package_source_archive(&source_archive_path, installation, package)?;
        let relative_source_path = format!("sources/{file_name}");
        let bundle_entry_path = Path::new(ADDON_SOURCE_ENTRY_ROOT).join(&file_name);
        add_path_to_zip(zip, &source_archive_path, &bundle_entry_path)?;

        entries.push(BundleAddonSourceEntry {
            comparison_key,
            package_id: package.package_id.clone(),
            path: relative_source_path,
            content_sha256: package.content_sha256.clone(),
            addon_directories: package.addon_directories.clone(),
        });
    }

    Ok(BundleAddonSourceIndex {
        schema_version: 1,
        sources: entries,
    })
}

fn unique_bundle_source_archive_name(
    comparison_key: &str,
    package_id: &str,
    index: usize,
    used_file_names: &mut Vec<String>,
) -> String {
    let mut base = safe_file_part(comparison_key);
    if base.is_empty() {
        base = safe_file_part(package_id);
    }
    if base.is_empty() {
        base = format!("package-{index}");
    }

    let mut candidate = format!("{base}.zip");
    let mut suffix = 2usize;
    while used_file_names.iter().any(|item| item == &candidate) {
        candidate = format!("{base}-{suffix}.zip");
        suffix += 1;
    }
    used_file_names.push(candidate.clone());
    candidate
}

fn write_addon_package_source_archive(
    archive_path: &Path,
    installation: &DetectedFlavorInstallation,
    package: &AddonLockPackage,
) -> AppResult<()> {
    let file = File::create(archive_path)?;
    let mut zip = ZipWriter::new(file);
    let mut archived_files = 0usize;

    for addon_directory in &package.addon_directories {
        validate_plain_name("addon", addon_directory)?;
        let source = installation.addon_dir.join(addon_directory);
        if !source.is_dir() {
            return Err(AppError::NotFound(format!(
                "tracked addon directory does not exist: {}",
                source.display()
            )));
        }
        archived_files += add_path_to_zip(&mut zip, &source, Path::new(addon_directory))?;
    }

    zip.finish()?;
    if archived_files == 0 {
        return Err(AppError::Validation(format!(
            "tracked package `{}` does not contain any addon files",
            package.package_id
        )));
    }

    Ok(())
}

pub(super) fn add_character_wtf_to_zip(
    zip: &mut ZipWriter<File>,
    wtf_dir: &Path,
    character: &CharacterResource,
    account: &str,
) -> AppResult<usize> {
    validate_plain_name("server", &character.source_server)?;
    validate_plain_name("character", &character.source_character)?;
    validate_plain_name("account", account)?;
    let character_dir = wtf_dir
        .join("Account")
        .join(account)
        .join(&character.source_server)
        .join(&character.source_character);

    if !character_dir.exists() {
        return Err(AppError::NotFound(format!(
            "character WTF directory does not exist: {}",
            character_dir.display()
        )));
    }

    add_path_to_zip(
        zip,
        &character_dir,
        &Path::new("wtf/characters")
            .join(account)
            .join(&character.source_server)
            .join(&character.source_character),
    )
}

pub(super) fn resolve_character_account(
    wtf_dir: &Path,
    character: &CharacterResource,
) -> AppResult<String> {
    if let Some(account) = &character.source_account {
        validate_plain_name("account", account)?;
        return Ok(account.clone());
    }

    let mut matches = Vec::new();
    let account_root = wtf_dir.join("Account");
    if !account_root.exists() {
        return Err(AppError::NotFound(format!(
            "account root does not exist: {}",
            account_root.display()
        )));
    }

    for entry in fs::read_dir(account_root)? {
        let entry = entry?;
        let account_dir = entry.path();
        if !account_dir.is_dir() {
            continue;
        }

        let candidate = account_dir
            .join(&character.source_server)
            .join(&character.source_character);
        if candidate.exists() {
            matches.push(entry.file_name().to_string_lossy().to_string());
        }
    }

    match matches.as_slice() {
        [account] => Ok(account.clone()),
        [] => Err(AppError::NotFound(format!(
            "no account contains character `{}` on server `{}`",
            character.source_character, character.source_server
        ))),
        many => Err(AppError::Validation(format!(
            "character `{}` on server `{}` exists in multiple accounts: {:?}. Set source_account explicitly.",
            character.source_character, character.source_server, many
        ))),
    }
}

pub(super) fn add_path_to_zip(
    zip: &mut ZipWriter<File>,
    source_path: &Path,
    archive_path: &Path,
) -> AppResult<usize> {
    if !source_path.exists() {
        return Ok(0);
    }

    if source_path.is_file() {
        write_file_to_zip(zip, source_path, archive_path)?;
        return Ok(1);
    }

    let mut archived_files = 0usize;
    for entry in WalkDir::new(source_path).follow_links(false) {
        let entry = entry.map_err(|error| AppError::Validation(error.to_string()))?;
        let path = entry.path();
        let relative = path
            .strip_prefix(source_path)
            .map_err(|error| AppError::Validation(error.to_string()))?;

        if relative.as_os_str().is_empty() || should_skip_path(relative) {
            continue;
        }

        let target_path = archive_path.join(relative);
        if entry.file_type().is_dir() {
            zip.add_directory(to_zip_path(&target_path), zip_dir_options())?;
            continue;
        }

        write_file_to_zip(zip, path, &target_path)?;
        archived_files += 1;
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

pub(super) fn write_toml_to_zip<T: Serialize>(
    zip: &mut ZipWriter<File>,
    archive_path: &str,
    value: &T,
) -> AppResult<usize> {
    zip.start_file(archive_path, zip_file_options())?;
    zip.write_all(toml::to_string_pretty(value)?.as_bytes())?;
    Ok(1)
}

pub(super) fn read_manifest_from_archive(
    archive: &mut ZipArchive<File>,
) -> AppResult<BundleManifest> {
    let mut manifest_file = archive.by_name(MANIFEST_ENTRY)?;
    let mut content = String::new();
    manifest_file.read_to_string(&mut content)?;
    Ok(toml::from_str(&content)?)
}

pub(super) fn count_bundle_entries(archive: &mut ZipArchive<File>) -> AppResult<BundleEntryCounts> {
    let mut counts = BundleEntryCounts::default();

    for index in 0..archive.len() {
        let file = archive.by_index(index)?;
        if file.is_dir() {
            continue;
        }

        counts.total_files += 1;
        let name = file.name();
        if name == MANIFEST_ENTRY || name.starts_with("metadata/") {
            counts.metadata += 1;
        } else if name.starts_with("addons/") {
            counts.addons += 1;
        } else if name.starts_with("wtf/common/") {
            counts.wtf_common += 1;
        } else if name.starts_with("wtf/characters/") {
            counts.wtf_characters += 1;
        } else if name.starts_with("fonts/") {
            counts.fonts += 1;
        } else if name.starts_with("interface/") {
            counts.interface_assets += 1;
        }
    }

    Ok(counts)
}
