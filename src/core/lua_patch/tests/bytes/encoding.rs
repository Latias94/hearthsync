use super::super::*;

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
