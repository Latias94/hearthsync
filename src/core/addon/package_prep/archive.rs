use std::fs::File;
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use crate::core::addon_layout::{
    AddonRootPrefixMatchKind, addon_root_prefix_match_kind,
    discover_addon_roots_from_entry_segments,
};
use crate::core::archive_io::{copy_reader_to_path, validated_zip_file_entry_segments};
use crate::core::archive_path::{
    PlatformPathCollisionKind, PlatformPathPrefixConflictKind, find_platform_path_collision,
    find_platform_path_prefix_conflict, platform_path_collision_key,
};
use crate::core::error::{AppError, AppResult};
use crate::core::install::HostPlatform;

use super::PreparedAddonDirectory;
use super::inspect::inspect_addon_directory;

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
        let stage_path = stage_root.join(&root.addon_name);
        let addon = inspect_addon_directory(&stage_path, &root.addon_name)?;
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
    addon_roots: Vec<PlannedAddonRoot>,
    planned_entries: Vec<PlannedArchiveEntry>,
}

#[derive(Debug)]
struct DiscoveredArchiveFile {
    archive_index: usize,
    archive_name: String,
    segments: Vec<String>,
}

#[derive(Debug)]
struct PlannedAddonRoot {
    archive_root: Vec<String>,
    addon_name: String,
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
        let Some(segments) = validated_zip_file_entry_segments(
            "addon archive entry",
            &entry_name,
            entry.is_symlink(),
            entry.is_dir(),
        )?
        else {
            continue;
        };
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
        target_platform,
    )
    .into_iter()
    .map(|archive_root| {
        Ok(PlannedAddonRoot {
            addon_name: addon_name_for_archive_root(&archive_root)?.to_string(),
            archive_root,
        })
    })
    .collect::<AppResult<Vec<_>>>()?;
    let planned_entries = plan_archive_entries(&archive_files, &addon_roots, target_platform)?;
    validate_planned_destination_collisions(&addon_roots, &planned_entries, target_platform)?;

    Ok(DiscoveredArchiveLayout {
        addon_roots,
        planned_entries,
    })
}

fn plan_archive_entries(
    archive_files: &[DiscoveredArchiveFile],
    addon_roots: &[PlannedAddonRoot],
    target_platform: HostPlatform,
) -> AppResult<Vec<PlannedArchiveEntry>> {
    let mut planned_entries = Vec::new();

    for entry in archive_files {
        let Some(root_index) = match_addon_root(&entry.segments, addon_roots, target_platform)
        else {
            continue;
        };
        let root = &addon_roots[root_index];
        let relative = &entry.segments[root.archive_root.len()..];

        let mut destination_relative = PathBuf::from(&root.addon_name);
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
    addon_roots: &[PlannedAddonRoot],
    planned_entries: &[PlannedArchiveEntry],
    target_platform: HostPlatform,
) -> AppResult<()> {
    if let Some(collision) =
        find_platform_path_collision(addon_roots.iter(), target_platform, |root| {
            Path::new(&root.addon_name)
        })
    {
        return match collision.kind {
            PlatformPathCollisionKind::Exact => Err(AppError::Validation(format!(
                "addon archive contains multiple addon roots that map to the same target directory: `{}` and `{}` -> {}",
                format_archive_root(&collision.previous.archive_root),
                format_archive_root(&collision.current.archive_root),
                collision.current.addon_name
            ))),
            PlatformPathCollisionKind::CaseInsensitive => Err(AppError::Validation(format!(
                "addon archive contains case-insensitive addon directory collisions: `{}` -> {} and `{}` -> {} would map to the same path on Windows/default macOS targets",
                format_archive_root(&collision.previous.archive_root),
                collision.previous.addon_name,
                format_archive_root(&collision.current.archive_root),
                collision.current.addon_name
            ))),
        };
    }

    if let Some(conflict) =
        find_addon_root_file_conflict(addon_roots, planned_entries, target_platform)
    {
        return match conflict.kind {
            PlatformPathCollisionKind::Exact => Err(AppError::Validation(format!(
                "addon archive contains conflicting addon directory and file targets: `{}` -> {} and `{}` -> {}",
                format_archive_root(&conflict.root.archive_root),
                conflict.root.addon_name,
                conflict.entry.archive_name,
                conflict.entry.destination_relative.display()
            ))),
            PlatformPathCollisionKind::CaseInsensitive => Err(AppError::Validation(format!(
                "addon archive contains case-insensitive addon directory and file target conflicts: `{}` -> {} and `{}` -> {} would map to the same path on Windows/default macOS targets",
                format_archive_root(&conflict.root.archive_root),
                conflict.root.addon_name,
                conflict.entry.archive_name,
                conflict.entry.destination_relative.display()
            ))),
        };
    }

    if let Some(collision) =
        find_platform_path_collision(planned_entries.iter(), target_platform, |entry| {
            entry.destination_relative.as_path()
        })
    {
        return match collision.kind {
            PlatformPathCollisionKind::Exact => Err(AppError::Validation(format!(
                "addon archive maps multiple entries onto the same target path: `{}` and `{}` -> {}",
                collision.previous.archive_name,
                collision.current.archive_name,
                collision.current.destination_relative.display()
            ))),
            PlatformPathCollisionKind::CaseInsensitive => Err(AppError::Validation(format!(
                "addon archive contains case-insensitive target path collisions: `{}` -> {} and `{}` -> {} would map to the same path on Windows/default macOS targets",
                collision.previous.archive_name,
                collision.previous.destination_relative.display(),
                collision.current.archive_name,
                collision.current.destination_relative.display()
            ))),
        };
    }

    let Some(conflict) =
        find_platform_path_prefix_conflict(planned_entries.iter(), target_platform, |entry| {
            entry.destination_relative.as_path()
        })
    else {
        return Ok(());
    };

    match conflict.kind {
        PlatformPathPrefixConflictKind::Exact => Err(AppError::Validation(format!(
            "addon archive contains conflicting file and directory target paths: `{}` -> {} and `{}` -> {}",
            conflict.ancestor.archive_name,
            conflict.ancestor.destination_relative.display(),
            conflict.descendant.archive_name,
            conflict.descendant.destination_relative.display()
        ))),
        PlatformPathPrefixConflictKind::CaseInsensitive => Err(AppError::Validation(format!(
            "addon archive contains case-insensitive file and directory target path conflicts: `{}` -> {} and `{}` -> {} would create file/directory collisions on Windows/default macOS targets",
            conflict.ancestor.archive_name,
            conflict.ancestor.destination_relative.display(),
            conflict.descendant.archive_name,
            conflict.descendant.destination_relative.display()
        ))),
    }
}

#[derive(Debug, Clone, Copy)]
struct AddonRootFileConflict<'a> {
    root: &'a PlannedAddonRoot,
    entry: &'a PlannedArchiveEntry,
    kind: PlatformPathCollisionKind,
}

fn find_addon_root_file_conflict<'a>(
    addon_roots: &'a [PlannedAddonRoot],
    planned_entries: &'a [PlannedArchiveEntry],
    target_platform: HostPlatform,
) -> Option<AddonRootFileConflict<'a>> {
    addon_roots.iter().find_map(|root| {
        let root_path = Path::new(&root.addon_name);
        let root_key = platform_path_collision_key(root_path, target_platform);

        planned_entries.iter().find_map(|entry| {
            if platform_path_collision_key(entry.destination_relative.as_path(), target_platform)
                != root_key
            {
                return None;
            }

            let kind = if entry.destination_relative.as_path() == root_path {
                PlatformPathCollisionKind::Exact
            } else {
                PlatformPathCollisionKind::CaseInsensitive
            };
            Some(AddonRootFileConflict { root, entry, kind })
        })
    })
}

fn match_addon_root(
    segments: &[String],
    roots: &[PlannedAddonRoot],
    target_platform: HostPlatform,
) -> Option<usize> {
    let mut case_insensitive_match = None;

    for (index, root) in roots.iter().enumerate() {
        match addon_root_prefix_match_kind(segments, &root.archive_root, target_platform) {
            Some(AddonRootPrefixMatchKind::Exact) => return Some(index),
            Some(AddonRootPrefixMatchKind::CaseInsensitive) if case_insensitive_match.is_none() => {
                case_insensitive_match = Some(index);
            }
            Some(AddonRootPrefixMatchKind::CaseInsensitive) => {}
            None => {}
        }
    }

    case_insensitive_match
}

fn addon_name_for_archive_root(root: &[String]) -> AppResult<&str> {
    root.last()
        .map(String::as_str)
        .ok_or_else(|| AppError::Validation("invalid addon root".to_string()))
}

fn format_archive_root(root: &[String]) -> String {
    root.join("/")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{PlannedAddonRoot, PlannedArchiveEntry, validate_planned_destination_collisions};
    use crate::core::install::HostPlatform;

    #[test]
    fn validate_planned_destination_collisions_rejects_case_insensitive_addon_roots_on_macos() {
        let error = validate_planned_destination_collisions(
            &[planned_root("WeakAuras"), planned_root("weakauras")],
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
            &[planned_root("WeakAuras"), planned_root("weakauras")],
            &[],
            HostPlatform::Linux,
        )
        .expect("linux should allow case-distinct addon roots");
    }

    #[test]
    fn validate_planned_destination_collisions_rejects_case_insensitive_files_on_windows() {
        let error = validate_planned_destination_collisions(
            &[planned_root("WeakAuras")],
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
            &[planned_root("WeakAuras")],
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

    #[test]
    fn validate_planned_destination_collisions_rejects_addon_root_file_conflicts() {
        let error = validate_planned_destination_collisions(
            &[planned_root("WeakAuras")],
            &[PlannedArchiveEntry {
                archive_index: 0,
                archive_name: "WeakAuras".to_string(),
                destination_relative: PathBuf::from("WeakAuras"),
                root_index: 0,
            }],
            HostPlatform::Windows,
        )
        .expect_err("file target should not collide with addon root directory");

        assert!(
            error
                .to_string()
                .contains("conflicting addon directory and file targets")
        );
    }

    #[test]
    fn validate_planned_destination_collisions_rejects_case_insensitive_prefix_conflicts() {
        let error = validate_planned_destination_collisions(
            &[],
            &[
                PlannedArchiveEntry {
                    archive_index: 0,
                    archive_name: "WeakAuras".to_string(),
                    destination_relative: PathBuf::from("WeakAuras"),
                    root_index: 0,
                },
                PlannedArchiveEntry {
                    archive_index: 1,
                    archive_name: "weakauras/Config.lua".to_string(),
                    destination_relative: PathBuf::from("weakauras/Config.lua"),
                    root_index: 0,
                },
            ],
            HostPlatform::MacOs,
        )
        .expect_err("case-insensitive file/directory hierarchy should fail");

        assert!(
            error
                .to_string()
                .contains("case-insensitive file and directory target path conflicts")
        );
    }

    fn planned_root(addon_name: &str) -> PlannedAddonRoot {
        PlannedAddonRoot {
            archive_root: vec![addon_name.to_string()],
            addon_name: addon_name.to_string(),
        }
    }
}
