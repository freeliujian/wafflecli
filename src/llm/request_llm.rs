use ratatui::widgets::List;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::error::Error;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use subprocess::*;
use tokio::process::Command;
use tokio::time::timeout;

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

enum MessagesStateManage {
    Generating,
    Thinking,
    Stream,
    ToolCalling,
    ToolExecuting,
    TollResult,
    WaitingUser,
    Completed,
    Error,
    Cancelled,
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
    let result: ChatResponse = resp.json().await?;

    if let Some(usage) = result.usage {
        println!("Token 使用: {}", usage.total_tokens);
    }

    Ok(())
}

fn agents() {
    let current_dir = env::current_dir().expect("没有找到当前目录");
    let dir_name = current_dir.to_string_lossy().into_owned();
    let system = format!(
        "You are a coding agent at {cwd}. 
        Use bash to inspect and change the workspace. Act first, then report clearly.",
        cwd = dir_name
    );
}

#[derive(Serialize)]
struct LoopState {
    messages: Vec<Message>,
    turn_count: i32,
    transition_reason: Option<String>,
}

impl LoopState {
    fn new(list: Vec<Message>) -> Self {
        LoopState {
            messages: list,
            turn_count: 1,
            transition_reason: None,
        }
    }

    async fn run_bash(command: String) -> Result<String, Box<dyn Error>> {
        let dangerous_pattern: Vec<String> = ["rm -rf /", "sudo", "shutdown", "reboot", "> /dev/"]
            .map(String::from)
            .to_vec();
        for pattern in dangerous_pattern.iter() {
            if command.contains(pattern) {
                return Err(
                    format!("Error: Dangerous command blocked (found: {})", pattern).into(),
                );
            }
        }

        let output = run_command_with_timeout(&command, 120).await?;
        Ok(output)
    }

    fn extract_text(&mut self, content: Option<&[Value]>) -> Option<String> {
        let content = match content {
            Some(list) if !list.is_empty() => list,
            _ => return Some(String::new()),
        };

        let mut texts = Vec::new();

        for block in content {
            let text = match block {
                Value::Object(map) => {
                    map.get("text").and_then(|v| v.as_str())
                },
                _ => None,
            };

            if let Some(t) = text {
                if !t.trim().is_empty() {
                    texts.push(t);
                }
            }
        }

        Some(texts.join("\n").trim().to_string())
    }

    fn execute_tool_calls(response_content: Vec<ChatResponse>)  {

    }

    fn run_one_turn(&mut self, state:&LoopState) -> Option<bool> {
        None
    }

    fn agent_loop(&mut self, state:LoopState) -> Option<bool> {
        loop {
            self.run_one_turn(&state);
        }
    }


}
async fn run_command_with_timeout(command: &str, timeout_secs: u64) -> Result<String, String> {
    let child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(std::env::current_dir().unwrap())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    let output = match timeout(Duration::from_secs(timeout_secs), child.wait_with_output()).await {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => return Err(e.to_string()),
        Err(_) => {
            return Err(format!("Command timed out after {} seconds", timeout_secs));
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "Command failed (code: {}): {}",
            output.status.code().unwrap_or(-1),
            stderr
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(stdout)
}
