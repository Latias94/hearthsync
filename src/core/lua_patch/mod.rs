mod bytes;
mod model;
mod policy;
mod syntax;
#[cfg(test)]
mod tests;
mod text;

use std::fs;
use std::path::Path;

use self::bytes::rewrite_lua_bytes;
use self::policy::DEFAULT_LUA_REWRITE_POLICY_REGISTRY;
use crate::core::error::AppResult;

pub use self::model::{CharacterMapping, LuaRewriteOptions};
pub use self::text::rewrite_lua_text;

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
    let Some(capabilities) = DEFAULT_LUA_REWRITE_POLICY_REGISTRY.analyze(path_hint, bytes) else {
        return Ok(None);
    };
    if mappings.is_empty() {
        return Ok(None);
    }

    let rewrite_options = options.limit_to(capabilities);
    if rewrite_options.is_disabled() {
        return Ok(None);
    }

    if let Ok(content) = std::str::from_utf8(bytes) {
        let rewritten = rewrite_lua_text(content, mappings, rewrite_options);
        if rewritten != content {
            return Ok(Some(rewritten.into_bytes()));
        }
        return Ok(None);
    }

    Ok(rewrite_lua_bytes(bytes, mappings, rewrite_options))
}
