use indoc::indoc;
use reqwest;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: Option<f32>,
    max_token: Option<u32>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize, Debug)]
struct Choice {
    message: MessageResponse,
}

#[derive(Deserialize, Debug)]
struct ChatResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Deserialize, Debug)]
struct MessageResponse {
    content: String,
}

#[derive(Deserialize, Debug)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

pub async fn request_llm() -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let api_key = "";
    let request_body = ChatRequest {
        model: "kimi-k2.5".to_string(),
        messages: vec![
            Message {
                role: "system".to_string(),
                content: "你是一个有帮助的 AI 助手。".to_string(),
            },
            Message {
                role: "user".to_string(),
                content: "Rust 如何实现一个简单的 CLI 工具？".to_string(),
            },
        ],
        temperature: Some(1.0),
        max_token: Some(2048),
    };
    let resp = client
        .post("https://api.moonshot.cn/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;
    // let text = resp.text().await?;
    let result: ChatResponse = resp.json().await?;
    // println!("原始响应: {}", text);

    if let Some(usage) = result.usage {
        println!("Token 使用: {}", usage.total_tokens);
    }

    Ok(())
}

fn basic_creator_skill() {
    let skill_str = indoc! {
        "
        ---
        skill_name: qodercli_creator_skill
        description: 这是一个用于 QoderCLI 的元技能（meta-skill），能够根据用户需求自动生成新的 skill 文件，并确保符合 QoderCLI 的 skill 规范。
        ---
        "
    };
}
