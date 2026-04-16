use std::path::Path;

use super::AddonSourceRef;

pub(super) fn derive_package_id(source: &AddonSourceRef, addon_names: &[&str]) -> String {
    let base = match source {
        AddonSourceRef::LocalArchive { path } => path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string()),
        AddonSourceRef::HttpArchive { url } => Path::new(url)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string()),
        AddonSourceRef::CurseForgeMod { mod_id, file_id } => Some(match file_id {
            Some(file_id) => format!("curseforge-{mod_id}-{file_id}"),
            None => format!("curseforge-{mod_id}"),
        }),
        AddonSourceRef::GitHubRelease {
            owner,
            repo,
            tag,
            asset_name,
        } => asset_name
            .as_deref()
            .and_then(|value| Path::new(value).file_stem().and_then(|stem| stem.to_str()))
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
            .or_else(|| {
                tag.as_ref()
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| format!("{repo}-{value}"))
            })
            .or_else(|| Some(format!("{owner}-{repo}"))),
    }
    .or_else(|| addon_names.first().map(|name| (*name).to_string()))
    .unwrap_or_else(|| "addon-package".to_string());

    let mut slug = String::new();
    for character in base.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }

    slug.trim_matches('-').to_string()
}
