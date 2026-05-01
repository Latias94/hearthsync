mod encoding;
mod identity;
mod profile;
mod range;
mod rewrite;

use self::identity::collect_identity_replacements;
use self::profile::collect_profile_replacements;
use self::range::apply_range_replacements;
use super::{CharacterMapping, LuaRewriteOptions};

pub(super) fn rewrite_lua_bytes(
    content: &[u8],
    mappings: &[CharacterMapping],
    options: LuaRewriteOptions,
) -> Option<Vec<u8>> {
    let mut replacements = Vec::new();

    if options.rewrite_profile_keys {
        collect_profile_replacements(content, mappings, &mut replacements);
    }

    if options.rewrite_identity_strings {
        collect_identity_replacements(content, mappings, &mut replacements);
    }

    let rewritten = apply_range_replacements(content, replacements);
    if rewritten == content {
        None
    } else {
        Some(rewritten)
    }
}
