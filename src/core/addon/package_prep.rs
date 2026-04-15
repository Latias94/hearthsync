use std::fs::{self, File};
use std::path::{Path, PathBuf};

use tempfile::{TempDir, tempdir};
use zip::ZipArchive;

use super::provider::{AddonProviderContext, materialize_source_input, materialize_source_ref};
use super::{AddonSourceRef, PreparedAddonDirectory, PreparedAddonPackage, TrackedAddon};
use crate::core::error::{AppError, AppResult};

pub(crate) fn prepare_package_from_source_input_with_flavor(
    source: &str,
    target_flavor: Option<crate::core::install::WowFlavor>,
) -> AppResult<PreparedAddonPackage> {
    let stage_dir = tempdir()?;
    let materialized = materialize_source_input(
        source,
        stage_dir.path(),
        AddonProviderContext { target_flavor },
    )?;
    prepare_package_from_archive(
        materialized.source_ref,
        &materialized.archive_path,
        stage_dir,
    )
}

pub(crate) fn prepare_package_from_source_ref_with_flavor(
    source: &AddonSourceRef,
    target_flavor: Option<crate::core::install::WowFlavor>,
) -> AppResult<PreparedAddonPackage> {
    let stage_dir = tempdir()?;
    let materialized = materialize_source_ref(
        source,
        stage_dir.path(),
        AddonProviderContext { target_flavor },
    )?;
    prepare_package_from_archive(
        materialized.source_ref,
        &materialized.archive_path,
        stage_dir,
    )
}

pub(crate) fn prepare_package_from_archive_with_source(
    source: AddonSourceRef,
    archive_path: &Path,
) -> AppResult<PreparedAddonPackage> {
    let stage_dir = tempdir()?;
    prepare_package_from_archive(source, archive_path, stage_dir)
}

fn prepare_package_from_archive(
    source: AddonSourceRef,
    archive_path: &Path,
    stage_dir: TempDir,
) -> AppResult<PreparedAddonPackage> {
    let addons = extract_archive_addons(archive_path, stage_dir.path())?;
    if addons.is_empty() {
        return Err(AppError::Validation(
            "archive does not contain any detectable addon directories".to_string(),
        ));
    }

    let addon_names = addons
        .iter()
        .map(|addon| addon.addon.directory_name.as_str())
        .collect::<Vec<_>>();
    let package_id = derive_package_id(&source, &addon_names);

    Ok(PreparedAddonPackage {
        source,
        package_id,
        addons,
        metadata: None,
        _stage_dir: stage_dir,
    })
}

fn extract_archive_addons(
    archive_path: &Path,
    stage_root: &Path,
) -> AppResult<Vec<PreparedAddonDirectory>> {
    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;
    let addon_roots = discover_archive_addon_roots(&mut archive)?;
    if addon_roots.is_empty() {
        return Ok(Vec::new());
    }

    let mut file_counts = vec![0usize; addon_roots.len()];

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }

        let entry_name = entry.name().to_string();
        let segments = safe_zip_segments(&entry_name)?;
        if segments.is_empty() {
            continue;
        }

        let Some(root_index) = match_addon_root(&segments, &addon_roots) else {
            continue;
        };
        let root = &addon_roots[root_index];
        let addon_name = root
            .last()
            .map(String::as_str)
            .ok_or_else(|| AppError::Validation("invalid addon root".to_string()))?;
        let relative = &segments[root.len()..];
        let destination =
            join_segments(stage_root, &[addon_name]).join(join_segments(Path::new(""), relative));
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = File::create(destination)?;
        std::io::copy(&mut entry, &mut output)?;
        file_counts[root_index] += 1;
    }

    let mut prepared = Vec::new();
    for (index, root) in addon_roots.iter().enumerate() {
        let addon_name = root
            .last()
            .map(String::as_str)
            .ok_or_else(|| AppError::Validation("invalid addon root".to_string()))?;
        let stage_path = stage_root.join(addon_name);
        let addon = inspect_staged_addon(&stage_path, addon_name)?;
        prepared.push(PreparedAddonDirectory {
            addon,
            stage_path,
            file_count: file_counts[index],
        });
    }

    prepared.sort_by(|left, right| left.addon.directory_name.cmp(&right.addon.directory_name));
    Ok(prepared)
}

fn discover_archive_addon_roots(archive: &mut ZipArchive<File>) -> AppResult<Vec<Vec<String>>> {
    let mut roots = Vec::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }

        let entry_name = entry.name().to_string();
        let segments = safe_zip_segments(&entry_name)?;
        if segments.len() < 2 {
            continue;
        }

        let file_name = segments
            .last()
            .copied()
            .ok_or_else(|| AppError::Validation("invalid archive entry".to_string()))?;
        if !file_name.ends_with(".toc") {
            continue;
        }

        let Some(file_stem) = Path::new(file_name)
            .file_stem()
            .and_then(|stem| stem.to_str())
        else {
            continue;
        };
        let parent_name = segments[segments.len() - 2];
        if file_stem != parent_name {
            continue;
        }

        let root = segments[..segments.len() - 1]
            .iter()
            .map(|segment| (*segment).to_string())
            .collect::<Vec<_>>();
        if !roots.contains(&root) {
            roots.push(root);
        }
    }

    roots.sort_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
    Ok(roots)
}

fn match_addon_root(segments: &[&str], roots: &[Vec<String>]) -> Option<usize> {
    roots.iter().position(|root| {
        root.len() <= segments.len()
            && root
                .iter()
                .zip(segments.iter())
                .all(|(left, right)| left.as_str() == *right)
    })
}

fn inspect_staged_addon(stage_path: &Path, addon_name: &str) -> AppResult<TrackedAddon> {
    let toc_path = find_primary_toc(stage_path, addon_name)?;
    let (toc_file, title, version) = if let Some(path) = toc_path {
        let toc_file = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string());
        let content = fs::read_to_string(&path).unwrap_or_default();
        let title = extract_toc_field(&content, "Title");
        let version = extract_toc_field(&content, "Version");
        (toc_file, title, version)
    } else {
        (None, None, None)
    };

    Ok(TrackedAddon {
        directory_name: addon_name.to_string(),
        toc_file,
        title,
        version,
    })
}

pub(super) fn find_primary_toc(stage_path: &Path, addon_name: &str) -> AppResult<Option<PathBuf>> {
    if !stage_path.exists() {
        return Ok(None);
    }

    let preferred = stage_path.join(format!("{addon_name}.toc"));
    if preferred.exists() {
        return Ok(Some(preferred));
    }

    for entry in fs::read_dir(stage_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("toc"))
        {
            return Ok(Some(path));
        }
    }

    Ok(None)
}

fn extract_toc_field(content: &str, field: &str) -> Option<String> {
    let needle = format!("## {field}:");
    content.lines().find_map(|line| {
        line.trim()
            .strip_prefix(&needle)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn derive_package_id(source: &AddonSourceRef, addon_names: &[&str]) -> String {
    let base = match source {
        AddonSourceRef::LocalArchive { path } => path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string()),
        AddonSourceRef::HttpArchive { url } => Path::new(url)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string()),
        AddonSourceRef::CurseForgeMod { mod_id, file_id } => Some(match file_id {
            Some(file_id) => format!("curseforge-{mod_id}-{file_id}"),
            None => format!("curseforge-{mod_id}"),
        }),
        AddonSourceRef::GitHubRelease {
            owner,
            repo,
            tag,
            asset_name,
        } => asset_name
            .as_deref()
            .and_then(|value| Path::new(value).file_stem().and_then(|stem| stem.to_str()))
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
            .or_else(|| {
                tag.as_ref()
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| format!("{repo}-{value}"))
            })
            .or_else(|| Some(format!("{owner}-{repo}"))),
    }
    .or_else(|| addon_names.first().map(|name| (*name).to_string()))
    .unwrap_or_else(|| "addon-package".to_string());

    let mut slug = String::new();
    for character in base.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }

    slug.trim_matches('-').to_string()
}

fn safe_zip_segments(entry_name: &str) -> AppResult<Vec<&str>> {
    let mut segments = Vec::new();
    for segment in entry_name.split('/') {
        if segment.is_empty() {
            continue;
        }

        if segment == "." || segment == ".." || segment.contains('\\') {
            return Err(AppError::Validation(format!(
                "unsafe archive path: `{entry_name}`"
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
