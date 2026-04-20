use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use crate::core::addon_layout::discover_addon_roots_from_entry_segments;
use crate::core::archive_io::copy_reader_to_path;
use crate::core::archive_path::{platform_path_collision_key, safe_zip_segments};
use crate::core::error::{AppError, AppResult};
use crate::core::install::HostPlatform;

use super::PreparedAddonDirectory;
use super::inspect::inspect_staged_addon;

pub(super) fn extract_archive_addons(
    archive_path: &Path,
    stage_root: &Path,
    target_platform: HostPlatform,
) -> AppResult<Vec<PreparedAddonDirectory>> {
    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;
    let layout = discover_archive_addon_layout(&mut archive, target_platform)?;
    if layout.addon_roots.is_empty() {
        return Ok(Vec::new());
    }

    let mut file_counts = vec![0usize; layout.addon_roots.len()];

    for planned_entry in &layout.planned_entries {
        let mut entry = archive.by_index(planned_entry.archive_index)?;
        let destination = stage_root.join(&planned_entry.destination_relative);
        copy_reader_to_path(&mut entry, &destination)?;
        file_counts[planned_entry.root_index] += 1;
    }

    let mut prepared = Vec::new();
    for (index, root) in layout.addon_roots.iter().enumerate() {
        let addon_name = addon_name_for_root(root)?;
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

#[derive(Debug)]
struct DiscoveredArchiveLayout {
    addon_roots: Vec<Vec<String>>,
    planned_entries: Vec<PlannedArchiveEntry>,
}

#[derive(Debug)]
struct DiscoveredArchiveFile {
    archive_index: usize,
    archive_name: String,
    segments: Vec<String>,
}

#[derive(Debug)]
struct PlannedArchiveEntry {
    archive_index: usize,
    archive_name: String,
    destination_relative: PathBuf,
    root_index: usize,
}

fn discover_archive_addon_layout(
    archive: &mut ZipArchive<File>,
    target_platform: HostPlatform,
) -> AppResult<DiscoveredArchiveLayout> {
    let mut archive_files = Vec::new();

    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let entry_name = entry.name().to_string();
        reject_unsupported_archive_symlink_entry(&entry_name, entry.is_symlink())?;
        if entry.is_dir() {
            continue;
        }

        let segments = safe_zip_segments(&entry_name)?
            .into_iter()
            .map(|segment| segment.to_string())
            .collect::<Vec<_>>();
        if segments.is_empty() {
            continue;
        }

        archive_files.push(DiscoveredArchiveFile {
            archive_index: index,
            archive_name: entry_name,
            segments,
        });
    }

    let addon_roots = discover_addon_roots_from_entry_segments(
        archive_files.iter().map(|entry| entry.segments.as_slice()),
    );
    let planned_entries = plan_archive_entries(&archive_files, &addon_roots)?;
    validate_planned_destination_collisions(&addon_roots, &planned_entries, target_platform)?;

    Ok(DiscoveredArchiveLayout {
        addon_roots,
        planned_entries,
    })
}

fn plan_archive_entries(
    archive_files: &[DiscoveredArchiveFile],
    addon_roots: &[Vec<String>],
) -> AppResult<Vec<PlannedArchiveEntry>> {
    let mut planned_entries = Vec::new();

    for entry in archive_files {
        let Some(root_index) = match_addon_root(&entry.segments, addon_roots) else {
            continue;
        };
        let root = &addon_roots[root_index];
        let addon_name = addon_name_for_root(root)?;
        let relative = &entry.segments[root.len()..];

        let mut destination_relative = PathBuf::from(addon_name);
        for segment in relative {
            destination_relative.push(segment);
        }

        planned_entries.push(PlannedArchiveEntry {
            archive_index: entry.archive_index,
            archive_name: entry.archive_name.clone(),
            destination_relative,
            root_index,
        });
    }

    Ok(planned_entries)
}

fn validate_planned_destination_collisions(
    addon_roots: &[Vec<String>],
    planned_entries: &[PlannedArchiveEntry],
    target_platform: HostPlatform,
) -> AppResult<()> {
    let mut seen_addon_roots = BTreeMap::<String, &Vec<String>>::new();

    for root in addon_roots {
        let addon_name = addon_name_for_root(root)?;
        let destination = Path::new(addon_name);
        let key = platform_path_collision_key(destination, target_platform);
        let Some(previous) = seen_addon_roots.insert(key, root) else {
            continue;
        };

        let previous_addon_name = addon_name_for_root(previous)?;
        if previous_addon_name == addon_name {
            return Err(AppError::Validation(format!(
                "addon archive contains multiple addon roots that map to the same target directory: `{}` and `{}` -> {}",
                format_archive_root(previous),
                format_archive_root(root),
                destination.display()
            )));
        }

        return Err(AppError::Validation(format!(
            "addon archive contains case-insensitive addon directory collisions: `{}` -> {} and `{}` -> {} would map to the same path on Windows/default macOS targets",
            format_archive_root(previous),
            previous_addon_name,
            format_archive_root(root),
            addon_name
        )));
    }

    let mut seen_entries = BTreeMap::<String, &PlannedArchiveEntry>::new();
    for entry in planned_entries {
        let key = platform_path_collision_key(&entry.destination_relative, target_platform);
        let Some(previous) = seen_entries.insert(key, entry) else {
            continue;
        };

        if previous.destination_relative == entry.destination_relative {
            return Err(AppError::Validation(format!(
                "addon archive maps multiple entries onto the same target path: `{}` and `{}` -> {}",
                previous.archive_name,
                entry.archive_name,
                entry.destination_relative.display()
            )));
        }

        return Err(AppError::Validation(format!(
            "addon archive contains case-insensitive target path collisions: `{}` -> {} and `{}` -> {} would map to the same path on Windows/default macOS targets",
            previous.archive_name,
            previous.destination_relative.display(),
            entry.archive_name,
            entry.destination_relative.display()
        )));
    }

    Ok(())
}

fn match_addon_root(segments: &[String], roots: &[Vec<String>]) -> Option<usize> {
    roots.iter().position(|root| {
        root.len() <= segments.len()
            && root
                .iter()
                .zip(segments.iter())
                .all(|(left, right)| left == right)
    })
}

fn addon_name_for_root(root: &[String]) -> AppResult<&str> {
    root.last()
        .map(String::as_str)
        .ok_or_else(|| AppError::Validation("invalid addon root".to_string()))
}

fn format_archive_root(root: &[String]) -> String {
    root.join("/")
}

fn reject_unsupported_archive_symlink_entry(entry_name: &str, is_symlink: bool) -> AppResult<()> {
    if is_symlink {
        return Err(AppError::Validation(format!(
            "addon archive entry uses unsupported symlink metadata: {entry_name}"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{PlannedArchiveEntry, validate_planned_destination_collisions};
    use crate::core::install::HostPlatform;

    #[test]
    fn validate_planned_destination_collisions_rejects_case_insensitive_addon_roots_on_macos() {
        let error = validate_planned_destination_collisions(
            &[vec!["WeakAuras".to_string()], vec!["weakauras".to_string()]],
            &[],
            HostPlatform::MacOs,
        )
        .expect_err("case-insensitive addon roots should fail on macOS");

        assert!(
            error
                .to_string()
                .contains("case-insensitive addon directory collisions")
        );
    }

    #[test]
    fn validate_planned_destination_collisions_allows_case_distinct_addon_roots_on_linux() {
        validate_planned_destination_collisions(
            &[vec!["WeakAuras".to_string()], vec!["weakauras".to_string()]],
            &[],
            HostPlatform::Linux,
        )
        .expect("linux should allow case-distinct addon roots");
    }

    #[test]
    fn validate_planned_destination_collisions_rejects_case_insensitive_files_on_windows() {
        let error = validate_planned_destination_collisions(
            &[vec!["WeakAuras".to_string()]],
            &[
                PlannedArchiveEntry {
                    archive_index: 0,
                    archive_name: "WeakAuras/Core.lua".to_string(),
                    destination_relative: PathBuf::from("WeakAuras/Core.lua"),
                    root_index: 0,
                },
                PlannedArchiveEntry {
                    archive_index: 1,
                    archive_name: "WeakAuras/core.lua".to_string(),
                    destination_relative: PathBuf::from("WeakAuras/core.lua"),
                    root_index: 0,
                },
            ],
            HostPlatform::Windows,
        )
        .expect_err("case-insensitive file collision should fail on Windows");

        assert!(
            error
                .to_string()
                .contains("case-insensitive target path collisions")
        );
    }

    #[test]
    fn validate_planned_destination_collisions_allows_case_distinct_files_on_linux() {
        validate_planned_destination_collisions(
            &[vec!["WeakAuras".to_string()]],
            &[
                PlannedArchiveEntry {
                    archive_index: 0,
                    archive_name: "WeakAuras/Core.lua".to_string(),
                    destination_relative: PathBuf::from("WeakAuras/Core.lua"),
                    root_index: 0,
                },
                PlannedArchiveEntry {
                    archive_index: 1,
                    archive_name: "WeakAuras/core.lua".to_string(),
                    destination_relative: PathBuf::from("WeakAuras/core.lua"),
                    root_index: 0,
                },
            ],
            HostPlatform::Linux,
        )
        .expect("linux should allow case-distinct file paths");
    }
}
