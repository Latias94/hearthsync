use serde::{Deserialize, Serialize};

use super::AddonSourceRef;
use super::curseforge::{
    CurseForgeFile, CurseForgeGameVersionType, CurseForgeSortableGameVersion,
    select_curseforge_version_type, select_latest_curseforge_file, validate_curseforge_file,
};
use super::github::{GitHubRelease, GitHubReleaseAsset, select_github_release_asset};
use super::parse::{parse_curseforge_source, parse_github_source};
use crate::core::install::WowFlavor;

#[derive(Debug, Deserialize, Serialize)]
struct AddonSourceFixture {
    source: AddonSourceRef,
}

#[test]
fn addon_source_ref_uses_canonical_provider_kind_names() {
    let github = AddonSourceFixture {
        source: AddonSourceRef::GitHubRelease {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            tag: None,
            asset_name: None,
        },
    };
    let curseforge = AddonSourceFixture {
        source: AddonSourceRef::CurseForgeMod {
            mod_id: 12345,
            file_id: None,
        },
    };

    assert!(
        toml::to_string(&github)
            .expect("github source toml")
            .contains("kind = \"github_release\"")
    );
    assert!(
        toml::to_string(&curseforge)
            .expect("curseforge source toml")
            .contains("kind = \"curseforge_mod\"")
    );
}

#[test]
fn addon_source_ref_accepts_legacy_provider_kind_names() {
    let github: AddonSourceFixture = toml::from_str(
        r#"
source = { kind = "git_hub_release", owner = "owner", repo = "repo" }
"#,
    )
    .expect("legacy github source");
    let curseforge: AddonSourceFixture = toml::from_str(
        r#"
source = { kind = "curse_forge_mod", mod_id = 12345 }
"#,
    )
    .expect("legacy curseforge source");

    assert_eq!(
        github.source,
        AddonSourceRef::GitHubRelease {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            tag: None,
            asset_name: None,
        }
    );
    assert_eq!(
        curseforge.source,
        AddonSourceRef::CurseForgeMod {
            mod_id: 12345,
            file_id: None,
        }
    );
}

#[test]
fn parse_curseforge_source_with_explicit_file() {
    let source = parse_curseforge_source("curseforge:12345@67890")
        .expect("parse")
        .expect("source ref");

    assert_eq!(
        source,
        AddonSourceRef::CurseForgeMod {
            mod_id: 12345,
            file_id: Some(67890),
        }
    );
}

#[test]
fn parse_curseforge_source_without_file() {
    let source = parse_curseforge_source("curseforge:12345")
        .expect("parse")
        .expect("source ref");

    assert_eq!(
        source,
        AddonSourceRef::CurseForgeMod {
            mod_id: 12345,
            file_id: None,
        }
    );
}

#[test]
fn parse_github_source_with_tag_and_asset() {
    let source = parse_github_source("github:owner/repo@v1.2.3#addon.zip")
        .expect("parse")
        .expect("source ref");

    assert_eq!(
        source,
        AddonSourceRef::GitHubRelease {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            tag: Some("v1.2.3".to_string()),
            asset_name: Some("addon.zip".to_string()),
        }
    );
}

#[test]
fn parse_github_source_without_tag() {
    let source = parse_github_source("github:owner/repo")
        .expect("parse")
        .expect("source ref");

    assert_eq!(
        source,
        AddonSourceRef::GitHubRelease {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            tag: None,
            asset_name: None,
        }
    );
}

#[test]
fn select_github_release_asset_requires_disambiguation() {
    let release = GitHubRelease {
        assets: vec![
            GitHubReleaseAsset {
                name: "a.zip".to_string(),
                browser_download_url: "https://example.com/a.zip".to_string(),
            },
            GitHubReleaseAsset {
                name: "b.zip".to_string(),
                browser_download_url: "https://example.com/b.zip".to_string(),
            },
        ],
    };

    let error = select_github_release_asset(&release, None).expect_err("ambiguous");
    assert!(error.to_string().contains("multiple `.zip` assets"));
}

#[test]
fn select_github_release_asset_matches_explicit_asset() {
    let release = GitHubRelease {
        assets: vec![
            GitHubReleaseAsset {
                name: "addon.zip".to_string(),
                browser_download_url: "https://example.com/addon.zip".to_string(),
            },
            GitHubReleaseAsset {
                name: "addon.txt".to_string(),
                browser_download_url: "https://example.com/addon.txt".to_string(),
            },
        ],
    };

    let asset = select_github_release_asset(&release, Some("addon.zip")).expect("asset");
    assert_eq!(asset.name, "addon.zip");
}

#[test]
fn select_latest_curseforge_file_prefers_newest_available_zip() {
    let file = select_latest_curseforge_file(
        vec![
            CurseForgeFile {
                id: 1,
                file_name: "addon-old.zip".to_string(),
                file_date: "2026-04-01T12:00:00Z".to_string(),
                download_url: Some("https://example.com/old.zip".to_string()),
                is_available: true,
                sortable_game_versions: vec![CurseForgeSortableGameVersion {
                    game_version_type_id: 517,
                }],
            },
            CurseForgeFile {
                id: 2,
                file_name: "addon-new.zip".to_string(),
                file_date: "2026-04-02T12:00:00Z".to_string(),
                download_url: Some("https://example.com/new.zip".to_string()),
                is_available: true,
                sortable_game_versions: vec![CurseForgeSortableGameVersion {
                    game_version_type_id: 517,
                }],
            },
            CurseForgeFile {
                id: 3,
                file_name: "addon.txt".to_string(),
                file_date: "2026-04-03T12:00:00Z".to_string(),
                download_url: Some("https://example.com/skip.txt".to_string()),
                is_available: true,
                sortable_game_versions: vec![CurseForgeSortableGameVersion {
                    game_version_type_id: 517,
                }],
            },
        ],
        None,
    )
    .expect("latest file");

    assert_eq!(file.id, 2);
    assert_eq!(file.file_name, "addon-new.zip");
}

#[test]
fn select_latest_curseforge_file_filters_by_version_type() {
    let file = select_latest_curseforge_file(
        vec![
            CurseForgeFile {
                id: 1,
                file_name: "addon-retail.zip".to_string(),
                file_date: "2026-04-01T12:00:00Z".to_string(),
                download_url: Some("https://example.com/retail.zip".to_string()),
                is_available: true,
                sortable_game_versions: vec![CurseForgeSortableGameVersion {
                    game_version_type_id: 517,
                }],
            },
            CurseForgeFile {
                id: 2,
                file_name: "addon-classic.zip".to_string(),
                file_date: "2026-04-02T12:00:00Z".to_string(),
                download_url: Some("https://example.com/classic.zip".to_string()),
                is_available: true,
                sortable_game_versions: vec![CurseForgeSortableGameVersion {
                    game_version_type_id: 775,
                }],
            },
        ],
        Some(517),
    )
    .expect("latest file");

    assert_eq!(file.id, 1);
}

#[test]
fn select_curseforge_version_type_matches_retail_slug() {
    let version_type = select_curseforge_version_type(
        &[
            CurseForgeGameVersionType {
                id: 775,
                name: "WoW Classic".to_string(),
                slug: "wow_classic".to_string(),
            },
            CurseForgeGameVersionType {
                id: 517,
                name: "WoW Retail".to_string(),
                slug: "wow_retail".to_string(),
            },
        ],
        WowFlavor::Retail,
    )
    .expect("version type");

    assert_eq!(version_type.id, 517);
}

#[test]
fn validate_curseforge_file_rejects_missing_download_url() {
    let error = validate_curseforge_file(CurseForgeFile {
        id: 1,
        file_name: "addon.zip".to_string(),
        file_date: "2026-04-02T12:00:00Z".to_string(),
        download_url: None,
        is_available: true,
        sortable_game_versions: Vec::new(),
    })
    .expect_err("missing download url");

    assert!(error.to_string().contains("download URL"));
}
