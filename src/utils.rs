use anyhow;
use chrono::Duration;
use octocrab_wasi::{ models::issues::Issue, params::{ issues::Sort, Direction, State } };
use regex::Regex;
use serde::{ Deserialize, Serialize };
use serde_json::{ json, Map, Value };
use openai_flows::{ chat::{ ChatModel, ChatOptions }, OpenAIFlows };
use std::{ collections::{ HashMap, HashSet }, env };
use http_req::{ request::{ Request, Method }, response::Response, uri::Uri };

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub struct Payload {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub labels: Option<Vec<String>>,
    pub creator: String,
    pub essence: Option<String>,
}

pub async fn why_labels(
    issue: &Issue,
    _contributors_set: HashSet<String>
) -> anyhow::Result<Payload> {
    let issue_creator_name = &issue.user.login;
    let issue_title = issue.title.to_string();
    let issue_number = issue.number;
    let issue_body = match &issue.body {
        Some(body) => body.to_string(),
        None => "".to_string(),
    };
    let issue_url = issue.url.to_string();
    let _source_url = issue.html_url.to_string();

    let labels = issue.labels
        .iter()
        .map(|lab| lab.name.clone())
        .collect::<Vec<String>>();

    let issue_body = issue_body.chars().take(32_000).collect::<String>();

    let system_prompt = String::from(
        "You're a programming bot tasked to analyze GitHub issues data."
    );

    let user_prompt = format!(
        r#"You are tasked with refining and simplifying the information presented in a GitHub issue while keeping the original author's perspective. Think of it as if the original author decided to restate their issue in a more concise manner, focusing on clarity and brevity without losing the essence or the technical specifics of their original message.
        Issue text: {issue_body}
        Instructions:
        - Condense the issue's content by focusing on the primary technical details, proposals, and challenges mentioned, as if restating them directly in the author's voice.
        - Maintain the original tone and perspective of the author. Your summary should read as though the author themselves is offering a clearer, more straightforward version of their original text.
        - Include key actionable items, technical specifics, and any proposed solutions or requests made by the author, ensuring these elements are presented succinctly.
        - Avoid shifting to a third-person narrative. The goal is to simplify the author's message without altering the viewpoint from which it is delivered.
        - Preserve any direct quotes, technical terms, or specific examples the author used to illustrate their points, but ensure they are integrated into the summary seamlessly and without unnecessary elaboration.
        - Aim for a summary that allows quick grasping of core points and intentions, aiding efficient understanding and response. 
        - Explicitly remove unnecessary new lines, spaces, and combine multiple new lines into one. Pay special attention to avoid consecutive new lines (i.e., '\n\n') in your summary. Escape special characters as needed for command line compatibility.
        - Do not add extraneous wordings or notations like 'summary', '###', etc., from the original text.
        Your summary's effectiveness in capturing the essence while staying true to the author's intent is crucial for accurate content analysis and label assignment training."#
    );

    let essence = chat_inner(&system_prompt, &user_prompt).await?;

    Ok(Payload {
        number: issue_number,
        title: issue_title,
        url: issue_url,
        labels: Some(labels),
        creator: issue_creator_name.to_string(),
        essence: Some(essence),
    })
}

pub fn parse_labels_from_response(input: &str) -> anyhow::Result<Vec<String>> {
    let known_labels = vec![
        "LFX Mentorship",
        "c-Test",
        "question",
        "c-WASI-NN",
        "breaking changes",
        "c-Plugin",
        "priority:medium",
        "platform-android",
        "c-Example",
        "bug",
        "arch-arm32",
        "platform-windows",
        "c-Internal",
        "platform-iOS",
        "c-WASI-Threads",
        "compiler-MSVC",
        "compiler-gcc",
        "OSPP",
        "c-WASI",
        "c-Interpreter",
        "Improvement",
        "help wanted",
        "Hacktoberfest",
        "c-function-references",
        "documentation",
        "binding-rust",
        "hacktoberfest-spam",
        "platform-macos",
        "priority:high",
        "integration",
        "c-CAPI",
        "github_actions",
        "Cannot-Reproduce",
        "compiler-llvm",
        "binding-python",
        "platform-OHOS",
        "platform-linux",
        "c-AOT",
        "c-CLI",
        "duplicate",
        "arch-x86_64",
        "good first issue",
        "c-Container",
        "invalid",
        "arch-arm64",
        "wontfix",
        "c-CMake",
        "c-Installer",
        "fuzz-different-behavior",
        "enhancement",
        "c-ExceptionHandling",
        "dependencies",
        "feature",
        "binding-java",
        "binding-go",
        "priority:low",
        "c-WASI-Crypto",
        "c-CI"
    ];

    let pattern = regex::Regex::new(r"`([^`]+)`").unwrap();
    let mut known_extracted = Vec::new();

    if let Some(captures) = pattern.captures(input) {
        if let Some(matched) = captures.get(1) {
            let mut modified_input = matched
                .as_str()
                .split(",")
                .map(|x| x.trim())
                .collect::<Vec<_>>()
                .join("§");
            for &label in known_labels.iter() {
                if modified_input.contains(label) {
                    known_extracted.push(label.to_string());
                    let _ = modified_input.replace(label, "§"); // Mark positions
                }
            }

            known_extracted.extend(
                modified_input
                    .split("§")
                    .filter_map(|x| (
                        if x.trim().is_empty() {
                            None
                        } else {
                            Some(x.trim().to_string())
                        }
                    ))
            );
        } else {
            log::info!("No match found for the capture group.");
        }
    } else {
        log::info!("No match found in the input string.");
    }

    Ok(known_extracted)
}

pub async fn chat_inner(system_prompt: &str, user_prompt: &str) -> anyhow::Result<String> {
    let openai = OpenAIFlows::new();

    let co = ChatOptions {
        model: ChatModel::GPT35Turbo16K,
        restart: false,
        system_prompt: Some(&system_prompt),
        max_tokens: Some(512),
        ..Default::default()
    };

    match openai.chat_completion("chat_id", &user_prompt, &co).await {
        Ok(r) => Ok(r.choice),
        Err(_e) => Err(anyhow::Error::msg(_e.to_string())),
    }
}

pub async fn add_labels_to_github_issue(
    owner: &str,
    repo: &str,
    issue_number: u64,
    labels: Vec<String>
) -> anyhow::Result<Vec<u8>> {
    let token = env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN is required");
    let base_url = format!(
        "https://api.github.com/repos/{}/{}/issues/{}/labels",
        owner,
        repo,
        issue_number
    );
    let base_url = Uri::try_from(base_url.as_str()).unwrap();
    let mut writer = Vec::new();

    let body = json!({ "labels": labels });
    let body_bytes = serde_json::to_vec(&body).expect("Failed to serialize body to bytes");

    match
        Request::new(&base_url)
            .method(Method::POST)
            .header("Accept", "application/vnd.github+json")
            .header("Authorization", &format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "your-app-name")
            .header("Content-Length", &body_bytes.len().to_string())
            .body(&body_bytes)
            .send(&mut writer)
    {
        Ok(res) => {
            if !res.status_code().is_success() {
                log::error!("GitHub HTTP error: {:?}", res.status_code());
                return Err(anyhow::anyhow!("GitHub HTTP error: {:?}", res.status_code()));
            }
            Ok(writer)
        }
        Err(e) => {
            log::error!("Error getting response from GitHub: {:?}", e);
            Err(anyhow::anyhow!("Error getting response from GitHub: {:?}", e))
        }
    }
}
