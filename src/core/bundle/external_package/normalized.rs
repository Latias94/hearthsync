use std::path::Path;

use std::collections::BTreeMap;

use super::types::{ExternalPackageAnalysis, ExternalPackageEntry};
use crate::core::archive_path::{
    PlatformPathPrefixConflictKind, find_platform_path_collision,
    find_platform_path_prefix_conflict,
};
use crate::core::error::{AppError, AppResult};
use crate::core::install::HostPlatform;

pub(super) fn validate_unique_normalized_paths(
    analysis: &ExternalPackageAnalysis,
) -> AppResult<()> {
    let unique_entries = collect_unique_normalized_entries(analysis)?;
    if let Some(collision) = find_platform_path_collision(
        unique_entries.values().copied(),
        HostPlatform::Windows,
        |entry| Path::new(&entry.normalized_path),
    ) {
        return Err(AppError::Validation(format!(
            "external package contains case-insensitive target path collisions: `{}` and `{}` would map to the same path on Windows/default macOS targets",
            collision.previous.normalized_path, collision.current.normalized_path
        )));
    }

    let Some(conflict) = find_platform_path_prefix_conflict(
        unique_entries.values().copied(),
        HostPlatform::Windows,
        |entry| Path::new(&entry.normalized_path),
    ) else {
        return Ok(());
    };

    match conflict.kind {
        PlatformPathPrefixConflictKind::Exact => Err(AppError::Validation(format!(
            "external package normalizes conflicting file and directory target paths: `{}` and `{}`",
            conflict.ancestor.normalized_path, conflict.descendant.normalized_path
        ))),
        PlatformPathPrefixConflictKind::CaseInsensitive => Err(AppError::Validation(format!(
            "external package contains case-insensitive file and directory target path conflicts: `{}` and `{}` would create file/directory collisions on Windows/default macOS targets",
            conflict.ancestor.normalized_path, conflict.descendant.normalized_path
        ))),
    }
}

pub(super) fn build_external_package_entry_source_map(
    analysis: &ExternalPackageAnalysis,
) -> AppResult<BTreeMap<String, String>> {
    Ok(collect_unique_normalized_entries(analysis)?
        .into_iter()
        .map(|(normalized_path, entry)| (normalized_path, entry.source_path.clone()))
        .collect())
}

fn collect_unique_normalized_entries(
    analysis: &ExternalPackageAnalysis,
) -> AppResult<BTreeMap<String, &ExternalPackageEntry>> {
    let mut unique_entries = BTreeMap::new();
    for entry in &analysis.entries {
        if unique_entries
            .insert(entry.normalized_path.clone(), entry)
            .is_some()
        {
            return Err(AppError::Validation(format!(
                "external package normalizes multiple files onto the same target path: {}",
                entry.normalized_path
            )));
        }
    }

    Ok(unique_entries)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{build_external_package_entry_source_map, validate_unique_normalized_paths};
    use crate::core::bundle::types::apply::ApplyGroup;

    use super::super::types::{
        ExternalPackageAnalysis, ExternalPackageEntry, ExternalPackageSourceKind,
        ExternalPackageSummary,
    };
    use crate::core::manifest::BundleResources;

    #[test]
    fn validate_unique_normalized_paths_rejects_exact_duplicates() {
        let error = validate_unique_normalized_paths(&analysis_with_entries(vec![
            entry("src/A/WeakAuras.toc", "addons/WeakAuras/WeakAuras.toc"),
            entry("src/B/WeakAuras.toc", "addons/WeakAuras/WeakAuras.toc"),
        ]))
        .expect_err("exact duplicate normalized paths should fail");

        assert!(
            error
                .to_string()
                .contains("normalizes multiple files onto the same target path")
        );
    }

    #[test]
    fn validate_unique_normalized_paths_rejects_case_insensitive_duplicates() {
        let error = validate_unique_normalized_paths(&analysis_with_entries(vec![
            entry("src/Fonts/FRIZQT__.ttf", "fonts/FRIZQT__.ttf"),
            entry("src/fonts/frizqt__.ttf", "fonts/frizqt__.ttf"),
        ]))
        .expect_err("case-insensitive normalized paths should fail");

        assert!(
            error
                .to_string()
                .contains("case-insensitive target path collisions")
        );
    }

    #[test]
    fn validate_unique_normalized_paths_rejects_case_insensitive_prefix_conflicts() {
        let error = validate_unique_normalized_paths(&analysis_with_entries(vec![
            entry("src/WeakAuras", "addons/WeakAuras"),
            entry("src/weakauras/Config.lua", "addons/weakauras/Config.lua"),
        ]))
        .expect_err("case-insensitive file/directory paths should fail");

        assert!(
            error
                .to_string()
                .contains("case-insensitive file and directory target path conflicts")
        );
    }

    #[test]
    fn build_external_package_entry_source_map_preserves_unique_sources() {
        let entry_source_map =
            build_external_package_entry_source_map(&analysis_with_entries(vec![
                entry(
                    "src/WeakAuras/WeakAuras.toc",
                    "addons/WeakAuras/WeakAuras.toc",
                ),
                entry("src/Fonts/FRIZQT__.ttf", "fonts/FRIZQT__.ttf"),
            ]))
            .expect("unique normalized paths");

        assert_eq!(
            entry_source_map.get("addons/WeakAuras/WeakAuras.toc"),
            Some(&"src/WeakAuras/WeakAuras.toc".to_string())
        );
        assert_eq!(
            entry_source_map.get("fonts/FRIZQT__.ttf"),
            Some(&"src/Fonts/FRIZQT__.ttf".to_string())
        );
    }

    fn analysis_with_entries(entries: Vec<ExternalPackageEntry>) -> ExternalPackageAnalysis {
        ExternalPackageAnalysis {
            source_path: PathBuf::from("fixture"),
            source_kind: ExternalPackageSourceKind::Directory,
            package_id: "fixture".to_string(),
            package_name: "fixture".to_string(),
            entries,
            resources: BundleResources {
                addons: Vec::new(),
                wtf_common: false,
                wtf_characters: Vec::new(),
                fonts: false,
                interface_assets: Vec::new(),
                addon_lock: false,
                addon_indexes: Vec::new(),
            },
            summary: ExternalPackageSummary::default(),
            warnings: Vec::new(),
        }
    }

    fn entry(source_path: &str, normalized_path: &str) -> ExternalPackageEntry {
        ExternalPackageEntry {
            source_path: source_path.to_string(),
            normalized_path: normalized_path.to_string(),
            group: ApplyGroup::Addons,
            wtf_scope: None,
            source_account: None,
            source_server: None,
            source_character: None,
        }
    }
}
