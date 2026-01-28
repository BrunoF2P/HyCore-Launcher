use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct TokenRequest {
    pub uuid: String,
    pub name: String,
    pub scopes: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TokenResponse {
    #[serde(rename = "identityToken", alias = "IdentityToken")]
    pub identity_token: Option<String>,
    #[serde(rename = "sessionToken", alias = "SessionToken")]
    pub session_token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuthTokens {
    pub identity_token: String,
    pub session_token: String,
    /// Effective UUID used for this player (always populated)
    pub user_id: String,
    pub name: String,
}

pub async fn fetch_auth_tokens(
    uuid: &str,
    name: &str,
    auth_domain: &str,
) -> anyhow::Result<AuthTokens> {
    let client = Client::builder().timeout(Duration::from_secs(10)).build()?;

    let req_body = TokenRequest {
        uuid: uuid.to_string(),
        name: name.to_string(),
        scopes: vec!["hytale:server".to_string(), "hytale:client".to_string()],
    };

    let endpoint = format!("https://sessions.{}/game-session/child", auth_domain);
    log::info!("Attempting to fetch auth tokens from {}", endpoint);

    let mut resp = client
        .post(&endpoint)
        .json(&req_body)
        .header("User-Agent", "HyPrism-Launcher")
        .send()
        .await;

    if resp.is_err() || !resp.as_ref().unwrap().status().is_success() {
        let fallback_endpoint = format!("https://{}/game-session/child", auth_domain);
        log::info!(
            "Primary auth endpoint failed, retrying with fallback: {}",
            fallback_endpoint
        );

        resp = Ok(client
            .post(&fallback_endpoint)
            .json(&req_body)
            .header("User-Agent", "HyPrism-Launcher")
            .send()
            .await?);
    }

    let final_resp = resp?;
    if !final_resp.status().is_success() {
        anyhow::bail!(
            "Auth server (including fallback) returned status {}",
            final_resp.status()
        );
    }

    let token_resp: TokenResponse = final_resp.json().await?;

    let identity = token_resp
        .identity_token
        .ok_or_else(|| anyhow::anyhow!("No identity token received"))?;

    let session = token_resp
        .session_token
        .ok_or_else(|| anyhow::anyhow!("No session token received"))?;

    log::info!("Auth tokens received successfully");

    Ok(AuthTokens {
        identity_token: identity,
        session_token: session,
        // Legacy flow does not return a UUID from the server,
        // so we reuse the UUID that was sent in the request.
        user_id: uuid.to_string(),
        name: name.to_string(),
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CustomAuthResponse {
    pub success: bool,
    pub authenticated: bool,
    pub identity_token: String,
    pub session_token: String,
    /// Some servers may omit the `user` block
    #[serde(default)]
    pub user: Option<UserData>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserData {
    pub uuid: String,
    pub name: String,
    #[serde(default)]
    pub premium: bool,
}

/// Custom auth flow that uses a persistent UUID.
///
/// Sends `name` and `uuid` to the Sanasol auth server and:
/// - uses the UUID returned in `user.uuid` if present;
/// - otherwise, falls back to the local UUID provided.
pub async fn fetch_custom_auth_with_uuid(
    username: &str,
    uuid: &str,
    auth_domain: &str,
) -> anyhow::Result<AuthTokens> {
    let client = Client::builder().timeout(Duration::from_secs(10)).build()?;
    let url = format!("https://{}/auth/authenticateByAccessToken", auth_domain);

    log::info!(
        "Fetching custom auth tokens from {} as '{}' with UUID {}",
        url,
        username,
        uuid
    );

    let resp = client
        .post(&url)
        .json(&serde_json::json!({
            "name": username,
            "uuid": uuid,
        }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let _ = resp.text().await; // consume body to avoid leaking into logs
        log::error!(
            "Custom auth failed: status {} (body omitted for security)",
            status
        );
        anyhow::bail!("Custom auth server returned status {}", status);
    }

    let body = resp.text().await?;
    // Do NOT log `body` — it contains plain-text tokens.

    let auth_resp: CustomAuthResponse = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            log::error!(
                "Failed to parse custom auth response: {} (body omitted for security)",
                e
            );
            anyhow::bail!("Failed to parse custom auth response: {}", e);
        }
    };

    let final_uuid = auth_resp
        .user
        .as_ref()
        .map(|u| u.uuid.clone())
        .unwrap_or_else(|| uuid.to_string());

    let final_name = auth_resp
        .user
        .as_ref()
        .map(|u| u.name.clone())
        .unwrap_or_else(|| username.to_string());

    log::info!(
        "Custom auth success for {} (UUID: {})",
        final_name,
        final_uuid
    );

    Ok(AuthTokens {
        identity_token: auth_resp.identity_token,
        session_token: auth_resp.session_token,
        user_id: final_uuid,
        name: final_name,
    })
}

/// Kept for backwards compatibility with older call sites.
/// Generates a temporary UUID and delegates to `fetch_custom_auth_with_uuid`.
#[deprecated(note = "Use fetch_custom_auth_with_uuid instead")]
pub async fn fetch_custom_auth_tokens(
    username: &str,
    auth_domain: &str,
) -> anyhow::Result<AuthTokens> {
    let temp_uuid = Uuid::new_v4().to_string();
    fetch_custom_auth_with_uuid(username, &temp_uuid, auth_domain).await
}
