use super::super::CharacterMapping;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum LuaTextEncoding {
    Utf8,
    Latin1,
}

#[derive(Debug, Clone)]
pub(super) struct EncodedMapping {
    pub(super) source_server: Vec<u8>,
    pub(super) source_character: Vec<u8>,
    pub(super) target_server: Vec<u8>,
    pub(super) target_character: Vec<u8>,
}

pub(super) fn encoded_mapping_variants(mapping: &CharacterMapping) -> Vec<EncodedMapping> {
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

pub(super) fn encode_text_for_rewrite(text: &str, encoding: LuaTextEncoding) -> Option<Vec<u8>> {
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
