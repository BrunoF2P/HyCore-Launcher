use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tauri::{Emitter, Window};

use super::env::get_hycore_data_dir;
use super::types::UpdateStatus;

use crate::error::AppError;

pub async fn download_with_resume(url: &str, dest: &Path, window: &Window) -> anyhow::Result<()> {
    let client = &crate::http::HTTP_CLIENT;

    let mut downloaded = if dest.exists() {
        let size = fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
        if size > 0 {
            log::info!("Found partial file: {} bytes", size);
        }
        size
    } else {
        0
    };

    let mut request = client.get(url).timeout(Duration::from_secs(300));

    if downloaded > 0 {
        let range_header = format!("bytes={}-", downloaded);
        log::info!("Sending Range request: {}", range_header);
        request = request.header("Range", range_header);
    } else {
        log::info!("Starting download from {}", url);
    }

    let response = request
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("GET request failed: {}", e))?;

    let status = response.status();
    log::info!("GET response status: {}", status);

    if !status.is_success() {
        return Err(anyhow::anyhow!("Server returned error: {}", status));
    }

    let total_size = if status == reqwest::StatusCode::PARTIAL_CONTENT {
        let content_range = response
            .headers()
            .get("Content-Range")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                anyhow::anyhow!("Server used 206 but did not provide Content-Range header")
            })?;

        log::info!("Content-Range: {}", content_range);

        let size_str = content_range
            .split('/')
            .last()
            .ok_or_else(|| anyhow::anyhow!("Invalid Content-Range header"))?;

        let total = size_str
            .parse::<u64>()
            .map_err(|_| anyhow::anyhow!("Could not parse total size from Content-Range"))?;

        log::info!(
            "Total file size from Content-Range: {} bytes ({:.2} MB)",
            total,
            total as f64 / 1_048_576.0
        );
        total
    } else {
        if downloaded > 0 {
            log::warn!("Server doesn't support resume (returned 200 instead of 206), restarting");
            downloaded = 0;
        }

        let len = response
            .content_length()
            .ok_or_else(|| anyhow::anyhow!("Server did not provide Content-Length"))?;

        if len == 0 {
            return Err(anyhow::anyhow!("Server returned Content-Length: 0"));
        }

        log::info!(
            "Total file size from Content-Length: {} bytes ({:.2} MB)",
            len,
            len as f64 / 1_048_576.0
        );
        len
    };

    if downloaded == total_size {
        log::info!("File already complete");
        return Ok(());
    }

    if downloaded > total_size {
        log::warn!(
            "Local file ({}) larger than remote ({}), restarting",
            downloaded,
            total_size
        );
        downloaded = 0;
        let _ = fs::remove_file(dest);
    }

    if downloaded > 0 {
        let _ = window.emit(
            "update-status",
            UpdateStatus {
                stage: "download".to_string(),
                progress: (downloaded as f64 / total_size as f64) * 100.0,
                message: format!(
                    "Resuming from {:.1} MB / {:.1} MB...",
                    downloaded as f64 / 1_048_576.0,
                    total_size as f64 / 1_048_576.0
                ),
            },
        );
    }

    let mut file = if downloaded > 0 {
        log::info!("Opening file in append mode");
        fs::OpenOptions::new().append(true).open(dest)?
    } else {
        log::info!("Creating new file at {:?}", dest);
        fs::File::create(dest)?
    };

    let mut stream = response.bytes_stream();
    let mut last_emit = Instant::now();
    let start_time = Instant::now();
    let mut bytes_in_second = 0u64;

    log::info!("Starting stream download...");

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| {
            let err = format!("Stream error at {} bytes: {}", downloaded, e);
            log::error!("{}", err);
            anyhow::anyhow!(err)
        })?;

        file.write_all(&chunk).map_err(|e| {
            let err = format!("Write error at {} bytes: {}", downloaded, e);
            log::error!("{}", err);
            anyhow::anyhow!(err)
        })?;

        let chunk_size = chunk.len() as u64;
        downloaded += chunk_size;
        bytes_in_second += chunk_size;

        if last_emit.elapsed() > Duration::from_millis(500) {
            let progress = (downloaded as f64 / total_size as f64) * 100.0;
            let elapsed_ms = last_emit.elapsed().as_millis() as f64;
            let speed_mbps = if elapsed_ms > 0.0 {
                (bytes_in_second as f64 / 1_048_576.0) / (elapsed_ms / 1000.0)
            } else {
                0.0
            };
            bytes_in_second = 0;

            let _ = window.emit(
                "update-status",
                UpdateStatus {
                    stage: "download".to_string(),
                    progress: progress.min(100.0),
                    message: format!(
                        "{:.1} MB / {:.1} MB ({:.0}%) - {:.1} MB/s",
                        downloaded as f64 / 1_048_576.0,
                        total_size as f64 / 1_048_576.0,
                        progress,
                        speed_mbps
                    ),
                },
            );
            last_emit = Instant::now();
        }
    }

    log::info!("Stream finished");

    if downloaded != total_size {
        let err = format!(
            "Download incomplete: {} / {} bytes ({:.1}%)",
            downloaded,
            total_size,
            (downloaded as f64 / total_size as f64) * 100.0
        );
        log::error!("{}", err);
        return Err(anyhow::anyhow!("{}. Run update again to resume.", err));
    }

    file.sync_all()?;

    let elapsed = start_time.elapsed().as_secs();
    let avg_speed = if elapsed > 0 {
        total_size as f64 / elapsed as f64 / 1_048_576.0
    } else {
        total_size as f64 / 1_048_576.0
    };
    log::info!(
        "Download complete: {} bytes in {}s (avg {:.2} MB/s)",
        total_size,
        elapsed,
        avg_speed
    );

    Ok(())
}

pub async fn download_with_retry(
    url: &str,
    dest: &Path,
    window: &Window,
    retries: u32,
) -> anyhow::Result<()> {
    for attempt in 0..retries {
        match download_with_resume(url, dest, window).await {
            Ok(_) => {
                let file_size = fs::metadata(dest).map(|m| m.len()).unwrap_or(0);

                if file_size == 0 {
                    return Err(anyhow::anyhow!("Downloaded file is empty"));
                }

                return Ok(());
            }
            Err(e) if attempt < retries - 1 => {
                let _current_size = fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
                let delay = Duration::from_secs(3u64.pow(attempt));

                let _ = window.emit(
                    "update-status",
                    UpdateStatus {
                        stage: "download".to_string(),
                        progress: 0.0,
                        message: format!("Error: {} - Retrying in {}s...", e, delay.as_secs()),
                    },
                );

                log::warn!("Retry {}/{} failed: {}", attempt + 1, retries, e);
                tokio::time::sleep(delay).await;
                continue;
            }
            Err(e) => {
                log::error!("Download failed after {} attempts: {}", retries, e);
                return Err(anyhow::anyhow!("Download failed: {}", e));
            }
        }
    }
    Err(anyhow::anyhow!("Max retries reached"))
}

#[allow(dead_code)]
pub fn verify_pwr_file(path: &Path, expected_hash: &str) -> anyhow::Result<()> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;

    let hash = format!("{:x}", hasher.finalize());
    if hash != expected_hash {
        return Err(anyhow::anyhow!("File corrupted: hash mismatch"));
    }
    Ok(())
}

#[tauri::command]
pub async fn validate_pwr_file(version: u32) -> Result<bool, AppError> {
    let pwr_path = get_hycore_data_dir().join(format!("{}.pwr", version));

    if !pwr_path.exists() {
        return Ok(false);
    }

    let file_size = fs::metadata(&pwr_path).map(|m| m.len()).unwrap_or(0);

    Ok(file_size > 1_000_000)
}
