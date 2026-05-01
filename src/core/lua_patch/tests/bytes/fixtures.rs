use super::super::*;

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
