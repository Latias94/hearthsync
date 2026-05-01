use std::collections::BTreeMap;
use std::path::Path;

use crate::core::install::HostPlatform;

pub(in crate::core) fn platform_path_collision_key(path: &Path, platform: HostPlatform) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    match platform {
        HostPlatform::Windows | HostPlatform::MacOs => normalized.to_lowercase(),
        HostPlatform::Linux | HostPlatform::Unknown => normalized,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::core) enum PlatformPathCollisionKind {
    Exact,
    CaseInsensitive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::core) enum PlatformPathPrefixConflictKind {
    Exact,
    CaseInsensitive,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::core) struct PlatformPathCollision<'a, T> {
    pub previous: &'a T,
    pub current: &'a T,
    pub kind: PlatformPathCollisionKind,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::core) struct PlatformPathPrefixConflict<'a, T> {
    pub ancestor: &'a T,
    pub descendant: &'a T,
    pub kind: PlatformPathPrefixConflictKind,
}

pub(in crate::core) fn find_platform_path_collision<'a, T, I, F>(
    items: I,
    platform: HostPlatform,
    path_for: F,
) -> Option<PlatformPathCollision<'a, T>>
where
    I: IntoIterator<Item = &'a T>,
    F: Fn(&T) -> &Path,
{
    let mut seen = BTreeMap::<String, &'a T>::new();

    for item in items {
        let path = path_for(item);
        let key = platform_path_collision_key(path, platform);
        let Some(previous) = seen.insert(key, item) else {
            continue;
        };

        let kind = if path_for(previous) == path {
            PlatformPathCollisionKind::Exact
        } else {
            PlatformPathCollisionKind::CaseInsensitive
        };
        return Some(PlatformPathCollision {
            previous,
            current: item,
            kind,
        });
    }

    None
}

pub(in crate::core) fn find_platform_path_prefix_conflict<'a, T, I, F>(
    items: I,
    platform: HostPlatform,
    path_for: F,
) -> Option<PlatformPathPrefixConflict<'a, T>>
where
    I: IntoIterator<Item = &'a T>,
    F: Fn(&T) -> &Path,
{
    let mut seen_paths = BTreeMap::<String, &'a T>::new();
    let mut descendants_by_ancestor = BTreeMap::<String, &'a T>::new();

    for item in items {
        let path = path_for(item);

        for ancestor in proper_ancestors(path) {
            let ancestor_key = platform_path_collision_key(ancestor, platform);
            let Some(ancestor_item) = seen_paths.get(&ancestor_key) else {
                continue;
            };

            let kind = if path.starts_with(path_for(ancestor_item)) {
                PlatformPathPrefixConflictKind::Exact
            } else {
                PlatformPathPrefixConflictKind::CaseInsensitive
            };
            return Some(PlatformPathPrefixConflict {
                ancestor: ancestor_item,
                descendant: item,
                kind,
            });
        }

        let key = platform_path_collision_key(path, platform);
        if let Some(descendant_item) = descendants_by_ancestor.get(&key) {
            let kind = if path_for(descendant_item).starts_with(path) {
                PlatformPathPrefixConflictKind::Exact
            } else {
                PlatformPathPrefixConflictKind::CaseInsensitive
            };
            return Some(PlatformPathPrefixConflict {
                ancestor: item,
                descendant: descendant_item,
                kind,
            });
        }

        seen_paths.insert(key, item);
        for ancestor in proper_ancestors(path) {
            let ancestor_key = platform_path_collision_key(ancestor, platform);
            descendants_by_ancestor.entry(ancestor_key).or_insert(item);
        }
    }

    None
}

fn proper_ancestors(path: &Path) -> impl Iterator<Item = &Path> {
    path.ancestors()
        .skip(1)
        .take_while(|ancestor| !ancestor.as_os_str().is_empty())
}
