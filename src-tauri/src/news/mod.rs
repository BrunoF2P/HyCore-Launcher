use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

pub async fn fetch_news() -> Result<Vec<NewsItem>, String> {
    log::info!("Fetching news from Hytale API...");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| {
            let err_msg = e.to_string();
            log::error!("Failed to build HTTP client: {}", err_msg);
            err_msg
        })?;

    let response = client
        .get("https://hytale.com/api/blog/post/published")
        .send()
        .await
        .map_err(|e| {
            let err_msg = e.to_string();
            log::error!("API request failed: {}", err_msg);
            err_msg
        })?;

    let api_posts: Vec<ApiPost> = response.json().await.map_err(|e| {
        let err_msg = e.to_string();
        log::error!("Failed to parse API response: {}", err_msg);
        err_msg
    })?;

    log::info!("Successfully fetched {} posts from API", api_posts.len());

    let mut news_items = Vec::new();

    for post in api_posts.into_iter().take(3) {
        let date_parsed = DateTime::parse_from_rfc3339(&post.published_at)
            .map(|dt| dt.with_timezone(&Utc))
            .ok();

        let date_str = date_parsed
            .map(|dt| format!("{}/{}/{}", dt.day(), dt.month(), dt.year()))
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
                dt.month(),
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
    // Attempt to use a more stable path in home directory for Linux
    let mut path = dirs::data_dir().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    path.push("hycore");
    let _ = fs::create_dir_all(&path);
    path.push("news_cache.json");
    path
}

fn save_cache(items: &[NewsItem]) -> Result<(), String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let cache = NewsCache {
        items: items.to_vec(),
        timestamp,
    };

    let json = serde_json::to_string(&cache).map_err(|e| e.to_string())?;
    fs::write(get_cache_path(), json).map_err(|e| {
        let err_msg = e.to_string();
        log::error!("Failed to write news cache: {}", err_msg);
        err_msg
    })?;
    Ok(())
}

pub fn load_cache() -> Result<Vec<NewsItem>, String> {
    let path = get_cache_path();
    if path.exists() {
        log::info!("Loading news from cache...");
        let json = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let cache: NewsCache = serde_json::from_str(&json).map_err(|e| {
            log::error!("Failed to parse news cache");
            e.to_string()
        })?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Cache valid for 1 hour (3600 seconds)
        if now - cache.timestamp < 3600 {
            log::info!("News cache is valid ({} items)", cache.items.len());
            Ok(cache.items)
        } else {
            log::info!("News cache expired");
            Err("Cache expired".to_string())
        }
    } else {
        log::info!("No news cache found");
        Err("No cache found".to_string())
    }
}
