use std::collections::BTreeMap;
use std::path::Component;
use std::path::{Path, PathBuf};

use crate::core::error::{AppError, AppResult};
use crate::core::install::HostPlatform;

pub(in crate::core) fn validate_portable_path_segment(
    segment: &str,
    segment_kind: &str,
) -> AppResult<()> {
    if !is_safe_archive_segment(segment) {
        return Err(AppError::Validation(format!(
            "invalid {segment_kind} name: `{segment}`"
        )));
    }

    Ok(())
}

pub(in crate::core) fn safe_zip_segments(archive_name: &str) -> AppResult<Vec<&str>> {
    let mut segments = Vec::new();
    for segment in archive_name.split('/') {
        if !is_safe_archive_segment(segment) {
            return Err(AppError::Validation(format!(
                "unsafe archive path: `{archive_name}`"
            )));
        }

        segments.push(segment);
    }

    Ok(segments)
}

pub(in crate::core) fn safe_zip_segments_under<'a>(
    path: &'a str,
    root_segment: &str,
    path_kind: &str,
) -> AppResult<Vec<&'a str>> {
    let segments = safe_zip_segments(path)
        .map_err(|_| AppError::Validation(format!("unsafe {path_kind}: `{path}`")))?;

    if segments.first().copied() != Some(root_segment) || segments.len() < 2 {
        return Err(AppError::Validation(format!(
            "{path_kind} must be under `{root_segment}/`: {path}"
        )));
    }

    Ok(segments)
}

fn is_safe_archive_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && !segment.ends_with([' ', '.'])
        && !segment.chars().any(|char| {
            char.is_control()
                || matches!(char, '<' | '>' | ':' | '"' | '|' | '?' | '*' | '/' | '\\')
        })
        && !is_windows_reserved_device_name(segment)
}

fn is_windows_reserved_device_name(segment: &str) -> bool {
    let stem = segment
        .split('.')
        .next()
        .unwrap_or(segment)
        .trim_end_matches([' ', '.']);
    let upper = stem.to_ascii_uppercase();

    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || is_numbered_windows_device_name(&upper, "COM")
        || is_numbered_windows_device_name(&upper, "LPT")
}

fn is_numbered_windows_device_name(value: &str, prefix: &str) -> bool {
    let Some(suffix) = value.strip_prefix(prefix) else {
        return false;
    };

    suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'1'..=b'9')
}

pub(in crate::core) fn join_segments(root: &Path, segments: &[&str]) -> PathBuf {
    let mut path = root.to_path_buf();
    for segment in segments {
        path.push(segment);
    }
    path
}

pub(in crate::core) fn to_zip_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

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

pub(in crate::core) fn safe_relative_segments(
    relative_path: &Path,
    path_kind: &str,
) -> AppResult<Vec<String>> {
    let mut segments = Vec::new();

    for component in relative_path.components() {
        let Component::Normal(segment) = component else {
            return Err(AppError::Validation(format!(
                "unsafe {path_kind}: {}",
                relative_path.display()
            )));
        };

        let segment = segment.to_string_lossy().to_string();
        if !is_safe_archive_segment(&segment) {
            return Err(AppError::Validation(format!(
                "unsafe {path_kind}: {}",
                relative_path.display()
            )));
        }

        segments.push(segment);
    }

    Ok(segments)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{
        PlatformPathCollisionKind, PlatformPathPrefixConflictKind, find_platform_path_collision,
        find_platform_path_prefix_conflict, platform_path_collision_key, safe_relative_segments,
        safe_zip_segments, safe_zip_segments_under, to_zip_path, validate_portable_path_segment,
    };
    use crate::core::install::HostPlatform;

    #[test]
    fn validate_portable_path_segment_rejects_reserved_device_names() {
        let error = validate_portable_path_segment("CON", "account")
            .expect_err("reserved device name should fail");

        assert!(error.to_string().contains("invalid account name"));
    }

    #[test]
    fn validate_portable_path_segment_rejects_trailing_space_or_dot() {
        for segment in ["WeakAuras ", "WeakAuras."] {
            let error = validate_portable_path_segment(segment, "addon")
                .expect_err("trailing space or dot should fail");

            assert!(error.to_string().contains("invalid addon name"));
        }
    }

    #[test]
    fn validate_portable_path_segment_rejects_path_separators() {
        for segment in ["Interface/Buttons", r"Interface\Buttons"] {
            let error = validate_portable_path_segment(segment, "interface asset")
                .expect_err("path separators should fail");

            assert!(error.to_string().contains("invalid interface asset name"));
        }
    }

    #[test]
    fn validate_portable_path_segment_accepts_portable_names() {
        validate_portable_path_segment("ACC#1", "account").expect("portable plain segment");
    }

    #[test]
    fn safe_zip_segments_rejects_empty_segments() {
        let error =
            safe_zip_segments("addons//WeakAuras/WeakAuras.toc").expect_err("empty segment");

        assert!(error.to_string().contains("unsafe archive path"));
    }

    #[test]
    fn safe_zip_segments_rejects_windows_reserved_characters() {
        let error = safe_zip_segments("addons/Weak:Auras/WeakAuras.toc")
            .expect_err("colon should be rejected");

        assert!(error.to_string().contains("unsafe archive path"));
    }

    #[test]
    fn safe_zip_segments_rejects_windows_device_names() {
        for archive_name in [
            "addons/CON/Config.lua",
            "addons/aux.lua",
            "addons/com1.txt",
            "addons/LPT9",
        ] {
            let error = safe_zip_segments(archive_name).expect_err("device name should fail");
            assert!(error.to_string().contains("unsafe archive path"));
        }
    }

    #[test]
    fn safe_zip_segments_rejects_trailing_space_or_dot() {
        for archive_name in ["addons/WeakAuras /Core.lua", "addons/WeakAuras./Core.lua"] {
            let error =
                safe_zip_segments(archive_name).expect_err("trailing space or dot should fail");
            assert!(error.to_string().contains("unsafe archive path"));
        }
    }

    #[test]
    fn safe_zip_segments_accepts_portable_names() {
        assert_eq!(
            safe_zip_segments("wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua")
                .expect("portable path"),
            vec![
                "wtf",
                "common",
                "accounts",
                "ACCOUNT",
                "SavedVariables",
                "Details.lua"
            ]
        );
    }

    #[test]
    fn safe_zip_segments_under_rejects_non_portable_paths_with_context() {
        let error = safe_zip_segments_under("sources/CON.zip", "sources", "addon lock source path")
            .expect_err("non-portable path should fail");

        assert!(error.to_string().contains("unsafe addon lock source path"));
    }

    #[test]
    fn safe_zip_segments_under_requires_root_prefix() {
        let error = safe_zip_segments_under(
            "archives/providers/WeakAuras.zip",
            "sources",
            "bundle addon source path",
        )
        .expect_err("wrong root should fail");

        assert!(
            error
                .to_string()
                .contains("bundle addon source path must be under `sources/`")
        );
    }

    #[test]
    fn safe_zip_segments_under_accepts_portable_rooted_paths() {
        assert_eq!(
            safe_zip_segments_under(
                "sources/providers/curseforge/WeakAuras.zip",
                "sources",
                "bundle addon source path",
            )
            .expect("portable rooted path"),
            vec!["sources", "providers", "curseforge", "WeakAuras.zip"]
        );
    }

    #[test]
    fn safe_relative_segments_rejects_non_normal_components() {
        let error = safe_relative_segments(
            Path::new("./AuthorUI/Interface/AddOns/WeakAuras"),
            "directory entry path",
        )
        .expect_err("relative path should reject current-directory components");

        assert!(error.to_string().contains("unsafe directory entry path"));
    }

    #[test]
    fn safe_relative_segments_rejects_windows_reserved_segments() {
        for relative_path in [
            Path::new("AuthorUI/Interface/AddOns/Weak:Auras"),
            Path::new("AuthorUI/Fonts/CON.ttf"),
            Path::new("AuthorUI/Fonts/FRIZQT__.ttf "),
        ] {
            let error = safe_relative_segments(relative_path, "directory entry path")
                .expect_err("relative path should reject non-portable segments");
            assert!(error.to_string().contains("unsafe directory entry path"));
        }
    }

    #[test]
    fn safe_relative_segments_accepts_portable_names() {
        assert_eq!(
            safe_relative_segments(
                Path::new("AuthorUI/WTF/Account/SavedVariables/Details.lua"),
                "directory entry path",
            )
            .expect("portable relative path"),
            vec![
                "AuthorUI".to_string(),
                "WTF".to_string(),
                "Account".to_string(),
                "SavedVariables".to_string(),
                "Details.lua".to_string()
            ]
        );
    }

    #[test]
    fn to_zip_path_normalizes_windows_separators() {
        assert_eq!(
            to_zip_path(Path::new(r"Interface\AddOns\WeakAuras\WeakAuras.toc")),
            "Interface/AddOns/WeakAuras/WeakAuras.toc"
        );
    }

    #[test]
    fn platform_path_collision_key_folds_windows_and_macos_paths() {
        assert_eq!(
            platform_path_collision_key(
                Path::new("Interface/AddOns/WeakAuras"),
                HostPlatform::Windows,
            ),
            "interface/addons/weakauras"
        );
        assert_eq!(
            platform_path_collision_key(
                Path::new("Interface/AddOns/WeakAuras"),
                HostPlatform::MacOs
            ),
            "interface/addons/weakauras"
        );
    }

    #[test]
    fn platform_path_collision_key_preserves_linux_case() {
        assert_ne!(
            platform_path_collision_key(
                Path::new("Interface/AddOns/WeakAuras"),
                HostPlatform::Linux,
            ),
            platform_path_collision_key(
                Path::new("interface/addons/weakauras"),
                HostPlatform::Linux,
            )
        );
    }

    #[test]
    fn find_platform_path_collision_reports_exact_duplicates() {
        let paths = [
            PathBuf::from("Interface/AddOns/WeakAuras"),
            PathBuf::from("Interface/AddOns/WeakAuras"),
        ];

        let collision =
            find_platform_path_collision(paths.iter(), HostPlatform::Windows, PathBuf::as_path)
                .expect("duplicate paths should collide");

        assert_eq!(collision.kind, PlatformPathCollisionKind::Exact);
        assert_eq!(
            collision.previous,
            &PathBuf::from("Interface/AddOns/WeakAuras")
        );
        assert_eq!(
            collision.current,
            &PathBuf::from("Interface/AddOns/WeakAuras")
        );
    }

    #[test]
    fn find_platform_path_collision_reports_case_insensitive_duplicates() {
        let paths = [
            PathBuf::from("Interface/AddOns/WeakAuras"),
            PathBuf::from("interface/addons/weakauras"),
        ];

        let collision =
            find_platform_path_collision(paths.iter(), HostPlatform::MacOs, PathBuf::as_path)
                .expect("case-distinct macOS paths should collide");

        assert_eq!(collision.kind, PlatformPathCollisionKind::CaseInsensitive);
        assert_eq!(
            collision.previous,
            &PathBuf::from("Interface/AddOns/WeakAuras")
        );
        assert_eq!(
            collision.current,
            &PathBuf::from("interface/addons/weakauras")
        );
    }

    #[test]
    fn find_platform_path_collision_preserves_linux_case_sensitivity() {
        let paths = [
            PathBuf::from("Interface/AddOns/WeakAuras"),
            PathBuf::from("interface/addons/weakauras"),
        ];

        assert!(
            find_platform_path_collision(paths.iter(), HostPlatform::Linux, PathBuf::as_path)
                .is_none()
        );
    }

    #[test]
    fn find_platform_path_prefix_conflict_reports_exact_ancestor_conflicts() {
        let paths = [
            PathBuf::from("Interface/AddOns/WeakAuras"),
            PathBuf::from("Interface/AddOns/WeakAuras/Config.lua"),
        ];

        let conflict = find_platform_path_prefix_conflict(
            paths.iter(),
            HostPlatform::Windows,
            PathBuf::as_path,
        )
        .expect("prefix conflict");

        assert_eq!(conflict.kind, PlatformPathPrefixConflictKind::Exact);
        assert_eq!(
            conflict.ancestor,
            &PathBuf::from("Interface/AddOns/WeakAuras")
        );
        assert_eq!(
            conflict.descendant,
            &PathBuf::from("Interface/AddOns/WeakAuras/Config.lua")
        );
    }

    #[test]
    fn find_platform_path_prefix_conflict_reports_case_insensitive_conflicts() {
        let paths = [
            PathBuf::from("Interface/AddOns/WeakAuras"),
            PathBuf::from("Interface/AddOns/weakauras/Config.lua"),
        ];

        let conflict =
            find_platform_path_prefix_conflict(paths.iter(), HostPlatform::MacOs, PathBuf::as_path)
                .expect("case-insensitive prefix conflict");

        assert_eq!(
            conflict.kind,
            PlatformPathPrefixConflictKind::CaseInsensitive
        );
        assert_eq!(
            conflict.ancestor,
            &PathBuf::from("Interface/AddOns/WeakAuras")
        );
        assert_eq!(
            conflict.descendant,
            &PathBuf::from("Interface/AddOns/weakauras/Config.lua")
        );
    }

    #[test]
    fn find_platform_path_prefix_conflict_preserves_linux_case_sensitivity() {
        let paths = [
            PathBuf::from("Interface/AddOns/WeakAuras"),
            PathBuf::from("Interface/AddOns/weakauras/Config.lua"),
        ];

        assert!(
            find_platform_path_prefix_conflict(paths.iter(), HostPlatform::Linux, PathBuf::as_path)
                .is_none()
        );
    }
}
