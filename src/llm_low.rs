use reqwest::{ header::{ HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE }, Client };
use serde::{ Deserialize, Serialize };
use std::env;
use http_req::{ request::{ Request, Method }, response::Response, uri::Uri };

pub async fn completion_inner_async(user_input: &str) -> anyhow::Result<String> {
    let llm_endpoint = "https://api-inference.huggingface.co/models/jaykchen/tiny";
    let llm_api_key = env::var("LLM_API_KEY").expect("LLM_API_KEY-must-be-set");
    let base_url = Uri::try_from(llm_endpoint).expect("Failed to parse URL");

    let mut writer = Vec::new(); // This will store the response body

    let query = serde_json::json!({
        "inputs": user_input,
        "wait_for_model": true,
        "max_length": 500,
    });
    let query_bytes = serde_json::to_vec(&query).expect("Failed to serialize query to bytes");

    // let query_str = query.to_string();
    let query_len = query_bytes.len().to_string();
    // Prepare and send the HTTP request
    match
        Request::new(&base_url)
            .method(Method::POST)
            .header("Content-Type", "application/json")
            .header("Authorization", &format!("Bearer {}", llm_api_key))
            .header("Content-Length", &query_len)
            .body(&query_bytes)
            .send(&mut writer)
    {
        Ok(res) => {
            if !res.status_code().is_success() {
                log::error!("HTTP error with status {:?}", res.status_code());
                return Err(anyhow::anyhow!("HTTP error with status {:?}", res.status_code()));
            }

            // Attempt to parse the response body into the expected structure
            let completion_response: Vec<Choice> = serde_json
                ::from_slice(&writer)
                .expect("Failed to parse response from API");

            if let Some(choice) = completion_response.get(0) {
                log::info!("Choice: {:?}", choice);
                Ok(choice.generated_text.clone())
            } else {
                Err(anyhow::anyhow!("No completion choices found in the response"))
            }
        }
        Err(e) => {
            log::error!("Error getting response from API: {:?}", e);

            Err(anyhow::anyhow!("Error getting response from API: {:?}", e))
        }
    }
}


#[derive(Serialize, Deserialize, Debug)]
pub struct Choice {
    pub generated_text: String,
}

use serde_json::Value; // Make sure serde_json is in your Cargo.toml

