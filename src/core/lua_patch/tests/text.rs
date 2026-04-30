use super::super::rewrite_lua_text;
use super::*;

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
