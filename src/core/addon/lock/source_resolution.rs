use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::core::addon::{
    prepare_package_from_archive_with_source, prepare_package_from_source_ref_with_provider,
};
use crate::core::archive_path::{join_segments, safe_zip_segments_under};
use crate::core::error::{AppError, AppResult};

use super::{AddonLockPackage, AddonLockSourceOverride};

#[derive(Debug, Clone, Deserialize)]
struct AddonLockSidecarSourceIndex {
    schema_version: u32,
    sources: Vec<AddonLockSidecarSourceEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct AddonLockSidecarSourceEntry {
    comparison_key: String,
    path: String,
}

pub(super) fn resolved_source_override_map(
    lock_path: &Path,
    source_overrides: &[AddonLockSourceOverride],
) -> AppResult<BTreeMap<String, PathBuf>> {
    let mut map = load_sidecar_source_overrides(lock_path)?;
    let mut explicit_keys = BTreeSet::new();
    for source_override in source_overrides {
        if source_override.comparison_key.trim().is_empty() {
            return Err(AppError::Validation(
                "addon lock source override comparison key must not be empty".to_string(),
            ));
        }
        if !explicit_keys.insert(source_override.comparison_key.clone()) {
            return Err(AppError::Validation(format!(
                "duplicate addon lock source override for `{}`",
                source_override.comparison_key
            )));
        }
        map.insert(
            source_override.comparison_key.clone(),
            source_override.archive_path.clone(),
        );
    }
    Ok(map)
}

fn load_sidecar_source_overrides(lock_path: &Path) -> AppResult<BTreeMap<String, PathBuf>> {
    let Some(lock_dir) = lock_path.parent() else {
        return Ok(BTreeMap::new());
    };
    let source_index_path = lock_dir.join("sources.toml");
    if !source_index_path.is_file() {
        return Ok(BTreeMap::new());
    }

    let content = fs::read_to_string(&source_index_path)?;
    let source_index = toml::from_str::<AddonLockSidecarSourceIndex>(&content)?;
    if source_index.schema_version != 1 {
        return Err(AppError::Validation(format!(
            "unsupported addon lock source index schema version: {}",
            source_index.schema_version
        )));
    }

    let mut map = BTreeMap::new();
    for source in source_index.sources {
        if source.comparison_key.trim().is_empty() {
            return Err(AppError::Validation(format!(
                "addon lock source index `{}` contains an empty comparison key",
                source_index_path.display()
            )));
        }
        let segments = safe_sidecar_source_segments(&source.path)?;
        let archive_path = join_segments(lock_dir, &segments);
        if map
            .insert(source.comparison_key.clone(), archive_path)
            .is_some()
        {
            return Err(AppError::Validation(format!(
                "duplicate addon lock source index entry for `{}`",
                source.comparison_key
            )));
        }
    }

    Ok(map)
}

fn safe_sidecar_source_segments(path: &str) -> AppResult<Vec<&str>> {
    safe_zip_segments_under(path, "sources", "addon lock source path")
}

pub(super) fn prepare_expected_lock_package_with_provider<P>(
    provider: &P,
    expected: &AddonLockPackage,
    source_override_path: Option<&Path>,
    target_flavor: crate::core::install::WowFlavor,
    target_platform: crate::core::install::HostPlatform,
    cancellation: &dyn crate::core::task::CancellationToken,
) -> AppResult<crate::core::addon::PreparedAddonPackage>
where
    P: crate::core::addon::AddonProvider + ?Sized,
{
    match source_override_path {
        Some(path) => {
            prepare_package_from_archive_with_source(expected.source.clone(), path, target_platform)
        }
        None => prepare_package_from_source_ref_with_provider(
            provider,
            &expected.source,
            Some(target_flavor),
            target_platform,
            cancellation,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::safe_sidecar_source_segments;

    #[test]
    fn safe_sidecar_source_segments_rejects_non_portable_paths() {
        for path in [
            "sources//provider.zip",
            "sources/CON.zip",
            "sources/addon. ",
        ] {
            let error = safe_sidecar_source_segments(path)
                .expect_err("non-portable sidecar path should fail");

            assert!(error.to_string().contains("unsafe addon lock source path"));
        }
    }

    #[test]
    fn safe_sidecar_source_segments_requires_sources_root() {
        let error = safe_sidecar_source_segments("archives/provider.zip")
            .expect_err("non-sources root should fail");

        assert!(
            error
                .to_string()
                .contains("addon lock source path must be under `sources/`")
        );
    }

    #[test]
    fn safe_sidecar_source_segments_accepts_portable_sources_paths() {
        assert_eq!(
            safe_sidecar_source_segments("sources/providers/curseforge/WeakAuras.zip")
                .expect("portable sidecar path"),
            vec!["sources", "providers", "curseforge", "WeakAuras.zip"]
        );
    }
}
