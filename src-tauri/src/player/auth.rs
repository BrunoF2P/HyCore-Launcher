use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

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
}

pub async fn fetch_auth_tokens(
    uuid: &str,
    name: &str,
    auth_domain: &str,
) -> anyhow::Result<AuthTokens> {
    let client = Client::builder().timeout(Duration::from_secs(10)).build()?;

    let endpoint = format!("https://sessions.{}/game-session/child", auth_domain);
    log::info!("Fetching auth tokens from {}", endpoint);

    let req_body = TokenRequest {
        uuid: uuid.to_string(),
        name: name.to_string(),
        scopes: vec!["hytale:server".to_string(), "hytale:client".to_string()],
    };

    let resp = client
        .post(&endpoint)
        .json(&req_body)
        .header("User-Agent", "HyPrism-Launcher")
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Auth server returned status {}", resp.status());
    }

    let token_resp: TokenResponse = resp.json().await?;

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
    })
}
