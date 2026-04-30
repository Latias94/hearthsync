use super::syntax::{
    find_direct_string_key, find_matching_brace, for_each_named_table, for_each_top_level_table,
    parse_bracketed_string_key, parse_key, parse_string_literal, skip_ascii_whitespace,
    visit_direct_table_entries,
};
use super::{CharacterMapping, LuaRewriteOptions};

pub fn rewrite_lua_text(
    content: &str,
    mappings: &[CharacterMapping],
    options: LuaRewriteOptions,
) -> String {
    let mut rewritten = content.to_string();

    if options.rewrite_profile_keys {
        rewritten = rewrite_scoped_profile_text(&rewritten, mappings);
    }

    if options.rewrite_identity_strings {
        rewritten = rewrite_scoped_identity_text(&rewritten, mappings);
    }

    rewritten
}

fn rewrite_scoped_identity_text(content: &str, mappings: &[CharacterMapping]) -> String {
    let rewrites = build_identity_string_rewrites(mappings);
    if rewrites.is_empty() {
        return content.to_string();
    }

    let mut replacements = Vec::new();
    collect_identity_key_replacements(content, mappings, &rewrites, &mut replacements);
    collect_identity_field_value_replacements(content, &rewrites, &mut replacements);
    collect_name_realm_pair_replacements(content, mappings, &mut replacements);

    apply_range_replacements(content, replacements)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdentityRewriteKind {
    Character,
    Server,
    Combined,
}

#[derive(Debug, Clone)]
struct IdentityStringRewrite {
    source: String,
    target: String,
    kind: IdentityRewriteKind,
}

fn build_identity_string_rewrites(mappings: &[CharacterMapping]) -> Vec<IdentityStringRewrite> {
    let mut rewrites = Vec::new();
    for mapping in mappings {
        push_identity_string_rewrite(
            &mut rewrites,
            mapping.source_profile_key(),
            mapping.target_profile_key(),
            IdentityRewriteKind::Combined,
        );
        push_identity_string_rewrite(
            &mut rewrites,
            mapping.source_compact_identity_key(),
            mapping.target_compact_identity_key(),
            IdentityRewriteKind::Combined,
        );
        push_identity_string_rewrite(
            &mut rewrites,
            mapping.source_reverse_compact_identity_key(),
            mapping.target_reverse_compact_identity_key(),
            IdentityRewriteKind::Combined,
        );
        push_identity_string_rewrite(
            &mut rewrites,
            mapping.source_character.clone(),
            mapping.target_character.clone(),
            IdentityRewriteKind::Character,
        );
        push_identity_string_rewrite(
            &mut rewrites,
            mapping.source_server.clone(),
            mapping.target_server.clone(),
            IdentityRewriteKind::Server,
        );
    }

    rewrites
}

fn push_identity_string_rewrite(
    rewrites: &mut Vec<IdentityStringRewrite>,
    source: String,
    target: String,
    kind: IdentityRewriteKind,
) {
    if !source.is_empty()
        && source != target
        && !rewrites
            .iter()
            .any(|rewrite| rewrite.source == source && rewrite.target == target)
    {
        rewrites.push(IdentityStringRewrite {
            source,
            target,
            kind,
        });
    }
}

fn collect_identity_key_replacements(
    content: &str,
    mappings: &[CharacterMapping],
    rewrites: &[IdentityStringRewrite],
    replacements: &mut Vec<TextRangeReplacement>,
) {
    const CHARACTER: &[IdentityRewriteKind] = &[IdentityRewriteKind::Character];
    const SERVER: &[IdentityRewriteKind] = &[IdentityRewriteKind::Server];
    const COMBINED: &[IdentityRewriteKind] = &[IdentityRewriteKind::Combined];

    collect_top_level_identity_key_replacements(content, COMBINED, rewrites, replacements);
    collect_top_level_realm_character_map_replacements(content, mappings, replacements);

    for table_name in [
        "char",
        "profileKeys",
        "profiles",
        "searchHistoryList",
        "Toons",
        "value",
        "worldBoss",
    ] {
        collect_named_table_identity_key_replacements(
            content,
            table_name,
            COMBINED,
            None,
            rewrites,
            replacements,
        );
    }

    collect_named_table_identity_key_replacements(
        content,
        "currentrealm",
        CHARACTER,
        Some(SERVER),
        rewrites,
        replacements,
    );
    collect_named_table_identity_key_replacements(
        content,
        "totals",
        SERVER,
        None,
        rewrites,
        replacements,
    );
    collect_named_realm_character_map_replacements(content, "faction", mappings, replacements);
}

fn collect_top_level_identity_key_replacements(
    content: &str,
    allowed_kinds: &[IdentityRewriteKind],
    rewrites: &[IdentityStringRewrite],
    replacements: &mut Vec<TextRangeReplacement>,
) {
    let bytes = content.as_bytes();
    for_each_top_level_table(content.as_bytes(), |start, end| {
        visit_direct_table_entries(content.as_bytes(), start, end, |key, value_start| {
            if bytes.get(value_start) != Some(&b'{') {
                return;
            }
            if let Some(rewrite) = find_identity_rewrite_with_kinds(
                &content[key.name_start..key.name_end],
                rewrites,
                allowed_kinds,
            ) {
                replacements.push(TextRangeReplacement {
                    start: key.name_start,
                    end: key.name_end,
                    replacement: rewrite.target.clone(),
                });
            }
        });
    });
}

fn collect_top_level_realm_character_map_replacements(
    content: &str,
    mappings: &[CharacterMapping],
    replacements: &mut Vec<TextRangeReplacement>,
) {
    for_each_top_level_table(content.as_bytes(), |start, end| {
        collect_direct_realm_character_map_replacements(
            content,
            start,
            end,
            mappings,
            replacements,
        );
    });
}

fn collect_named_table_identity_key_replacements(
    content: &str,
    table_name: &str,
    key_allowed_kinds: &[IdentityRewriteKind],
    value_allowed_kinds: Option<&[IdentityRewriteKind]>,
    rewrites: &[IdentityStringRewrite],
    replacements: &mut Vec<TextRangeReplacement>,
) {
    for_each_named_table(content.as_bytes(), table_name.as_bytes(), |start, end| {
        collect_direct_identity_key_replacements(
            content,
            start,
            end,
            key_allowed_kinds,
            value_allowed_kinds,
            rewrites,
            replacements,
        );
    });
}

fn collect_named_realm_character_map_replacements(
    content: &str,
    table_name: &str,
    mappings: &[CharacterMapping],
    replacements: &mut Vec<TextRangeReplacement>,
) {
    for_each_named_table(content.as_bytes(), table_name.as_bytes(), |start, end| {
        collect_direct_realm_character_map_replacements(
            content,
            start,
            end,
            mappings,
            replacements,
        );
    });
}

fn collect_direct_identity_key_replacements(
    content: &str,
    start: usize,
    end: usize,
    key_allowed_kinds: &[IdentityRewriteKind],
    value_allowed_kinds: Option<&[IdentityRewriteKind]>,
    rewrites: &[IdentityStringRewrite],
    replacements: &mut Vec<TextRangeReplacement>,
) {
    visit_direct_table_entries(content.as_bytes(), start, end, |key, value_start| {
        if let Some(rewrite) = find_identity_rewrite_with_kinds(
            &content[key.name_start..key.name_end],
            rewrites,
            key_allowed_kinds,
        ) {
            replacements.push(TextRangeReplacement {
                start: key.name_start,
                end: key.name_end,
                replacement: rewrite.target.clone(),
            });
        }

        let Some(value_allowed_kinds) = value_allowed_kinds else {
            return;
        };
        let Some(value) = parse_string_literal(content.as_bytes(), value_start) else {
            return;
        };
        if let Some(rewrite) = find_identity_rewrite_with_kinds(
            &content[value.content_start..value.content_end],
            rewrites,
            value_allowed_kinds,
        ) {
            replacements.push(TextRangeReplacement {
                start: value.content_start,
                end: value.content_end,
                replacement: rewrite.target.clone(),
            });
        }
    });
}

fn collect_direct_realm_character_map_replacements(
    content: &str,
    start: usize,
    end: usize,
    mappings: &[CharacterMapping],
    replacements: &mut Vec<TextRangeReplacement>,
) {
    visit_direct_table_entries(content.as_bytes(), start, end, |realm_key, value_start| {
        let realm = &content[realm_key.name_start..realm_key.name_end];
        for mapping in mappings
            .iter()
            .filter(|mapping| mapping.source_server == realm)
        {
            let Some(character_table_end) = find_matching_brace(content.as_bytes(), value_start)
            else {
                continue;
            };
            let Some(character_key) = find_direct_string_key(
                content.as_bytes(),
                value_start + 1,
                character_table_end,
                mapping.source_character.as_bytes(),
            ) else {
                continue;
            };

            replacements.push(TextRangeReplacement {
                start: realm_key.name_start,
                end: realm_key.name_end,
                replacement: mapping.target_server.clone(),
            });
            replacements.push(TextRangeReplacement {
                start: character_key.name_start,
                end: character_key.name_end,
                replacement: mapping.target_character.clone(),
            });
        }
    });
}

fn collect_identity_field_value_replacements(
    content: &str,
    rewrites: &[IdentityStringRewrite],
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
        let Some(allowed_kinds) = identity_field_rewrite_kinds(key_name) else {
            index = key.full_end.max(index + 1);
            continue;
        };

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

        if let Some(rewrite) = find_identity_rewrite_with_kinds(
            &content[value.content_start..value.content_end],
            rewrites,
            allowed_kinds,
        ) {
            replacements.push(TextRangeReplacement {
                start: value.content_start,
                end: value.content_end,
                replacement: rewrite.target.clone(),
            });
        }

        index = value.full_end;
    }
}

fn identity_field_rewrite_kinds(key_name: &str) -> Option<&'static [IdentityRewriteKind]> {
    const CHARACTER: &[IdentityRewriteKind] = &[IdentityRewriteKind::Character];
    const SERVER: &[IdentityRewriteKind] = &[IdentityRewriteKind::Server];
    const COMBINED: &[IdentityRewriteKind] = &[IdentityRewriteKind::Combined];

    if key_name.ends_with("profileKey") {
        return Some(COMBINED);
    }

    match key_name {
        "playerName" | "character" | "LastPlayerFullName" => Some(CHARACTER),
        "realm" | "realmName" | "server" | "guildrealm" | "realmKey" | "rwsKey" | "LastRealm" => {
            Some(SERVER)
        }
        _ => None,
    }
}

fn collect_name_realm_pair_replacements(
    content: &str,
    mappings: &[CharacterMapping],
    replacements: &mut Vec<TextRangeReplacement>,
) {
    let bytes = content.as_bytes();
    collect_name_realm_pair_replacements_in_range(
        content,
        bytes,
        0,
        bytes.len(),
        mappings,
        replacements,
    );
}

fn collect_name_realm_pair_replacements_in_range(
    content: &str,
    bytes: &[u8],
    start: usize,
    end: usize,
    mappings: &[CharacterMapping],
    replacements: &mut Vec<TextRangeReplacement>,
) {
    let mut index = start;
    while index < end {
        if let Some(literal) = parse_string_literal(bytes, index) {
            index = literal.full_end;
            continue;
        }

        if bytes[index] != b'{' {
            index += 1;
            continue;
        }

        let Some(table_end) = find_matching_brace(bytes, index) else {
            index += 1;
            continue;
        };

        collect_direct_name_realm_pair_replacements(
            content,
            index + 1,
            table_end,
            mappings,
            replacements,
        );
        collect_name_realm_pair_replacements_in_range(
            content,
            bytes,
            index + 1,
            table_end,
            mappings,
            replacements,
        );
        index = table_end + 1;
    }
}

#[derive(Debug, Clone, Copy)]
struct DirectStringField {
    value_start: usize,
    value_end: usize,
}

fn collect_direct_name_realm_pair_replacements(
    content: &str,
    start: usize,
    end: usize,
    mappings: &[CharacterMapping],
    replacements: &mut Vec<TextRangeReplacement>,
) {
    let mut name_field = None;
    let mut realm_name_field = None;

    collect_direct_string_fields(content, start, end, |field_name, field| match field_name {
        "name" => name_field = Some(field),
        "realmName" => realm_name_field = Some(field),
        _ => {}
    });

    let (Some(name_field), Some(realm_name_field)) = (name_field, realm_name_field) else {
        return;
    };

    let name = &content[name_field.value_start..name_field.value_end];
    let realm_name = &content[realm_name_field.value_start..realm_name_field.value_end];
    let Some(mapping) = mappings
        .iter()
        .find(|mapping| mapping.source_character == name && mapping.source_server == realm_name)
    else {
        return;
    };

    replacements.push(TextRangeReplacement {
        start: name_field.value_start,
        end: name_field.value_end,
        replacement: mapping.target_character.clone(),
    });
    replacements.push(TextRangeReplacement {
        start: realm_name_field.value_start,
        end: realm_name_field.value_end,
        replacement: mapping.target_server.clone(),
    });
}

fn collect_direct_string_fields(
    content: &str,
    start: usize,
    end: usize,
    mut collect: impl FnMut(&str, DirectStringField),
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

        let Some(key) = parse_key(bytes, index) else {
            index += 1;
            continue;
        };

        let mut value_start = skip_ascii_whitespace(bytes, key.full_end);
        if value_start >= end || bytes[value_start] != b'=' {
            index = key.full_end.max(index + 1);
            continue;
        }

        value_start = skip_ascii_whitespace(bytes, value_start + 1);
        let Some(value) = parse_string_literal(bytes, value_start) else {
            index = key.full_end.max(index + 1);
            continue;
        };

        collect(
            &content[key.name_start..key.name_end],
            DirectStringField {
                value_start: value.content_start,
                value_end: value.content_end,
            },
        );

        index = value.full_end;
    }
}

fn find_identity_rewrite_with_kinds<'a>(
    value: &str,
    rewrites: &'a [IdentityStringRewrite],
    allowed_kinds: &[IdentityRewriteKind],
) -> Option<&'a IdentityStringRewrite> {
    rewrites
        .iter()
        .find(|rewrite| rewrite.source == value && allowed_kinds.contains(&rewrite.kind))
}

fn rewrite_scoped_profile_text(content: &str, mappings: &[CharacterMapping]) -> String {
    let rewrites = build_profile_string_rewrites(mappings);
    if rewrites.is_empty() {
        return content.to_string();
    }

    let mut replacements = Vec::new();
    collect_direct_table_replacements(content, "profileKeys", true, &rewrites, &mut replacements);
    collect_direct_table_replacements(content, "profiles", false, &rewrites, &mut replacements);
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

#[derive(Debug, Clone)]
struct TextRangeReplacement {
    start: usize,
    end: usize,
    replacement: String,
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
        if !key_name.ends_with("profileKey") {
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

fn apply_range_replacements(content: &str, mut replacements: Vec<TextRangeReplacement>) -> String {
    if replacements.is_empty() {
        return content.to_string();
    }

    replacements.sort_by(|left, right| {
        right
            .start
            .cmp(&left.start)
            .then_with(|| right.end.cmp(&left.end))
            .then_with(|| left.replacement.cmp(&right.replacement))
    });

    let mut filtered = Vec::new();
    for replacement in replacements {
        if filtered.iter().any(|existing: &TextRangeReplacement| {
            existing.start == replacement.start
                && existing.end == replacement.end
                && existing.replacement == replacement.replacement
        }) {
            continue;
        }

        if let Some(previous) = filtered.last()
            && replacement.end > previous.start
        {
            continue;
        }

        filtered.push(replacement);
    }

    let mut rewritten = content.to_string();
    for replacement in filtered {
        rewritten.replace_range(replacement.start..replacement.end, &replacement.replacement);
    }

    rewritten
}

fn push_replacement(replacements: &mut Vec<(String, String)>, from: String, to: String) {
    if !from.is_empty() && from != to {
        replacements.push((from, to));
    }
}
