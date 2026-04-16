use super::{CharacterMapping, LuaRewriteOptions};

pub fn rewrite_lua_text(
    content: &str,
    mappings: &[CharacterMapping],
    options: LuaRewriteOptions,
) -> String {
    apply_replacements(content, build_text_replacements(mappings, options))
}

pub(super) fn build_text_replacements(
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
