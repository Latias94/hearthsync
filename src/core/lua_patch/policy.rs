use std::path::Path;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct LuaRewriteCapabilities {
    pub(crate) rewrite_profile_keys: bool,
    pub(crate) rewrite_identity_strings: bool,
}

impl LuaRewriteCapabilities {
    const IDENTITY_ONLY: Self = Self {
        rewrite_profile_keys: false,
        rewrite_identity_strings: true,
    };

    fn merge(self, other: Self) -> Self {
        Self {
            rewrite_profile_keys: self.rewrite_profile_keys || other.rewrite_profile_keys,
            rewrite_identity_strings: self.rewrite_identity_strings
                || other.rewrite_identity_strings,
        }
    }
}

#[derive(Debug)]
pub(crate) struct LuaRewritePolicyRegistry {
    profile_key_markers: &'static [&'static [u8]],
    rules: &'static [LuaRewriteRule],
}

impl LuaRewritePolicyRegistry {
    const fn new(
        profile_key_markers: &'static [&'static [u8]],
        rules: &'static [LuaRewriteRule],
    ) -> Self {
        Self {
            profile_key_markers,
            rules,
        }
    }

    pub(crate) fn analyze(&self, path: &Path, bytes: &[u8]) -> Option<LuaRewriteCapabilities> {
        let target = classify_lua_rewrite_target(path)?;
        let signal_capabilities = self.detect_signal_capabilities(bytes);
        let matched_rule_capabilities = self.matched_rule_capabilities(&target.file_name);
        Some(signal_capabilities.merge(matched_rule_capabilities))
    }

    fn detect_signal_capabilities(&self, bytes: &[u8]) -> LuaRewriteCapabilities {
        LuaRewriteCapabilities {
            rewrite_profile_keys: bytes_contain_any_ascii_marker(bytes, self.profile_key_markers),
            rewrite_identity_strings: false,
        }
    }

    fn matched_rule_capabilities(&self, file_name: &str) -> LuaRewriteCapabilities {
        self.rules
            .iter()
            .copied()
            .filter(|rule| rule.matches(file_name))
            .fold(LuaRewriteCapabilities::default(), |capabilities, rule| {
                capabilities.merge(rule.capabilities)
            })
    }
}

pub(crate) static DEFAULT_LUA_REWRITE_POLICY_REGISTRY: LuaRewritePolicyRegistry =
    LuaRewritePolicyRegistry::new(PROFILE_KEY_MARKERS, LUA_REWRITE_RULES);

const PROFILE_KEY_MARKERS: &[&[u8]] = &[b"profileKeys", b"ProfileKeys"];

#[derive(Debug, Clone, Copy)]
struct LuaRewriteRule {
    matcher: LuaRewriteRuleMatcher,
    capabilities: LuaRewriteCapabilities,
}

impl LuaRewriteRule {
    fn matches(self, file_name: &str) -> bool {
        match self.matcher {
            LuaRewriteRuleMatcher::Exact(expected) => file_name == expected,
            LuaRewriteRuleMatcher::Prefix(prefix) => file_name.starts_with(prefix),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum LuaRewriteRuleMatcher {
    Exact(&'static str),
    Prefix(&'static str),
}

const fn identity_exact_rule(file_name: &'static str) -> LuaRewriteRule {
    LuaRewriteRule {
        matcher: LuaRewriteRuleMatcher::Exact(file_name),
        capabilities: LuaRewriteCapabilities::IDENTITY_ONLY,
    }
}

const fn identity_prefix_rule(prefix: &'static str) -> LuaRewriteRule {
    LuaRewriteRule {
        matcher: LuaRewriteRuleMatcher::Prefix(prefix),
        capabilities: LuaRewriteCapabilities::IDENTITY_ONLY,
    }
}

const LUA_REWRITE_RULES: &[LuaRewriteRule] = &[
    identity_exact_rule("auraupdater.lua"),
    identity_exact_rule("bagsync.lua"),
    identity_exact_rule("clique.lua"),
    identity_exact_rule("details.lua"),
    identity_exact_rule("elvui.lua"),
    identity_exact_rule("eventstracker.lua"),
    identity_exact_rule("exwindcore.lua"),
    identity_exact_rule("meetingstone.lua"),
    identity_exact_rule("newbeebox.lua"),
    identity_exact_rule("pawn.lua"),
    identity_exact_rule("savedinstances.lua"),
    identity_exact_rule("tinytooltip-remake.lua"),
    identity_exact_rule("weakauras.lua"),
    identity_exact_rule("weakaurasarchive.lua"),
    identity_exact_rule("worldquesttracker.lua"),
    identity_exact_rule("zygorguidesviewer.lua"),
    identity_prefix_rule("dbm-"),
    identity_prefix_rule("details_"),
    identity_prefix_rule("handynotes_"),
];

fn bytes_contain_any_ascii_marker(bytes: &[u8], markers: &[&[u8]]) -> bool {
    markers.iter().any(|marker| {
        !marker.is_empty() && bytes.windows(marker.len()).any(|window| window == *marker)
    })
}

#[derive(Debug, Clone)]
struct LuaRewriteTarget {
    file_name: String,
}

fn classify_lua_rewrite_target(path: &Path) -> Option<LuaRewriteTarget> {
    let file_name = path.file_name()?.to_str()?.to_ascii_lowercase();
    if !file_name.ends_with(".lua") {
        return None;
    }

    let segments = path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .map(|segment| segment.to_ascii_lowercase())
        .collect::<Vec<_>>();

    if segments.len() >= 6
        && segments[segments.len() - 6] == "wtf"
        && segments[segments.len() - 5] == "common"
        && segments[segments.len() - 4] == "accounts"
        && segments[segments.len() - 2] == "savedvariables"
    {
        return Some(LuaRewriteTarget { file_name });
    }

    if segments.len() >= 5
        && segments[segments.len() - 5] == "wtf"
        && segments[segments.len() - 4] == "common"
        && segments[segments.len() - 3] == "root"
        && segments[segments.len() - 2] == "savedvariables"
    {
        return Some(LuaRewriteTarget { file_name });
    }

    if segments.len() >= 7
        && segments[segments.len() - 7] == "wtf"
        && segments[segments.len() - 6] == "characters"
        && segments[segments.len() - 2] == "savedvariables"
    {
        return Some(LuaRewriteTarget { file_name });
    }

    if segments.len() >= 4
        && segments[segments.len() - 4] == "account"
        && segments[segments.len() - 2] == "savedvariables"
    {
        return Some(LuaRewriteTarget { file_name });
    }

    if segments.len() >= 6
        && segments[segments.len() - 6] == "account"
        && segments[segments.len() - 2] == "savedvariables"
    {
        return Some(LuaRewriteTarget { file_name });
    }

    None
}
