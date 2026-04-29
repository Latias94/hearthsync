use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct CurseForgeApiResponse<T> {
    pub(super) data: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CurseForgePaginatedResponse<T> {
    pub(super) data: T,
}

#[derive(Debug, Clone)]
pub(super) struct CurseForgeWowContext {
    pub(super) game_id: u32,
    pub(super) version_type_id: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CurseForgeGame {
    pub(super) id: u32,
    pub(super) name: String,
    pub(super) slug: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CurseForgeGameVersionType {
    pub(crate) id: u32,
    pub(crate) name: String,
    pub(crate) slug: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CurseForgeFile {
    pub(crate) id: u32,
    pub(crate) file_name: String,
    pub(crate) file_date: String,
    pub(crate) download_url: Option<String>,
    pub(crate) is_available: bool,
    #[serde(default = "default_curseforge_release_type")]
    pub(crate) release_type: u8,
    #[serde(default)]
    pub(crate) dependencies: Vec<CurseForgeFileDependency>,
    #[serde(default)]
    pub(crate) hashes: Vec<CurseForgeFileHash>,
    #[serde(default)]
    pub(crate) file_length: Option<u64>,
    #[serde(default)]
    pub(crate) sortable_game_versions: Vec<CurseForgeSortableGameVersion>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CurseForgeFileHash {
    pub(crate) value: String,
    pub(crate) algo: u8,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CurseForgeFileDependency {
    pub(crate) mod_id: u32,
    pub(crate) relation_type: u8,
}

const fn default_curseforge_release_type() -> u8 {
    1
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CurseForgeSortableGameVersion {
    pub(crate) game_version_type_id: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CurseForgeSearchMod {
    pub(super) id: u32,
    pub(super) name: String,
    pub(super) summary: Option<String>,
    pub(super) download_count: u64,
    #[serde(default)]
    pub(super) latest_files_indexes: Vec<CurseForgeFileIndex>,
    pub(super) links: CurseForgeSearchModLinks,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CurseForgeSearchModLinks {
    pub(super) website_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CurseForgeFileIndex {
    pub(super) file_id: u32,
    pub(super) game_version_type_id: u32,
}
