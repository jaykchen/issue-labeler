use anyhow;
use chrono::Duration;
use octocrab_wasi::{ models::issues::Issue, params::{ issues::Sort, Direction, State } };
use regex::Regex;
use serde::{ Deserialize, Serialize };
use serde_json::{ json, Map, Value };
use openai_flows::{ chat::{ ChatModel, ChatOptions }, OpenAIFlows };
use std::{ collections::{ HashMap, HashSet }, env };
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

    let pattern = Regex::new(r"### Response:\n.*?`([^`]+)`").unwrap();

    let known_labels_set: HashSet<String> = known_labels
        .into_iter()
        .map(|s| s.to_lowercase())
        .collect();

    if let Some(captures) = pattern.captures(input) {
        if let Some(matched) = captures.get(1) {
            let labels = matched.as_str();
            let extracted_labels: Vec<&str> = labels.split(',').map(str::trim).collect();

            let mut final_labels = Vec::new();

            for label in extracted_labels {
                // Normalize the label for comparison
                let normalized_label = label.to_lowercase();

                // Check if the normalized label is "close enough" to any known label
                // This is a basic check, replace with fuzzy matching if needed
                let close_enough_label = known_labels_set
                    .iter()
                    .find(|&known_label| known_label == &normalized_label);

                match close_enough_label {
                    Some(known_label) => final_labels.push(known_label.clone()),
                    None => final_labels.push(normalized_label), // If not found, treat as a new/unknown label
                }
            }

            log::info!("Extracted labels: {:?}", final_labels);
            Ok(final_labels)
        } else {
            Err(anyhow::anyhow!("No labels found within backticks"))
        }
    } else {
        Err(anyhow::anyhow!("'Response' section not found or does not follow the expected format"))
    }
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
