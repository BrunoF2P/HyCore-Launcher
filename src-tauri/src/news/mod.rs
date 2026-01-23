use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::AppError;
use crate::updater::env::get_hycore_data_dir;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[tauri::command]
pub async fn get_news() -> Result<Vec<NewsItem>, AppError> {
    log::info!("Fetching news...");
    match fetch_news().await {
        Ok(items) => {
            log::info!("News fetched successfully ({} items)", items.len());
            Ok(items)
        }
        Err(e) => {
            log::warn!("Failed to fetch news from server, loading cache: {}", e);
            load_cache().map_err(AppError::from)
        }
    }
}

const CACHE_TTL_SECS: u64 = 3600;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NewsItem {
    pub title: String,
    pub date: String,
    pub author: String,
    pub image_url: String,
    pub summary: String,
    pub link: String,
}

#[derive(Serialize, Deserialize)]
struct NewsCache {
    items: Vec<NewsItem>,
    timestamp: u64,
}

#[derive(Deserialize)]
struct ApiCoverImage {
    #[serde(rename = "s3Key")]
    s3_key: String,
}

#[derive(Deserialize)]
struct ApiPost {
    title: String,
    author: String,
    #[serde(rename = "publishedAt")]
    published_at: String,
    slug: String,
    #[serde(rename = "bodyExcerpt")]
    body_excerpt: String,
    #[serde(rename = "coverImage")]
    cover_image: ApiCoverImage,
}

pub async fn fetch_news() -> anyhow::Result<Vec<NewsItem>> {
    log::info!("Fetching news from Hytale API...");
    let client = &crate::http::HTTP_CLIENT;

    let response = client
        .get("https://hytale.com/api/blog/post/published")
        .send()
        .await?;

    let api_posts: Vec<ApiPost> = response.json().await?;

    log::info!("Successfully fetched {} posts from API", api_posts.len());

    let mut news_items = Vec::new();

    for post in api_posts.into_iter().take(3) {
        let date_parsed = OffsetDateTime::parse(&post.published_at, &Rfc3339).ok();

        let date_str = date_parsed
            .map(|dt| format!("{}/{}/{}", dt.day(), dt.month() as u8, dt.year()))
            .unwrap_or_else(|| {
                post.published_at
                    .split('T')
                    .next()
                    .unwrap_or("")
                    .to_string()
            });

        let link = if let Some(dt) = date_parsed {
            format!(
                "https://hytale.com/news/{}/{}/{}",
                dt.year(),
                dt.month() as u8,
                post.slug
            )
        } else {
            format!("https://hytale.com/news/{}", post.slug)
        };

        let image_url = format!(
            "https://cdn.hytale.com/variants/blog_cover_{}",
            post.cover_image.s3_key
        );

        if !post.title.is_empty() && !post.slug.is_empty() {
            news_items.push(NewsItem {
                title: post.title,
                date: date_str,
                author: post.author,
                image_url,
                summary: post.body_excerpt,
                link,
            });
        }
    }

    if !news_items.is_empty() {
        log::info!("Saving news to cache...");
        let _ = save_cache(&news_items);
    }

    Ok(news_items)
}

fn get_cache_path() -> PathBuf {
    get_hycore_data_dir().join("news_cache.json")
}

fn save_cache(items: &[NewsItem]) -> anyhow::Result<()> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let cache = NewsCache {
        items: items.to_vec(),
        timestamp,
    };

    let json = serde_json::to_string(&cache)?;
    fs::write(get_cache_path(), json)?;
    Ok(())
}

pub fn load_cache() -> anyhow::Result<Vec<NewsItem>> {
    let path = get_cache_path();
    if path.exists() {
        log::info!("Loading news from cache...");
        let json = fs::read_to_string(path)?;
        let cache: NewsCache = serde_json::from_str(&json)?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Cache valid for 1 hour
        if now - cache.timestamp < CACHE_TTL_SECS {
            log::info!("News cache is valid ({} items)", cache.items.len());
            Ok(cache.items)
        } else {
            log::info!("News cache expired");
            Err(anyhow::anyhow!("Cache expired"))
        }
    } else {
        log::info!("No news cache found");
        Err(anyhow::anyhow!("No cache found"))
    }
}
