#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LuaTextEncoding {
    Utf8,
    Latin1,
}

pub(super) fn rewrite_lua_bytes(
    content: &[u8],
    replacements: &[(String, String)],
) -> Option<Vec<u8>> {
    let rewritten = apply_byte_replacements(content, build_byte_replacements(replacements));
    if rewritten == content {
        None
    } else {
        Some(rewritten)
    }
}

fn build_byte_replacements(replacements: &[(String, String)]) -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut encoded = Vec::new();

    for (from, to) in replacements {
        for encoding in [LuaTextEncoding::Utf8, LuaTextEncoding::Latin1] {
            let Some(from_bytes) = encode_text_for_rewrite(from, encoding) else {
                continue;
            };
            let Some(to_bytes) = encode_text_for_rewrite(to, encoding) else {
                continue;
            };
            if from_bytes != to_bytes {
                encoded.push((from_bytes, to_bytes));
            }
        }
    }

    encoded.sort_by(|left, right| {
        right
            .0
            .len()
            .cmp(&left.0.len())
            .then_with(|| left.0.cmp(&right.0))
            .then_with(|| left.1.cmp(&right.1))
    });
    encoded.dedup();
    encoded
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

fn apply_byte_replacements(content: &[u8], mut replacements: Vec<(Vec<u8>, Vec<u8>)>) -> Vec<u8> {
    if replacements.is_empty() {
        return content.to_vec();
    }

    replacements.sort_by(|left, right| right.0.len().cmp(&left.0.len()));

    let mut staged = content.to_vec();
    let mut placeholders = Vec::new();

    for (index, (from, to)) in replacements.into_iter().enumerate() {
        if from.is_empty()
            || !staged
                .windows(from.len())
                .any(|window| window == from.as_slice())
        {
            continue;
        }

        let placeholder = unique_byte_placeholder(&staged, index);
        staged = replace_bytes(&staged, &from, &placeholder);
        placeholders.push((placeholder, to));
    }

    for (placeholder, to) in placeholders {
        staged = replace_bytes(&staged, &placeholder, &to);
    }

    staged
}

fn unique_byte_placeholder(content: &[u8], index: usize) -> Vec<u8> {
    let mut placeholder = format!("__HEARTHSYNC_REWRITE_{index}__").into_bytes();
    while content
        .windows(placeholder.len())
        .any(|window| window == placeholder.as_slice())
    {
        placeholder.push(b'_');
    }
    placeholder
}

fn replace_bytes(content: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    if from.is_empty() {
        return content.to_vec();
    }

    let mut rewritten = Vec::with_capacity(content.len());
    let mut index = 0usize;
    while index < content.len() {
        if index + from.len() <= content.len() && &content[index..index + from.len()] == from {
            rewritten.extend_from_slice(to);
            index += from.len();
        } else {
            rewritten.push(content[index]);
            index += 1;
        }
    }
    rewritten
}
