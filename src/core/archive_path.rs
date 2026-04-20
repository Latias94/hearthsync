use std::collections::BTreeMap;
use std::path::Component;
use std::path::{Path, PathBuf};

use crate::core::error::{AppError, AppResult};
use crate::core::install::HostPlatform;

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

fn is_safe_archive_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && !segment.ends_with([' ', '.'])
        && !segment.chars().any(|char| {
            char.is_control() || matches!(char, '<' | '>' | ':' | '"' | '|' | '?' | '*' | '\\')
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

#[derive(Debug, Clone, Copy)]
pub(in crate::core) struct PlatformPathCollision<'a, T> {
    pub previous: &'a T,
    pub current: &'a T,
    pub kind: PlatformPathCollisionKind,
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
        PlatformPathCollisionKind, find_platform_path_collision, platform_path_collision_key,
        safe_relative_segments, safe_zip_segments,
    };
    use crate::core::install::HostPlatform;

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
}
