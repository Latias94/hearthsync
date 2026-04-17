use super::*;

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
        let archive_path = join_sidecar_source_segments(lock_dir, &segments);
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
    let mut segments = Vec::new();
    for segment in path.split('/') {
        if segment.is_empty() {
            continue;
        }
        if segment == "." || segment == ".." || segment.contains('\\') {
            return Err(AppError::Validation(format!(
                "unsafe addon lock source path: `{path}`"
            )));
        }
        segments.push(segment);
    }
    if segments.first().copied() != Some("sources") || segments.len() < 2 {
        return Err(AppError::Validation(format!(
            "addon lock source path must be under `sources/`: {path}"
        )));
    }
    Ok(segments)
}

fn join_sidecar_source_segments(root: &Path, segments: &[&str]) -> PathBuf {
    let mut path = root.to_path_buf();
    for segment in segments {
        path.push(segment);
    }
    path
}

pub(super) fn prepare_expected_lock_package_with_provider<P>(
    provider: &P,
    expected: &AddonLockPackage,
    source_override_path: Option<&Path>,
    target_flavor: crate::core::install::WowFlavor,
    cancellation: &dyn crate::core::task::CancellationToken,
) -> AppResult<crate::core::addon::PreparedAddonPackage>
where
    P: crate::core::addon::AddonProvider + ?Sized,
{
    match source_override_path {
        Some(path) => prepare_package_from_archive_with_source(expected.source.clone(), path),
        None => prepare_package_from_source_ref_with_provider(
            provider,
            &expected.source,
            Some(target_flavor),
            cancellation,
        ),
    }
}
