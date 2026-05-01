mod identity;
mod profile;
mod range;

use self::identity::rewrite_scoped_identity_text;
use self::profile::rewrite_scoped_profile_text;
use super::{CharacterMapping, LuaRewriteOptions};

pub fn rewrite_lua_text(
    content: &str,
    mappings: &[CharacterMapping],
    options: LuaRewriteOptions,
) -> String {
    let mut rewritten = content.to_string();

    if options.rewrite_profile_keys {
        rewritten = rewrite_scoped_profile_text(&rewritten, mappings);
    }

    if options.rewrite_identity_strings {
        rewritten = rewrite_scoped_identity_text(&rewritten, mappings);
    }

    rewritten
}
