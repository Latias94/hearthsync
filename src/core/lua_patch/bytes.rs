use super::{CharacterMapping, LuaRewriteOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LuaTextEncoding {
    Utf8,
    Latin1,
}

pub(super) fn rewrite_lua_bytes(
    content: &[u8],
    mappings: &[CharacterMapping],
    options: LuaRewriteOptions,
) -> Option<Vec<u8>> {
    let mut replacements = Vec::new();

    if options.rewrite_profile_keys {
        collect_profile_replacements(content, mappings, &mut replacements);
    }

    if options.rewrite_identity_strings {
        collect_identity_replacements(content, mappings, &mut replacements);
    }

    let rewritten = apply_range_replacements(content, replacements);
    if rewritten == content {
        None
    } else {
        Some(rewritten)
    }
}

fn collect_profile_replacements(
    content: &[u8],
    mappings: &[CharacterMapping],
    replacements: &mut Vec<ByteRangeReplacement>,
) {
    let rewrites = build_profile_byte_rewrites(mappings);
    if rewrites.is_empty() {
        return;
    }

    collect_direct_table_replacements(content, b"profileKeys", true, &rewrites, replacements);
    collect_direct_table_replacements(content, b"profiles", false, &rewrites, replacements);
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

fn collect_identity_replacements(
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ByteRewriteKind {
    Character,
    Server,
    Combined,
}

#[derive(Debug, Clone)]
struct ByteStringRewrite {
    source: Vec<u8>,
    target: Vec<u8>,
    kind: ByteRewriteKind,
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

fn push_byte_rewrite(
    rewrites: &mut Vec<ByteStringRewrite>,
    source: String,
    target: String,
    kind: ByteRewriteKind,
) {
    if source.is_empty() || source == target {
        return;
    }

    for encoding in [LuaTextEncoding::Utf8, LuaTextEncoding::Latin1] {
        let Some(source_bytes) = encode_text_for_rewrite(&source, encoding) else {
            continue;
        };
        let Some(target_bytes) = encode_text_for_rewrite(&target, encoding) else {
            continue;
        };
        if source_bytes == target_bytes {
            continue;
        }
        if rewrites.iter().any(|rewrite| {
            rewrite.source == source_bytes && rewrite.target == target_bytes && rewrite.kind == kind
        }) {
            continue;
        }

        rewrites.push(ByteStringRewrite {
            source: source_bytes,
            target: target_bytes,
            kind,
        });
    }
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

        if content[key.name_start..key.name_end] == *table_name {
            if let Some(table_end) = find_matching_brace(content, value_start) {
                collect_direct_child_table_replacements(
                    content,
                    value_start + 1,
                    table_end,
                    rewrite_values,
                    rewrites,
                    replacements,
                );
            }
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

        if !content[key.name_start..key.name_end].ends_with(b"profileKey") {
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

fn find_byte_rewrite<'a>(
    value: &[u8],
    rewrites: &'a [ByteStringRewrite],
) -> Option<&'a ByteStringRewrite> {
    rewrites.iter().find(|rewrite| rewrite.source == value)
}

fn find_byte_rewrite_with_kinds<'a>(
    value: &[u8],
    rewrites: &'a [ByteStringRewrite],
    allowed_kinds: &[ByteRewriteKind],
) -> Option<&'a ByteStringRewrite> {
    rewrites
        .iter()
        .find(|rewrite| rewrite.source == value && allowed_kinds.contains(&rewrite.kind))
}

#[derive(Debug, Clone)]
struct EncodedMapping {
    source_server: Vec<u8>,
    source_character: Vec<u8>,
    target_server: Vec<u8>,
    target_character: Vec<u8>,
}

fn encoded_mapping_variants(mapping: &CharacterMapping) -> Vec<EncodedMapping> {
    let mut variants = Vec::new();
    for encoding in [LuaTextEncoding::Utf8, LuaTextEncoding::Latin1] {
        let Some(source_server) = encode_text_for_rewrite(&mapping.source_server, encoding) else {
            continue;
        };
        let Some(source_character) = encode_text_for_rewrite(&mapping.source_character, encoding)
        else {
            continue;
        };
        let Some(target_server) = encode_text_for_rewrite(&mapping.target_server, encoding) else {
            continue;
        };
        let Some(target_character) = encode_text_for_rewrite(&mapping.target_character, encoding)
        else {
            continue;
        };

        if variants.iter().any(|variant: &EncodedMapping| {
            variant.source_server == source_server
                && variant.source_character == source_character
                && variant.target_server == target_server
                && variant.target_character == target_character
        }) {
            continue;
        }

        variants.push(EncodedMapping {
            source_server,
            source_character,
            target_server,
            target_character,
        });
    }

    variants
}

fn for_each_top_level_table(content: &[u8], mut visit: impl FnMut(usize, usize)) {
    let mut index = 0usize;

    while index < content.len() {
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
        visit(index + 1, table_end);
        index = table_end + 1;
    }
}

fn for_each_named_table(content: &[u8], table_name: &[u8], mut visit: impl FnMut(usize, usize)) {
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

        if content[key.name_start..key.name_end] != *table_name {
            index = key.full_end.max(index + 1);
            continue;
        }

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

        let Some(table_end) = find_matching_brace(content, value_start) else {
            index = key.full_end.max(index + 1);
            continue;
        };
        visit(value_start + 1, table_end);
        index = table_end + 1;
    }
}

fn visit_direct_table_entries(
    content: &[u8],
    start: usize,
    end: usize,
    mut visit: impl FnMut(ParsedKey, usize),
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

        value_start = skip_ascii_whitespace(content, value_start + 1);
        visit(key, value_start);

        if value_start < end {
            if let Some(literal) = parse_string_literal(content, value_start) {
                index = literal.full_end;
                continue;
            }
            if content[value_start] == b'{' {
                if let Some(table_end) = find_matching_brace(content, value_start) {
                    index = table_end + 1;
                    continue;
                }
            }
        }

        index = value_start.max(index + 1);
    }
}

fn find_direct_string_key(
    content: &[u8],
    start: usize,
    end: usize,
    expected_key: &[u8],
) -> Option<ParsedKey> {
    let mut matched = None;
    visit_direct_table_entries(content, start, end, |key, _| {
        if matched.is_none() && content[key.name_start..key.name_end] == *expected_key {
            matched = Some(key);
        }
    });
    matched
}

#[derive(Debug, Clone)]
struct ByteRangeReplacement {
    start: usize,
    end: usize,
    replacement: Vec<u8>,
}

fn apply_range_replacements(
    content: &[u8],
    mut replacements: Vec<ByteRangeReplacement>,
) -> Vec<u8> {
    if replacements.is_empty() {
        return content.to_vec();
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
        if filtered.iter().any(|existing: &ByteRangeReplacement| {
            existing.start == replacement.start
                && existing.end == replacement.end
                && existing.replacement == replacement.replacement
        }) {
            continue;
        }

        if let Some(previous) = filtered.last() {
            if replacement.end > previous.start {
                continue;
            }
        }

        filtered.push(replacement);
    }

    let mut rewritten = content.to_vec();
    for replacement in filtered {
        rewritten.splice(replacement.start..replacement.end, replacement.replacement);
    }

    rewritten
}

#[derive(Debug, Clone, Copy)]
struct ParsedStringLiteral {
    content_start: usize,
    content_end: usize,
    full_end: usize,
}

#[derive(Debug, Clone, Copy)]
struct ParsedKey {
    name_start: usize,
    name_end: usize,
    full_end: usize,
}

fn parse_key(bytes: &[u8], index: usize) -> Option<ParsedKey> {
    parse_bracketed_string_key(bytes, index).or_else(|| parse_identifier_key(bytes, index))
}

fn parse_bracketed_string_key(bytes: &[u8], index: usize) -> Option<ParsedKey> {
    if bytes.get(index) != Some(&b'[') {
        return None;
    }

    let string_start = skip_ascii_whitespace(bytes, index + 1);
    let literal = parse_string_literal(bytes, string_start)?;
    let closing = skip_ascii_whitespace(bytes, literal.full_end);
    if bytes.get(closing) != Some(&b']') {
        return None;
    }

    Some(ParsedKey {
        name_start: literal.content_start,
        name_end: literal.content_end,
        full_end: closing + 1,
    })
}

fn parse_identifier_key(bytes: &[u8], index: usize) -> Option<ParsedKey> {
    let first = *bytes.get(index)?;
    if !(first == b'_' || first.is_ascii_alphabetic()) {
        return None;
    }

    let mut end = index + 1;
    while let Some(ch) = bytes.get(end) {
        if *ch == b'_' || ch.is_ascii_alphanumeric() {
            end += 1;
        } else {
            break;
        }
    }

    Some(ParsedKey {
        name_start: index,
        name_end: end,
        full_end: end,
    })
}

fn parse_string_literal(bytes: &[u8], index: usize) -> Option<ParsedStringLiteral> {
    let quote = *bytes.get(index)?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }

    let mut cursor = index + 1;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'\\' => {
                cursor = cursor.saturating_add(2);
            }
            current if current == quote => {
                return Some(ParsedStringLiteral {
                    content_start: index + 1,
                    content_end: cursor,
                    full_end: cursor + 1,
                });
            }
            _ => {
                cursor += 1;
            }
        }
    }

    None
}

fn skip_ascii_whitespace(bytes: &[u8], mut index: usize) -> usize {
    while let Some(ch) = bytes.get(index) {
        if ch.is_ascii_whitespace() {
            index += 1;
        } else {
            break;
        }
    }

    index
}

fn find_matching_brace(bytes: &[u8], open_index: usize) -> Option<usize> {
    if bytes.get(open_index) != Some(&b'{') {
        return None;
    }

    let mut depth = 0usize;
    let mut index = open_index;
    while index < bytes.len() {
        if let Some(literal) = parse_string_literal(bytes, index) {
            index = literal.full_end;
            continue;
        }

        match bytes[index] {
            b'{' => depth += 1,
            b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }

        index += 1;
    }

    None
}

fn encode_text_for_rewrite(text: &str, encoding: LuaTextEncoding) -> Option<Vec<u8>> {
    match encoding {
        LuaTextEncoding::Utf8 => Some(text.as_bytes().to_vec()),
        LuaTextEncoding::Latin1 => text.chars().map(latin1_char_to_byte).collect(),
    }
}

fn latin1_char_to_byte(ch: char) -> Option<u8> {
    let codepoint = ch as u32;
    if codepoint <= u8::MAX as u32 {
        Some(codepoint as u8)
    } else {
        None
    }
}
