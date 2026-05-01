use super::super::CharacterMapping;
use super::super::syntax::{
    find_direct_string_key, find_matching_brace, for_each_named_table, for_each_top_level_table,
    parse_key, parse_string_literal, skip_ascii_whitespace, visit_direct_table_entries,
};
use super::encoding::encoded_mapping_variants;
use super::range::ByteRangeReplacement;
use super::rewrite::{
    ByteRewriteKind, ByteStringRewrite, find_byte_rewrite_with_kinds, push_byte_rewrite,
};
pub(super) fn collect_identity_replacements(
    content: &[u8],
    mappings: &[CharacterMapping],
    replacements: &mut Vec<ByteRangeReplacement>,
) {
    let rewrites = build_identity_byte_rewrites(mappings);
    if rewrites.is_empty() {
        return;
    }

    collect_identity_key_replacements(content, mappings, &rewrites, replacements);
    collect_identity_field_value_replacements(content, &rewrites, replacements);
    collect_name_realm_pair_replacements(content, mappings, replacements);
}

fn build_identity_byte_rewrites(mappings: &[CharacterMapping]) -> Vec<ByteStringRewrite> {
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
            mapping.source_compact_identity_key(),
            mapping.target_compact_identity_key(),
            ByteRewriteKind::Combined,
        );
        push_byte_rewrite(
            &mut rewrites,
            mapping.source_reverse_compact_identity_key(),
            mapping.target_reverse_compact_identity_key(),
            ByteRewriteKind::Combined,
        );
        push_byte_rewrite(
            &mut rewrites,
            mapping.source_character.clone(),
            mapping.target_character.clone(),
            ByteRewriteKind::Character,
        );
        push_byte_rewrite(
            &mut rewrites,
            mapping.source_server.clone(),
            mapping.target_server.clone(),
            ByteRewriteKind::Server,
        );
    }

    rewrites
}

fn collect_identity_key_replacements(
    content: &[u8],
    mappings: &[CharacterMapping],
    rewrites: &[ByteStringRewrite],
    replacements: &mut Vec<ByteRangeReplacement>,
) {
    const CHARACTER: &[ByteRewriteKind] = &[ByteRewriteKind::Character];
    const SERVER: &[ByteRewriteKind] = &[ByteRewriteKind::Server];
    const COMBINED: &[ByteRewriteKind] = &[ByteRewriteKind::Combined];

    collect_top_level_identity_key_replacements(content, COMBINED, rewrites, replacements);
    collect_top_level_realm_character_map_replacements(content, mappings, replacements);

    for table_name in [
        b"char".as_slice(),
        b"profileKeys",
        b"profiles",
        b"searchHistoryList",
        b"Toons",
        b"value",
        b"worldBoss",
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
        b"currentrealm",
        CHARACTER,
        Some(SERVER),
        rewrites,
        replacements,
    );
    collect_named_table_identity_key_replacements(
        content,
        b"totals",
        SERVER,
        None,
        rewrites,
        replacements,
    );
    collect_named_realm_character_map_replacements(content, b"faction", mappings, replacements);
}

fn collect_top_level_identity_key_replacements(
    content: &[u8],
    allowed_kinds: &[ByteRewriteKind],
    rewrites: &[ByteStringRewrite],
    replacements: &mut Vec<ByteRangeReplacement>,
) {
    for_each_top_level_table(content, |start, end| {
        visit_direct_table_entries(content, start, end, |key, value_start| {
            if content.get(value_start) != Some(&b'{') {
                return;
            }
            if let Some(rewrite) = find_byte_rewrite_with_kinds(
                &content[key.name_start..key.name_end],
                rewrites,
                allowed_kinds,
            ) {
                replacements.push(ByteRangeReplacement {
                    start: key.name_start,
                    end: key.name_end,
                    replacement: rewrite.target.clone(),
                });
            }
        });
    });
}

fn collect_top_level_realm_character_map_replacements(
    content: &[u8],
    mappings: &[CharacterMapping],
    replacements: &mut Vec<ByteRangeReplacement>,
) {
    for_each_top_level_table(content, |start, end| {
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
    content: &[u8],
    table_name: &[u8],
    key_allowed_kinds: &[ByteRewriteKind],
    value_allowed_kinds: Option<&[ByteRewriteKind]>,
    rewrites: &[ByteStringRewrite],
    replacements: &mut Vec<ByteRangeReplacement>,
) {
    for_each_named_table(content, table_name, |start, end| {
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
    content: &[u8],
    table_name: &[u8],
    mappings: &[CharacterMapping],
    replacements: &mut Vec<ByteRangeReplacement>,
) {
    for_each_named_table(content, table_name, |start, end| {
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
    content: &[u8],
    start: usize,
    end: usize,
    key_allowed_kinds: &[ByteRewriteKind],
    value_allowed_kinds: Option<&[ByteRewriteKind]>,
    rewrites: &[ByteStringRewrite],
    replacements: &mut Vec<ByteRangeReplacement>,
) {
    visit_direct_table_entries(content, start, end, |key, value_start| {
        if let Some(rewrite) = find_byte_rewrite_with_kinds(
            &content[key.name_start..key.name_end],
            rewrites,
            key_allowed_kinds,
        ) {
            replacements.push(ByteRangeReplacement {
                start: key.name_start,
                end: key.name_end,
                replacement: rewrite.target.clone(),
            });
        }

        let Some(value_allowed_kinds) = value_allowed_kinds else {
            return;
        };
        let Some(value) = parse_string_literal(content, value_start) else {
            return;
        };
        if let Some(rewrite) = find_byte_rewrite_with_kinds(
            &content[value.content_start..value.content_end],
            rewrites,
            value_allowed_kinds,
        ) {
            replacements.push(ByteRangeReplacement {
                start: value.content_start,
                end: value.content_end,
                replacement: rewrite.target.clone(),
            });
        }
    });
}

fn collect_direct_realm_character_map_replacements(
    content: &[u8],
    start: usize,
    end: usize,
    mappings: &[CharacterMapping],
    replacements: &mut Vec<ByteRangeReplacement>,
) {
    visit_direct_table_entries(content, start, end, |realm_key, value_start| {
        for mapping in mappings {
            for encoded in encoded_mapping_variants(mapping) {
                if content[realm_key.name_start..realm_key.name_end] != encoded.source_server {
                    continue;
                }

                let Some(character_table_end) = find_matching_brace(content, value_start) else {
                    continue;
                };
                let Some(character_key) = find_direct_string_key(
                    content,
                    value_start + 1,
                    character_table_end,
                    &encoded.source_character,
                ) else {
                    continue;
                };

                replacements.push(ByteRangeReplacement {
                    start: realm_key.name_start,
                    end: realm_key.name_end,
                    replacement: encoded.target_server.clone(),
                });
                replacements.push(ByteRangeReplacement {
                    start: character_key.name_start,
                    end: character_key.name_end,
                    replacement: encoded.target_character.clone(),
                });
            }
        }
    });
}

fn collect_identity_field_value_replacements(
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

        let Some(allowed_kinds) =
            identity_field_rewrite_kinds(&content[key.name_start..key.name_end])
        else {
            index = key.full_end.max(index + 1);
            continue;
        };

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

        if let Some(rewrite) = find_byte_rewrite_with_kinds(
            &content[value.content_start..value.content_end],
            rewrites,
            allowed_kinds,
        ) {
            replacements.push(ByteRangeReplacement {
                start: value.content_start,
                end: value.content_end,
                replacement: rewrite.target.clone(),
            });
        }

        index = value.full_end;
    }
}

fn identity_field_rewrite_kinds(key_name: &[u8]) -> Option<&'static [ByteRewriteKind]> {
    const CHARACTER: &[ByteRewriteKind] = &[ByteRewriteKind::Character];
    const SERVER: &[ByteRewriteKind] = &[ByteRewriteKind::Server];
    const COMBINED: &[ByteRewriteKind] = &[ByteRewriteKind::Combined];

    if key_name.ends_with(b"profileKey") {
        return Some(COMBINED);
    }

    match key_name {
        b"playerName" | b"character" | b"LastPlayerFullName" => Some(CHARACTER),
        b"realm" | b"realmName" | b"server" | b"guildrealm" | b"realmKey" | b"rwsKey"
        | b"LastRealm" => Some(SERVER),
        _ => None,
    }
}

fn collect_name_realm_pair_replacements(
    content: &[u8],
    mappings: &[CharacterMapping],
    replacements: &mut Vec<ByteRangeReplacement>,
) {
    collect_name_realm_pair_replacements_in_range(
        content,
        0,
        content.len(),
        mappings,
        replacements,
    );
}

fn collect_name_realm_pair_replacements_in_range(
    content: &[u8],
    start: usize,
    end: usize,
    mappings: &[CharacterMapping],
    replacements: &mut Vec<ByteRangeReplacement>,
) {
    let mut index = start;
    while index < end {
        if let Some(literal) = parse_string_literal(content, index) {
            index = literal.full_end;
            continue;
        }

        if content[index] != b'{' {
            index += 1;
            continue;
        }

        let Some(table_end) = find_matching_brace(content, index) else {
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
    content: &[u8],
    start: usize,
    end: usize,
    mappings: &[CharacterMapping],
    replacements: &mut Vec<ByteRangeReplacement>,
) {
    let mut name_field = None;
    let mut realm_name_field = None;

    collect_direct_string_fields(content, start, end, |field_name, field| match field_name {
        b"name" => name_field = Some(field),
        b"realmName" => realm_name_field = Some(field),
        _ => {}
    });

    let (Some(name_field), Some(realm_name_field)) = (name_field, realm_name_field) else {
        return;
    };

    let name = &content[name_field.value_start..name_field.value_end];
    let realm_name = &content[realm_name_field.value_start..realm_name_field.value_end];

    for mapping in mappings {
        for encoded in encoded_mapping_variants(mapping) {
            if name != encoded.source_character || realm_name != encoded.source_server {
                continue;
            }

            replacements.push(ByteRangeReplacement {
                start: name_field.value_start,
                end: name_field.value_end,
                replacement: encoded.target_character,
            });
            replacements.push(ByteRangeReplacement {
                start: realm_name_field.value_start,
                end: realm_name_field.value_end,
                replacement: encoded.target_server,
            });
            return;
        }
    }
}

fn collect_direct_string_fields(
    content: &[u8],
    start: usize,
    end: usize,
    mut collect: impl FnMut(&[u8], DirectStringField),
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

        let Some(key) = parse_key(content, index) else {
            index += 1;
            continue;
        };

        let mut value_start = skip_ascii_whitespace(content, key.full_end);
        if value_start >= end || content[value_start] != b'=' {
            index = key.full_end.max(index + 1);
            continue;
        }

        value_start = skip_ascii_whitespace(content, value_start + 1);
        let Some(value) = parse_string_literal(content, value_start) else {
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
