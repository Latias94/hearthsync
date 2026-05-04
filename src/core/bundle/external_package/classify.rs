use super::source_entry::SourceEntry;
use super::types::{
    ExternalPackageEntry, ExternalPackageLayout, ExternalPackageWarning,
    ExternalPackageWarningCategory, ExternalPackageWarningCode,
};
use crate::core::addon_layout::{
    AddonRootPrefixMatchKind, addon_root_prefix_match_kind,
    discover_addon_roots_from_entry_segments,
};
use crate::core::bundle::entry_layout::classify_bundle_archive_entry;
use crate::core::bundle::shared::path::validate_plain_name;
use crate::core::error::{AppError, AppResult};
use crate::core::install::HostPlatform;

pub(super) fn classify_source_entries(
    source_entries: &[SourceEntry],
    layout: ExternalPackageLayout,
    source_account: Option<&str>,
    source_server: Option<&str>,
    source_character: Option<&str>,
) -> AppResult<(Vec<ExternalPackageEntry>, Vec<ExternalPackageWarning>)> {
    match layout {
        ExternalPackageLayout::Auto | ExternalPackageLayout::Generic => {
            classify_generic_source_entries(source_entries)
        }
        ExternalPackageLayout::NewBeeBoxAddon => classify_generic_source_entries(source_entries),
        ExternalPackageLayout::NewBeeBoxFont => {
            classify_newbeebox_font_entries(source_entries).map(|entries| (entries, Vec::new()))
        }
        ExternalPackageLayout::NewBeeBoxMaterial => {
            classify_newbeebox_material_entries(source_entries).map(|entries| (entries, Vec::new()))
        }
        ExternalPackageLayout::NewBeeBoxWtfAccount => {
            classify_newbeebox_wtf_account_entries(source_entries, source_account)
                .map(|entries| (entries, Vec::new()))
        }
        ExternalPackageLayout::NewBeeBoxWtfCharacter => classify_newbeebox_wtf_character_entries(
            source_entries,
            source_account,
            source_server,
            source_character,
        )
        .map(|entries| (entries, Vec::new())),
    }
}

fn classify_generic_source_entries(
    source_entries: &[SourceEntry],
) -> AppResult<(Vec<ExternalPackageEntry>, Vec<ExternalPackageWarning>)> {
    let addon_roots = discover_addon_roots_from_entry_segments(
        source_entries.iter().map(|entry| entry.segments.as_slice()),
        HostPlatform::Windows,
    );
    let mut entries = Vec::new();
    let mut warnings = Vec::new();

    for source_entry in source_entries {
        match classify_addon_path(&source_entry.segments, &addon_roots) {
            AddonPathClassification::Recognized(normalized_path) => {
                entries.push(build_normalized_external_entry(
                    &source_entry.source_path,
                    normalized_path,
                )?);
                continue;
            }
            AddonPathClassification::MissingRoot => {
                warnings.push(build_addon_root_not_detected_warning(
                    &source_entry.source_path,
                ));
                continue;
            }
            AddonPathClassification::None => {}
        }

        match classify_non_addon_entry(source_entry)? {
            ClassifiedExternalEntry::Recognized(entry) => entries.push(entry),
            ClassifiedExternalEntry::Ignored => {}
            ClassifiedExternalEntry::Warn(message) => warnings.push(message),
        }
    }

    Ok((entries, warnings))
}

fn classify_addon_path(
    segments: &[String],
    addon_roots: &[Vec<String>],
) -> AddonPathClassification {
    let root = find_addon_root(segments, addon_roots);
    let Some(root) = root else {
        if find_segment_index(segments, "AddOns").is_some() {
            return AddonPathClassification::MissingRoot;
        }
        return AddonPathClassification::None;
    };
    let Some(addon_name) = root.last() else {
        return AddonPathClassification::None;
    };
    let relative = &segments[root.len()..];

    AddonPathClassification::Recognized(join_normalized_segments("addons", addon_name, relative))
}

enum ClassifiedExternalEntry {
    Recognized(ExternalPackageEntry),
    Ignored,
    Warn(ExternalPackageWarning),
}

enum AddonPathClassification {
    Recognized(String),
    MissingRoot,
    None,
}

enum WtfSuffixClassification {
    Recognized(String),
    Warning(WtfWarningKind),
}

enum RootedNonAddonClassification {
    Recognized(String),
    None,
}

enum WtfWarningKind {
    UnsupportedLayout,
    RootSavedVariablesPathWithoutFile,
    AccountPathWithoutFile,
    AccountSavedVariablesPathWithoutFile,
    UnsupportedNestedAccountLayout,
}

fn classify_non_addon_entry(source_entry: &SourceEntry) -> AppResult<ClassifiedExternalEntry> {
    if let Some(entry) = classify_wtf_entry(source_entry)? {
        return Ok(entry);
    }

    match classify_rooted_non_addon_path(&source_entry.segments) {
        RootedNonAddonClassification::Recognized(normalized_path) => {
            return build_recognized_entry(&source_entry.source_path, normalized_path);
        }
        RootedNonAddonClassification::None => {}
    }

    Ok(ClassifiedExternalEntry::Ignored)
}

fn classify_newbeebox_font_entries(
    source_entries: &[SourceEntry],
) -> AppResult<Vec<ExternalPackageEntry>> {
    let mut entries = Vec::new();

    for source_entry in source_entries {
        if source_entry.segments.is_empty() {
            continue;
        }

        let normalized_path = if source_entry.segments[0].eq_ignore_ascii_case("Fonts") {
            let rest = &source_entry.segments[1..];
            if rest.is_empty() {
                continue;
            }
            join_exact_normalized_segments(&["fonts"], rest)
        } else if source_entry.segments.len() == 1 {
            join_exact_normalized_segments(&["fonts"], &source_entry.segments)
        } else {
            continue;
        };

        entries.push(build_normalized_external_entry(
            &source_entry.source_path,
            normalized_path,
        )?);
    }

    Ok(entries)
}

fn classify_newbeebox_material_entries(
    source_entries: &[SourceEntry],
) -> AppResult<Vec<ExternalPackageEntry>> {
    let mut entries = Vec::new();

    for source_entry in source_entries {
        if source_entry.segments.is_empty() {
            continue;
        }

        let normalized_path = if source_entry.segments[0].eq_ignore_ascii_case("Interface") {
            let rest = &source_entry.segments[1..];
            if rest.is_empty() || rest[0].eq_ignore_ascii_case("AddOns") {
                continue;
            }
            join_exact_normalized_segments(&["interface"], rest)
        } else {
            join_exact_normalized_segments(&["interface"], &source_entry.segments)
        };

        entries.push(build_normalized_external_entry(
            &source_entry.source_path,
            normalized_path,
        )?);
    }

    Ok(entries)
}

fn classify_newbeebox_wtf_account_entries(
    source_entries: &[SourceEntry],
    source_account: Option<&str>,
) -> AppResult<Vec<ExternalPackageEntry>> {
    let source_account = require_layout_segment(
        source_account,
        "source_account",
        "NewBeeBox account WTF packages require `source_account`",
    )?;
    let mut entries = Vec::new();

    for source_entry in source_entries {
        if source_entry.segments.is_empty() {
            continue;
        }

        let normalized_path = if source_entry.segments[0].eq_ignore_ascii_case("WTF") {
            match classify_wtf_suffix(&source_entry.segments) {
                WtfSuffixClassification::Recognized(normalized_path) => normalized_path,
                WtfSuffixClassification::Warning(_) => continue,
            }
        } else if source_entry.segments[0].eq_ignore_ascii_case("SavedVariables") {
            if source_entry.segments.len() < 2 {
                continue;
            }
            join_exact_normalized_segments(
                &["wtf", "common", "accounts", source_account],
                &source_entry.segments,
            )
        } else {
            join_exact_normalized_segments(
                &["wtf", "common", "accounts", source_account],
                &source_entry.segments,
            )
        };

        entries.push(build_normalized_external_entry(
            &source_entry.source_path,
            normalized_path,
        )?);
    }

    Ok(entries)
}

fn classify_newbeebox_wtf_character_entries(
    source_entries: &[SourceEntry],
    source_account: Option<&str>,
    source_server: Option<&str>,
    source_character: Option<&str>,
) -> AppResult<Vec<ExternalPackageEntry>> {
    let source_account = require_layout_segment(
        source_account,
        "source_account",
        "NewBeeBox character WTF packages require `source_account`",
    )?;
    let source_server = require_layout_segment(
        source_server,
        "source_server",
        "NewBeeBox character WTF packages require `source_server`",
    )?;
    let inferred_character = infer_newbeebox_source_character(source_entries, source_character)?;
    let mut entries = Vec::new();

    for source_entry in source_entries {
        let rest = newbeebox_character_entry_rest(&source_entry.segments, &inferred_character);
        if rest.is_empty() {
            continue;
        }

        let normalized_path = join_exact_normalized_segments(
            &[
                "wtf",
                "characters",
                source_account,
                source_server,
                &inferred_character,
            ],
            rest,
        );
        entries.push(build_normalized_external_entry(
            &source_entry.source_path,
            normalized_path,
        )?);
    }

    Ok(entries)
}

fn require_layout_segment<'a>(
    value: Option<&'a str>,
    field: &str,
    message: &str,
) -> AppResult<&'a str> {
    let value = value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::Validation(message.to_string()))?;
    validate_plain_name(field, value)?;
    Ok(value)
}

fn infer_newbeebox_source_character(
    source_entries: &[SourceEntry],
    source_character: Option<&str>,
) -> AppResult<String> {
    if let Some(source_character) = source_character.filter(|value| !value.trim().is_empty()) {
        validate_plain_name("source_character", source_character)?;
        return Ok(source_character.to_string());
    }

    let mut candidates = Vec::<&str>::new();
    for source_entry in source_entries {
        if source_entry.segments.len() < 2 {
            return Err(AppError::Validation(
                "NewBeeBox character WTF packages require `source_character` when entries are not wrapped in one character directory".to_string(),
            ));
        }

        let candidate = source_entry.segments[0].as_str();
        if !candidates
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(candidate))
        {
            candidates.push(candidate);
        }
    }

    if candidates.len() != 1 {
        return Err(AppError::Validation(
            "NewBeeBox character WTF packages must contain exactly one source character directory or provide `source_character`".to_string(),
        ));
    }

    let source_character = candidates[0];
    validate_plain_name("source_character", source_character)?;
    Ok(source_character.to_string())
}

fn newbeebox_character_entry_rest<'a>(
    segments: &'a [String],
    source_character: &str,
) -> &'a [String] {
    if segments
        .first()
        .is_some_and(|segment| segment.eq_ignore_ascii_case(source_character))
    {
        &segments[1..]
    } else {
        segments
    }
}

fn classify_wtf_entry(source_entry: &SourceEntry) -> AppResult<Option<ClassifiedExternalEntry>> {
    let Some(wtf_index) = find_segment_index(&source_entry.segments, "WTF") else {
        return Ok(None);
    };
    Ok(Some(
        match classify_wtf_suffix(&source_entry.segments[wtf_index..]) {
            WtfSuffixClassification::Recognized(normalized_path) => {
                build_recognized_entry(&source_entry.source_path, normalized_path)?
            }
            WtfSuffixClassification::Warning(warning_kind) => ClassifiedExternalEntry::Warn(
                build_wtf_warning(&source_entry.source_path, warning_kind),
            ),
        },
    ))
}

fn build_recognized_entry(
    source_path: &str,
    normalized_path: String,
) -> AppResult<ClassifiedExternalEntry> {
    Ok(ClassifiedExternalEntry::Recognized(
        build_normalized_external_entry(source_path, normalized_path)?,
    ))
}

fn classify_wtf_suffix(suffix: &[String]) -> WtfSuffixClassification {
    if suffix.len() == 2 && suffix[1].eq_ignore_ascii_case("Config.wtf") {
        return WtfSuffixClassification::Recognized("wtf/common/Config.wtf".to_string());
    }

    if suffix.len() < 4 || !suffix[1].eq_ignore_ascii_case("Account") {
        return WtfSuffixClassification::Warning(WtfWarningKind::UnsupportedLayout);
    }

    let account = &suffix[2];
    if account.eq_ignore_ascii_case("SavedVariables") {
        let rest = &suffix[3..];
        if rest.is_empty() {
            return WtfSuffixClassification::Warning(
                WtfWarningKind::RootSavedVariablesPathWithoutFile,
            );
        }

        return WtfSuffixClassification::Recognized(join_exact_normalized_segments(
            &["wtf", "common", "root", "SavedVariables"],
            rest,
        ));
    }

    let rest = &suffix[3..];
    if rest.is_empty() {
        return WtfSuffixClassification::Warning(WtfWarningKind::AccountPathWithoutFile);
    }

    if rest[0].eq_ignore_ascii_case("SavedVariables") {
        if rest.len() < 2 {
            return WtfSuffixClassification::Warning(
                WtfWarningKind::AccountSavedVariablesPathWithoutFile,
            );
        }

        return WtfSuffixClassification::Recognized(join_exact_normalized_segments(
            &["wtf", "common", "accounts", account],
            rest,
        ));
    }

    if rest.len() >= 3 {
        return WtfSuffixClassification::Recognized(join_exact_normalized_segments(
            &["wtf", "characters", account, &rest[0], &rest[1]],
            &rest[2..],
        ));
    }

    if rest.len() == 1 {
        return WtfSuffixClassification::Recognized(join_exact_normalized_segments(
            &["wtf", "common", "accounts", account],
            rest,
        ));
    }

    WtfSuffixClassification::Warning(WtfWarningKind::UnsupportedNestedAccountLayout)
}

fn classify_rooted_non_addon_path(segments: &[String]) -> RootedNonAddonClassification {
    if let Some(normalized_path) = classify_rooted_path(segments, "Fonts", &["fonts"]) {
        return RootedNonAddonClassification::Recognized(normalized_path);
    }

    let Some(interface_index) = find_segment_index(segments, "Interface") else {
        return RootedNonAddonClassification::None;
    };
    let rest = &segments[interface_index + 1..];
    if rest.is_empty() {
        return RootedNonAddonClassification::None;
    }
    if rest[0].eq_ignore_ascii_case("AddOns") {
        return RootedNonAddonClassification::None;
    }

    RootedNonAddonClassification::Recognized(join_exact_normalized_segments(&["interface"], rest))
}

fn classify_rooted_path(
    segments: &[String],
    root_name: &str,
    normalized_prefix: &[&str],
) -> Option<String> {
    let rooted_index = find_segment_index(segments, root_name)?;
    let rest = &segments[rooted_index + 1..];
    if rest.is_empty() {
        return None;
    }

    Some(join_exact_normalized_segments(normalized_prefix, rest))
}

fn build_normalized_external_entry(
    source_path: &str,
    normalized_path: String,
) -> AppResult<ExternalPackageEntry> {
    let Some(classified_entry) = classify_bundle_archive_entry(&normalized_path)? else {
        return Err(AppError::Validation(format!(
            "normalized external package path does not match bundle layout: {normalized_path}"
        )));
    };
    let metadata = classified_entry.metadata();
    let group = metadata.group;
    let wtf_scope = metadata.wtf_scope;
    let source_account = metadata.source_account.map(str::to_string);
    let source_server = metadata.source_server.map(str::to_string);
    let source_character = metadata.source_character.map(str::to_string);

    Ok(ExternalPackageEntry {
        source_path: source_path.to_string(),
        normalized_path,
        group,
        wtf_scope,
        source_account,
        source_server,
        source_character,
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

fn build_addon_root_not_detected_warning(source_path: &str) -> ExternalPackageWarning {
    build_external_package_warning(
        ExternalPackageWarningCategory::Addon,
        ExternalPackageWarningCode::AddonRootNotDetected,
        source_path,
        format!(
            "entry is under `AddOns` but no addon root was detected from a `.toc` file: {source_path}"
        ),
    )
}

fn build_wtf_warning(source_path: &str, warning_kind: WtfWarningKind) -> ExternalPackageWarning {
    let (code, message) = match warning_kind {
        WtfWarningKind::UnsupportedLayout => (
            ExternalPackageWarningCode::UnsupportedWtfLayout,
            format!(
                "WTF path does not match a supported account or character layout: {source_path}"
            ),
        ),
        WtfWarningKind::RootSavedVariablesPathWithoutFile => (
            ExternalPackageWarningCode::WtfSavedVariablesPathWithoutFile,
            format!(
                "root-level `WTF/Account/SavedVariables` entry does not point to a file: {source_path}"
            ),
        ),
        WtfWarningKind::AccountPathWithoutFile => (
            ExternalPackageWarningCode::WtfAccountPathWithoutFile,
            format!("WTF account entry does not point to a file path: {source_path}"),
        ),
        WtfWarningKind::AccountSavedVariablesPathWithoutFile => (
            ExternalPackageWarningCode::WtfSavedVariablesPathWithoutFile,
            format!("WTF account `SavedVariables` entry does not point to a file: {source_path}"),
        ),
        WtfWarningKind::UnsupportedNestedAccountLayout => (
            ExternalPackageWarningCode::UnsupportedWtfNestedAccountLayout,
            format!(
                "WTF path is nested under an account but does not match a supported file layout: {source_path}"
            ),
        ),
    };

    build_external_package_warning(
        ExternalPackageWarningCategory::Wtf,
        code,
        source_path,
        message,
    )
}

fn find_addon_root<'a>(
    segments: &[String],
    addon_roots: &'a [Vec<String>],
) -> Option<&'a [String]> {
    let mut case_insensitive_match = None;

    for root in addon_roots {
        match addon_root_prefix_match_kind(segments, root, HostPlatform::Windows) {
            Some(AddonRootPrefixMatchKind::Exact) => return Some(root.as_slice()),
            Some(AddonRootPrefixMatchKind::CaseInsensitive) if case_insensitive_match.is_none() => {
                case_insensitive_match = Some(root.as_slice());
            }
            Some(AddonRootPrefixMatchKind::CaseInsensitive) => {}
            None => {}
        }
    }

    case_insensitive_match
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

#[cfg(test)]
mod tests {
    use super::{
        AddonPathClassification, RootedNonAddonClassification, WtfSuffixClassification,
        WtfWarningKind, classify_addon_path, classify_rooted_non_addon_path, classify_wtf_suffix,
    };

    #[test]
    fn classify_addon_path_recognizes_addon_rooted_entry() {
        let roots = vec![vec![
            "AuthorUI".to_string(),
            "Interface".to_string(),
            "AddOns".to_string(),
            "WeakAuras".to_string(),
        ]];
        let segments = segments(&[
            "AuthorUI",
            "Interface",
            "AddOns",
            "WeakAuras",
            "WeakAuras.toc",
        ]);

        match classify_addon_path(&segments, &roots) {
            AddonPathClassification::Recognized(normalized_path) => {
                assert_eq!(normalized_path, "addons/WeakAuras/WeakAuras.toc");
            }
            AddonPathClassification::MissingRoot => {
                panic!("recognized addon entry should not warn about missing root")
            }
            AddonPathClassification::None => panic!("recognized addon entry should be classified"),
        }
    }

    #[test]
    fn classify_addon_path_marks_addons_subtree_without_root() {
        let segments = segments(&[
            "AuthorUI",
            "Interface",
            "AddOns",
            "BrokenAddon",
            "README.txt",
        ]);

        match classify_addon_path(&segments, &[]) {
            AddonPathClassification::MissingRoot => {}
            AddonPathClassification::Recognized(_) => {
                panic!("missing addon root should not be recognized")
            }
            AddonPathClassification::None => {
                panic!("AddOns subtree without root should request warning")
            }
        }
    }

    #[test]
    fn classify_addon_path_recognizes_case_mixed_subtree_on_windows_portability_floor() {
        let roots = vec![vec![
            "AuthorUI".to_string(),
            "Interface".to_string(),
            "AddOns".to_string(),
            "WeakAuras".to_string(),
        ]];
        let segments = segments(&["AuthorUI", "Interface", "AddOns", "weakauras", "Core.lua"]);

        match classify_addon_path(&segments, &roots) {
            AddonPathClassification::Recognized(normalized_path) => {
                assert_eq!(normalized_path, "addons/WeakAuras/Core.lua");
            }
            AddonPathClassification::MissingRoot => {
                panic!("case-mixed addon subtree should still be recognized")
            }
            AddonPathClassification::None => {
                panic!("case-mixed addon subtree should be classified")
            }
        }
    }

    #[test]
    fn classify_rooted_non_addon_path_recognizes_fonts_layout() {
        let segments = segments(&["AuthorUI", "Fonts", "FRIZQT__.ttf"]);

        match classify_rooted_non_addon_path(&segments) {
            RootedNonAddonClassification::Recognized(normalized_path) => {
                assert_eq!(normalized_path, "fonts/FRIZQT__.ttf");
            }
            RootedNonAddonClassification::None => panic!("fonts layout should be recognized"),
        }
    }

    #[test]
    fn classify_rooted_non_addon_path_recognizes_interface_layout() {
        let segments = segments(&["AuthorUI", "Interface", "SharedXML", "texture.blp"]);

        match classify_rooted_non_addon_path(&segments) {
            RootedNonAddonClassification::Recognized(normalized_path) => {
                assert_eq!(normalized_path, "interface/SharedXML/texture.blp");
            }
            RootedNonAddonClassification::None => {
                panic!("interface asset should be recognized")
            }
        }
    }

    #[test]
    fn classify_rooted_non_addon_path_skips_interface_addons_subtree() {
        let segments = segments(&[
            "AuthorUI",
            "Interface",
            "AddOns",
            "BrokenAddon",
            "README.txt",
        ]);

        match classify_rooted_non_addon_path(&segments) {
            RootedNonAddonClassification::Recognized(_) => {
                panic!("interface addon subtree should not be recognized as non-addon resource")
            }
            RootedNonAddonClassification::None => {}
        }
    }

    #[test]
    fn classify_wtf_suffix_recognizes_root_saved_variables_layout() {
        let suffix = segments(&["WTF", "Account", "SavedVariables", "Broken.lua"]);

        match classify_wtf_suffix(&suffix) {
            WtfSuffixClassification::Recognized(normalized_path) => {
                assert_eq!(normalized_path, "wtf/common/root/SavedVariables/Broken.lua");
            }
            WtfSuffixClassification::Warning(_) => {
                panic!("root saved variables file should be recognized")
            }
        }
    }

    #[test]
    fn classify_wtf_suffix_warns_when_account_saved_variables_has_no_file() {
        let suffix = segments(&["WTF", "Account", "ACCOUNT", "SavedVariables"]);

        match classify_wtf_suffix(&suffix) {
            WtfSuffixClassification::Recognized(_) => {
                panic!("account SavedVariables directory without file should warn")
            }
            WtfSuffixClassification::Warning(
                WtfWarningKind::AccountSavedVariablesPathWithoutFile,
            ) => {}
            WtfSuffixClassification::Warning(_) => {
                panic!("unexpected warning kind for account SavedVariables directory")
            }
        }
    }

    #[test]
    fn classify_wtf_suffix_warns_when_nested_account_layout_is_unsupported() {
        let suffix = segments(&["WTF", "Account", "ACCOUNT", "ServerOnly", "CharacterOnly"]);

        match classify_wtf_suffix(&suffix) {
            WtfSuffixClassification::Recognized(_) => {
                panic!("unsupported nested account layout should warn")
            }
            WtfSuffixClassification::Warning(WtfWarningKind::UnsupportedNestedAccountLayout) => {}
            WtfSuffixClassification::Warning(_) => {
                panic!("unexpected warning kind for unsupported nested account layout")
            }
        }
    }

    fn segments(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }
}
