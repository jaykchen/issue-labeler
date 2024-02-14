use reqwest::{
    header::{ HeaderMap, HeaderValue, USER_AGENT, AUTHORIZATION, CONTENT_TYPE },
    Client,
};
use serde::{ Deserialize, Serialize };
use std::env;
use http_req::{ request::{ Request, Method }, response::Response, uri::Uri };

pub async fn completion_inner_async(user_input: &str) -> anyhow::Result<String> {
    let llm_endpoint = env
        ::var("llm_endpoint")
        .unwrap_or("http://43.129.206.18:3000/generate".to_string());
    // let llm_endpoint = "https://api-inference.huggingface.co/models/jaykchen/tiny";
    let llm_api_key = env::var("LLM_API_KEY").expect("LLM_API_KEY-must-be-set");
    let base_url = Uri::try_from(llm_endpoint.as_str()).expect("Failed to parse URL");

    let mut writer = Vec::new(); // This will store the response body

    let query =
        serde_json::json!({
        "inputs": user_input,
        // "wait_for_model": true,
        // "max_length": 500,
    });
    let query_bytes = serde_json::to_vec(&query).expect("Failed to serialize query to bytes");

    // let query_str = query.to_string();
    let query_len = query_bytes.len().to_string();
    // Prepare and send the HTTP request

    for n in 0..2 {
        match
            Request::new(&base_url)
                .method(Method::POST)
                .header("User-Agent", "curl/8.4.0")
                .header("Accept", "*/*")
                .header("Content-Type", "application/json")
                .header("Authorization", &format!("Bearer {}", llm_api_key))
                .header("Content-Length", &query_len)
                .body(&query_bytes)
                .send(&mut writer)
        {
            Ok(res) => {
                if !res.status_code().is_success() {
                    log::error!("HTTP error with status {:?}", res.status_code());
                    continue;
                }

                // Attempt to parse the response body into the expected structure
                let completion_response: GeneratedResponse = serde_json::from_slice(&writer)?;

                log::info!("GeneratedResponse: {:?}", completion_response);
                return Ok(completion_response.generated_text.clone());
            }
            Err(e) => {
                log::error!("Error getting response from API: {:?}", e);

                return Err(anyhow::anyhow!("Error getting response from API: {:?}", e));
            }
        }
        use std::thread::sleep;
        use std::time::Duration;

        sleep(Duration::from_secs(40));
    }
    Ok(String::new())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GeneratedResponse {
    pub generated_text: String,
}

use serde_json::Value; // Make sure serde_json is in your Cargo.toml
