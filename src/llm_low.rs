use reqwest::{ header::{ HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE }, Client };
use serde::{ Deserialize, Serialize };
use std::env;

pub async fn completion_inner_async(user_input: &str) -> anyhow::Result<String> {
    let llm_endpoint = "https://api-inference.huggingface.co/models/jaykchen/tiny".to_string();
    let llm_api_key = env::var("LLM_API_KEY").expect("LLM_API_KEY-must-be-set");

    let client = Client::new();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", llm_api_key)).unwrap()
    );

    let body = serde_json::json!({
        "inputs": user_input,
    });

    use anyhow::Context;

    let response = client
        .post(llm_endpoint)
        .headers(headers)
        .json(&body)
        .send().await
        .context("Failed to send request to API")?; // Adds context to the error

    let status_code = response.status();

    if status_code.is_success() {
        let response_body = response.text().await.context("Failed to read response body")?;

        let completion_response: Vec<Choice> = serde_json
            ::from_str(&response_body)
            .context("Failed to parse response from API")?;

        if let Some(choice) = completion_response.get(0) {

            log::info!("choice: {:?}", choice);
            Ok(choice.generated_text.clone())
        } else {
            Err(anyhow::anyhow!("No completion choices found in the response"))
        }
    } else {
        Err(anyhow::anyhow!("Failed to get a successful response from the API"))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Choice {
    pub generated_text: String,
}

use serde_json::Value; // Make sure serde_json is in your Cargo.toml

