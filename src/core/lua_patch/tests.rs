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
