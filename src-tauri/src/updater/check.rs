use super::cleanup;
use super::types::LocalVersionInfo;
use crate::http::HTTP_CLIENT;
use crate::platform::{get_hytale_arch, get_hytale_os};
use futures_util::{stream, StreamExt};

pub async fn find_latest_version() -> anyhow::Result<u32> {
    let os = get_hytale_os();
    let arch = get_hytale_arch();

    let tasks = stream::iter(1..=20).map(|v| {
        let url = format!(
            "https://game-patches.hytale.com/patches/{}/{}/release/0/{}.pwr",
            os, arch, v
        );
        async move {
            let resp = HTTP_CLIENT.head(&url).send().await;
            (v, resp.map(|r| r.status().is_success()).unwrap_or(false))
        }
    });

    let _ = cleanup::cleanup_incomplete_downloads();

    let mut results = tasks.buffer_unordered(10);
    let mut latest = 0;

    while let Some((version, exists)) = results.next().await {
        if exists && version > latest {
            latest = version;
        }
    }

    if latest == 0 {
        return Err(anyhow::anyhow!(
            "Could not find any game version on patch server"
        ));
    }

    log::info!("Latest available game version: {}", latest);
    Ok(latest)
}

pub async fn get_remote_metadata(version: u32) -> anyhow::Result<LocalVersionInfo> {
    let client = &HTTP_CLIENT;

    let os = get_hytale_os();
    let arch = get_hytale_arch();

    let url = format!(
        "https://game-patches.hytale.com/patches/{}/{}/release/0/{}.pwr",
        os, arch, version
    );

    let metadata_response = client
        .head(&url)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch metadata (HEAD): {}", e))?;

    if !metadata_response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Server returned status {}",
            metadata_response.status()
        ));
    }

    let headers = metadata_response.headers();

    Ok(LocalVersionInfo {
        version,
        size: headers
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok()),
        last_modified: headers
            .get(reqwest::header::LAST_MODIFIED)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
        etag: headers
            .get(reqwest::header::ETAG)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
    })
}
