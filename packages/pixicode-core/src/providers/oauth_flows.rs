//! OAuth2 flows — Google and Azure auth URL + code exchange.
//!
//! Credentials from env: GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET; AZURE_TENANT_ID, AZURE_CLIENT_ID, AZURE_CLIENT_SECRET.

use crate::providers::auth::OAuthToken;
use std::time::Duration;

/// Build Google OAuth2 authorization URL (user opens in browser, then redirects back with ?code=...).
///
/// Env: GOOGLE_CLIENT_ID. Optional: redirect_uri (defaults to http://localhost:4096/auth/callback if not set).
pub fn google_auth_url(redirect_uri: Option<&str>, scope: Option<&str>, state: Option<&str>) -> Option<String> {
    let client_id = std::env::var("GOOGLE_CLIENT_ID").ok()?;
    let redirect = redirect_uri.unwrap_or("http://localhost:4096/auth/callback");
    let scope = scope.unwrap_or("https://www.googleapis.com/auth/cloud-platform openid email profile");
    let state = state.unwrap_or("pixicode_google");
    let url = url::Url::parse_with_params(
        "https://accounts.google.com/o/oauth2/v2/auth",
        &[
            ("response_type", "code"),
            ("client_id", &client_id),
            ("redirect_uri", redirect),
            ("scope", scope),
            ("access_type", "offline"),
            ("prompt", "consent"),
            ("state", state),
        ],
    )
    .ok()?;
    Some(url.to_string())
}

/// Exchange Google authorization code for tokens.
///
/// Env: GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET.
pub async fn google_exchange_code(
    code: &str,
    redirect_uri: Option<&str>,
) -> Result<OAuthToken, String> {
    let client_id = std::env::var("GOOGLE_CLIENT_ID").map_err(|_| "GOOGLE_CLIENT_ID not set")?;
    let client_secret = std::env::var("GOOGLE_CLIENT_SECRET").map_err(|_| "GOOGLE_CLIENT_SECRET not set")?;
    let redirect = redirect_uri.unwrap_or("http://localhost:4096/auth/callback");

    let params = [
        ("code", code),
        ("client_id", &client_id),
        ("client_secret", &client_secret),
        ("redirect_uri", redirect),
        ("grant_type", "authorization_code"),
    ];

    let res = reqwest::Client::new()
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(format!("Google token error: {}", body));
    }

    let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let access_token = json
        .get("access_token")
        .and_then(|t| t.as_str())
        .ok_or("no access_token in response")?
        .to_string();
    let expires_in = json.get("expires_in").and_then(|e| e.as_u64());
    let expires_at = expires_in.map(|secs| std::time::SystemTime::now() + Duration::from_secs(secs));
    let refresh_token = json.get("refresh_token").and_then(|t| t.as_str()).map(String::from);
    let token_type = json.get("token_type").and_then(|t| t.as_str()).unwrap_or("Bearer").to_string();
    let scope = json.get("scope").and_then(|s| s.as_str()).map(String::from);

    Ok(OAuthToken {
        access_token,
        token_type,
        expires_in,
        refresh_token,
        scope,
        expires_at,
    })
}

/// Build Azure AD OAuth2 authorization URL.
///
/// Env: AZURE_TENANT_ID, AZURE_CLIENT_ID.
pub fn azure_auth_url(redirect_uri: Option<&str>, scope: Option<&str>, state: Option<&str>) -> Option<String> {
    let tenant_id = std::env::var("AZURE_TENANT_ID").ok()?;
    let client_id = std::env::var("AZURE_CLIENT_ID").ok()?;
    let redirect = redirect_uri.unwrap_or("http://localhost:4096/auth/callback");
    let scope = scope.unwrap_or("https://cognitiveservices.azure.com/.default openid");
    let state = state.unwrap_or("pixicode_azure");

    let url = url::Url::parse_with_params(
        &format!("https://login.microsoftonline.com/{}/oauth2/v2.0/authorize", tenant_id),
        &[
            ("response_type", "code"),
            ("client_id", &client_id),
            ("redirect_uri", redirect),
            ("scope", scope),
            ("state", state),
        ],
    )
    .ok()?;
    Some(url.to_string())
}

/// Exchange Azure AD authorization code for tokens.
///
/// Env: AZURE_TENANT_ID, AZURE_CLIENT_ID, AZURE_CLIENT_SECRET.
pub async fn azure_exchange_code(
    code: &str,
    redirect_uri: Option<&str>,
) -> Result<OAuthToken, String> {
    let tenant_id = std::env::var("AZURE_TENANT_ID").map_err(|_| "AZURE_TENANT_ID not set")?;
    let client_id = std::env::var("AZURE_CLIENT_ID").map_err(|_| "AZURE_CLIENT_ID not set")?;
    let client_secret = std::env::var("AZURE_CLIENT_SECRET").map_err(|_| "AZURE_CLIENT_SECRET not set")?;
    let redirect = redirect_uri.unwrap_or("http://localhost:4096/auth/callback");

    let token_url = format!("https://login.microsoftonline.com/{}/oauth2/v2.0/token", tenant_id);
    let params = [
        ("code", code),
        ("client_id", &client_id),
        ("client_secret", &client_secret),
        ("redirect_uri", redirect),
        ("grant_type", "authorization_code"),
    ];

    let res = reqwest::Client::new()
        .post(&token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(format!("Azure token error: {}", body));
    }

    let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let access_token = json
        .get("access_token")
        .and_then(|t| t.as_str())
        .ok_or("no access_token in response")?
        .to_string();
    let expires_in = json.get("expires_in").and_then(|e| e.as_u64());
    let expires_at = expires_in.map(|secs| std::time::SystemTime::now() + Duration::from_secs(secs));
    let refresh_token = json.get("refresh_token").and_then(|t| t.as_str()).map(String::from);
    let token_type = json.get("token_type").and_then(|t| t.as_str()).unwrap_or("Bearer").to_string();
    let scope = json.get("scope").and_then(|s| s.as_str()).map(String::from);

    Ok(OAuthToken {
        access_token,
        token_type,
        expires_in,
        refresh_token,
        scope,
        expires_at,
    })
}
