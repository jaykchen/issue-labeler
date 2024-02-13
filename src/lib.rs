pub mod llm_low;
pub mod utils;
use chrono::{ Datelike, Duration, Timelike, Utc };
use dotenv::dotenv;
use flowsnet_platform_sdk::logger;
use github_flows::{ get_octo, GithubLogin };
use llm_low::completion_inner_async;
use octocrab_wasi::params::State;
use octocrab_wasi::{ params::issues::Sort, params::Direction };
use openai_flows::{ chat::{ ChatModel, ChatOptions }, OpenAIFlows };
use schedule_flows::{ schedule_cron_job, schedule_handler };
use std::{ collections::{ HashMap, HashSet }, env };
use utils::*;

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn on_deploy() {
    // let now = Utc::now();
    // let now_minute = now.minute() + 2;
    // let cron_time = format!("{:02} {:02} {:02} * *", now_minute, now.hour(), now.day());
    let cron_time = format!("2 2 * * *");
    schedule_cron_job(cron_time, String::from("cron_job_evoked")).await;
}

#[schedule_handler]
async fn handler(body: Vec<u8>) {
    dotenv().ok();
    logger::init();

    let _ = inner().await;
}
async fn inner() -> anyhow::Result<()> {
    let octocrab = get_octo(&GithubLogin::Default);

    let issue_handle = octocrab.issues("wasmedge", "wasmedge");
    let report_issue_handle = octocrab.issues("jaykchen", "issue-labeler");

    let list = issue_handle
        .list()
        .state(State::Open)
        // .milestone(1234)
        // .assignee("ferris")
        // .creator("octocrab")
        // .mentioned("octocat")
        // .labels(&labels)
        .sort(Sort::Created)
        .direction(Direction::Descending)
        .per_page(10)
        .page(1u8)
        .send().await?;

    let n_days_ago = (Utc::now() - Duration::days(1)).naive_utc();
    let contributors_set = HashSet::new();
    for issue in list.items {
        log::info!("{:?}", issue.title);
        // if issue.pull_request.is_some() {
        //     continue;
        // }
        // let labels = issue.labels.clone();
        // if
        //     issue.created_at.naive_utc() < n_days_ago
        //     // || !issue.labels.is_empty()
        // {
        //     continue;
        // }

        let payload = why_labels(&issue, contributors_set.clone()).await?;
        let title = payload.title.clone();
        let creator = payload.creator.clone();
        let essence = payload.essence.clone();
        log::info!("{:?}", essence.clone().unwrap_or_default());

        let question = format!(
            "Can you assign labels to the GitHub issue titled `{title}` created by `{creator}`, stating `{essence:?}`?"
        );

        let query = format!(
            r#"Below is an instruction that describes a task, paired with an input that provides further context. Write a response that appropriately completes the request.

        ### Instruction:
        You're a programming bot tasked to analyze GitHub issues data and assign labels to them"
        ### Input:
        {question}
        
        ### Response:"#
        );

        let res = completion_inner_async(&query).await?;

        let labels: Vec<String> = parse_labels_from_response(&res)?;
        if labels.is_empty() {
            continue;
        }
        // let label_slice: Vec<String> = labels.iter().map(|label| label.to_string()).collect();
        let report_issue = report_issue_handle
            .create(title.clone())
            .body("demo")
            .labels(labels)
            .send().await?;

        break;
    }

    Ok(())
}
