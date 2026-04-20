use super::super::types::apply::{ApplyGroup, WtfScope};
use super::source_entry::SourceEntry;
use super::types::{
    ExternalPackageEntry, ExternalPackageWarning, ExternalPackageWarningCategory,
    ExternalPackageWarningCode,
};
use crate::core::addon_layout::discover_addon_roots_from_entry_segments;
use crate::core::bundle::wtf_scope::{classify_account_wtf_scope, classify_character_wtf_scope};

pub(super) fn classify_source_entries(
    source_entries: &[SourceEntry],
) -> (Vec<ExternalPackageEntry>, Vec<ExternalPackageWarning>) {
    let addon_roots = discover_addon_roots_from_entry_segments(
        source_entries.iter().map(|entry| entry.segments.as_slice()),
    );
    let mut entries = Vec::new();
    let mut warnings = Vec::new();

    for source_entry in source_entries {
        if let Some(addon_entry) = classify_addon_entry(source_entry, &addon_roots) {
            entries.push(addon_entry);
            continue;
        }

        match classify_non_addon_entry(source_entry) {
            ClassifiedExternalEntry::Recognized(entry) => entries.push(entry),
            ClassifiedExternalEntry::Ignored => {}
            ClassifiedExternalEntry::Warn(message) => warnings.push(message),
        }
    }

    (entries, warnings)
}

fn classify_addon_entry(
    source_entry: &SourceEntry,
    addon_roots: &[Vec<String>],
) -> Option<ExternalPackageEntry> {
    let root = addon_roots
        .iter()
        .find(|root| starts_with_segments(&source_entry.segments, root))?;
    let addon_name = root.last()?.clone();
    let relative = &source_entry.segments[root.len()..];
    let normalized_path = join_normalized_segments("addons", &addon_name, relative);

    Some(ExternalPackageEntry {
        source_path: source_entry.source_path.clone(),
        normalized_path,
        group: ApplyGroup::Addons,
        wtf_scope: None,
        source_account: None,
        source_server: None,
        source_character: None,
    })
}

enum ClassifiedExternalEntry {
    Recognized(ExternalPackageEntry),
    Ignored,
    Warn(ExternalPackageWarning),
}

fn classify_non_addon_entry(source_entry: &SourceEntry) -> ClassifiedExternalEntry {
    if let Some(entry) = classify_wtf_entry(source_entry) {
        return entry;
    }

    if let Some(entry) = classify_fonts_entry(source_entry) {
        return ClassifiedExternalEntry::Recognized(entry);
    }

    if let Some(entry) = classify_interface_entry(source_entry) {
        return ClassifiedExternalEntry::Recognized(entry);
    }

    if find_segment_index(&source_entry.segments, "AddOns").is_some() {
        return ClassifiedExternalEntry::Warn(build_external_package_warning(
            ExternalPackageWarningCategory::Addon,
            ExternalPackageWarningCode::AddonRootNotDetected,
            &source_entry.source_path,
            format!(
                "entry is under `AddOns` but no addon root was detected from a `.toc` file: {}",
                source_entry.source_path
            ),
        ));
    }

    ClassifiedExternalEntry::Ignored
}

fn classify_wtf_entry(source_entry: &SourceEntry) -> Option<ClassifiedExternalEntry> {
    let wtf_index = find_segment_index(&source_entry.segments, "WTF")?;
    let suffix = &source_entry.segments[wtf_index..];
    if suffix.len() == 2 && suffix[1].eq_ignore_ascii_case("Config.wtf") {
        return Some(ClassifiedExternalEntry::Recognized(ExternalPackageEntry {
            source_path: source_entry.source_path.clone(),
            normalized_path: "wtf/common/Config.wtf".to_string(),
            group: ApplyGroup::WtfCommon,
            wtf_scope: Some(WtfScope::GlobalConfig),
            source_account: None,
            source_server: None,
            source_character: None,
        }));
    }

    if suffix.len() < 4 || !suffix[1].eq_ignore_ascii_case("Account") {
        return Some(ClassifiedExternalEntry::Warn(
            build_external_package_warning(
                ExternalPackageWarningCategory::Wtf,
                ExternalPackageWarningCode::UnsupportedWtfLayout,
                &source_entry.source_path,
                format!(
                    "WTF path does not match a supported account or character layout: {}",
                    source_entry.source_path
                ),
            ),
        ));
    }

    let account = &suffix[2];
    if account.eq_ignore_ascii_case("SavedVariables") {
        let rest = &suffix[3..];
        if rest.is_empty() {
            return Some(ClassifiedExternalEntry::Warn(
                build_external_package_warning(
                    ExternalPackageWarningCategory::Wtf,
                    ExternalPackageWarningCode::WtfSavedVariablesPathWithoutFile,
                    &source_entry.source_path,
                    format!(
                        "root-level `WTF/Account/SavedVariables` entry does not point to a file: {}",
                        source_entry.source_path
                    ),
                ),
            ));
        }

        return Some(ClassifiedExternalEntry::Recognized(ExternalPackageEntry {
            source_path: source_entry.source_path.clone(),
            normalized_path: join_exact_normalized_segments(
                &["wtf", "common", "root", "SavedVariables"],
                rest,
            ),
            group: ApplyGroup::WtfCommon,
            wtf_scope: Some(WtfScope::RootSavedVariables),
            source_account: None,
            source_server: None,
            source_character: None,
        }));
    }

    let rest = &suffix[3..];
    if rest.is_empty() {
        return Some(ClassifiedExternalEntry::Warn(
            build_external_package_warning(
                ExternalPackageWarningCategory::Wtf,
                ExternalPackageWarningCode::WtfAccountPathWithoutFile,
                &source_entry.source_path,
                format!(
                    "WTF account entry does not point to a file path: {}",
                    source_entry.source_path
                ),
            ),
        ));
    }

    if rest[0].eq_ignore_ascii_case("SavedVariables") {
        if rest.len() < 2 {
            return Some(ClassifiedExternalEntry::Warn(
                build_external_package_warning(
                    ExternalPackageWarningCategory::Wtf,
                    ExternalPackageWarningCode::WtfSavedVariablesPathWithoutFile,
                    &source_entry.source_path,
                    format!(
                        "WTF account `SavedVariables` entry does not point to a file: {}",
                        source_entry.source_path
                    ),
                ),
            ));
        }

        return Some(ClassifiedExternalEntry::Recognized(ExternalPackageEntry {
            source_path: source_entry.source_path.clone(),
            normalized_path: join_exact_normalized_segments(
                &["wtf", "common", "accounts", account],
                rest,
            ),
            group: ApplyGroup::WtfCommon,
            wtf_scope: Some(WtfScope::AccountSavedVariables),
            source_account: Some(account.clone()),
            source_server: None,
            source_character: None,
        }));
    }

    if rest.len() >= 3 {
        let server = &rest[0];
        let character = &rest[1];
        let character_relative = &rest[2..];
        return Some(ClassifiedExternalEntry::Recognized(ExternalPackageEntry {
            source_path: source_entry.source_path.clone(),
            normalized_path: join_exact_normalized_segments(
                &["wtf", "characters", account, server, character],
                character_relative,
            ),
            group: ApplyGroup::WtfCharacters,
            wtf_scope: Some(classify_character_wtf_scope(character_relative)),
            source_account: Some(account.clone()),
            source_server: Some(server.clone()),
            source_character: Some(character.clone()),
        }));
    }

    if rest.len() == 1 {
        return Some(ClassifiedExternalEntry::Recognized(ExternalPackageEntry {
            source_path: source_entry.source_path.clone(),
            normalized_path: join_exact_normalized_segments(
                &["wtf", "common", "accounts", account],
                rest,
            ),
            group: ApplyGroup::WtfCommon,
            wtf_scope: Some(classify_account_wtf_scope(rest)),
            source_account: Some(account.clone()),
            source_server: None,
            source_character: None,
        }));
    }

    Some(ClassifiedExternalEntry::Warn(
        build_external_package_warning(
            ExternalPackageWarningCategory::Wtf,
            ExternalPackageWarningCode::UnsupportedWtfNestedAccountLayout,
            &source_entry.source_path,
            format!(
                "WTF path is nested under an account but does not match a supported file layout: {}",
                source_entry.source_path
            ),
        ),
    ))
}

fn classify_fonts_entry(source_entry: &SourceEntry) -> Option<ExternalPackageEntry> {
    let fonts_index = find_segment_index(&source_entry.segments, "Fonts")?;
    let rest = &source_entry.segments[fonts_index + 1..];
    if rest.is_empty() {
        return None;
    }

    Some(ExternalPackageEntry {
        source_path: source_entry.source_path.clone(),
        normalized_path: join_exact_normalized_segments(&["fonts"], rest),
        group: ApplyGroup::Fonts,
        wtf_scope: None,
        source_account: None,
        source_server: None,
        source_character: None,
    })
}

fn classify_interface_entry(source_entry: &SourceEntry) -> Option<ExternalPackageEntry> {
    let interface_index = find_segment_index(&source_entry.segments, "Interface")?;
    let rest = &source_entry.segments[interface_index + 1..];
    if rest.is_empty() {
        return None;
    }
    if rest[0].eq_ignore_ascii_case("AddOns") {
        return None;
    }

    Some(ExternalPackageEntry {
        source_path: source_entry.source_path.clone(),
        normalized_path: join_exact_normalized_segments(&["interface"], rest),
        group: ApplyGroup::InterfaceAssets,
        wtf_scope: None,
        source_account: None,
        source_server: None,
        source_character: None,
    })
}

fn build_external_package_warning(
    category: ExternalPackageWarningCategory,
    code: ExternalPackageWarningCode,
    source_path: &str,
    message: String,
) -> ExternalPackageWarning {
    ExternalPackageWarning {
        category,
        code,
        source_path: source_path.to_string(),
        message,
    }
}

fn starts_with_segments(segments: &[String], prefix: &[String]) -> bool {
    prefix.len() <= segments.len()
        && prefix
            .iter()
            .zip(segments.iter())
            .all(|(left, right)| left == right)
}

fn join_normalized_segments(root: &str, name: &str, rest: &[String]) -> String {
    let mut segments = vec![root.to_string(), name.to_string()];
    segments.extend(rest.iter().cloned());
    segments.join("/")
}

fn join_exact_normalized_segments(prefix: &[&str], rest: &[String]) -> String {
    let mut segments = prefix
        .iter()
        .map(|segment| (*segment).to_string())
        .collect::<Vec<_>>();
    segments.extend(rest.iter().cloned());
    segments.join("/")
}

fn find_segment_index(segments: &[String], needle: &str) -> Option<usize> {
    segments
        .iter()
        .position(|segment| segment.eq_ignore_ascii_case(needle))
}
