use std::fs;
use std::path::Path;

use serde::Serialize;

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
    path: &Path,
    mappings: &[CharacterMapping],
    options: LuaRewriteOptions,
) -> AppResult<bool> {
    if !should_rewrite_lua(path) || mappings.is_empty() {
        return Ok(false);
    }

    let bytes = fs::read(path)?;
    let Ok(content) = String::from_utf8(bytes) else {
        return Ok(false);
    };

    let rewritten = rewrite_lua_text(&content, mappings, options);
    if rewritten == content {
        return Ok(false);
    }

    fs::write(path, rewritten.as_bytes())?;
    Ok(true)
}

pub fn preview_lua_file_rewrite(
    path: &Path,
    mappings: &[CharacterMapping],
    options: LuaRewriteOptions,
) -> AppResult<Option<Vec<u8>>> {
    if !should_rewrite_lua(path) || mappings.is_empty() {
        return Ok(None);
    }

    let bytes = fs::read(path)?;
    let Ok(content) = String::from_utf8(bytes) else {
        return Ok(None);
    };

    let rewritten = rewrite_lua_text(&content, mappings, options);
    if rewritten == content {
        return Ok(None);
    }

    Ok(Some(rewritten.into_bytes()))
}

pub fn rewrite_lua_text(
    content: &str,
    mappings: &[CharacterMapping],
    options: LuaRewriteOptions,
) -> String {
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

    apply_replacements(content, replacements)
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

fn should_rewrite_lua(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("lua"))
}

#[cfg(test)]
mod tests {
    use super::{CharacterMapping, LuaRewriteOptions, rewrite_lua_text};

    #[test]
    fn rewrite_lua_text_updates_profile_keys_and_identity_strings() {
        let input = r#"
TestDB = {
  ["profileKeys"] = {
    ["Examplemage - Illidan"] = "Default",
  },
  ["profiles"] = {
    ["Default.Illidan.Examplemage"] = {
      ["playerName"] = "Examplemage",
      ["realm"] = "Illidan",
    },
  },
}
"#;

        let output = rewrite_lua_text(
            input,
            &[CharacterMapping {
                source_account: Some("ACCOUNT".to_string()),
                source_server: "Illidan".to_string(),
                source_character: "Examplemage".to_string(),
                target_account: "TARGET".to_string(),
                target_server: "Stormrage".to_string(),
                target_character: "Targetmage".to_string(),
            }],
            LuaRewriteOptions {
                rewrite_profile_keys: true,
                rewrite_identity_strings: true,
            },
        );

        assert!(output.contains("Targetmage - Stormrage"));
        assert!(output.contains("Default.Stormrage.Targetmage"));
        assert!(output.contains(r#"["playerName"] = "Targetmage""#));
        assert!(output.contains(r#"["realm"] = "Stormrage""#));
    }
}
