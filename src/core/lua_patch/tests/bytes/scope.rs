use super::super::*;

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
