use std::collections::BTreeSet;
use std::path::Path;

use crate::core::install::HostPlatform;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AddonRootPrefixMatchKind {
    Exact,
    CaseInsensitive,
}

pub(crate) fn discover_addon_roots_from_entry_segments<'a>(
    entry_segments: impl IntoIterator<Item = &'a [String]>,
    platform: HostPlatform,
) -> Vec<Vec<String>> {
    let mut candidates = BTreeSet::new();

    for segments in entry_segments {
        if segments.len() < 2 {
            continue;
        }

        let Some(file_name) = segments.last().map(String::as_str) else {
            continue;
        };
        if !is_toc_file_name(file_name) {
            continue;
        }

        candidates.insert(segments[..segments.len() - 1].to_vec());
    }

    let mut ordered = candidates.into_iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.len().cmp(&right.len()).then_with(|| left.cmp(right)));

    let mut roots: Vec<Vec<String>> = Vec::new();
    for candidate in ordered {
        if roots
            .iter()
            .any(|root| has_proper_platform_prefix(&candidate, root.as_slice(), platform))
        {
            continue;
        }

        roots.push(candidate);
    }

    roots.sort_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
    roots
}

fn is_toc_file_name(file_name: &str) -> bool {
    Path::new(file_name)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("toc"))
}

pub(crate) fn addon_root_prefix_match_kind(
    path: &[String],
    prefix: &[String],
    platform: HostPlatform,
) -> Option<AddonRootPrefixMatchKind> {
    if !path_has_exact_prefix(path, prefix) {
        return if matches!(platform, HostPlatform::Windows | HostPlatform::MacOs)
            && path_has_case_insensitive_prefix(path, prefix)
        {
            Some(AddonRootPrefixMatchKind::CaseInsensitive)
        } else {
            None
        };
    }

    Some(AddonRootPrefixMatchKind::Exact)
}

fn has_proper_platform_prefix(path: &[String], prefix: &[String], platform: HostPlatform) -> bool {
    prefix.len() < path.len() && addon_root_prefix_match_kind(path, prefix, platform).is_some()
}

fn path_has_exact_prefix(path: &[String], prefix: &[String]) -> bool {
    prefix.len() <= path.len()
        && prefix
            .iter()
            .zip(path.iter())
            .all(|(left, right)| left == right)
}

fn path_has_case_insensitive_prefix(path: &[String], prefix: &[String]) -> bool {
    prefix.len() <= path.len()
        && prefix
            .iter()
            .zip(path.iter())
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
}

#[cfg(test)]
mod tests {
    use super::discover_addon_roots_from_entry_segments;
    use crate::core::install::HostPlatform;

    #[test]
    fn discovers_roots_from_any_toc_file_name() {
        let entries = [
            vec!["DBM-Core".to_string(), "DBM-Core_Mainline.toc".to_string()],
            vec!["DBM-Core".to_string(), "Core.lua".to_string()],
            vec!["WeakAuras".to_string(), "WeakAuras.toc".to_string()],
        ];

        let roots = discover_addon_roots_from_entry_segments(
            entries.iter().map(|segments| segments.as_slice()),
            HostPlatform::Windows,
        );

        assert_eq!(
            roots,
            vec![vec!["DBM-Core".to_string()], vec!["WeakAuras".to_string()],]
        );
    }

    #[test]
    fn ignores_nested_roots_when_an_ancestor_already_has_a_toc() {
        let entries = [
            vec!["Addon".to_string(), "Addon.toc".to_string()],
            vec![
                "Addon".to_string(),
                "Modules".to_string(),
                "Module.toc".to_string(),
            ],
        ];

        let roots = discover_addon_roots_from_entry_segments(
            entries.iter().map(|segments| segments.as_slice()),
            HostPlatform::Windows,
        );

        assert_eq!(roots, vec![vec!["Addon".to_string()]]);
    }

    #[test]
    fn ignores_case_distinct_nested_roots_on_windows_like_platforms() {
        let entries = [
            vec!["Addon".to_string(), "Addon.toc".to_string()],
            vec![
                "addon".to_string(),
                "Modules".to_string(),
                "Module.toc".to_string(),
            ],
        ];

        let roots = discover_addon_roots_from_entry_segments(
            entries.iter().map(|segments| segments.as_slice()),
            HostPlatform::Windows,
        );

        assert_eq!(roots, vec![vec!["Addon".to_string()]]);
    }

    #[test]
    fn preserves_case_distinct_nested_roots_on_linux() {
        let entries = [
            vec!["Addon".to_string(), "Addon.toc".to_string()],
            vec![
                "addon".to_string(),
                "Modules".to_string(),
                "Module.toc".to_string(),
            ],
        ];

        let roots = discover_addon_roots_from_entry_segments(
            entries.iter().map(|segments| segments.as_slice()),
            HostPlatform::Linux,
        );

        assert_eq!(
            roots,
            vec![
                vec!["addon".to_string(), "Modules".to_string()],
                vec!["Addon".to_string()],
            ]
        );
    }

    #[test]
    fn matches_toc_extension_case_insensitively() {
        let entries = [vec!["Questie".to_string(), "Questie.TOC".to_string()]];

        let roots = discover_addon_roots_from_entry_segments(
            entries.iter().map(|segments| segments.as_slice()),
            HostPlatform::Windows,
        );

        assert_eq!(roots, vec![vec!["Questie".to_string()]]);
    }
}
