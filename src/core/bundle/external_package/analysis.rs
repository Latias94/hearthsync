use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::super::shared::path::safe_file_part;
use super::super::types::apply::ApplyGroup;
use super::types::{
    ExternalPackageAnalysis, ExternalPackageEntry, ExternalPackageLayout,
    ExternalPackagePublicSharingReason, ExternalPackagePublicSharingReasonCode,
    ExternalPackagePublicSharingSeverity, ExternalPackagePublicSharingStatus,
    ExternalPackagePublicSharingSummary, ExternalPackageSourceCharacterSummary,
    ExternalPackageSourceIdentitySummary, ExternalPackageSourceKind, ExternalPackageSummary,
    ExternalPackageWarning, ExternalPackageWarningCategory, ExternalPackageWarningGroup,
    ExternalPackageWtfScopeSummary,
};
use crate::core::bundle::types::apply::WtfScopeRisk;
use crate::core::manifest::{BundleResources, CharacterResource};

pub(super) fn build_analysis(
    source_path: PathBuf,
    source_kind: ExternalPackageSourceKind,
    layout: ExternalPackageLayout,
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
        layout,
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
    let mut wtf_scope_groups = BTreeMap::new();
    let mut source_accounts = BTreeSet::new();
    let mut source_characters = BTreeSet::new();
    let mut entries_with_source_account = 0usize;
    let mut entries_with_source_character = 0usize;
    for warning in warnings {
        *warning_groups
            .entry((warning.category, warning.code))
            .or_insert(0usize) += 1;
    }
    for entry in entries {
        if let Some(source_account) = entry.source_account.as_deref() {
            source_accounts.insert(source_account.to_string());
            entries_with_source_account += 1;
        }
        if let (Some(source_server), Some(source_character)) = (
            entry.source_server.as_deref(),
            entry.source_character.as_deref(),
        ) {
            source_characters.insert(ExternalPackageSourceCharacterSummary {
                source_account: entry.source_account.clone(),
                source_server: source_server.to_string(),
                source_character: source_character.to_string(),
            });
            entries_with_source_character += 1;
        }
        if let Some(wtf_scope) = entry.wtf_scope {
            *wtf_scope_groups.entry(wtf_scope).or_insert(0usize) += 1;
        }
    }

    let wtf_scopes = wtf_scope_groups
        .into_iter()
        .map(|(scope, count)| ExternalPackageWtfScopeSummary {
            scope,
            risk: scope.risk(),
            count,
        })
        .collect::<Vec<_>>();
    let source_identities = ExternalPackageSourceIdentitySummary {
        source_accounts: source_accounts.into_iter().collect(),
        source_characters: source_characters.into_iter().collect(),
        entries_with_source_account,
        entries_with_source_character,
    };
    let public_sharing =
        build_public_sharing_summary(warnings.len(), &wtf_scopes, &source_identities);

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
        wtf_scopes,
        source_identities,
        public_sharing,
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

fn build_public_sharing_summary(
    warning_count: usize,
    wtf_scopes: &[ExternalPackageWtfScopeSummary],
    source_identities: &ExternalPackageSourceIdentitySummary,
) -> ExternalPackagePublicSharingSummary {
    let mut reasons = Vec::new();

    push_reason(
        &mut reasons,
        ExternalPackagePublicSharingSeverity::ReviewRequired,
        ExternalPackagePublicSharingReasonCode::NormalizationWarnings,
        warning_count,
        "package normalization produced warnings; review ignored or unsupported files before public sharing",
    );
    push_reason(
        &mut reasons,
        ExternalPackagePublicSharingSeverity::ReviewRequired,
        ExternalPackagePublicSharingReasonCode::HighRiskWtfScope,
        count_wtf_risk(wtf_scopes, WtfScopeRisk::High),
        "package contains account-wide SavedVariables or other high-risk WTF data",
    );
    push_reason(
        &mut reasons,
        ExternalPackagePublicSharingSeverity::ReviewRequired,
        ExternalPackagePublicSharingReasonCode::UnknownRiskWtfScope,
        count_wtf_risk(wtf_scopes, WtfScopeRisk::Unknown),
        "package contains WTF data with unknown sharing risk",
    );
    push_reason(
        &mut reasons,
        ExternalPackagePublicSharingSeverity::ReviewRequired,
        ExternalPackagePublicSharingReasonCode::MediumRiskWtfScope,
        count_wtf_risk(wtf_scopes, WtfScopeRisk::Medium),
        "package contains global, account-root, character SavedVariables, or character-state WTF data",
    );
    push_reason(
        &mut reasons,
        ExternalPackagePublicSharingSeverity::Advisory,
        ExternalPackagePublicSharingReasonCode::LowRiskWtfScope,
        count_wtf_risk(wtf_scopes, WtfScopeRisk::Low),
        "package contains cache-like WTF data; it is low risk but still worth reviewing",
    );
    push_reason(
        &mut reasons,
        ExternalPackagePublicSharingSeverity::ReviewRequired,
        ExternalPackagePublicSharingReasonCode::SourceAccountIdentity,
        source_identities.entries_with_source_account,
        "package paths expose source account identity",
    );
    push_reason(
        &mut reasons,
        ExternalPackagePublicSharingSeverity::ReviewRequired,
        ExternalPackagePublicSharingReasonCode::SourceCharacterIdentity,
        source_identities.entries_with_source_character,
        "package paths expose source character and realm identity",
    );

    let review_required_count = reasons
        .iter()
        .filter(|reason| reason.severity == ExternalPackagePublicSharingSeverity::ReviewRequired)
        .count();
    let advisory_count = reasons
        .iter()
        .filter(|reason| reason.severity == ExternalPackagePublicSharingSeverity::Advisory)
        .count();
    let status = if review_required_count > 0 {
        ExternalPackagePublicSharingStatus::ReviewRequired
    } else if advisory_count > 0 {
        ExternalPackagePublicSharingStatus::Advisory
    } else {
        ExternalPackagePublicSharingStatus::Ready
    };

    ExternalPackagePublicSharingSummary {
        status,
        public_ready: review_required_count == 0,
        review_required_count,
        advisory_count,
        reasons,
    }
}

fn count_wtf_risk(wtf_scopes: &[ExternalPackageWtfScopeSummary], risk: WtfScopeRisk) -> usize {
    wtf_scopes
        .iter()
        .filter(|scope| scope.risk == risk)
        .map(|scope| scope.count)
        .sum()
}

fn push_reason(
    reasons: &mut Vec<ExternalPackagePublicSharingReason>,
    severity: ExternalPackagePublicSharingSeverity,
    code: ExternalPackagePublicSharingReasonCode,
    count: usize,
    message: &str,
) {
    if count == 0 {
        return;
    }

    reasons.push(ExternalPackagePublicSharingReason {
        severity,
        code,
        count,
        message: message.to_string(),
    });
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
