use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::error::Error;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Serialize, Debug, Clone)]
struct Tool {
    #[serde(rename = "type")]
    type_: String,
    function: ToolFunction,
}

#[derive(Serialize, Debug, Clone)]
struct ToolFunction {
    name: String,
    description: String,
    parameters: Value,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    tools: Option<Vec<Tool>>,
    tool_choice: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
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
    content: Option<String>,
    tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Deserialize, Debug)]
struct ToolCall {
    id: String,
    #[serde(rename = "type")]
    type_: String,
    function: ToolCallFunction,
}

#[derive(Deserialize, Debug)]
struct ToolCallFunction {
    name: String,
    arguments: String,
}

#[derive(Deserialize, Debug)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Serialize, Debug)]
struct ToolResult {
    #[serde(rename = "type")]
    type_: String,
    tool_use_id: String,
    content: String,
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
#[derive(Clone)]
pub struct LoopState {
    pub messages: Vec<Message>,
    turn_count: i32,
    transition_reason: Option<String>,
    client: reqwest::Client,
}

impl LoopState {
    pub fn new(list: Vec<Message>) -> Self {
        let client = reqwest::Client::new();
        LoopState {
            messages: list,
            turn_count: 1,
            transition_reason: None,
            client,
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
        let content: &[Value] = match content {
            Some(list) if !list.is_empty() => list,
            _ => return Some(String::new()),
        };

        let mut texts = Vec::new();

        for block in content {
            let text = match block {
                Value::Object(map) => map.get("text").and_then(|v| v.as_str()),
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

    async fn execute_tool_calls(
        &self,
        response_content: Vec<Value>,
    ) -> Result<Vec<ToolResult>, Box<dyn Error>> {
        let mut results = Vec::new();

        for block in response_content {
            let type_str = match block.get("type").and_then(|v| v.as_str()) {
                Some(t) => t,
                None => continue,
            };

            if type_str != "tool_use" {
                continue;
            }

            let command = match block
                .get("input")
                .and_then(|i| i.get("command"))
                .and_then(|c| c.as_str())
            {
                Some(cmd) => cmd.to_string(),
                None => {
                    eprintln!("Warning: tool_use block missing command");
                    continue;
                }
            };

            let tool_id = block
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            println!("\x1b[33m$ {}\x1b[0m", command);

            match LoopState::run_bash(command).await {
                Ok(output) => {
                    let display_output = if output.len() > 200 {
                        &output[..200]
                    } else {
                        &output
                    };
                    println!("{}", display_output);

                    results.push(ToolResult {
                        type_: "tool_result".to_string(),
                        tool_use_id: tool_id,
                        content: output,
                    });
                }
                Err(e) => {
                    let error_msg = format!("Error: {}", e);
                    eprintln!("{}", error_msg);

                    results.push(ToolResult {
                        type_: "tool_result".to_string(),
                        tool_use_id: tool_id,
                        content: error_msg,
                    });
                }
            }
        }

        Ok(results)
    }

    async fn run_one_turn(&mut self) -> Result<Option<Vec<ToolResult>>, Box<dyn Error>> {
        let tool = Tool {
            type_: String::from("function"),
            function: ToolFunction {
                name: String::from("bash"),
                description: String::from("Run a shell command in the current workspace."),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The shell command to execute"
                        }
                    },
                    "required": ["command"]
                }),
            },
        };

        let client = self.client.clone();
        let api_key: String = String::from("sk-ieTXY0Hs8XH8OUUhoKAAfoJPlJF2K5k52d9Vbg8MVOwKRC9J");
        let current_dir = env::current_dir().unwrap_or_default();
        let env_path = current_dir.to_string_lossy().into_owned();

        let system_prompt = format!(
            "You are a coding agent at {}. Use bash to inspect and change the workspace. Act first, then report clearly.",
            env_path
        );

        let mut all_messages = vec![Message {
            role: "system".to_string(),
            content: system_prompt,
        }];
        let cp_messages = self.messages.clone();
        all_messages.extend(cp_messages);

        let request_body = ChatRequest {
            model: "kimi-k2.5".to_string(),
            messages: all_messages,
            temperature: Some(1.0),
            max_tokens: Some(8000),
            tools: Some(vec![tool]),
            tool_choice: Some("auto".to_string()),
        };

        let resp = client
            .post("https://api.moonshot.cn/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let error_text = resp.text().await.unwrap_or_default();
            return Err(format!("Moonshot 错误，status 为：{}: {}", status, error_text).into());
        }

        let result: ChatResponse = resp.json().await?;

        if let Some(choice) = result.choices.first() {
            let message = &choice.message;

            if let Some(content) = &message.content {
                self.messages.push(Message {
                    role: "assistant".to_string(),
                    content: content.clone(),
                });
            }

            if let Some(tool_calls) = &message.tool_calls {
                println!("模型请求调用工具:");
                for tool_call in tool_calls {
                    println!(
                        "  - 工具: {}, 参数: {}",
                        tool_call.function.name, tool_call.function.arguments
                    );
                }

                let tool_calls_value: Vec<Value> = tool_calls
                    .iter()
                    .map(|tc| {
                        serde_json::json!({
                            "type": "tool_use",
                            "id": tc.id,
                            "input": {
                                "command": tc.function.arguments
                            }
                        })
                    })
                    .collect();

                let tool_results = self.execute_tool_calls(tool_calls_value).await?;

                for result in &tool_results {
                    self.messages.push(Message {
                        role: "user".to_string(),
                        content: format!(
                            "Tool result ({}): {}",
                            result.tool_use_id, result.content
                        ),
                    });
                }

                if let Some(usage) = result.usage {
                    println!("Token 使用: {}", usage.total_tokens);
                }

                return Ok(Some(tool_results));
            }

            if let Some(usage) = result.usage {
                println!("Token 使用: {}", usage.total_tokens);
            }
        }

        Err("调用失败".into())
    }

    pub async fn agent_loop(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            match self.run_one_turn().await {
                Ok(Some(_)) => {
                    self.turn_count += 1;
                    if self.turn_count > 10 {
                        self.transition_reason = Some("达到最大轮数限制".to_string());
                        break;
                    }
                }
                Ok(None) => {
                    self.transition_reason = Some("对话自然结束".to_string());
                    break;
                }
                Err(e) => {
                    self.transition_reason = Some(format!("错误: {}", e));
                    return Err(e);
                }
            }
        }
        Ok(())
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

pub async fn run_agent_from_pairs(
    messages: Vec<(String, String)>,
) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let list: Vec<Message> = messages
        .into_iter()
        .map(|(role, content)| Message { role, content })
        .collect();

    let mut state = LoopState::new(list);
    state.agent_loop().await?;

    let out = state
        .messages
        .into_iter()
        .map(|m| (m.role, m.content))
        .collect();

    Ok(out)
}
