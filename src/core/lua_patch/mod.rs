mod policy;
#[cfg(test)]
mod tests;

use std::fs;
use std::path::Path;

use serde::Serialize;

use self::policy::{DEFAULT_LUA_REWRITE_POLICY_REGISTRY, LuaRewriteCapabilities};
use crate::core::error::AppResult;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CharacterMapping {
    pub source_account: Option<String>,
    pub source_server: String,
    pub source_character: String,
    pub target_account: String,
    pub target_server: String,
    pub target_character: String,
}

impl CharacterMapping {
    pub fn source_profile_key(&self) -> String {
        format!("{} - {}", self.source_character, self.source_server)
    }

    pub fn target_profile_key(&self) -> String {
        format!("{} - {}", self.target_character, self.target_server)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LuaRewriteOptions {
    pub rewrite_profile_keys: bool,
    pub rewrite_identity_strings: bool,
}

pub fn rewrite_lua_file(
    path_hint: &Path,
    path: &Path,
    mappings: &[CharacterMapping],
    options: LuaRewriteOptions,
) -> AppResult<bool> {
    let Some(rewritten) = preview_lua_file_rewrite(path_hint, path, mappings, options)? else {
        return Ok(false);
    };

    fs::write(path, rewritten)?;
    Ok(true)
}

pub fn preview_lua_file_rewrite(
    path_hint: &Path,
    path: &Path,
    mappings: &[CharacterMapping],
    options: LuaRewriteOptions,
) -> AppResult<Option<Vec<u8>>> {
    let bytes = fs::read(path)?;
    preview_lua_bytes_rewrite(path_hint, &bytes, mappings, options)
}

pub fn preview_lua_bytes_rewrite(
    path_hint: &Path,
    bytes: &[u8],
    mappings: &[CharacterMapping],
    options: LuaRewriteOptions,
) -> AppResult<Option<Vec<u8>>> {
    let Some(capabilities) = DEFAULT_LUA_REWRITE_POLICY_REGISTRY.analyze(path_hint, bytes) else {
        return Ok(None);
    };
    if mappings.is_empty() {
        return Ok(None);
    };

    let rewrite_options = options.limit_to(capabilities);

    if rewrite_options.is_disabled() {
        return Ok(None);
    }

    let replacements = build_text_replacements(mappings, rewrite_options);
    if replacements.is_empty() {
        return Ok(None);
    }

    if let Ok(content) = std::str::from_utf8(bytes) {
        let rewritten = rewrite_lua_text(content, mappings, rewrite_options);
        if rewritten != content {
            return Ok(Some(rewritten.into_bytes()));
        }
    }

    let rewritten = apply_byte_replacements(bytes, build_byte_replacements(&replacements));
    if rewritten == bytes {
        return Ok(None);
    }

    Ok(Some(rewritten))
}

pub fn rewrite_lua_text(
    content: &str,
    mappings: &[CharacterMapping],
    options: LuaRewriteOptions,
) -> String {
    apply_replacements(content, build_text_replacements(mappings, options))
}

fn build_text_replacements(
    mappings: &[CharacterMapping],
    options: LuaRewriteOptions,
) -> Vec<(String, String)> {
    let mut replacements = Vec::new();
    for mapping in mappings {
        if options.rewrite_profile_keys {
            push_replacement(
                &mut replacements,
                mapping.source_profile_key(),
                mapping.target_profile_key(),
            );
            push_replacement(
                &mut replacements,
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
                &mut replacements,
                format!("{}.{}", mapping.source_server, mapping.source_character),
                format!("{}.{}", mapping.target_server, mapping.target_character),
            );
        }

        if options.rewrite_identity_strings {
            let source_profile_key = mapping.source_profile_key();
            let target_profile_key = mapping.target_profile_key();
            for (prefix, suffix) in [("\"", "\""), ("'", "'"), ("[\"", "\"]"), ("['", "']")] {
                push_replacement(
                    &mut replacements,
                    format!("{prefix}{source_profile_key}{suffix}"),
                    format!("{prefix}{target_profile_key}{suffix}"),
                );
            }

            for (source, target) in [
                (&mapping.source_character, &mapping.target_character),
                (&mapping.source_server, &mapping.target_server),
            ] {
                for (prefix, suffix) in [("\"", "\""), ("'", "'"), ("[\"", "\"]"), ("['", "']")] {
                    push_replacement(
                        &mut replacements,
                        format!("{prefix}{source}{suffix}"),
                        format!("{prefix}{target}{suffix}"),
                    );
                }
            }
        }
    }

    replacements
}

fn push_replacement(replacements: &mut Vec<(String, String)>, from: String, to: String) {
    if !from.is_empty() && from != to {
        replacements.push((from, to));
    }
}

fn apply_replacements(content: &str, mut replacements: Vec<(String, String)>) -> String {
    if replacements.is_empty() {
        return content.to_string();
    }

    replacements.sort_by(|left, right| right.0.len().cmp(&left.0.len()));

    let mut staged = content.to_string();
    let mut placeholders = Vec::new();

    for (index, (from, to)) in replacements.into_iter().enumerate() {
        if !staged.contains(&from) {
            continue;
        }

        let placeholder = format!("__HEARTHSYNC_REWRITE_{index}__");
        staged = staged.replace(&from, &placeholder);
        placeholders.push((placeholder, to));
    }

    for (placeholder, to) in placeholders {
        staged = staged.replace(&placeholder, &to);
    }

    staged
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LuaTextEncoding {
    Utf8,
    Latin1,
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

impl LuaRewriteOptions {
    fn limit_to(self, capabilities: LuaRewriteCapabilities) -> Self {
        Self {
            rewrite_profile_keys: self.rewrite_profile_keys && capabilities.rewrite_profile_keys,
            rewrite_identity_strings: self.rewrite_identity_strings
                && capabilities.rewrite_identity_strings,
        }
    }

    fn is_disabled(self) -> bool {
        !self.rewrite_profile_keys && !self.rewrite_identity_strings
    }
}
