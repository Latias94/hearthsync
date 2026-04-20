use std::collections::{BTreeMap, BTreeSet};

use super::types::ExternalPackageAnalysis;
use crate::core::error::{AppError, AppResult};

pub(super) fn validate_unique_normalized_paths(
    analysis: &ExternalPackageAnalysis,
) -> AppResult<()> {
    let mut seen = BTreeSet::new();
    let mut case_insensitive_seen = BTreeMap::new();
    for entry in &analysis.entries {
        if !seen.insert(entry.normalized_path.clone()) {
            return Err(AppError::Validation(format!(
                "external package normalizes multiple files onto the same target path: {}",
                entry.normalized_path
            )));
        }

        let folded = entry.normalized_path.to_lowercase();
        if let Some(previous) = case_insensitive_seen.insert(folded, entry.normalized_path.clone())
            && previous != entry.normalized_path
        {
            return Err(AppError::Validation(format!(
                "external package contains case-insensitive target path collisions: `{previous}` and `{}` would map to the same path on Windows/default macOS targets",
                entry.normalized_path
            )));
        }
    }

    Ok(())
}

pub(super) fn build_external_package_entry_source_map(
    analysis: &ExternalPackageAnalysis,
) -> AppResult<BTreeMap<String, String>> {
    let mut entry_source_map = BTreeMap::new();
    for entry in &analysis.entries {
        if entry_source_map
            .insert(entry.normalized_path.clone(), entry.source_path.clone())
            .is_some()
        {
            return Err(AppError::Validation(format!(
                "external package normalizes multiple files onto the same target path: {}",
                entry.normalized_path
            )));
        }
    }

    Ok(entry_source_map)
}
