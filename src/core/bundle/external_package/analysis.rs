use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::super::shared::path::safe_file_part;
use super::super::types::ApplyGroup;
use super::types::{
    ExternalPackageAnalysis, ExternalPackageEntry, ExternalPackageSourceKind,
    ExternalPackageSummary, ExternalPackageWarning, ExternalPackageWarningCategory,
    ExternalPackageWarningGroup,
};
use crate::core::manifest::{BundleResources, CharacterResource};

pub(super) fn build_analysis(
    source_path: PathBuf,
    source_kind: ExternalPackageSourceKind,
    total_source_files: usize,
    mut entries: Vec<ExternalPackageEntry>,
    mut warnings: Vec<ExternalPackageWarning>,
) -> ExternalPackageAnalysis {
    entries.sort_by(|left, right| {
        left.normalized_path
            .cmp(&right.normalized_path)
            .then_with(|| left.source_path.cmp(&right.source_path))
    });
    warnings.sort();
    warnings.dedup();

    let summary = build_summary(total_source_files, &entries, &warnings);
    let resources = build_resources(&entries);

    ExternalPackageAnalysis {
        package_id: package_id_from_source_path(&source_path),
        package_name: package_name_from_source_path(&source_path),
        source_path,
        source_kind,
        entries,
        resources,
        summary,
        warnings,
    }
}

fn build_summary(
    total_files: usize,
    entries: &[ExternalPackageEntry],
    warnings: &[ExternalPackageWarning],
) -> ExternalPackageSummary {
    let mut warning_groups = BTreeMap::new();
    for warning in warnings {
        *warning_groups
            .entry((warning.category, warning.code))
            .or_insert(0usize) += 1;
    }

    let mut summary = ExternalPackageSummary {
        total_files,
        normalized_files: entries.len(),
        ignored_files: total_files.saturating_sub(entries.len()),
        warning_count: warnings.len(),
        addon_warning_count: warnings
            .iter()
            .filter(|warning| warning.category == ExternalPackageWarningCategory::Addon)
            .count(),
        wtf_warning_count: warnings
            .iter()
            .filter(|warning| warning.category == ExternalPackageWarningCategory::Wtf)
            .count(),
        warning_groups: warning_groups
            .into_iter()
            .map(|((category, code), count)| ExternalPackageWarningGroup {
                category,
                code,
                count,
            })
            .collect(),
        ..ExternalPackageSummary::default()
    };

    for entry in entries {
        match entry.group {
            ApplyGroup::Addons => summary.addons += 1,
            ApplyGroup::WtfCommon => summary.wtf_common += 1,
            ApplyGroup::WtfCharacters => summary.wtf_characters += 1,
            ApplyGroup::Fonts => summary.fonts += 1,
            ApplyGroup::InterfaceAssets => summary.interface_assets += 1,
            ApplyGroup::Metadata => {}
        }
    }

    summary
}

fn build_resources(entries: &[ExternalPackageEntry]) -> BundleResources {
    let mut addons = BTreeSet::new();
    let mut characters = BTreeSet::new();
    let mut interface_assets = BTreeSet::new();
    let mut wtf_common = false;
    let mut fonts = false;

    for entry in entries {
        match entry.group {
            ApplyGroup::Addons => {
                if let Some(addon_name) = normalized_path_tail(&entry.normalized_path, "addons") {
                    addons.insert(addon_name.to_string());
                }
            }
            ApplyGroup::WtfCommon => {
                wtf_common = true;
            }
            ApplyGroup::WtfCharacters => {
                if let (Some(source_account), Some(source_server), Some(source_character)) = (
                    entry.source_account.as_deref(),
                    entry.source_server.as_deref(),
                    entry.source_character.as_deref(),
                ) {
                    characters.insert((
                        source_account.to_string(),
                        source_server.to_string(),
                        source_character.to_string(),
                    ));
                }
            }
            ApplyGroup::Fonts => {
                fonts = true;
            }
            ApplyGroup::InterfaceAssets => {
                if let Some(asset_name) = normalized_path_tail(&entry.normalized_path, "interface")
                {
                    interface_assets.insert(asset_name.to_string());
                }
            }
            ApplyGroup::Metadata => {}
        }
    }

    BundleResources {
        addons: addons.into_iter().collect(),
        wtf_common,
        wtf_characters: characters
            .into_iter()
            .map(
                |(source_account, source_server, source_character)| CharacterResource {
                    source_account: Some(source_account),
                    source_server,
                    source_character,
                    target_hint: None,
                },
            )
            .collect(),
        fonts,
        interface_assets: interface_assets.into_iter().collect(),
        addon_lock: false,
        addon_indexes: Vec::new(),
    }
}

fn package_id_from_source_path(path: &Path) -> String {
    let candidate = package_name_from_source_path(path);
    let normalized = safe_file_part(&candidate);
    if normalized.is_empty() {
        "external-package".to_string()
    } else {
        normalized
    }
}

fn package_name_from_source_path(path: &Path) -> String {
    path.file_stem()
        .or_else(|| path.file_name())
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("external-package")
        .to_string()
}

fn normalized_path_tail<'a>(normalized_path: &'a str, root: &str) -> Option<&'a str> {
    normalized_path
        .strip_prefix(root)
        .and_then(|value| value.strip_prefix('/'))
        .and_then(|value| value.split('/').next())
}
