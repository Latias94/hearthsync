use super::*;

pub(super) enum BundleArchiveEntry<'a> {
    Metadata {
        rest: Vec<&'a str>,
    },
    Addon {
        rest: Vec<&'a str>,
    },
    CommonConfig,
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

pub(super) fn classify_bundle_archive_entry(
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
