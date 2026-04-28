use std::{
    fs,
    path::{Path, PathBuf},
};

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

#[test]
fn rewrite_lua_text_preserves_preexisting_placeholder_literals() {
    let input = r#"
TestDB = {
  ["notes"] = "__HEARTHSYNC_REWRITE_0__",
  ["profileKeys"] = {
    ["Examplemage - Illidan"] = "Default",
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

    assert!(output.contains(r#"["notes"] = "__HEARTHSYNC_REWRITE_0__""#));
    assert!(output.contains(r#"["Targetmage - Stormrage"] = "Default""#));
    assert!(!output.contains(r#"["notes"] = "Targetmage - Stormrage""#));
}

#[test]
fn rewrite_lua_text_scopes_profile_key_rewrites_to_known_tables_and_fields() {
    let input = r#"
TestDB = {
  ["notes"] = "Examplemage - Illidan",
  ["profileKeys"] = {
    ["Examplemage - Illidan"] = "Examplemage - Illidan",
    ["Altmage - Illidan"] = "Default.Illidan.Examplemage",
  },
  ["profiles"] = {
    ["Default.Illidan.Examplemage"] = {
      ["message"] = "Examplemage - Illidan",
    },
    ["Illidan.Examplemage"] = {},
  },
  ["char"] = {
    ["Examplemage - Illidan"] = {
      ["spec1_profileKey"] = "Examplemage - Illidan",
      ["note"] = "Examplemage - Illidan",
    },
  },
}
"#;

    let output = rewrite_lua_text(
        input,
        &[sample_mapping()],
        LuaRewriteOptions {
            rewrite_profile_keys: true,
            rewrite_identity_strings: false,
        },
    );

    assert!(output.contains(r#"["Targetmage - Stormrage"] = "Targetmage - Stormrage""#));
    assert!(output.contains(r#"["Altmage - Illidan"] = "Default.Stormrage.Targetmage""#));
    assert!(output.contains(r#"["Default.Stormrage.Targetmage"] = {"#));
    assert!(output.contains(r#"["Stormrage.Targetmage"] = {}"#));
    assert!(output.contains(r#"["spec1_profileKey"] = "Targetmage - Stormrage""#));
    assert!(output.contains(r#"["notes"] = "Examplemage - Illidan""#));
    assert!(output.contains(r#"["message"] = "Examplemage - Illidan""#));
    assert!(output.contains(r#"["char"] = {"#));
    assert!(output.contains(r#"["Examplemage - Illidan"] = {"#));
    assert!(output.contains(r#"["note"] = "Examplemage - Illidan""#));
}

#[test]
fn rewrite_lua_text_scopes_identity_rewrites_to_keys_and_explicit_fields() {
    let input = r#"
TestDB = {
  ["notes"] = "Examplemage - Illidan",
  ["identityKeys"] = {
    ["Examplemage - Illidan"] = true,
    ["Examplemage-Illidan"] = true,
    ["Illidan-Examplemage"] = true,
  },
  ["details"] = {
    ["playerName"] = "Examplemage",
    ["realm"] = "Illidan",
    ["server"] = "Illidan",
    ["character"] = "Examplemage",
    ["lastPlayerName"] = "Examplemage",
  },
  ["pawn"] = {
    ["LastPlayerFullName"] = "Examplemage",
    ["LastRealm"] = "Illidan",
  },
  ["player"] = {
    ["name"] = "Examplemage",
    ["realmName"] = "Illidan",
    ["playerGUID"] = "Player-57-00000001",
  },
  ["mount"] = {
    ["name"] = "Examplemage",
    ["species"] = "goat",
  },
}
"#;

    let output = rewrite_lua_text(
        input,
        &[sample_mapping()],
        LuaRewriteOptions {
            rewrite_profile_keys: false,
            rewrite_identity_strings: true,
        },
    );

    assert!(output.contains(r#"["Examplemage - Illidan"] = true"#));
    assert!(output.contains(r#"["Examplemage-Illidan"] = true"#));
    assert!(output.contains(r#"["Illidan-Examplemage"] = true"#));
    assert!(output.contains(r#"["playerName"] = "Targetmage""#));
    assert!(output.contains(r#"["realm"] = "Stormrage""#));
    assert!(output.contains(r#"["server"] = "Stormrage""#));
    assert!(output.contains(r#"["character"] = "Targetmage""#));
    assert!(output.contains(r#"["LastPlayerFullName"] = "Targetmage""#));
    assert!(output.contains(r#"["LastRealm"] = "Stormrage""#));
    assert!(output.contains(r#"["name"] = "Targetmage""#));
    assert!(output.contains(r#"["realmName"] = "Stormrage""#));
    assert!(output.contains(r#"["playerGUID"] = "Player-57-00000001""#));
    assert!(output.contains(r#"["notes"] = "Examplemage - Illidan""#));
    assert!(output.contains(r#"["lastPlayerName"] = "Examplemage""#));
    assert!(output.contains(r#"["mount"] = {"#));
    assert!(output.contains(r#"["species"] = "goat""#));
}

#[test]
fn rewrite_lua_text_scopes_identity_key_rewrites_to_known_containers() {
    let input = r#"
TestDB = {
  ["Examplemage - Illidan"] = true,
  ["Examplemage-Illidan"] = true,
  ["Illidan-Examplemage"] = true,
  ["profileKeys"] = {
    ["Examplemage - Illidan"] = "Default",
  },
  ["char"] = {
    ["Examplemage - Illidan"] = {
      ["enabled"] = true,
    },
  },
  ["faction"] = {
    ["Illidan"] = {
      ["Examplemage"] = "Alliance",
    },
  },
  ["worldBoss"] = {
    ["Examplemage-Illidan"] = {
      ["realm"] = "Illidan",
    },
  },
  ["currentrealm"] = {
    ["Examplemage"] = "Illidan",
  },
  ["totals"] = {
    ["Illidan"] = {
      ["money"] = 42,
    },
  },
  ["cache"] = {
    ["Examplemage - Illidan"] = true,
    ["Illidan"] = {
      ["Examplemage"] = 1,
    },
  },
}
"#;

    let output = rewrite_lua_text(
        input,
        &[sample_mapping()],
        LuaRewriteOptions {
            rewrite_profile_keys: false,
            rewrite_identity_strings: true,
        },
    );

    assert!(output.contains(r#"["Examplemage - Illidan"] = true"#));
    assert!(output.contains(r#"["Examplemage-Illidan"] = true"#));
    assert!(output.contains(r#"["Illidan-Examplemage"] = true"#));
    assert!(output.contains(r#"["Targetmage - Stormrage"] = "Default""#));
    assert!(output.contains(r#"["Targetmage - Stormrage"] = {"#));
    assert!(output.contains(r#"["Stormrage"] = {"#));
    assert!(output.contains(r#"["Targetmage"] = "Alliance""#));
    assert!(output.contains(r#"["Targetmage-Stormrage"] = {"#));
    assert!(output.contains(r#"["Targetmage"] = "Stormrage""#));
    assert!(output.contains(r#"["money"] = 42"#));
    assert!(output.contains(r#"["cache"] = {"#));
    assert!(output.contains(r#"["Examplemage - Illidan"] = true,"#));
    assert!(output.contains(r#"["Illidan"] = {"#));
    assert!(output.contains(r#"["Examplemage"] = 1,"#));
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

fn localized_profile_mapping() -> CharacterMapping {
    CharacterMapping {
        source_account: Some("ACCOUNT".to_string()),
        source_server: "迅捷微风".to_string(),
        source_character: "露露缇娅".to_string(),
        target_account: "TARGET".to_string(),
        target_server: "白银之手".to_string(),
        target_character: "暮光花雨".to_string(),
    }
}

fn localized_bagsync_mapping() -> CharacterMapping {
    CharacterMapping {
        source_account: Some("ACCOUNT".to_string()),
        source_server: "贫瘠之地".to_string(),
        source_character: "焱天狼".to_string(),
        target_account: "TARGET".to_string(),
        target_server: "白银之手".to_string(),
        target_character: "暮光花雨".to_string(),
    }
}

fn localized_newbeebox_mapping() -> CharacterMapping {
    CharacterMapping {
        source_account: Some("ACCOUNT".to_string()),
        source_server: "迅捷微风".to_string(),
        source_character: "露露缇娅丶".to_string(),
        target_account: "TARGET".to_string(),
        target_server: "白银之手".to_string(),
        target_character: "暮光花雨".to_string(),
    }
}

fn testdata_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("core")
        .join("lua_patch")
        .join("testdata")
        .join(name)
}

fn load_text_fixture_bytes(name: &str) -> Vec<u8> {
    fs::read(testdata_path(name)).expect("fixture bytes")
}

fn load_escaped_byte_fixture(name: &str) -> Vec<u8> {
    let fixture = fs::read_to_string(testdata_path(name)).expect("fixture text");
    parse_escaped_bytes(&fixture)
}

fn parse_escaped_bytes(input: &str) -> Vec<u8> {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] != b'\\' {
            output.push(bytes[index]);
            index += 1;
            continue;
        }

        let escape = bytes
            .get(index + 1)
            .copied()
            .expect("escape marker should have a value");
        match escape {
            b'\\' => {
                output.push(b'\\');
                index += 2;
            }
            b'n' => {
                output.push(b'\n');
                index += 2;
            }
            b'r' => {
                output.push(b'\r');
                index += 2;
            }
            b't' => {
                output.push(b'\t');
                index += 2;
            }
            b'x' => {
                let hex = std::str::from_utf8(
                    bytes
                        .get(index + 2..index + 4)
                        .expect("hex escape should include two digits"),
                )
                .expect("hex escape should be valid ascii");
                output.push(u8::from_str_radix(hex, 16).expect("hex escape should be valid byte"));
                index += 4;
            }
            _ => panic!("unsupported fixture escape: \\{}", escape as char),
        }
    }

    output
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
fn preview_lua_bytes_rewrite_allows_root_saved_variables() {
    let rewritten = preview_lua_bytes_rewrite(
        Path::new("wtf/common/root/SavedVariables/Details.lua"),
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
fn preview_lua_bytes_rewrite_rejects_unknown_savedvariables_file_with_generic_identity_fields() {
    let rewritten = preview_lua_bytes_rewrite(
        Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/Syndicator.lua"),
        br#"
SyndicatorDB = {
  ["characters"] = {
    {
      ["character"] = "Examplemage",
      ["realm"] = "Illidan",
    },
    {
      ["character"] = "Altmage",
      ["realm"] = "Illidan",
    },
  },
}
"#,
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
fn preview_lua_bytes_rewrite_rejects_realistic_wim_history_fixture() {
    let payload = load_text_fixture_bytes("wim_realistic_utf8.lua");
    let rewritten = preview_lua_bytes_rewrite(
        Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/WIM.lua"),
        &payload,
        &[localized_profile_mapping()],
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
    for (path, fixture) in [
        (
            "wtf/characters/ACCOUNT/Illidan/Examplemage/SavedVariables/MeetingStone.lua",
            "meetingstone_profilekeys.lua",
        ),
        (
            "wtf/common/accounts/ACCOUNT/SavedVariables/EventsTracker.lua",
            "eventstracker_character_keys.lua",
        ),
        (
            "wtf/common/accounts/ACCOUNT/SavedVariables/SavedInstances.lua",
            "savedinstances_toon_keys.lua",
        ),
    ] {
        let payload = load_text_fixture_bytes(fixture);
        let rewritten = preview_lua_bytes_rewrite(
            Path::new(path),
            &payload,
            &[sample_mapping()],
            LuaRewriteOptions {
                rewrite_profile_keys: false,
                rewrite_identity_strings: true,
            },
        )
        .expect("preview")
        .expect("rewritten bytes");

        assert!(
            rewritten
                .windows(b"Targetmage - Stormrage".len())
                .any(|window| window == b"Targetmage - Stormrage"),
            "{fixture} should contain the rewritten identity"
        );
        assert!(
            !rewritten
                .windows(b"Examplemage - Illidan".len())
                .any(|window| window == b"Examplemage - Illidan"),
            "{fixture} should not keep the source identity"
        );
    }
}

#[test]
fn preview_lua_bytes_rewrite_allows_known_identity_prefix_rule_without_field_markers() {
    let rewritten = preview_lua_bytes_rewrite(
        Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/DBM-Core.lua"),
        br#"DBMCoreDB = { ["char"] = { ["Examplemage - Illidan"] = {} } }"#,
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
fn preview_lua_bytes_rewrite_does_not_byte_fallback_after_utf8_scope_miss() {
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

    assert!(rewritten.is_none());
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
fn preview_lua_bytes_rewrite_scopes_invalid_utf8_profile_key_rewrites() {
    let rewritten = preview_lua_bytes_rewrite(
        Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua"),
        b"prefix\xff DetailsDB = { [\"notes\"] = \"Examplemage - Illidan\", [\"profileKeys\"] = { [\"Examplemage - Illidan\"] = \"Examplemage - Illidan\" } }\xffsuffix",
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
            .windows(br#"["notes"] = "Examplemage - Illidan""#.len())
            .any(|window| window == br#"["notes"] = "Examplemage - Illidan""#)
    );
    assert!(
        rewritten
            .windows(b"Targetmage - Stormrage".len())
            .any(|window| window == b"Targetmage - Stormrage")
    );
}

#[test]
fn preview_lua_bytes_rewrite_scopes_invalid_utf8_identity_field_rewrites() {
    let rewritten = preview_lua_bytes_rewrite(
        Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua"),
        b"prefix\xff DetailsDB = { [\"notes\"] = \"Examplemage\", [\"lastPlayerName\"] = \"Examplemage\", [\"playerName\"] = \"Examplemage\", [\"realm\"] = \"Illidan\" }\xffsuffix",
        &[sample_mapping()],
        LuaRewriteOptions {
            rewrite_profile_keys: false,
            rewrite_identity_strings: true,
        },
    )
    .expect("preview")
    .expect("rewritten bytes");

    assert!(
        rewritten
            .windows(br#"["notes"] = "Examplemage""#.len())
            .any(|window| window == br#"["notes"] = "Examplemage""#)
    );
    assert!(
        rewritten
            .windows(br#"["lastPlayerName"] = "Examplemage""#.len())
            .any(|window| window == br#"["lastPlayerName"] = "Examplemage""#)
    );
    assert!(
        rewritten
            .windows(br#"["playerName"] = "Targetmage""#.len())
            .any(|window| window == br#"["playerName"] = "Targetmage""#)
    );
    assert!(
        rewritten
            .windows(br#"["realm"] = "Stormrage""#.len())
            .any(|window| window == br#"["realm"] = "Stormrage""#)
    );
}

#[test]
fn preview_lua_bytes_rewrite_scopes_invalid_utf8_identity_key_rewrites() {
    let rewritten = preview_lua_bytes_rewrite(
        Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/ElvUI.lua"),
        b"prefix\xff E = { [\"char\"] = { [\"Examplemage - Illidan\"] = {} }, [\"cache\"] = { [\"Examplemage - Illidan\"] = true } }\xffsuffix",
        &[sample_mapping()],
        LuaRewriteOptions {
            rewrite_profile_keys: false,
            rewrite_identity_strings: true,
        },
    )
    .expect("preview")
    .expect("rewritten bytes");

    assert!(
        rewritten
            .windows(br#"["Targetmage - Stormrage"] = {}"#.len())
            .any(|window| window == br#"["Targetmage - Stormrage"] = {}"#)
    );
    assert!(
        rewritten
            .windows(br#"["cache"] = { ["Examplemage - Illidan"] = true }"#.len())
            .any(|window| window == br#"["cache"] = { ["Examplemage - Illidan"] = true }"#)
    );
}

#[test]
fn preview_lua_bytes_rewrite_rewrites_sanitized_realistic_utf8_fixture() {
    let payload = load_text_fixture_bytes("details_realistic_utf8.lua");
    let rewritten = preview_lua_bytes_rewrite(
        Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua"),
        &payload,
        &[sample_mapping()],
        LuaRewriteOptions {
            rewrite_profile_keys: true,
            rewrite_identity_strings: true,
        },
    )
    .expect("preview")
    .expect("rewritten bytes");

    let rewritten_text = String::from_utf8(rewritten).expect("utf8 fixture should remain utf8");
    assert!(rewritten_text.contains("Targetmage - Stormrage"));
    assert!(rewritten_text.contains(r#"["playerName"] = "Targetmage""#));
    assert!(rewritten_text.contains(r#"["realm"] = "Stormrage""#));
    assert!(rewritten_text.contains(r#"["lastPlayerName"] = "Examplemage""#));
    assert!(rewritten_text.contains("中文提示：保留原样"));
    assert!(rewritten_text.contains("欢迎回来，冒险者"));
    assert!(rewritten_text.contains("伊利丹服务器公告不应该被误改"));
    assert!(!rewritten_text.contains(r#"["playerName"] = "Examplemage""#));
    assert!(!rewritten_text.contains(r#"["realm"] = "Illidan""#));
}

#[test]
fn preview_lua_bytes_rewrite_rewrites_realistic_clique_utf8_fixture() {
    let payload = load_text_fixture_bytes("clique_realistic_utf8.lua");
    let rewritten = preview_lua_bytes_rewrite(
        Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/Clique.lua"),
        &payload,
        &[localized_profile_mapping()],
        LuaRewriteOptions {
            rewrite_profile_keys: true,
            rewrite_identity_strings: true,
        },
    )
    .expect("preview")
    .expect("rewritten bytes");

    let rewritten_text = String::from_utf8(rewritten).expect("utf8 fixture should remain utf8");
    assert!(rewritten_text.contains("暮光花雨 - 白银之手"));
    assert!(rewritten_text.contains(r#"["spec1_profileKey"] = "暮光花雨 - 白银之手""#));
    assert!(rewritten_text.contains(r#"["spec2_profileKey"] = "暮光花雨 - 白银之手""#));
    assert!(rewritten_text.contains(r#"["猫冬 - 丹莫德"] = "露露缇娅 - 奶德""#));
    assert!(rewritten_text.contains(r#"["spell"] = "回春术""#));
    assert!(rewritten_text.contains(r#"["notes"] = "保留中文说明""#));
    assert!(!rewritten_text.contains("露露缇娅 - 迅捷微风"));
}

#[test]
fn preview_lua_bytes_rewrite_rewrites_realistic_addonskins_utf8_fixture() {
    let payload = load_text_fixture_bytes("addonskins_realistic_utf8.lua");
    let rewritten = preview_lua_bytes_rewrite(
        Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/AddOnSkins.lua"),
        &payload,
        &[localized_profile_mapping()],
        LuaRewriteOptions {
            rewrite_profile_keys: true,
            rewrite_identity_strings: true,
        },
    )
    .expect("preview")
    .expect("rewritten bytes");

    let rewritten_text = String::from_utf8(rewritten).expect("utf8 fixture should remain utf8");
    assert!(rewritten_text.contains(r#"["暮光花雨 - 白银之手"] = {"#));
    assert!(rewritten_text.contains(r#"["暮光花雨 - 白银之手"] = "Default""#));
    assert!(rewritten_text.contains(r#"["萌萌奶露 - 萨尔"] = {"#));
    assert!(rewritten_text.contains(r#"["WeakAuras"] = true"#));
    assert!(rewritten_text.contains(r#"["BugSack"] = false"#));
    assert!(!rewritten_text.contains(r#"["露露缇娅 - 迅捷微风"] = {"#));
    assert!(!rewritten_text.contains(r#"["露露缇娅 - 迅捷微风"] = "Default""#));
}

#[test]
fn preview_lua_bytes_rewrite_rewrites_realistic_rurutiasuite_utf8_fixture() {
    let payload = load_text_fixture_bytes("rurutiasuite_realistic_utf8.lua");
    let rewritten = preview_lua_bytes_rewrite(
        Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/RurutiaSuite.lua"),
        &payload,
        &[localized_profile_mapping()],
        LuaRewriteOptions {
            rewrite_profile_keys: true,
            rewrite_identity_strings: true,
        },
    )
    .expect("preview")
    .expect("rewritten bytes");

    let rewritten_text = String::from_utf8(rewritten).expect("utf8 fixture should remain utf8");
    assert!(rewritten_text.contains(r#"["暮光花雨 - 白银之手"] = "Default""#));
    assert!(rewritten_text.contains(r#"["露露緹婭 - 迅捷微风"] = "Default""#));
    assert!(rewritten_text.contains(r#"["露露缇娅old20260322013345 - 迅捷微风"] = "Default""#));
    assert!(rewritten_text.contains(r#"["text"] = "Rurutia 4.16.2-N Retail 露露緹婭@BiliBili""#));
    assert!(rewritten_text.contains(r#"["notes"] = "露露缇娅 - 迅捷微风""#));
    assert!(!rewritten_text.contains(r#"["露露缇娅 - 迅捷微风"] = "Default""#));
}

#[test]
fn preview_lua_bytes_rewrite_rewrites_bigwigs_profile_keys_without_notes_identity() {
    let payload = load_text_fixture_bytes("bigwigs_profilekeys_utf8.lua");
    let rewritten = preview_lua_bytes_rewrite(
        Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/BigWigs.lua"),
        &payload,
        &[localized_profile_mapping()],
        LuaRewriteOptions {
            rewrite_profile_keys: true,
            rewrite_identity_strings: true,
        },
    )
    .expect("preview")
    .expect("rewritten bytes");

    let rewritten_text = String::from_utf8(rewritten).expect("utf8 fixture should remain utf8");
    assert!(rewritten_text.contains(r#"["暮光花雨 - 白银之手"] = "Default""#));
    assert!(rewritten_text.contains(r#"["萌萌奶露 - 萨尔"] = "Default""#));
    assert!(rewritten_text.contains("露露缇娅 - 迅捷微风 的团队提醒说明不应改写"));
    assert!(!rewritten_text.contains(r#"["露露缇娅 - 迅捷微风"] = "Default""#));
}

#[test]
fn preview_lua_bytes_rewrite_rewrites_realistic_ndui_bags_utf8_fixture() {
    let payload = load_text_fixture_bytes("ndui_bags_realistic_utf8.lua");
    let rewritten = preview_lua_bytes_rewrite(
        Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/NDui_Bags.lua"),
        &payload,
        &[localized_bagsync_mapping()],
        LuaRewriteOptions {
            rewrite_profile_keys: true,
            rewrite_identity_strings: true,
        },
    )
    .expect("preview")
    .expect("rewritten bytes");

    let rewritten_text = String::from_utf8(rewritten).expect("utf8 fixture should remain utf8");
    assert!(rewritten_text.contains(r#"["暮光花雨 - 白银之手"] = "Default""#));
    assert!(rewritten_text.contains(r#"["源小黑 - 贫瘠之地"] = "Default""#));
    assert!(rewritten_text.contains(r#"["焱天狼 - 阿古斯"] = "Default""#));
    assert!(rewritten_text.contains(r#"["FontName"] = "Rurutia""#));
    assert!(rewritten_text.contains(r#"["note"] = "焱天狼的背包配置""#));
    assert!(!rewritten_text.contains(r#"["焱天狼 - 贫瘠之地"] = "Default""#));
}

#[test]
fn preview_lua_bytes_rewrite_rewrites_realistic_bagsync_utf8_fixture() {
    let payload = load_text_fixture_bytes("bagsync_realistic_utf8.lua");
    let rewritten = preview_lua_bytes_rewrite(
        Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/BagSync.lua"),
        &payload,
        &[localized_bagsync_mapping()],
        LuaRewriteOptions {
            rewrite_profile_keys: false,
            rewrite_identity_strings: true,
        },
    )
    .expect("preview")
    .expect("rewritten bytes");

    let rewritten_text = String::from_utf8(rewritten).expect("utf8 fixture should remain utf8");
    assert!(rewritten_text.contains(r#"["白银之手"] = {"#));
    assert!(rewritten_text.contains(r#"["暮光花雨"] = {"#));
    assert!(rewritten_text.contains(r#"["guildrealm"] = "白银之手""#));
    assert!(rewritten_text.contains(r#"["realmKey"] = "白银之手""#));
    assert!(rewritten_text.contains(r#"["notes"] = "保留中文说明""#));
    assert!(!rewritten_text.contains(r#"["贫瘠之地"] = {"#));
    assert!(!rewritten_text.contains(r#"["焱天狼"] = {"#));
}

#[test]
fn preview_lua_bytes_rewrite_rewrites_realistic_elvui_utf8_fixture() {
    let payload = load_text_fixture_bytes("elvui_realistic_utf8.lua");
    let rewritten = preview_lua_bytes_rewrite(
        Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/ElvUI.lua"),
        &payload,
        &[localized_profile_mapping()],
        LuaRewriteOptions {
            rewrite_profile_keys: true,
            rewrite_identity_strings: true,
        },
    )
    .expect("preview")
    .expect("rewritten bytes");

    let rewritten_text = String::from_utf8(rewritten).expect("utf8 fixture should remain utf8");
    assert!(rewritten_text.contains(r#"["暮光花雨 - 白银之手"] = {"#));
    assert!(rewritten_text.contains(r#"["白银之手"] = {"#));
    assert!(rewritten_text.contains(r#"["暮光花雨"] = "Alliance""#));
    assert!(rewritten_text.contains(r#"["暮光花雨-白银之手"] = {"#));
    assert!(rewritten_text.contains(r#"["realm"] = "白银之手""#));
    assert!(rewritten_text.contains(r#"["露露缇娅 - 萨尔"] = "Default""#));
    assert!(rewritten_text.contains(r#"["露露缇娅-萨尔"] = {"#));
    assert!(rewritten_text.contains(r#"["namefont"] = "聊天""#));
    assert!(!rewritten_text.contains(r#"["露露缇娅 - 迅捷微风"] = {"#));
    assert!(!rewritten_text.contains(r#"["露露缇娅-迅捷微风"] = {"#));
}

#[test]
fn preview_lua_bytes_rewrite_rewrites_realistic_newbeebox_utf8_fixture() {
    let payload = load_text_fixture_bytes("newbeebox_realistic_utf8.lua");
    let rewritten = preview_lua_bytes_rewrite(
        Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/NewBeeBox.lua"),
        &payload,
        &[localized_newbeebox_mapping()],
        LuaRewriteOptions {
            rewrite_profile_keys: false,
            rewrite_identity_strings: true,
        },
    )
    .expect("preview")
    .expect("rewritten bytes");

    let rewritten_text = String::from_utf8(rewritten).expect("utf8 fixture should remain utf8");
    assert!(rewritten_text.contains(r#"["白银之手-暮光花雨"] = "67057金73银10铜""#));
    assert!(rewritten_text.contains(r#"["白银之手-暮光花雨"] = "3(水能堡)""#));
    assert!(rewritten_text.contains(r#"["name"] = "暮光花雨""#));
    assert!(rewritten_text.contains(r#"["realmName"] = "白银之手""#));
    assert!(rewritten_text.contains(r#"["playerGUID"] = "Player-917-047D8ED6""#));
    assert!(rewritten_text.contains(r#"["迅捷微风-露露緹婭丶"] = "4(恐惧陷坑)""#));
    assert!(rewritten_text.contains(r#"["name"] = "迅捷白山羊""#));
    assert!(!rewritten_text.contains(r#"["迅捷微风-露露缇娅丶"] = "67057金73银10铜""#));
    assert!(!rewritten_text.contains(r#"["name"] = "露露缇娅丶""#));
    assert!(!rewritten_text.contains(r#"["realmName"] = "迅捷微风""#));
}

#[test]
fn preview_lua_bytes_rewrite_rewrites_rarity_profile_keys_without_account_statistics_identity() {
    let payload = load_text_fixture_bytes("rarity_realistic_utf8.lua");
    let rewritten = preview_lua_bytes_rewrite(
        Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/Rarity.lua"),
        &payload,
        &[localized_bagsync_mapping()],
        LuaRewriteOptions {
            rewrite_profile_keys: true,
            rewrite_identity_strings: true,
        },
    )
    .expect("preview")
    .expect("rewritten bytes");

    let rewritten_text = String::from_utf8(rewritten).expect("utf8 fixture should remain utf8");
    assert!(rewritten_text.contains(r#"["暮光花雨 - 白银之手"] = "Default""#));
    assert!(rewritten_text.contains(r#"["艾露蒽之盾 - 贫瘠之地"] = "Default""#));
    assert!(rewritten_text.contains(r#"["playerName"] = "焱天狼""#));
    assert!(rewritten_text.contains(r#"["playerName"] = "艾露蒽之盾""#));
    assert_eq!(
        rewritten_text.matches(r#"["server"] = "贫瘠之地""#).count(),
        2
    );
    assert!(rewritten_text.contains("\"北贫瘠之地\""));
    assert!(rewritten_text.contains("\"南贫瘠之地\""));
    assert!(!rewritten_text.contains(r#"["焱天狼 - 贫瘠之地"] = "Default""#));
    assert!(!rewritten_text.contains(r#"["playerName"] = "暮光花雨""#));
    assert!(!rewritten_text.contains(r#"["server"] = "白银之手""#));
}

#[test]
fn preview_lua_bytes_rewrite_rejects_realistic_baganator_recent_character_cache() {
    let payload = load_text_fixture_bytes("baganator_recent_characters_utf8.lua");
    let rewritten = preview_lua_bytes_rewrite(
        Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/Baganator.lua"),
        &payload,
        &[localized_profile_mapping()],
        LuaRewriteOptions {
            rewrite_profile_keys: true,
            rewrite_identity_strings: true,
        },
    )
    .expect("preview");

    assert!(rewritten.is_none());
}

#[test]
fn preview_lua_bytes_rewrite_handles_real_world_fixture_invalid_utf8_payload() {
    let payload = load_escaped_byte_fixture("auctionator_invalid_utf8.lua.escape");
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
            .windows(br#"["RealmOne"]"#.len())
            .any(|window| window == br#"["RealmOne"]"#)
    );
    assert!(
        rewritten
            .windows(b"Targetmage - Stormrage".len())
            .any(|window| window == b"Targetmage - Stormrage")
    );
    assert!(
        rewritten
            .windows(10)
            .any(|window| window == [0xa1, b'G', b'v', b'e', b'r', b's', b'i', b'o', b'n', 0x02])
    );
}

#[test]
fn preview_lua_bytes_rewrite_rewrites_sanitized_realistic_latin1_fixture() {
    let payload = load_escaped_byte_fixture("pawn_realistic_latin1.lua.escape");
    let rewritten = preview_lua_bytes_rewrite(
        Path::new("wtf/characters/ACCOUNT/Illidan/Renée/SavedVariables/Pawn.lua"),
        &payload,
        &[CharacterMapping {
            source_account: Some("ACCOUNT".to_string()),
            source_server: "Illidan".to_string(),
            source_character: "Renée".to_string(),
            target_account: "TARGET".to_string(),
            target_server: "Stormrage".to_string(),
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
        rewritten
            .windows(b"Stormrage".len())
            .any(|window| window == b"Stormrage")
    );
    assert!(
        rewritten
            .windows(b"A\xf1o".len())
            .any(|window| window == b"A\xf1o")
    );
    assert!(
        !rewritten
            .windows(b"Ren\xe9e".len())
            .any(|window| window == b"Ren\xe9e")
    );
    assert!(
        !rewritten
            .windows(b"Illidan".len())
            .any(|window| window == b"Illidan")
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
