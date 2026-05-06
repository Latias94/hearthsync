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
        AddonSourceRef::WagoAddon {
            project_id,
            release_id,
        } => Some(match release_id {
            Some(release_id) => format!("wago-{project_id}-{release_id}"),
            None => format!("wago-{project_id}"),
        }),
        AddonSourceRef::TukuiAddon { slug, version } => Some(match version {
            Some(version) => format!("tukui-{slug}-{version}"),
            None => format!("tukui-{slug}"),
        }),
    }
    .or_else(|| addon_names.first().map(|name| (*name).to_string()))
    .unwrap_or_else(|| "addon-package".to_string());

    slugify_package_id(&base)
}

pub(crate) fn slugify_package_id(value: &str) -> String {
    let mut slug = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }

    slug.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::derive_package_id;
    use crate::core::addon::AddonSourceRef;

    #[test]
    fn derive_package_id_uses_github_owner_and_repo_when_release_is_unpinned() {
        let source = AddonSourceRef::GitHubRelease {
            owner: "Tercioo".to_string(),
            repo: "Plater-Nameplates".to_string(),
            tag: None,
            asset_name: None,
        };

        assert_eq!(derive_package_id(&source, &[]), "tercioo-plater-nameplates");
    }

    #[test]
    fn derive_package_id_uses_wago_project_id_when_release_is_unpinned() {
        let source = AddonSourceRef::WagoAddon {
            project_id: "VBNBxKx5".to_string(),
            release_id: None,
        };

        assert_eq!(derive_package_id(&source, &[]), "wago-vbnbxkx5");
    }

    #[test]
    fn derive_package_id_uses_tukui_slug_when_version_is_unpinned() {
        let source = AddonSourceRef::TukuiAddon {
            slug: "elvui".to_string(),
            version: None,
        };

        assert_eq!(derive_package_id(&source, &[]), "tukui-elvui");
    }
}
