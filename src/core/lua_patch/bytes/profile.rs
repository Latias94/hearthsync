use super::super::CharacterMapping;
use super::super::syntax::{
    find_matching_brace, parse_bracketed_string_key, parse_key, parse_string_literal,
    skip_ascii_whitespace,
};
use super::range::ByteRangeReplacement;
use super::rewrite::{ByteRewriteKind, ByteStringRewrite, find_byte_rewrite, push_byte_rewrite};
pub(super) fn collect_profile_replacements(
    content: &[u8],
    mappings: &[CharacterMapping],
    replacements: &mut Vec<ByteRangeReplacement>,
) {
    let rewrites = build_profile_byte_rewrites(mappings);
    if rewrites.is_empty() {
        return;
    }

    for table_name in [b"profileKeys".as_slice(), b"ProfileKeys"] {
        collect_direct_table_replacements(content, table_name, true, &rewrites, replacements);
    }
    for table_name in [b"profiles".as_slice(), b"Profiles"] {
        collect_direct_table_replacements(content, table_name, false, &rewrites, replacements);
    }
    collect_profile_key_field_value_replacements(content, &rewrites, replacements);
}

fn build_profile_byte_rewrites(mappings: &[CharacterMapping]) -> Vec<ByteStringRewrite> {
    let mut rewrites = Vec::new();
    for mapping in mappings {
        push_byte_rewrite(
            &mut rewrites,
            mapping.source_profile_key(),
            mapping.target_profile_key(),
            ByteRewriteKind::Combined,
        );
        push_byte_rewrite(
            &mut rewrites,
            format!(
                "Default.{}.{}",
                mapping.source_server, mapping.source_character
            ),
            format!(
                "Default.{}.{}",
                mapping.target_server, mapping.target_character
            ),
            ByteRewriteKind::Combined,
        );
        push_byte_rewrite(
            &mut rewrites,
            format!("{}.{}", mapping.source_server, mapping.source_character),
            format!("{}.{}", mapping.target_server, mapping.target_character),
            ByteRewriteKind::Combined,
        );
    }

    rewrites
}

fn collect_direct_table_replacements(
    content: &[u8],
    table_name: &[u8],
    rewrite_values: bool,
    rewrites: &[ByteStringRewrite],
    replacements: &mut Vec<ByteRangeReplacement>,
) {
    let mut index = 0usize;

    while index < content.len() {
        if let Some(literal) = parse_string_literal(content, index) {
            index = literal.full_end;
            continue;
        }

        let Some(key) = parse_key(content, index) else {
            index += 1;
            continue;
        };

        let mut value_start = skip_ascii_whitespace(content, key.full_end);
        if value_start >= content.len() || content[value_start] != b'=' {
            index = key.full_end.max(index + 1);
            continue;
        }

        value_start = skip_ascii_whitespace(content, value_start + 1);
        if value_start >= content.len() || content[value_start] != b'{' {
            index = key.full_end.max(index + 1);
            continue;
        }

        if content[key.name_start..key.name_end] == *table_name
            && let Some(table_end) = find_matching_brace(content, value_start)
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
    content: &[u8],
    start: usize,
    end: usize,
    rewrite_values: bool,
    rewrites: &[ByteStringRewrite],
    replacements: &mut Vec<ByteRangeReplacement>,
) {
    let mut index = start;
    let mut depth = 0usize;

    while index < end {
        if let Some(literal) = parse_string_literal(content, index) {
            index = literal.full_end;
            continue;
        }

        match content[index] {
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

        let Some(key) = parse_bracketed_string_key(content, index) else {
            index += 1;
            continue;
        };

        let mut value_start = skip_ascii_whitespace(content, key.full_end);
        if value_start >= end || content[value_start] != b'=' {
            index = key.full_end.max(index + 1);
            continue;
        }

        if let Some(rewrite) = find_byte_rewrite(&content[key.name_start..key.name_end], rewrites) {
            replacements.push(ByteRangeReplacement {
                start: key.name_start,
                end: key.name_end,
                replacement: rewrite.target.clone(),
            });
        }

        if rewrite_values {
            value_start = skip_ascii_whitespace(content, value_start + 1);
            if let Some(value) = parse_string_literal(content, value_start) {
                if let Some(rewrite) =
                    find_byte_rewrite(&content[value.content_start..value.content_end], rewrites)
                {
                    replacements.push(ByteRangeReplacement {
                        start: value.content_start,
                        end: value.content_end,
                        replacement: rewrite.target.clone(),
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
    content: &[u8],
    rewrites: &[ByteStringRewrite],
    replacements: &mut Vec<ByteRangeReplacement>,
) {
    let mut index = 0usize;

    while index < content.len() {
        if let Some(literal) = parse_string_literal(content, index) {
            index = literal.full_end;
            continue;
        }

        let Some(key) = parse_key(content, index) else {
            index += 1;
            continue;
        };

        let key_name = &content[key.name_start..key.name_end];
        if !key_name.ends_with(b"profileKey") && !key_name.ends_with(b"ProfileKey") {
            index = key.full_end.max(index + 1);
            continue;
        }

        let mut value_start = skip_ascii_whitespace(content, key.full_end);
        if value_start >= content.len() || content[value_start] != b'=' {
            index = key.full_end.max(index + 1);
            continue;
        }

        value_start = skip_ascii_whitespace(content, value_start + 1);
        let Some(value) = parse_string_literal(content, value_start) else {
            index = key.full_end.max(index + 1);
            continue;
        };

        if let Some(rewrite) =
            find_byte_rewrite(&content[value.content_start..value.content_end], rewrites)
        {
            replacements.push(ByteRangeReplacement {
                start: value.content_start,
                end: value.content_end,
                replacement: rewrite.target.clone(),
            });
        }

        index = value.full_end;
    }
}
