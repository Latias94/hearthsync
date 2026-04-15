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
    let Some(capabilities) = analyze_lua_rewrite_capabilities(path_hint, bytes) else {
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

#[derive(Debug, Clone, Copy, Default)]
struct LuaRewriteCapabilities {
    rewrite_profile_keys: bool,
    rewrite_identity_strings: bool,
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

impl LuaRewriteCapabilities {
    const IDENTITY_ONLY: Self = Self {
        rewrite_profile_keys: false,
        rewrite_identity_strings: true,
    };

    fn merge(self, other: Self) -> Self {
        Self {
            rewrite_profile_keys: self.rewrite_profile_keys || other.rewrite_profile_keys,
            rewrite_identity_strings: self.rewrite_identity_strings
                || other.rewrite_identity_strings,
        }
    }
}

fn analyze_lua_rewrite_capabilities(path: &Path, bytes: &[u8]) -> Option<LuaRewriteCapabilities> {
    let target = classify_lua_rewrite_target(path)?;
    Some(detect_lua_rewrite_signals(bytes).merge(matched_rule_capabilities(&target.file_name)))
}

fn detect_lua_rewrite_signals(bytes: &[u8]) -> LuaRewriteCapabilities {
    LuaRewriteCapabilities {
        rewrite_profile_keys: bytes_contain_any_ascii_marker(bytes, PROFILE_KEY_MARKERS),
        rewrite_identity_strings: bytes_contain_any_ascii_marker(bytes, IDENTITY_FIELD_MARKERS),
    }
}

const PROFILE_KEY_MARKERS: &[&[u8]] = &[b"profileKeys"];

const IDENTITY_FIELD_MARKERS: &[&[u8]] =
    &[b"playerName", b"realm", b"LastPlayerFullName", b"LastRealm"];

#[derive(Debug, Clone, Copy)]
struct LuaRewriteRule {
    matcher: LuaRewriteRuleMatcher,
    capabilities: LuaRewriteCapabilities,
}

impl LuaRewriteRule {
    fn matches(self, file_name: &str) -> bool {
        match self.matcher {
            LuaRewriteRuleMatcher::Exact(expected) => file_name == expected,
            LuaRewriteRuleMatcher::Prefix(prefix) => file_name.starts_with(prefix),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum LuaRewriteRuleMatcher {
    Exact(&'static str),
    Prefix(&'static str),
}

const fn identity_exact_rule(file_name: &'static str) -> LuaRewriteRule {
    LuaRewriteRule {
        matcher: LuaRewriteRuleMatcher::Exact(file_name),
        capabilities: LuaRewriteCapabilities::IDENTITY_ONLY,
    }
}

const fn identity_prefix_rule(prefix: &'static str) -> LuaRewriteRule {
    LuaRewriteRule {
        matcher: LuaRewriteRuleMatcher::Prefix(prefix),
        capabilities: LuaRewriteCapabilities::IDENTITY_ONLY,
    }
}

const LUA_REWRITE_RULES: &[LuaRewriteRule] = &[
    identity_exact_rule("auraupdater.lua"),
    identity_exact_rule("bagsync.lua"),
    identity_exact_rule("details.lua"),
    identity_exact_rule("elvui.lua"),
    identity_exact_rule("eventstracker.lua"),
    identity_exact_rule("exwindcore.lua"),
    identity_exact_rule("meetingstone.lua"),
    identity_exact_rule("newbeebox.lua"),
    identity_exact_rule("pawn.lua"),
    identity_exact_rule("rarity.lua"),
    identity_exact_rule("savedinstances.lua"),
    identity_exact_rule("tinytooltip-remake.lua"),
    identity_exact_rule("weakauras.lua"),
    identity_exact_rule("weakaurasarchive.lua"),
    identity_exact_rule("worldquesttracker.lua"),
    identity_exact_rule("zygorguidesviewer.lua"),
    identity_prefix_rule("dbm-"),
    identity_prefix_rule("details_"),
    identity_prefix_rule("handynotes_"),
];

fn bytes_contain_any_ascii_marker(bytes: &[u8], markers: &[&[u8]]) -> bool {
    markers.iter().any(|marker| {
        !marker.is_empty() && bytes.windows(marker.len()).any(|window| window == *marker)
    })
}

fn matched_rule_capabilities(file_name: &str) -> LuaRewriteCapabilities {
    LUA_REWRITE_RULES
        .iter()
        .copied()
        .filter(|rule| rule.matches(file_name))
        .fold(LuaRewriteCapabilities::default(), |capabilities, rule| {
            capabilities.merge(rule.capabilities)
        })
}

#[derive(Debug, Clone)]
struct LuaRewriteTarget {
    file_name: String,
}

fn classify_lua_rewrite_target(path: &Path) -> Option<LuaRewriteTarget> {
    let file_name = path.file_name()?.to_str()?.to_ascii_lowercase();
    if !file_name.ends_with(".lua") {
        return None;
    }

    let segments = path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .map(|segment| segment.to_ascii_lowercase())
        .collect::<Vec<_>>();

    if segments.len() >= 6
        && segments[segments.len() - 6] == "wtf"
        && segments[segments.len() - 5] == "common"
        && segments[segments.len() - 4] == "accounts"
        && segments[segments.len() - 2] == "savedvariables"
    {
        return Some(LuaRewriteTarget { file_name });
    }

    if segments.len() >= 7
        && segments[segments.len() - 7] == "wtf"
        && segments[segments.len() - 6] == "characters"
        && segments[segments.len() - 2] == "savedvariables"
    {
        return Some(LuaRewriteTarget { file_name });
    }

    if segments.len() >= 4
        && segments[segments.len() - 4] == "account"
        && segments[segments.len() - 2] == "savedvariables"
    {
        return Some(LuaRewriteTarget { file_name });
    }

    if segments.len() >= 6
        && segments[segments.len() - 6] == "account"
        && segments[segments.len() - 2] == "savedvariables"
    {
        return Some(LuaRewriteTarget { file_name });
    }

    None
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{CharacterMapping, LuaRewriteOptions, preview_lua_bytes_rewrite, rewrite_lua_text};

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

    fn sample_mapping() -> CharacterMapping {
        CharacterMapping {
            source_account: Some("ACCOUNT".to_string()),
            source_server: "Illidan".to_string(),
            source_character: "Examplemage".to_string(),
            target_account: "TARGET".to_string(),
            target_server: "Stormrage".to_string(),
            target_character: "Targetmage".to_string(),
        }
    }

    #[test]
    fn preview_lua_bytes_rewrite_allows_account_saved_variables() {
        let rewritten = preview_lua_bytes_rewrite(
            Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua"),
            br#"DetailsDB = { ["profileKeys"] = { ["Examplemage - Illidan"] = "Default" } }"#,
            &[sample_mapping()],
            LuaRewriteOptions {
                rewrite_profile_keys: true,
                rewrite_identity_strings: true,
            },
        )
        .expect("preview");

        assert!(rewritten.is_some());
    }

    #[test]
    fn preview_lua_bytes_rewrite_allows_character_saved_variables() {
        let rewritten = preview_lua_bytes_rewrite(
            Path::new("wtf/characters/ACCOUNT/Illidan/Examplemage/SavedVariables/Pawn.lua"),
            br#"PawnOptions = { ["LastPlayerFullName"] = "Examplemage" }"#,
            &[sample_mapping()],
            LuaRewriteOptions {
                rewrite_profile_keys: false,
                rewrite_identity_strings: true,
            },
        )
        .expect("preview");

        assert!(rewritten.is_some());
    }

    #[test]
    fn preview_lua_bytes_rewrite_rejects_addon_lua_paths() {
        let rewritten = preview_lua_bytes_rewrite(
            Path::new("addons/WeakAuras/WeakAuras.lua"),
            br#"WeakAurasSaved = { ["profileKeys"] = { ["Examplemage - Illidan"] = "Default" } }"#,
            &[sample_mapping()],
            LuaRewriteOptions {
                rewrite_profile_keys: true,
                rewrite_identity_strings: true,
            },
        )
        .expect("preview");

        assert!(rewritten.is_none());
    }

    #[test]
    fn preview_lua_bytes_rewrite_rejects_account_root_lua_outside_saved_variables() {
        let rewritten = preview_lua_bytes_rewrite(
            Path::new("wtf/common/accounts/ACCOUNT/account-settings.lua"),
            br#"return "Examplemage""#,
            &[sample_mapping()],
            LuaRewriteOptions {
                rewrite_profile_keys: true,
                rewrite_identity_strings: true,
            },
        )
        .expect("preview");

        assert!(rewritten.is_none());
    }

    #[test]
    fn preview_lua_bytes_rewrite_rejects_unknown_savedvariables_file_without_rule_signals() {
        let rewritten = preview_lua_bytes_rewrite(
            Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/CustomAddon.lua"),
            br#"return "Examplemage""#,
            &[sample_mapping()],
            LuaRewriteOptions {
                rewrite_profile_keys: false,
                rewrite_identity_strings: true,
            },
        )
        .expect("preview");

        assert!(rewritten.is_none());
    }

    #[test]
    fn preview_lua_bytes_rewrite_allows_known_identity_exact_rules_without_field_markers() {
        for path in [
            "wtf/characters/ACCOUNT/Illidan/Examplemage/SavedVariables/MeetingStone.lua",
            "wtf/common/accounts/ACCOUNT/SavedVariables/EventsTracker.lua",
            "wtf/common/accounts/ACCOUNT/SavedVariables/SavedInstances.lua",
        ] {
            let rewritten = preview_lua_bytes_rewrite(
                Path::new(path),
                br#"ExampleDB = { ["Examplemage - Illidan"] = {} }"#,
                &[sample_mapping()],
                LuaRewriteOptions {
                    rewrite_profile_keys: false,
                    rewrite_identity_strings: true,
                },
            )
            .expect("preview");

            assert!(rewritten.is_some(), "{path} should be rewriteable");
        }
    }

    #[test]
    fn preview_lua_bytes_rewrite_allows_known_identity_prefix_rule_without_field_markers() {
        let rewritten = preview_lua_bytes_rewrite(
            Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/DBM-Core.lua"),
            br#"return "Examplemage - Illidan""#,
            &[sample_mapping()],
            LuaRewriteOptions {
                rewrite_profile_keys: false,
                rewrite_identity_strings: true,
            },
        )
        .expect("preview");

        assert!(rewritten.is_some());
    }

    #[test]
    fn preview_lua_bytes_rewrite_handles_invalid_utf8_payloads() {
        let rewritten = preview_lua_bytes_rewrite(
            Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua"),
            b"prefix\xff[\"profileKeys\"]={ [\"Examplemage - Illidan\"] = \"Default\" }\xffsuffix",
            &[sample_mapping()],
            LuaRewriteOptions {
                rewrite_profile_keys: true,
                rewrite_identity_strings: false,
            },
        )
        .expect("preview")
        .expect("rewritten bytes");

        assert_eq!(rewritten[6], 0xff);
        assert!(
            rewritten
                .windows(b"Targetmage - Stormrage".len())
                .any(|window| { window == b"Targetmage - Stormrage" })
        );
    }

    #[test]
    fn preview_lua_bytes_rewrite_handles_real_world_like_invalid_utf8_payload() {
        let payload = "AuctionatorDB = {\r\n[\"贫瘠之地\"] = \""
            .as_bytes()
            .iter()
            .copied()
            .chain([0xa1, b'G', b'v', b'e', b'r', b's', b'i', b'o', b'n', 0x02])
            .chain(
                "\",\r\n[\"profileKeys\"] = { [\"Examplemage - Illidan\"] = \"Default\" },\r\n}"
                    .as_bytes()
                    .iter()
                    .copied(),
            )
            .collect::<Vec<_>>();
        let rewritten = preview_lua_bytes_rewrite(
            Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/Auctionator.lua"),
            &payload,
            &[sample_mapping()],
            LuaRewriteOptions {
                rewrite_profile_keys: true,
                rewrite_identity_strings: false,
            },
        )
        .expect("preview")
        .expect("rewritten bytes");

        assert!(
            rewritten
                .windows("贫瘠之地".as_bytes().len())
                .any(|window| window == "贫瘠之地".as_bytes())
        );
        assert!(
            rewritten
                .windows(b"Targetmage - Stormrage".len())
                .any(|window| window == b"Targetmage - Stormrage")
        );
        assert!(
            rewritten.windows(10).any(
                |window| window == [0xa1, b'G', b'v', b'e', b'r', b's', b'i', b'o', b'n', 0x02]
            )
        );
    }

    #[test]
    fn preview_lua_bytes_rewrite_supports_latin1_strings() {
        let rewritten = preview_lua_bytes_rewrite(
            Path::new("wtf/characters/ACCOUNT/Illidan/Examplemage/SavedVariables/Pawn.lua"),
            b"PawnOptions = { [\"LastPlayerFullName\"] = \"Ren\xe9e\" }",
            &[CharacterMapping {
                source_account: Some("ACCOUNT".to_string()),
                source_server: "Illidan".to_string(),
                source_character: "Renée".to_string(),
                target_account: "TARGET".to_string(),
                target_server: "Illidan".to_string(),
                target_character: "Zoë".to_string(),
            }],
            LuaRewriteOptions {
                rewrite_profile_keys: false,
                rewrite_identity_strings: true,
            },
        )
        .expect("preview")
        .expect("rewritten bytes");

        assert!(
            rewritten
                .windows(b"Zo\xeb".len())
                .any(|window| window == b"Zo\xeb")
        );
        assert!(
            !rewritten
                .windows(b"Ren\xe9e".len())
                .any(|window| window == b"Ren\xe9e")
        );
    }
}
