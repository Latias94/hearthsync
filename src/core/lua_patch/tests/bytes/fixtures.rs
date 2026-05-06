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
fn preview_lua_bytes_rewrite_rewrites_weakaurasarchive_top_level_identity_keys_only() {
    let payload = load_text_fixture_bytes("weakaurasarchive_identity_keys_utf8.lua");
    let rewritten = preview_lua_bytes_rewrite(
        Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/WeakAurasArchive.lua"),
        &payload,
        &[sample_mapping()],
        LuaRewriteOptions {
            rewrite_profile_keys: false,
            rewrite_identity_strings: true,
        },
    )
    .expect("preview")
    .expect("rewritten bytes");

    let rewritten_text = String::from_utf8(rewritten).expect("utf8 fixture should remain utf8");
    assert!(rewritten_text.contains(r#"["Targetmage - Stormrage"] = {"#));
    assert!(rewritten_text.contains(r#"["Targetmage-Stormrage"] = {"#));
    assert!(rewritten_text.contains(r#"["Stormrage-Targetmage"] = {"#));
    assert!(rewritten_text.contains(r#"["author"] = "Examplemage - Illidan""#));
    assert!(rewritten_text.contains(r#"["cache"] = {"#));
    assert!(rewritten_text.contains(r#"["Examplemage - Illidan"] = {"#));
}

#[test]
fn preview_lua_bytes_rewrite_rewrites_zygorguidesviewer_profile_containers_without_notes() {
    let payload = load_text_fixture_bytes("zygorguidesviewer_realistic_utf8.lua");
    let rewritten = preview_lua_bytes_rewrite(
        Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/ZygorGuidesViewer.lua"),
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
    assert!(rewritten_text.contains(r#"["Targetmage - Stormrage"] = {"#));
    assert!(rewritten_text.contains(r#"["Targetmage - Stormrage"] = "Default""#));
    assert!(rewritten_text.contains(r#""LEVELING\\Dragon Isles""#));
    assert!(rewritten_text.contains("Examplemage - Illidan should remain in guide notes"));
    assert!(rewritten_text.contains(r#"["cache"] = {"#));
    assert!(rewritten_text.contains(r#"["Examplemage - Illidan"] = "outside known containers""#));
}

#[test]
fn preview_lua_bytes_rewrite_covers_common_allowlisted_addon_policy_fixtures() {
    struct Case {
        path: &'static str,
        fixture: &'static str,
        expected: &'static [&'static str],
        preserved: &'static [&'static str],
    }

    let cases = [
        Case {
            path: "wtf/common/accounts/ACCOUNT/SavedVariables/AuraUpdater.lua",
            fixture: "auraupdater_identity_container_utf8.lua",
            expected: &[r#"["Targetmage - Stormrage"] = {"#],
            preserved: &[r#"["Examplemage - Illidan"] = "outside known containers""#],
        },
        Case {
            path: "wtf/common/accounts/ACCOUNT/SavedVariables/Details_Streamer.lua",
            fixture: "details_streamer_profilekeys_utf8.lua",
            expected: &[
                r#"["Targetmage - Stormrage"] = "Default""#,
                r#"["Targetmage - Stormrage"] = {"#,
            ],
            preserved: &[
                r#"["streamer_name"] = "Examplemage - Illidan""#,
                "Examplemage - Illidan should remain in notes",
            ],
        },
        Case {
            path: "wtf/common/accounts/ACCOUNT/SavedVariables/ExWindCore.lua",
            fixture: "exwindcore_identity_fields_utf8.lua",
            expected: &[
                r#"["playerName"] = "Targetmage""#,
                r#"["realm"] = "Stormrage""#,
            ],
            preserved: &[
                "Examplemage on Illidan should stay in free text",
                r#"["lastPlayerName"] = "Examplemage""#,
            ],
        },
        Case {
            path: "wtf/common/accounts/ACCOUNT/SavedVariables/HandyNotes_TravelGuide.lua",
            fixture: "handynotes_travelguide_profilekeys_utf8.lua",
            expected: &[
                r#"["Targetmage - Stormrage"] = "Default""#,
                r#"["Targetmage - Stormrage"] = {"#,
            ],
            preserved: &[r#"["Examplemage - Illidan"] = "map note owner text""#],
        },
        Case {
            path: "wtf/common/accounts/ACCOUNT/SavedVariables/TinyTooltip-Remake.lua",
            fixture: "tinytooltip_remake_profilekeys_utf8.lua",
            expected: &[
                r#"["Targetmage - Stormrage"] = "Default""#,
                r#"["Targetmage - Stormrage"] = {"#,
            ],
            preserved: &[r#"["Examplemage - Illidan"] = "tooltip cache text""#],
        },
        Case {
            path: "wtf/common/accounts/ACCOUNT/SavedVariables/WeakAuras.lua",
            fixture: "weakauras_profilekeys_utf8.lua",
            expected: &[
                r#"["Targetmage - Stormrage"] = "Default""#,
                r#"["Targetmage - Stormrage"] = {"#,
            ],
            preserved: &[r#"["author"] = "Examplemage - Illidan""#],
        },
        Case {
            path: "wtf/common/accounts/ACCOUNT/SavedVariables/WorldQuestTracker.lua",
            fixture: "worldquesttracker_profilekeys_utf8.lua",
            expected: &[
                r#"["Targetmage - Stormrage"] = "Default""#,
                r#"["Targetmage - Stormrage"] = {"#,
            ],
            preserved: &[r#"["Examplemage - Illidan"] = "should remain historical text""#],
        },
    ];

    for case in cases {
        let payload = load_text_fixture_bytes(case.fixture);
        let rewritten = preview_lua_bytes_rewrite(
            Path::new(case.path),
            &payload,
            &[sample_mapping()],
            LuaRewriteOptions {
                rewrite_profile_keys: true,
                rewrite_identity_strings: true,
            },
        )
        .unwrap_or_else(|error| panic!("preview {}: {error}", case.fixture))
        .unwrap_or_else(|| panic!("{} should produce rewritten bytes", case.fixture));

        let rewritten_text = String::from_utf8(rewritten)
            .unwrap_or_else(|error| panic!("{} should remain utf8: {error}", case.fixture));
        let rewritten_text = rewritten_text.replace("\r\n", "\n");
        for expected in case.expected {
            assert!(
                rewritten_text.contains(expected),
                "{} should contain {expected}",
                case.fixture
            );
        }
        for preserved in case.preserved {
            assert!(
                rewritten_text.contains(preserved),
                "{} should preserve {preserved}",
                case.fixture
            );
        }
    }
}

#[test]
fn preview_lua_bytes_rewrite_covers_profile_marker_reduction_fixtures() {
    struct Case {
        path: &'static str,
        fixture: &'static str,
        expected: &'static [&'static str],
        preserved: &'static [&'static str],
    }

    let cases = [
        Case {
            path: "wtf/common/accounts/ACCOUNT/SavedVariables/Plater.lua",
            fixture: "plater_profilekeys_reduced_utf8.lua",
            expected: &[
                r#"["Targetmage - Stormrage"] = "Stormrage.Targetmage""#,
                r#"["Stormrage.Targetmage"] = {"#,
            ],
            preserved: &[
                "Examplemage - Illidan should remain in script text",
                r#"["Examplemage - Illidan"] = "historical script owner text""#,
            ],
        },
        Case {
            path: "wtf/common/accounts/ACCOUNT/SavedVariables/OmniCD.lua",
            fixture: "omnicd_profilekeys_char_reduced_utf8.lua",
            expected: &[
                r#"["Targetmage - Stormrage"] = "Default.Stormrage.Targetmage""#,
                r#"["Default.Stormrage.Targetmage"] = {"#,
            ],
            preserved: &[r#"["char"] = {
    ["Examplemage - Illidan"] = {"#],
        },
        Case {
            path: "wtf/common/accounts/ACCOUNT/SavedVariables/MRT.lua",
            fixture: "mrt_profilekeys_reduced_utf8.lua",
            expected: &[
                r#"["Targetmage - Stormrage"] = "Default""#,
                r#"["Stormrage.Targetmage"] = {"#,
                r#"["Default.Stormrage.Targetmage"] = {"#,
            ],
            preserved: &[
                "Examplemage - Illidan should remain in MRT note text",
                r#"["Examplemage - Illidan"] = "raid note author text""#,
            ],
        },
    ];

    for case in cases {
        let payload = load_text_fixture_bytes(case.fixture);
        let rewritten = preview_lua_bytes_rewrite(
            Path::new(case.path),
            &payload,
            &[sample_mapping()],
            LuaRewriteOptions {
                rewrite_profile_keys: true,
                rewrite_identity_strings: true,
            },
        )
        .unwrap_or_else(|error| panic!("preview {}: {error}", case.fixture))
        .unwrap_or_else(|| panic!("{} should produce rewritten bytes", case.fixture));

        let rewritten_text = String::from_utf8(rewritten)
            .unwrap_or_else(|error| panic!("{} should remain utf8: {error}", case.fixture));
        for expected in case.expected {
            assert!(
                rewritten_text.contains(expected),
                "{} should contain {expected}",
                case.fixture
            );
        }
        for preserved in case.preserved {
            assert!(
                rewritten_text.contains(preserved),
                "{} should preserve {preserved}",
                case.fixture
            );
        }
    }
}

#[test]
fn preview_lua_bytes_rewrite_covers_second_shape_identity_policy_fixtures() {
    struct Case {
        path: &'static str,
        fixture: &'static str,
        expected: &'static [&'static str],
        preserved: &'static [&'static str],
    }

    let cases = [
        Case {
            path: "wtf/common/accounts/ACCOUNT/SavedVariables/DBM-Party-WoD.lua",
            fixture: "dbm_party_compact_identity_utf8.lua",
            expected: &[r#"["Targetmage-Stormrage"] = {"#],
            preserved: &[
                "Examplemage-Illidan should remain in warning text",
                r#"["Examplemage-Illidan"] = "template cache text""#,
            ],
        },
        Case {
            path: "wtf/common/accounts/ACCOUNT/SavedVariables/Details_MythicPlus.lua",
            fixture: "details_mythicplus_identity_fields_utf8.lua",
            expected: &[
                r#"["playerName"] = "Targetmage""#,
                r#"["realm"] = "Stormrage""#,
            ],
            preserved: &[
                r#"["lastPlayerName"] = "Examplemage""#,
                "Examplemage on Illidan should stay in run notes",
            ],
        },
        Case {
            path: "wtf/common/accounts/ACCOUNT/SavedVariables/Details_MythicPlus.lua",
            fixture: "details_mythicplus_profiles_compact_utf8.lua",
            expected: &[r#"["Targetmage-Stormrage"] = {"#],
            preserved: &[r#"["Examplemage-Illidan"] = "compact cache text""#],
        },
        Case {
            path: "wtf/common/accounts/ACCOUNT/SavedVariables/MeetingStone.lua",
            fixture: "meetingstone_search_history_context_utf8.lua",
            expected: &[r#"["Targetmage - Stormrage"] = {"#],
            preserved: &[r#"["Examplemage - Illidan"] = "activity label text""#],
        },
        Case {
            path: "wtf/common/accounts/ACCOUNT/SavedVariables/SavedInstances.lua",
            fixture: "savedinstances_toon_multifield_utf8.lua",
            expected: &[r#"["Targetmage - Stormrage"] = {"#],
            preserved: &[
                "Examplemage - Illidan should remain in note text",
                r#"["Examplemage - Illidan"] = "historical lockout text""#,
            ],
        },
        Case {
            path: "wtf/common/accounts/ACCOUNT/SavedVariables/TinyTooltip-Remake.lua",
            fixture: "tinytooltip_remake_realm_field_utf8.lua",
            expected: &[r#"["realm"] = "Stormrage""#],
            preserved: &["Illidan should remain in free text"],
        },
    ];

    for case in cases {
        let payload = load_text_fixture_bytes(case.fixture);
        let rewritten = preview_lua_bytes_rewrite(
            Path::new(case.path),
            &payload,
            &[sample_mapping()],
            LuaRewriteOptions {
                rewrite_profile_keys: true,
                rewrite_identity_strings: true,
            },
        )
        .unwrap_or_else(|error| panic!("preview {}: {error}", case.fixture))
        .unwrap_or_else(|| panic!("{} should produce rewritten bytes", case.fixture));

        let rewritten_text = String::from_utf8(rewritten)
            .unwrap_or_else(|error| panic!("{} should remain utf8: {error}", case.fixture));
        for expected in case.expected {
            assert!(
                rewritten_text.contains(expected),
                "{} should contain {expected}",
                case.fixture
            );
        }
        for preserved in case.preserved {
            assert!(
                rewritten_text.contains(preserved),
                "{} should preserve {preserved}",
                case.fixture
            );
        }
    }
}

#[test]
fn preview_lua_bytes_rewrite_covers_controlled_shape_reductions() {
    struct Case {
        path: &'static str,
        fixture: &'static str,
        expected: &'static [&'static str],
        preserved: &'static [&'static str],
    }

    let cases = [
        Case {
            path: "wtf/common/accounts/ACCOUNT/SavedVariables/DBM-Core.lua",
            fixture: "dbm_core_reduced_compact_keys_utf8.lua",
            expected: &[r#"["Targetmage-Stormrage"] = {"#],
            preserved: &[
                "Examplemage-Illidan should remain in DBM option text",
                r#"["Examplemage-Illidan"] = true"#,
            ],
        },
        Case {
            path: "wtf/common/accounts/ACCOUNT/SavedVariables/DBM-Core.lua",
            fixture: "dbm_core_scalar_identity_tables_utf8.lua",
            expected: &[
                r#"["Targetmage-Stormrage"] = "Default""#,
                r#"["Targetmage-Stormrage"] = false"#,
                r#"["Targetmage-Stormrage"] = 20260505"#,
                r#"["Targetmage-Stormrage"] = {"#,
            ],
            preserved: &[
                "Examplemage-Illidan should remain in DBM option text",
                r#"["Examplemage-Illidan"] = true"#,
            ],
        },
        Case {
            path: "wtf/common/accounts/ACCOUNT/SavedVariables/EventsTracker.lua",
            fixture: "eventstracker_value_reduced_utf8.lua",
            expected: &[
                r#"["Targetmage - Stormrage"] = {"#,
                r#"["Targetmage-Stormrage"] = 7"#,
            ],
            preserved: &[r#"["Examplemage - Illidan"] = "historical event text""#],
        },
        Case {
            path: "wtf/common/accounts/ACCOUNT/SavedVariables/HandyNotes_Dragonflight.lua",
            fixture: "handynotes_dragonflight_value_reduced_utf8.lua",
            expected: &[r#"["Targetmage - Stormrage"] = {"#],
            preserved: &[r#"["Examplemage - Illidan"] = "map note owner text""#],
        },
        Case {
            path: "wtf/characters/ACCOUNT/Illidan/Examplemage/SavedVariables/MeetingStone.lua",
            fixture: "meetingstone_character_reduced_utf8.lua",
            expected: &[
                r#"["Targetmage - Stormrage"] = "Default""#,
                r#"["Targetmage - Stormrage"] = {"#,
                r#"["Targetmage-Stormrage"] = {"#,
            ],
            preserved: &[r#"["Examplemage-Illidan"] = 1234"#],
        },
        Case {
            path: "wtf/common/accounts/ACCOUNT/SavedVariables/SavedInstances.lua",
            fixture: "savedinstances_reduced_toon_compact_utf8.lua",
            expected: &[
                r#"["Targetmage - Stormrage"] = {"#,
                r#"["Targetmage-Stormrage"] = {"#,
            ],
            preserved: &[
                r#"["Examplemage - Illidan"] = "historical lockout text""#,
                r#"["Examplemage-Illidan"] = 99"#,
            ],
        },
        Case {
            path: "wtf/common/accounts/ACCOUNT/SavedVariables/WorldQuestTracker.lua",
            fixture: "worldquesttracker_reduced_realm_profiles_utf8.lua",
            expected: &[
                r#"["Targetmage - Stormrage"] = "Default""#,
                r#"["Targetmage - Stormrage"] = {"#,
                r#"["realm"] = "Stormrage""#,
            ],
            preserved: &[
                "Illidan should stay in quest note",
                r#"["Illidan"] = {"#,
                r#"["Examplemage"] = "historical quest owner""#,
            ],
        },
    ];

    for case in cases {
        let payload = load_text_fixture_bytes(case.fixture);
        let rewritten = preview_lua_bytes_rewrite(
            Path::new(case.path),
            &payload,
            &[sample_mapping()],
            LuaRewriteOptions {
                rewrite_profile_keys: true,
                rewrite_identity_strings: true,
            },
        )
        .unwrap_or_else(|error| panic!("preview {}: {error}", case.fixture))
        .unwrap_or_else(|| panic!("{} should produce rewritten bytes", case.fixture));

        let rewritten_text = String::from_utf8(rewritten)
            .unwrap_or_else(|error| panic!("{} should remain utf8: {error}", case.fixture));
        for expected in case.expected {
            assert!(
                rewritten_text.contains(expected),
                "{} should contain {expected}",
                case.fixture
            );
        }
        for preserved in case.preserved {
            assert!(
                rewritten_text.contains(preserved),
                "{} should preserve {preserved}",
                case.fixture
            );
        }
    }
}

#[test]
fn preview_lua_bytes_rewrite_keeps_weakauras_without_identity_markers_fail_closed() {
    let payload = load_text_fixture_bytes("weakauras_no_identity_utf8.lua");
    let rewritten = preview_lua_bytes_rewrite(
        Path::new("wtf/common/accounts/ACCOUNT/SavedVariables/WeakAuras.lua"),
        &payload,
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
fn preview_lua_bytes_rewrite_rejects_reduced_baganator_recent_character_cache() {
    let payload = load_text_fixture_bytes("baganator_recent_characters_reduced_utf8.lua");
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
