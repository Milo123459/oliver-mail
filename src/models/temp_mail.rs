use anyhow::{Context, Result, bail};
use rand::{Rng, distr::Alphanumeric, prelude::IndexedRandom};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Email {
    pub id: String,
    pub from: String,
    pub subject: String,
    pub intro: String,
    pub body: String,
    pub seen: bool,
    #[serde(default)]
    pub starred: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
struct MessagesResponse {
    #[serde(rename = "hydra:member")]
    messages: Vec<Message>,
}

#[derive(Debug, Deserialize)]
struct Message {
    id: String,
    from: MessageFrom,
    subject: String,
    intro: Option<String>,
    seen: bool,
    #[serde(rename = "createdAt")]
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct MessageFrom {
    address: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    token: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TempEmail {
    pub address: String,
    pub password: String,
    pub id: String,
    pub token: String,
}

#[derive(Debug, Deserialize)]
struct AccountResponse {
    id: String,
    address: String,
}

fn random_string(length: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

/// Creates a Mail.tm account using explicitly supplied credentials.
///
/// Unlike `create_account()`, this does not generate an address or password.
pub async fn create_account_with_credentials(address: String,password: String,) -> Result<TempEmail> {
    let client = crate::runtime::http();

    let address = address.trim().to_lowercase();
    let password = password.to_string();

    if address.is_empty() {
        bail!("Email address is required");
    }

    if !address.contains('@') {
        bail!("Email address must contain '@'");
    }

    if password.is_empty() {
        bail!("Password is required");
    }

    if password.len() < 8 {
        bail!("Password must be at least 8 characters");
    }

    let response = client
        .post("https://api.mail.tm/accounts")
        .json(&json!({
            "address": address,
            "password": password
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await?;

        bail!("Failed to create account: {} - {}", status, body);
    }

    let account: AccountResponse = response.json().await?;

    let token_response = client
        .post("https://api.mail.tm/token")
        .json(&json!({
            "address": address,
            "password": password
        }))
        .send()
        .await?;

    if !token_response.status().is_success() {
        let status = token_response.status();
        let body = token_response.text().await?;

        bail!("Failed to login to Mail.tm: {} - {}", status, body);
    }

    let token: TokenResponse = token_response.json().await?;

    println!("Temporary email created: {}", account.address);
    println!("Account ID: {}", account.id);
    println!("Mail.tm token acquired");

    Ok(TempEmail {
        address: account.address,
        password,
        id: account.id,
        token: token.token,
    })
}

/// Quick Generate.
///
/// This keeps the old behaviour used by the sidebar's
/// "Quick Generate" option.
pub async fn create_account() -> Result<TempEmail> {
    let username = random_string(12).to_lowercase();
    let password = random_string(20);

    let domains = get_domains().await?;

    let domain = domains
        .choose(&mut rand::rng())
        .context("No mail.tm domains available")?;

    let address = format!("{}@{}", username, domain);

    create_account_with_credentials(address, password).await
}

pub async fn get_domains() -> Result<Vec<String>> {
    let client = crate::runtime::http();

    let response = client
        .get("https://api.mail.tm/domains")
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        bail!("Failed to retrieve domains: {} - {}", status, body);
    }

    let domains: serde_json::Value = response.json().await?;

    let domain_list = domains["hydra:member"]
        .as_array()
        .context("No mail.tm domains available")?
        .iter()
        .filter_map(|domain| domain["domain"].as_str().map(|s| s.to_string()))
        .collect();

    Ok(domain_list)
}

async fn get_token(client: &Client, email: &TempEmail) -> Result<String> {
    let response = client
        .post("https://api.mail.tm/token")
        .json(&json!({
            "address": email.address,
            "password": email.password
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        bail!("Failed to login to Mail.tm: {} - {}", status, body);
    }

    let token: TokenResponse = response.json().await?;

    Ok(token.token)
}

pub async fn refresh_token(email: &TempEmail) -> Result<String> {
    get_token(crate::runtime::http(), email).await
}

pub async fn get_mail(email: &TempEmail) -> Result<Vec<Email>> {
    let client = crate::runtime::http();
    let token = get_token(client, email).await?;

    let response = client
        .get("https://api.mail.tm/messages")
        .bearer_auth(&token)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        bail!("Failed to retrieve mail: {} - {}", status, body);
    }

    let messages: MessagesResponse = response.json().await?;

    let emails = messages
        .messages
        .into_iter()
        .map(|message| Email {
            id: message.id,
            from: message.from.address,
            subject: message.subject,
            intro: message.intro.unwrap_or_default(),
            body: String::new(),
            seen: message.seen,
            starred: false,
            created_at: message.created_at,
        })
        .collect();

    Ok(emails)
}
