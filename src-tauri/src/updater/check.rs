use super::types::LocalVersionInfo;
use crate::http::HTTP_CLIENT;
use crate::platform::{get_hytale_arch, get_hytale_os};
use futures_util::StreamExt;

pub async fn find_latest_version(channel: &str) -> anyhow::Result<u32> {
    let os = get_hytale_os();
    let arch = get_hytale_arch();
    let mut max_found = 0;
    let mut consecutive_failures = 0;
    const MAX_CONSECUTIVE_FAILURES: u32 = 10;
    const MAX_PROBE_VERSION: u32 = 10000;

    log::info!(
        "Searching for latest version on channel {} via dynamic probing...",
        channel
    );

    // Initial search range 1 to MAX_PROBE_VERSION, probing 20 at a time
    for chunk_start in (1..MAX_PROBE_VERSION).step_by(20) {
        let chunk_end = (chunk_start + 20).min(MAX_PROBE_VERSION);

        let results = futures_util::stream::iter(chunk_start..chunk_end)
            .map(|v| {
                let os = os.clone();
                let arch = arch.clone();
                let channel = channel.to_string();
                let url = format!(
                    "https://game-patches.hytale.com/patches/{}/{}/{}/0/{}.pwr",
                    os, arch, channel, v
                );
                async move {
                    let resp = HTTP_CLIENT
                        .head(&url)
                        .timeout(std::time::Duration::from_secs(3))
                        .send()
                        .await;

                    match resp {
                        Ok(r) if r.status().is_success() => Some(v),
                        _ => None,
                    }
                }
            })
            .buffer_unordered(10)
            .collect::<Vec<Option<u32>>>()
            .await;

        for v_opt in results {
            if let Some(v) = v_opt {
                if v > max_found {
                    max_found = v;
                }
                consecutive_failures = 0;
            } else {
                if max_found > 0 {
                    consecutive_failures += 1;
                }
            }
        }

        if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
            log::debug!(
                "Stopping probe: reached {} consecutive failures after version {}",
                MAX_CONSECUTIVE_FAILURES,
                max_found
            );
            break;
        }

        // Safety: if we probed 100 versions and found nothing, stop.
        if chunk_start > 100 && max_found == 0 {
            log::warn!(
                "No versions found after probing {} potentially valid slots",
                chunk_start
            );
            break;
        }
    }

    if max_found == 0 {
        return Err(anyhow::anyhow!(
            "Could not find any game version on patch server. Connection issue or incorrect channel?"
        ));
    }

    log::info!("Latest discovered version: {}", max_found);
    Ok(max_found)
}

pub async fn find_all_versions(channel: &str) -> anyhow::Result<Vec<u32>> {
    let os = get_hytale_os();
    let arch = get_hytale_arch();
    let mut discovered = Vec::new();
    const MAX_PROBE_VERSION: u32 = 100; // Realistic limit for Hytale versions for now

    log::info!("Searching for all versions on channel {}...", channel);

    let chunks = (1..MAX_PROBE_VERSION).collect::<Vec<u32>>();
    let results = futures_util::stream::iter(chunks)
        .map(|v| {
            let os = os.clone();
            let arch = arch.clone();
            let channel = channel.to_string();
            let url = format!(
                "https://game-patches.hytale.com/patches/{}/{}/{}/0/{}.pwr",
                os, arch, channel, v
            );
            async move {
                let resp = HTTP_CLIENT
                    .head(&url)
                    .timeout(std::time::Duration::from_secs(3))
                    .send()
                    .await;

                match resp {
                    Ok(r) if r.status().is_success() => Some(v),
                    _ => None,
                }
            }
        })
        .buffer_unordered(15) // Probe 15 versions in parallel
        .collect::<Vec<Option<u32>>>()
        .await;

    for v_opt in results {
        if let Some(v) = v_opt {
            discovered.push(v);
        }
    }

    discovered.sort_by(|a, b| b.cmp(a)); // Newest first

    if discovered.is_empty() {
        return Err(anyhow::anyhow!("No versions found on channel {}", channel));
    }

    log::info!("Discovered {} versions: {:?}", discovered.len(), discovered);
    Ok(discovered)
}

pub async fn get_remote_metadata(version: u32, channel: &str) -> anyhow::Result<LocalVersionInfo> {
    let client = &HTTP_CLIENT;

    let os = get_hytale_os();
    let arch = get_hytale_arch();

    let url = format!(
        "https://game-patches.hytale.com/patches/{}/{}/{}/0/{}.pwr",
        os, arch, channel, version
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
        channel: channel.to_string(),
        size: headers
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok()),
        installed_at: None,
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
