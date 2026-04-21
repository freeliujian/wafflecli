use crate::tools::{Tool, ToolExecutor, get_tool_definitions};
use reqwest;
use ratatui::crossterm::execute;
use ratatui::crossterm::style::{Color, ResetColor, SetForegroundColor};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use std::env;
use std::error::Error;

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    tools: Option<Vec<Tool>>,
    tool_choice: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "system",
        }
    }
}

impl Serialize for MessageRole {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl std::str::FromStr for MessageRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" => Ok(MessageRole::User),
            "assistant" => Ok(MessageRole::Assistant),
            "system" => Ok(MessageRole::System),
            _ => Err(format!("Unknown message role: {}", s)),
        }
    }
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

async fn execute_tool(name: &str, args: &Value) -> String {
    let executor = ToolExecutor;
    match executor.execute(name, args.clone()).await {
        Ok(output) => output,
        Err(e) => format!("Error: {}", e),
    }
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

    pub async fn agent_loop(&mut self) -> Result<(), Box<dyn Error>> {
        let api_key: String = env::var("MOONSHOT_API_KEY")
            .unwrap_or_else(|_| "sk-ieTXY0Hs8XH8OUUhoKAAfoJPlJF2K5k52d9Vbg8MVOwKRC9J".to_string());
        let current_dir = env::current_dir().unwrap_or_default();
        let env_path = current_dir.to_string_lossy().into_owned();

        let system_prompt = format!(
            "You are a coding agent at {}. Use bash to inspect and change the workspace. Act first, then report clearly.",
            env_path
        );

        let tools = get_tool_definitions();

        loop {
            let mut all_messages = vec![Message {
                role: MessageRole::System,
                content: system_prompt.clone(),
            }];
            all_messages.extend(self.messages.clone());

            let request_body = ChatRequest {
                model: "kimi-k2.5".to_string(),
                messages: all_messages,
                temperature: Some(1.0),
                max_tokens: Some(8000),
                tools: Some(tools.clone()),
                tool_choice: Some("auto".to_string()),
            };

            let resp = self
                .client
                .post("https://api.moonshot.cn/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await?;

            if !resp.status().is_success() {
                let status = resp.status();
                let error_text = resp.text().await.unwrap_or_default();
                return Err(format!("Moonshot error, status: {}: {}", status, error_text).into());
            }

            let result: ChatResponse = resp.json().await?;

            if let Some(choice) = result.choices.first() {
                let message = &choice.message;

                if let Some(tool_calls) = &message.tool_calls {
                    let tool_calls_display: Vec<String> = tool_calls
                        .iter()
                        .map(|tc| format!("{}({})", tc.function.name, tc.function.arguments))
                        .collect();
                    self.messages.push(Message {
                        role: MessageRole::Assistant,
                        content: format!("Tool calls: {}", tool_calls_display.join(", ")),
                    });

                    for tool_call in tool_calls {
                        let tool_name = &tool_call.function.name;
                        let tool_id = &tool_call.id;
                        let args: Value = serde_json::from_str(&tool_call.function.arguments)
                            .unwrap_or(Value::Null);

                        let mut stdout = std::io::stdout();
                        let _ = execute!(
                            stdout,
                            SetForegroundColor(Color::Yellow),
                            ratatui::crossterm::style::Print(format!("> {}", tool_name)),
                            ResetColor,
                            ratatui::crossterm::style::Print("\n")
                        );

                        let output = execute_tool(tool_name, &args).await;

                        let display_output = if output.len() > 200 {
                            format!("{}...", &output[..200])
                        } else {
                            output.clone()
                        };
                        let mut stdout = std::io::stdout();
                        let _ = execute!(
                            stdout,
                            SetForegroundColor(Color::Cyan),
                            ratatui::crossterm::style::Print(display_output),
                            ResetColor,
                            ratatui::crossterm::style::Print("\n")
                        );

                        self.messages.push(Message {
                            role: MessageRole::User,
                            content: format!("Tool result ({}): {}", tool_id, output),
                        });
                    }

                    if let Some(usage) = result.usage {
                        let mut stdout = std::io::stdout();
                        let _ = execute!(
                            stdout,
                            SetForegroundColor(Color::DarkGrey),
                            ratatui::crossterm::style::Print(format!("Token usage: {}", usage.total_tokens)),
                            ResetColor,
                            ratatui::crossterm::style::Print("\n")
                        );
                    }

                    continue;
                }

                if let Some(content) = &message.content {
                    self.messages.push(Message {
                        role: MessageRole::Assistant,
                        content: content.clone(),
                    });
                }

                if let Some(usage) = result.usage {
                    let mut stdout = std::io::stdout();
                    let _ = execute!(
                        stdout,
                        SetForegroundColor(Color::DarkGrey),
                        ratatui::crossterm::style::Print(format!("Token usage: {}", usage.total_tokens)),
                        ResetColor,
                        ratatui::crossterm::style::Print("\n")
                    );
                }

                return Ok(());
            }

            return Err("No choices returned from Moonshot API".into());
        }
    }
}

pub async fn run_agent_from_pairs(
    messages: Vec<(String, String)>,
) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let list: Vec<Message> = messages
        .into_iter()
        .map(|(role, content)| Message {
            role: role.parse::<MessageRole>().expect("Invalid message role"),
            content,
        })
        .collect();

    let mut state = LoopState::new(list);
    state.agent_loop().await?;

    let out = state
        .messages
        .into_iter()
        .map(|m| (m.role.as_str().to_string(), m.content))
        .collect();

    Ok(out)
}
