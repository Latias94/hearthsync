use super::*;

pub(super) fn classify_account_wtf_scope(relative_segments: &[&str]) -> WtfScope {
    if relative_segments.is_empty() {
        return WtfScope::Unknown;
    }

    if is_saved_variables_segment(relative_segments[0]) {
        WtfScope::AccountSavedVariables
    } else if relative_segments
        .last()
        .is_some_and(|name| is_cache_like_wtf_file_name(name))
    {
        WtfScope::CacheLike
    } else {
        WtfScope::AccountRootFile
    }
}

pub(super) fn classify_character_wtf_scope(relative_segments: &[&str]) -> WtfScope {
    if relative_segments.is_empty() {
        return WtfScope::Unknown;
    }

    if is_saved_variables_segment(relative_segments[0]) {
        WtfScope::CharacterSavedVariables
    } else if relative_segments
        .last()
        .is_some_and(|name| is_cache_like_wtf_file_name(name))
    {
        WtfScope::CacheLike
    } else {
        WtfScope::CharacterState
    }
}

fn is_saved_variables_segment(segment: &str) -> bool {
    segment.eq_ignore_ascii_case("SavedVariables")
}

fn is_cache_like_wtf_file_name(file_name: &str) -> bool {
    let file_name = file_name.to_ascii_lowercase();
    matches!(
        file_name.as_str(),
        "bindings-cache.wtf" | "chat-cache.txt" | "config-cache.wtf" | "macros-cache.txt"
    ) || file_name.ends_with("-cache.wtf")
        || file_name.ends_with("-cache.txt")
        || file_name.ends_with("-cache.old")
}
