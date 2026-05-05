use std::collections::BTreeMap;

use super::types::{
    ExternalPackageEntry, ExternalPackagePublicSharingSeverity,
    ExternalPackageSensitiveWtfFileKind, ExternalPackageSensitiveWtfFileSummary,
};

pub(super) fn summarize_sensitive_wtf_files(
    entries: &[ExternalPackageEntry],
) -> Vec<ExternalPackageSensitiveWtfFileSummary> {
    let mut groups = BTreeMap::new();

    for entry in entries {
        let Some(kind) = classify_sensitive_wtf_file(&entry.normalized_path) else {
            continue;
        };
        let severity = public_sharing_severity(kind);
        *groups.entry((kind, severity)).or_insert(0usize) += 1;
    }

    groups
        .into_iter()
        .map(
            |((kind, severity), count)| ExternalPackageSensitiveWtfFileSummary {
                kind,
                severity,
                count,
            },
        )
        .collect()
}

fn classify_sensitive_wtf_file(
    normalized_path: &str,
) -> Option<ExternalPackageSensitiveWtfFileKind> {
    let segments = normalized_path.split('/').collect::<Vec<_>>();
    if !segments
        .first()
        .is_some_and(|segment| segment.eq_ignore_ascii_case("wtf"))
    {
        return None;
    }

    let file_name = segments.last()?;
    let lower_file_name = file_name.to_ascii_lowercase();

    if is_saved_variables_file(&segments, &lower_file_name) {
        return Some(ExternalPackageSensitiveWtfFileKind::SavedVariables);
    }

    match lower_file_name.as_str() {
        "chat-cache.txt" => Some(ExternalPackageSensitiveWtfFileKind::ChatCache),
        "macros-cache.txt" => Some(ExternalPackageSensitiveWtfFileKind::Macros),
        "bindings-cache.wtf" => Some(ExternalPackageSensitiveWtfFileKind::Bindings),
        "config.wtf" | "config-cache.wtf" => Some(ExternalPackageSensitiveWtfFileKind::GameConfig),
        "addons.txt" => Some(ExternalPackageSensitiveWtfFileKind::AddonEnablement),
        "layout-local.txt" => Some(ExternalPackageSensitiveWtfFileKind::LayoutState),
        _ => None,
    }
}

fn is_saved_variables_file(segments: &[&str], lower_file_name: &str) -> bool {
    lower_file_name.ends_with(".lua")
        && segments
            .iter()
            .any(|segment| segment.eq_ignore_ascii_case("SavedVariables"))
}

fn public_sharing_severity(
    kind: ExternalPackageSensitiveWtfFileKind,
) -> ExternalPackagePublicSharingSeverity {
    match kind {
        ExternalPackageSensitiveWtfFileKind::SavedVariables
        | ExternalPackageSensitiveWtfFileKind::ChatCache
        | ExternalPackageSensitiveWtfFileKind::Macros => {
            ExternalPackagePublicSharingSeverity::ReviewRequired
        }
        ExternalPackageSensitiveWtfFileKind::Bindings
        | ExternalPackageSensitiveWtfFileKind::GameConfig
        | ExternalPackageSensitiveWtfFileKind::AddonEnablement
        | ExternalPackageSensitiveWtfFileKind::LayoutState => {
            ExternalPackagePublicSharingSeverity::Advisory
        }
    }
}
