use anyhow::{Context, Result};
use axum::{
    extract::Query,
    response::Html,
    routing::get,
    Router,
};
use base64::{
    engine::general_purpose::URL_SAFE_NO_PAD,
    Engine,
};
use rand::{
    distr::Alphanumeric,
    Rng,
};
use serde::Deserialize;
use sha2::{
    Digest,
    Sha256,
};
use std::{env, sync::Arc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::oneshot;

const CLIENT_ID: &str =
    "830227318434-7mgfk7bucm5mt9sl8271oevg9bjj6vlu.apps.googleusercontent.com";

const REDIRECT_URI: &str =
    "http://127.0.0.1:49152/callback";

const SCOPE: &str =
    "https://www.googleapis.com/auth/gmail.readonly";

#[derive(Debug, Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GoogleTokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    pub scope: String,
    pub token_type: String,
}

#[derive(Clone, Debug)]
pub struct GoogleAccount {
    pub email: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
}

impl GoogleAccount {
    fn from_tokens(tokens: GoogleTokenResponse) -> Result<Self> {
        let refresh_token = tokens
            .refresh_token
            .context("Google did not return a refresh token")?;

        Ok(Self {
            email: String::new(),
            access_token: tokens.access_token,
            refresh_token,
            expires_at: now_unix_seconds() + tokens.expires_in,
        })
    }

    async fn load_email(&mut self) -> Result<()> {
        let access_token = self.ensure_access_token().await?.to_owned();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .context("Failed to create Google profile client")?;
        let response = client
            .get("https://gmail.googleapis.com/gmail/v1/users/me/profile")
            .bearer_auth(access_token)
            .send()
            .await
            .context("Failed to load Google account profile")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to load Google account profile: {} {}", status, body);
        }

        #[derive(Deserialize)]
        struct Profile {
            #[serde(rename = "emailAddress")]
            email_address: String,
        }

        self.email = response
            .json::<Profile>()
            .await
            .context("Failed to parse Google account profile")?
            .email_address;
        Ok(())
    }

    pub async fn ensure_access_token(&mut self) -> Result<&str> {
        if now_unix_seconds() + 60 < self.expires_at {
            return Ok(&self.access_token);
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .context("Failed to create Google token refresh client")?;
        let response = client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", CLIENT_ID),
                ("refresh_token", self.refresh_token.as_str()),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await
            .context("Failed to refresh Google access token")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Google token refresh failed: {} {}", status, body);
        }

        let tokens = response
            .json::<GoogleTokenResponse>()
            .await
            .context("Failed to parse Google token refresh response")?;

        self.access_token = tokens.access_token;
        self.expires_at = now_unix_seconds() + tokens.expires_in;
        Ok(&self.access_token)
    }
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

pub async fn login() -> Result<GoogleAccount> {
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    let (sender, receiver) = oneshot::channel::<Result<String>>();

    let sender = Arc::new(tokio::sync::Mutex::new(Some(sender)));
    let (shutdown_sender, shutdown_receiver) = oneshot::channel::<()>();

    let app = Router::new().route(
        "/callback",
        get({
            let sender = sender.clone();

            move |query: Query<CallbackQuery>| {
                let sender = sender.clone();

                async move {
                    if let Some(error) = query.error.as_deref() {
                        if let Some(sender) = sender.lock().await.take() {
                            let _ = sender.send(Err(anyhow::anyhow!(
                                "Google OAuth error: {}",
                                error
                            )));
                        }

                        return Html(
                            "<h2>Google login failed.</h2>\
                             <p>You can close this window.</p>"
                                .to_string(),
                        );
                    }

                    let Some(code) = query.code.as_deref() else {
                        if let Some(sender) = sender.lock().await.take() {
                            let _ = sender.send(Err(anyhow::anyhow!(
                                "Google did not return an authorization code."
                            )));
                        }

                        return Html(
                            "<h2>Google login failed.</h2>\
                             <p>No authorization code was returned.</p>"
                                .to_string(),
                        );
                    };

                    if let Some(sender) = sender.lock().await.take() {
                        let _ = sender.send(Ok(code.to_string()));
                    }

                    Html(
                        "<h2>Gmail connected!</h2>\
                         <p>You can close this window and return to Mail Box.</p>"
                            .to_string(),
                    )
                }
            }
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:49152")
        .await
        .context("Failed to start OAuth callback server")?;

    let oauth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth\
        ?client_id={}\
        &redirect_uri={}\
        &response_type=code\
        &scope={}\
        &access_type=offline\
        &prompt=consent\
        &code_challenge={}\
        &code_challenge_method=S256",
        urlencoding::encode(CLIENT_ID),
        urlencoding::encode(REDIRECT_URI),
        urlencoding::encode(SCOPE),
        urlencoding::encode(&code_challenge),
    );

    let callback_server = tokio::spawn(async move {
        let server = axum::serve(listener, app).with_graceful_shutdown(async move {
            let _ = shutdown_receiver.await;
        });

        if let Err(error) = server.await {
            if let Some(sender) = sender.lock().await.take() {
                let _ = sender.send(Err(anyhow::anyhow!(
                    "OAuth callback server failed: {}",
                    error
                )));
            }
        }
    });

    if let Err(error) = webbrowser::open(&oauth_url) {
        callback_server.abort();
        return Err(error).context("Failed to open Google OAuth page");
    }

    let code = match tokio::time::timeout(Duration::from_secs(120), receiver).await {
        Err(_) => {
            let _ = shutdown_sender.send(());
            callback_server.abort();
            return Err(anyhow::anyhow!(
                "Timed out waiting for the Google OAuth callback"
            ));
        }
        Ok(Ok(Ok(code))) => code,
        Ok(Ok(Err(error))) => {
            let _ = shutdown_sender.send(());
            callback_server.abort();
            return Err(error).context("OAuth callback channel closed");
        }
        Ok(Err(error)) => {
            let _ = shutdown_sender.send(());
            callback_server.abort();
            return Err(error).context("OAuth callback task failed");
        }
    };

    let _ = shutdown_sender.send(());
    callback_server.abort();

    let mut account = GoogleAccount::from_tokens(exchange_code(&code, &code_verifier).await?)?;
    account.load_email().await?;
    Ok(account)
}

fn generate_code_verifier() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}

fn generate_code_challenge(code_verifier: &str) -> String {
    let mut hasher = Sha256::new();

    hasher.update(code_verifier.as_bytes());

    let hash = hasher.finalize();

    URL_SAFE_NO_PAD.encode(hash)
}

async fn exchange_code(
    code: &str,
    code_verifier: &str,
) -> Result<GoogleTokenResponse> {
    dotenvy::dotenv().ok();
    let client_secret = env::var("GOOGLE_CLIENT_SECRET")
        .context("GOOGLE_CLIENT_SECRET is missing from the .env file")?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .context("Failed to create Google token exchange client")?;

    let response = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", code),
            ("client_id", CLIENT_ID),
            ("client_secret", client_secret.as_str()),
            ("redirect_uri", REDIRECT_URI),
            ("grant_type", "authorization_code"),
            ("code_verifier", code_verifier),
        ])
        .send()
        .await
        .context("Failed to contact Google token endpoint")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        anyhow::bail!(
            "Google token exchange failed: {} {}",
            status,
            body
        );
    }

    response
        .json::<GoogleTokenResponse>()
        .await
        .context("Failed to parse Google token response")
}

#[derive(Debug, Deserialize)]
struct GmailMessageList {
    messages: Option<Vec<GmailMessageRef>>,
}

#[derive(Debug, Deserialize)]
struct GmailMessageRef {
    id: String,
}

#[derive(Debug, Deserialize)]
struct GmailMessage {
    id: String,
    snippet: Option<String>,
    payload: Option<GmailPayload>,
    #[serde(rename = "internalDate")]
    internal_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GmailPayload {
    headers: Option<Vec<GmailHeader>>,
    body: Option<GmailBody>,
    parts: Option<Vec<GmailPayload>>,
}

#[derive(Clone, Debug, Deserialize)]
struct GmailHeader {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct GmailBody {
    data: Option<String>,
}

pub async fn get_gmail_mail(
    account: &mut GoogleAccount,
    limit: usize,
) -> Result<Vec<super::temp_mail::Email>> {
    let access_token = account.ensure_access_token().await?.to_owned();
    let limit = limit.clamp(1, 25);
    let response = reqwest::Client::new()
        .get("https://gmail.googleapis.com/gmail/v1/users/me/messages")
        .bearer_auth(&access_token)
        .query(&[("labelIds", "INBOX"), ("maxResults", &limit.to_string())])
        .send()
        .await
        .context("Failed to list Gmail messages")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Failed to list Gmail messages: {} {}", status, body);
    }

    let list = response
        .json::<GmailMessageList>()
        .await
        .context("Failed to parse Gmail message list")?;
    let client = reqwest::Client::new();
    let mut emails = Vec::new();

    for message_ref in list.messages.unwrap_or_default().into_iter().take(limit) {
        let response = client
            .get(format!(
                "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}",
                message_ref.id
            ))
            .bearer_auth(&access_token)
            .query(&[("format", "metadata"), ("metadataHeaders", "From"),
                ("metadataHeaders", "Subject")])
            .send()
            .await?;

        if !response.status().is_success() {
            continue;
        }

        if let Ok(message) = response.json::<GmailMessage>().await {
            emails.push(to_email(message, false));
        }
    }

    Ok(emails)
}

pub async fn get_gmail_message(
    account: &mut GoogleAccount,
    message_id: &str,
) -> Result<super::temp_mail::Email> {
    let access_token = account.ensure_access_token().await?.to_owned();
    let response = reqwest::Client::new()
        .get(format!(
            "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}",
            message_id
        ))
        .bearer_auth(access_token)
        .query(&[("format", "full")])
        .send()
        .await
        .context("Failed to load Gmail message")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Failed to load Gmail message: {} {}", status, body);
    }

    Ok(to_email(
        response
            .json::<GmailMessage>()
            .await
            .context("Failed to parse Gmail message")?,
        true,
    ))
}

fn to_email(message: GmailMessage, include_body: bool) -> super::temp_mail::Email {
    let headers = message
        .payload
        .as_ref()
        .and_then(|payload| payload.headers.as_ref())
        .cloned()
        .unwrap_or_default();
    let header = |name: &str| {
        headers
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case(name))
            .map(|header| header.value.clone())
            .unwrap_or_default()
    };

    let body = if include_body {
        message
            .payload
            .as_ref()
            .map(extract_body)
            .unwrap_or_default()
    } else {
        String::new()
    };

    super::temp_mail::Email {
        id: message.id,
        from: header("From"),
        subject: header("Subject"),
        intro: message.snippet.unwrap_or_default(),
        body,
        seen: true,
        created_at: message.internal_date.unwrap_or_default(),
    }
}

fn extract_body(payload: &GmailPayload) -> String {
    if let Some(data) = payload.body.as_ref().and_then(|body| body.data.as_ref()) {
        if let Ok(bytes) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(data) {
            return String::from_utf8_lossy(&bytes).into_owned();
        }
    }

    payload
        .parts
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(extract_body)
        .find(|body| !body.is_empty())
        .unwrap_or_default()
}