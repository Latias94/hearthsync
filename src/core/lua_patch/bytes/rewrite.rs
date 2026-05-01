use super::encoding::{LuaTextEncoding, encode_text_for_rewrite};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ByteRewriteKind {
    Character,
    Server,
    Combined,
}

#[derive(Debug, Clone)]
pub(super) struct ByteStringRewrite {
    source: Vec<u8>,
    pub(super) target: Vec<u8>,
    kind: ByteRewriteKind,
}

pub(super) fn push_byte_rewrite(
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

pub(super) fn find_byte_rewrite<'a>(
    value: &[u8],
    rewrites: &'a [ByteStringRewrite],
) -> Option<&'a ByteStringRewrite> {
    rewrites.iter().find(|rewrite| rewrite.source == value)
}

pub(super) fn find_byte_rewrite_with_kinds<'a>(
    value: &[u8],
    rewrites: &'a [ByteStringRewrite],
    allowed_kinds: &[ByteRewriteKind],
) -> Option<&'a ByteStringRewrite> {
    rewrites
        .iter()
        .find(|rewrite| rewrite.source == value && allowed_kinds.contains(&rewrite.kind))
}
