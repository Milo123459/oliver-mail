use reqwest::Client;
use rand::{distr::Alphanumeric, Rng};
use serde_json::json;

#[derive(Clone, Debug)]
pub struct TempEmail {
    pub address: String,
    pub password: String,
}

fn random_string(length: usize) -> String {
    rand::rng().sample_iter(&Alphanumeric).take(length).map(char::from).collect()
}

pub async fn create_account() -> Result<TempEmail, Box<dyn std::error::Error>> {
    let client = Client::new();

    let username = random_string(12).to_lowercase();
    let password = random_string(20);

    let domains_response = client.get("https://api.mail.tm/domains").send().await?;
    let domains: serde_json::Value = domains_response.json().await?;
    let domain = domains["hydra:member"][0]["domain"].as_str().ok_or("No domain available")?;

    let address = format!("{}@{}", username, domain);

    let response = client.post("https://api.mail.tm/accounts").json(&json!({"address": address,"password": password})).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await?;

        return Err(format!(
            "Failed to create account: {} - {}",
            status, body
        )
        .into());
    }

    println!("Temporary email created: {}", address);

    Ok(TempEmail {
        address,
        password,
    })
}