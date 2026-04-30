#[derive(Debug, Clone, Copy)]
pub(super) struct ParsedStringLiteral {
    pub(super) content_start: usize,
    pub(super) content_end: usize,
    pub(super) full_end: usize,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ParsedKey {
    pub(super) name_start: usize,
    pub(super) name_end: usize,
    pub(super) full_end: usize,
}

pub(super) fn for_each_top_level_table(content: &[u8], mut visit: impl FnMut(usize, usize)) {
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

pub(super) fn for_each_named_table(
    content: &[u8],
    table_name: &[u8],
    mut visit: impl FnMut(usize, usize),
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

pub(super) fn visit_direct_table_entries(
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
            if content[value_start] == b'{'
                && let Some(table_end) = find_matching_brace(content, value_start)
            {
                index = table_end + 1;
                continue;
            }
        }

        index = value_start.max(index + 1);
    }
}

pub(super) fn find_direct_string_key(
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

pub(super) fn parse_key(bytes: &[u8], index: usize) -> Option<ParsedKey> {
    parse_bracketed_string_key(bytes, index).or_else(|| parse_identifier_key(bytes, index))
}

pub(super) fn parse_bracketed_string_key(bytes: &[u8], index: usize) -> Option<ParsedKey> {
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

pub(super) fn parse_string_literal(bytes: &[u8], index: usize) -> Option<ParsedStringLiteral> {
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

pub(super) fn skip_ascii_whitespace(bytes: &[u8], mut index: usize) -> usize {
    while let Some(ch) = bytes.get(index) {
        if ch.is_ascii_whitespace() {
            index += 1;
        } else {
            break;
        }
    }

    index
}

pub(super) fn find_matching_brace(bytes: &[u8], open_index: usize) -> Option<usize> {
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
