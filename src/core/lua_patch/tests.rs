use std::{
    fs,
    path::{Path, PathBuf},
};

mod bytes;
mod text;

use super::{CharacterMapping, LuaRewriteOptions, preview_lua_bytes_rewrite};

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
