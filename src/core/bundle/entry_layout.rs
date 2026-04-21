use super::constants::MANIFEST_ENTRY;
use super::shared::path::safe_zip_segments;
use super::types::apply::{ApplyGroup, WtfScope};
use super::wtf_scope::{classify_account_wtf_scope, classify_character_wtf_scope};
use crate::core::error::AppResult;

pub(in crate::core::bundle) enum BundleArchiveEntry<'a> {
    Metadata {
        rest: Vec<&'a str>,
    },
    Addon {
        rest: Vec<&'a str>,
    },
    CommonConfig,
    CommonRootSavedVariables {
        rest: Vec<&'a str>,
    },
    CommonAccountSavedVariables {
        source_account: &'a str,
        rest: Vec<&'a str>,
    },
    CommonAccountFile {
        source_account: &'a str,
        rest: Vec<&'a str>,
    },
    CharacterFile {
        source_account: &'a str,
        server: &'a str,
        character: &'a str,
        rest: Vec<&'a str>,
    },
    Fonts {
        rest: Vec<&'a str>,
    },
    Interface {
        rest: Vec<&'a str>,
    },
}

pub(in crate::core::bundle) struct BundleArchiveEntryMetadata<'a> {
    pub(in crate::core::bundle) group: ApplyGroup,
    pub(in crate::core::bundle) wtf_scope: Option<WtfScope>,
    pub(in crate::core::bundle) source_account: Option<&'a str>,
    pub(in crate::core::bundle) source_server: Option<&'a str>,
    pub(in crate::core::bundle) source_character: Option<&'a str>,
}

impl<'a> BundleArchiveEntry<'a> {
    pub(in crate::core::bundle) fn metadata(&self) -> BundleArchiveEntryMetadata<'a> {
        match self {
            Self::Metadata { .. } => BundleArchiveEntryMetadata {
                group: ApplyGroup::Metadata,
                wtf_scope: None,
                source_account: None,
                source_server: None,
                source_character: None,
            },
            Self::Addon { .. } => BundleArchiveEntryMetadata {
                group: ApplyGroup::Addons,
                wtf_scope: None,
                source_account: None,
                source_server: None,
                source_character: None,
            },
            Self::CommonConfig => BundleArchiveEntryMetadata {
                group: ApplyGroup::WtfCommon,
                wtf_scope: Some(WtfScope::GlobalConfig),
                source_account: None,
                source_server: None,
                source_character: None,
            },
            Self::CommonRootSavedVariables { .. } => BundleArchiveEntryMetadata {
                group: ApplyGroup::WtfCommon,
                wtf_scope: Some(WtfScope::RootSavedVariables),
                source_account: None,
                source_server: None,
                source_character: None,
            },
            Self::CommonAccountSavedVariables { source_account, .. } => {
                BundleArchiveEntryMetadata {
                    group: ApplyGroup::WtfCommon,
                    wtf_scope: Some(WtfScope::AccountSavedVariables),
                    source_account: Some(*source_account),
                    source_server: None,
                    source_character: None,
                }
            }
            Self::CommonAccountFile {
                source_account,
                rest,
            } => BundleArchiveEntryMetadata {
                group: ApplyGroup::WtfCommon,
                wtf_scope: Some(classify_account_wtf_scope(rest)),
                source_account: Some(*source_account),
                source_server: None,
                source_character: None,
            },
            Self::CharacterFile {
                source_account,
                server,
                character,
                rest,
            } => BundleArchiveEntryMetadata {
                group: ApplyGroup::WtfCharacters,
                wtf_scope: Some(classify_character_wtf_scope(rest)),
                source_account: Some(*source_account),
                source_server: Some(*server),
                source_character: Some(*character),
            },
            Self::Fonts { .. } => BundleArchiveEntryMetadata {
                group: ApplyGroup::Fonts,
                wtf_scope: None,
                source_account: None,
                source_server: None,
                source_character: None,
            },
            Self::Interface { .. } => BundleArchiveEntryMetadata {
                group: ApplyGroup::InterfaceAssets,
                wtf_scope: None,
                source_account: None,
                source_server: None,
                source_character: None,
            },
        }
    }
}

pub(in crate::core::bundle) fn classify_bundle_archive_entry(
    archive_name: &str,
) -> AppResult<Option<BundleArchiveEntry<'_>>> {
    if archive_name == MANIFEST_ENTRY {
        return Ok(None);
    }

    let segments = safe_zip_segments(archive_name)?;
    if segments.is_empty() {
        return Ok(None);
    }

    let entry = match segments.as_slice() {
        ["metadata", rest @ ..] if !rest.is_empty() => BundleArchiveEntry::Metadata {
            rest: rest.to_vec(),
        },
        ["addons", rest @ ..] if !rest.is_empty() => BundleArchiveEntry::Addon {
            rest: rest.to_vec(),
        },
        ["wtf", "common", "Config.wtf"] => BundleArchiveEntry::CommonConfig,
        ["wtf", "common", "root", "SavedVariables", rest @ ..] if !rest.is_empty() => {
            BundleArchiveEntry::CommonRootSavedVariables {
                rest: rest.to_vec(),
            }
        }
        [
            "wtf",
            "common",
            "accounts",
            source_account,
            "SavedVariables",
            rest @ ..,
        ] if !rest.is_empty() => BundleArchiveEntry::CommonAccountSavedVariables {
            source_account,
            rest: rest.to_vec(),
        },
        ["wtf", "common", "accounts", source_account, rest @ ..] if !rest.is_empty() => {
            BundleArchiveEntry::CommonAccountFile {
                source_account,
                rest: rest.to_vec(),
            }
        }
        [
            "wtf",
            "characters",
            source_account,
            server,
            character,
            rest @ ..,
        ] if !rest.is_empty() => BundleArchiveEntry::CharacterFile {
            source_account,
            server,
            character,
            rest: rest.to_vec(),
        },
        ["fonts", rest @ ..] if !rest.is_empty() => BundleArchiveEntry::Fonts {
            rest: rest.to_vec(),
        },
        ["interface", rest @ ..] if !rest.is_empty() => BundleArchiveEntry::Interface {
            rest: rest.to_vec(),
        },
        _ => return Ok(None),
    };

    Ok(Some(entry))
}

#[cfg(test)]
mod tests {
    use super::classify_bundle_archive_entry;
    use crate::core::bundle::types::apply::{ApplyGroup, WtfScope};

    #[test]
    fn bundle_archive_entry_metadata_maps_common_account_cache_file() {
        let entry = classify_bundle_archive_entry("wtf/common/accounts/ACCOUNT/config-cache.wtf")
            .expect("valid bundle archive entry")
            .expect("recognized account entry");
        let metadata = entry.metadata();

        assert_eq!(metadata.group, ApplyGroup::WtfCommon);
        assert_eq!(metadata.wtf_scope, Some(WtfScope::CacheLike));
        assert_eq!(metadata.source_account, Some("ACCOUNT"));
        assert_eq!(metadata.source_server, None);
        assert_eq!(metadata.source_character, None);
    }

    #[test]
    fn bundle_archive_entry_metadata_maps_character_saved_variables_identity() {
        let entry = classify_bundle_archive_entry(
            "wtf/characters/ACCOUNT/Illidan/Examplemage/SavedVariables/Pawn.lua",
        )
        .expect("valid bundle archive entry")
        .expect("recognized character entry");
        let metadata = entry.metadata();

        assert_eq!(metadata.group, ApplyGroup::WtfCharacters);
        assert_eq!(metadata.wtf_scope, Some(WtfScope::CharacterSavedVariables));
        assert_eq!(metadata.source_account, Some("ACCOUNT"));
        assert_eq!(metadata.source_server, Some("Illidan"));
        assert_eq!(metadata.source_character, Some("Examplemage"));
    }
}
