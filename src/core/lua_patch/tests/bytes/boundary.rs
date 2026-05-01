use super::super::*;

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
