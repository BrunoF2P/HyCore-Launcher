use super::types::{CurseForgeMod, CurseForgeResponse, ModCategory, ModFile, SearchResult};
use reqwest::Url;

use crate::error::AppError;

use once_cell::sync::Lazy;
use std::sync::RwLock;
use std::time::{Duration, Instant};

const CURSE_FORGE_BASE_URL: &str = "https://api.curseforge.com/v1";
const HYTALE_GAME_ID: i32 = 70216;
const CF_API_KEY: &str = "$2a$10$bL4bIL5pUWqfcO7KQtnMReakwtfHbNKh6v1uTpKlzhwoueEJQnPnm";

struct Cache<T> {
    data: T,
    expires_at: Instant,
}

static CATEGORIES_CACHE: Lazy<RwLock<Option<Cache<Vec<ModCategory>>>>> =
    Lazy::new(|| RwLock::new(None));

fn builder(method: reqwest::Method, url: &str) -> reqwest::RequestBuilder {
    crate::http::HTTP_CLIENT
        .request(method, url)
        .header("x-api-key", CF_API_KEY)
        .header("Accept", "application/json")
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SearchModsParams {
    pub query: Option<String>,
    pub category_id: Option<i32>,
    pub class_id: Option<i32>,
    pub sort_field: Option<i32>, // 1=Featured, 2=Popularity, 3=LastUpdated, 4=Name, 5=Author, 6=TotalDownloads
    pub sort_order: Option<String>, // asc, desc
    pub page_size: Option<i32>,
    pub index: Option<i32>,
}

pub async fn search_mods(params: SearchModsParams) -> Result<SearchResult, AppError> {
    let url = format!("{}/mods/search", CURSE_FORGE_BASE_URL);

    let mut query_params = vec![("gameId", HYTALE_GAME_ID.to_string())];

    if let Some(q) = params.query {
        query_params.push(("searchFilter", q));
    }
    if let Some(cat) = params.category_id {
        query_params.push(("categoryId", cat.to_string()));
    }
    if let Some(class) = params.class_id {
        query_params.push(("classId", class.to_string()));
    }
    if let Some(sort) = params.sort_field {
        query_params.push(("sortField", sort.to_string()));
    }
    if let Some(order) = params.sort_order {
        query_params.push(("sortOrder", order));
    }
    let page_size = params.page_size.unwrap_or(20);
    query_params.push(("pageSize", page_size.to_string()));

    if let Some(idx) = params.index {
        query_params.push(("index", idx.to_string()));
    }

    let final_url = Url::parse_with_params(&url, &query_params).map_err(|e| e.to_string())?;

    let resp = builder(reqwest::Method::GET, final_url.as_str())
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(AppError::from(format!(
            "CurseForge API error: {}",
            resp.status()
        )));
    }

    let cf_resp: CurseForgeResponse<Vec<CurseForgeMod>> = resp.json().await?;

    let total_count = cf_resp
        .pagination
        .as_ref()
        .map(|p| p.total_count)
        .unwrap_or(0);

    Ok(SearchResult {
        mods: cf_resp.data,
        total_count,
        page_index: params.index.unwrap_or(0),
        page_size,
    })
}

pub async fn get_mods(mod_ids: Vec<i32>) -> Result<Vec<CurseForgeMod>, AppError> {
    let url = format!("{}/mods", CURSE_FORGE_BASE_URL);

    let body = serde_json::json!({
        "modIds": mod_ids
    });

    let resp = builder(reqwest::Method::POST, &url)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(AppError::from(format!(
            "CurseForge API error: {}",
            resp.status()
        )));
    }

    let cf_resp: CurseForgeResponse<Vec<CurseForgeMod>> = resp.json().await?;
    Ok(cf_resp.data)
}

pub async fn get_mod_details(mod_id: i32) -> Result<CurseForgeMod, AppError> {
    let url = format!("{}/mods/{}", CURSE_FORGE_BASE_URL, mod_id);

    let resp = builder(reqwest::Method::GET, &url).send().await?;

    if !resp.status().is_success() {
        return Err(AppError::from(format!(
            "CurseForge API error: {}",
            resp.status()
        )));
    }

    let cf_resp: CurseForgeResponse<CurseForgeMod> = resp.json().await?;
    Ok(cf_resp.data)
}

pub async fn get_mod_files(mod_id: i32) -> Result<Vec<ModFile>, AppError> {
    let url = format!("{}/mods/{}/files", CURSE_FORGE_BASE_URL, mod_id);

    let resp = builder(reqwest::Method::GET, &url).send().await?;

    if !resp.status().is_success() {
        return Err(AppError::from(format!(
            "CurseForge API error: {}",
            resp.status()
        )));
    }

    let cf_resp: CurseForgeResponse<Vec<ModFile>> = resp.json().await?;
    Ok(cf_resp.data)
}

pub async fn get_mod_file_details(mod_id: i32, file_id: i32) -> Result<ModFile, AppError> {
    let url = format!("{}/mods/{}/files/{}", CURSE_FORGE_BASE_URL, mod_id, file_id);

    let resp = builder(reqwest::Method::GET, &url).send().await?;

    if !resp.status().is_success() {
        return Err(AppError::from(format!(
            "CurseForge API error: {}",
            resp.status()
        )));
    }

    let cf_resp: CurseForgeResponse<ModFile> = resp.json().await?;
    Ok(cf_resp.data)
}

#[tauri::command]
pub async fn get_categories() -> Result<Vec<ModCategory>, AppError> {
    // Check cache first
    if let Ok(guard) = CATEGORIES_CACHE.read() {
        if let Some(cache) = guard.as_ref() {
            if cache.expires_at > Instant::now() {
                log::info!("Serving mod categories from cache");
                return Ok(cache.data.clone());
            }
        }
    }

    let url = format!("{}/categories", CURSE_FORGE_BASE_URL);
    let query_params = vec![("gameId", HYTALE_GAME_ID.to_string())];
    let final_url = Url::parse_with_params(&url, &query_params).map_err(|e| e.to_string())?;

    let resp = builder(reqwest::Method::GET, final_url.as_str())
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(AppError::from(format!(
            "CurseForge API error: {}",
            resp.status()
        )));
    }

    let cf_resp: CurseForgeResponse<Vec<ModCategory>> = resp.json().await?;

    // Update cache
    if let Ok(mut guard) = CATEGORIES_CACHE.write() {
        *guard = Some(Cache {
            data: cf_resp.data.clone(),
            expires_at: Instant::now() + Duration::from_secs(3600), // 1 hour TTL
        });
    }

    Ok(cf_resp.data)
}
