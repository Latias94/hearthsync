use super::super::CharacterMapping;
use super::super::syntax::{
    find_matching_brace, parse_bracketed_string_key, parse_key, parse_string_literal,
    skip_ascii_whitespace,
};
use super::range::{TextRangeReplacement, apply_range_replacements};
pub(super) fn rewrite_scoped_profile_text(content: &str, mappings: &[CharacterMapping]) -> String {
    let rewrites = build_profile_string_rewrites(mappings);
    if rewrites.is_empty() {
        return content.to_string();
    }

    let mut replacements = Vec::new();
    for table_name in ["profileKeys", "ProfileKeys"] {
        collect_direct_table_replacements(content, table_name, true, &rewrites, &mut replacements);
    }
    for table_name in ["profiles", "Profiles"] {
        collect_direct_table_replacements(content, table_name, false, &rewrites, &mut replacements);
    }
    collect_profile_key_field_value_replacements(content, &rewrites, &mut replacements);

    apply_range_replacements(content, replacements)
}

fn build_profile_string_rewrites(mappings: &[CharacterMapping]) -> Vec<(String, String)> {
    let mut rewrites = Vec::new();
    for mapping in mappings {
        push_replacement(
            &mut rewrites,
            mapping.source_profile_key(),
            mapping.target_profile_key(),
        );
        push_replacement(
            &mut rewrites,
            format!(
                "Default.{}.{}",
                mapping.source_server, mapping.source_character
            ),
            format!(
                "Default.{}.{}",
                mapping.target_server, mapping.target_character
            ),
        );
        push_replacement(
            &mut rewrites,
            format!("{}.{}", mapping.source_server, mapping.source_character),
            format!("{}.{}", mapping.target_server, mapping.target_character),
        );
    }

    rewrites
}

fn collect_direct_table_replacements(
    content: &str,
    table_name: &str,
    rewrite_values: bool,
    rewrites: &[(String, String)],
    replacements: &mut Vec<TextRangeReplacement>,
) {
    let bytes = content.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        if let Some(literal) = parse_string_literal(bytes, index) {
            index = literal.full_end;
            continue;
        }

        let Some(key) = parse_key(bytes, index) else {
            index += 1;
            continue;
        };

        let mut value_start = skip_ascii_whitespace(bytes, key.full_end);
        if value_start >= bytes.len() || bytes[value_start] != b'=' {
            index = key.full_end.max(index + 1);
            continue;
        }

        value_start = skip_ascii_whitespace(bytes, value_start + 1);
        if value_start >= bytes.len() || bytes[value_start] != b'{' {
            index = key.full_end.max(index + 1);
            continue;
        }

        if &content[key.name_start..key.name_end] == table_name
            && let Some(table_end) = find_matching_brace(bytes, value_start)
        {
            collect_direct_child_table_replacements(
                content,
                value_start + 1,
                table_end,
                rewrite_values,
                rewrites,
                replacements,
            );
        }

        index = key.full_end.max(index + 1);
    }
}

fn collect_direct_child_table_replacements(
    content: &str,
    start: usize,
    end: usize,
    rewrite_values: bool,
    rewrites: &[(String, String)],
    replacements: &mut Vec<TextRangeReplacement>,
) {
    let bytes = content.as_bytes();
    let mut index = start;
    let mut depth = 0usize;

    while index < end {
        if let Some(literal) = parse_string_literal(bytes, index) {
            index = literal.full_end;
            continue;
        }

        match bytes[index] {
            b'{' => {
                depth += 1;
                index += 1;
                continue;
            }
            b'}' => {
                depth = depth.saturating_sub(1);
                index += 1;
                continue;
            }
            _ => {}
        }

        if depth > 0 {
            index += 1;
            continue;
        }

        let Some(key) = parse_bracketed_string_key(bytes, index) else {
            index += 1;
            continue;
        };

        let mut value_start = skip_ascii_whitespace(bytes, key.full_end);
        if value_start >= end || bytes[value_start] != b'=' {
            index = key.full_end.max(index + 1);
            continue;
        }

        if let Some(replacement) =
            find_string_rewrite(&content[key.name_start..key.name_end], rewrites)
        {
            replacements.push(TextRangeReplacement {
                start: key.name_start,
                end: key.name_end,
                replacement: replacement.to_string(),
            });
        }

        if rewrite_values {
            value_start = skip_ascii_whitespace(bytes, value_start + 1);
            if let Some(value) = parse_string_literal(bytes, value_start) {
                if let Some(replacement) =
                    find_string_rewrite(&content[value.content_start..value.content_end], rewrites)
                {
                    replacements.push(TextRangeReplacement {
                        start: value.content_start,
                        end: value.content_end,
                        replacement: replacement.to_string(),
                    });
                }
                index = value.full_end;
                continue;
            }
        }

        index = key.full_end.max(index + 1);
    }
}

fn collect_profile_key_field_value_replacements(
    content: &str,
    rewrites: &[(String, String)],
    replacements: &mut Vec<TextRangeReplacement>,
) {
    let bytes = content.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        if let Some(literal) = parse_string_literal(bytes, index) {
            index = literal.full_end;
            continue;
        }

        let Some(key) = parse_key(bytes, index) else {
            index += 1;
            continue;
        };

        let key_name = &content[key.name_start..key.name_end];
        if !key_name.ends_with("profileKey") && !key_name.ends_with("ProfileKey") {
            index = key.full_end.max(index + 1);
            continue;
        }

        let mut value_start = skip_ascii_whitespace(bytes, key.full_end);
        if value_start >= bytes.len() || bytes[value_start] != b'=' {
            index = key.full_end.max(index + 1);
            continue;
        }

        value_start = skip_ascii_whitespace(bytes, value_start + 1);
        let Some(value) = parse_string_literal(bytes, value_start) else {
            index = key.full_end.max(index + 1);
            continue;
        };

        if let Some(replacement) =
            find_string_rewrite(&content[value.content_start..value.content_end], rewrites)
        {
            replacements.push(TextRangeReplacement {
                start: value.content_start,
                end: value.content_end,
                replacement: replacement.to_string(),
            });
        }

        index = value.full_end;
    }
}

fn find_string_rewrite<'a>(value: &str, rewrites: &'a [(String, String)]) -> Option<&'a str> {
    rewrites
        .iter()
        .find(|(source, _)| source == value)
        .map(|(_, target)| target.as_str())
}

fn push_replacement(replacements: &mut Vec<(String, String)>, from: String, to: String) {
    if !from.is_empty() && from != to {
        replacements.push((from, to));
    }
}
