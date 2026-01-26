use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeResponse<T> {
    pub data: T,
    pub pagination: Option<Pagination>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub index: i32,
    pub page_size: i32,
    pub result_count: i32,
    pub total_count: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeMod {
    pub id: i32,
    pub game_id: i32,
    pub name: String,
    pub slug: String,
    pub summary: String,
    pub download_count: i32,
    pub date_created: String,
    pub date_modified: String,
    pub date_released: String,
    pub logo: Option<ModLogo>,
    pub screenshots: Vec<ModScreenshot>,
    pub categories: Vec<ModCategory>,
    pub authors: Vec<ModAuthor>,
    pub latest_files: Vec<ModFile>,
    pub main_file_id: i32,
    pub allow_mod_distribution: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModLogo {
    pub id: i32,
    pub mod_id: i32,
    pub title: String,
    pub description: String,
    pub thumbnail_url: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModCategory {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub url: String,
    pub icon_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModAuthor {
    pub id: i32,
    pub name: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModScreenshot {
    pub id: i32,
    pub mod_id: i32,
    pub title: String,
    pub description: String,
    pub thumbnail_url: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModFile {
    pub id: i32,
    pub mod_id: i32,
    pub display_name: String,
    pub file_name: String,
    // Using u64 for file length as it's non-negative
    pub file_length: u64,
    pub download_url: Option<String>,
    pub file_date: String,
    pub release_type: i32, // 1=Release, 2=Beta, 3=Alpha
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub mods: Vec<CurseForgeMod>,
    pub total_count: i32,
    pub page_index: i32,
    pub page_size: i32,
}

// Installed Mod Structures

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Mod {
    pub id: String, // "cf-12345"
    pub name: String,
    pub slug: Option<String>,
    pub version: String,
    pub author: String,
    pub description: String,
    pub download_url: Option<String>,
    pub curse_forge_id: Option<i32>,
    pub file_id: Option<i32>,
    pub enabled: bool,
    pub installed_at: String,
    pub updated_at: String,
    pub file_path: String,
    pub icon_url: Option<String>,
    pub downloads: Option<i32>,
    pub category: Option<String>,
    pub latest_version: Option<String>,
    pub latest_file_id: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModManifest {
    pub mods: Vec<Mod>,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Modpack {
    pub name: String,
    pub mod_count: usize,
    pub created_at: String,
}
