use std::{
    env::{self, current_dir},
    error::Error,
    path::PathBuf,
    process::Stdio,
    time::Duration,
    vec,
};

use crate::llm::request_llm::{Message, MessageRole};
use serde_json::{Map, Value};
use std::collections::HashSet;
use tokio::{
    fs::{File, canonicalize, create_dir_all, write},
    io::AsyncReadExt,
    process::Command,
    time::timeout,
};

pub fn safe_path(p: String) -> Option<String> {
    Some(
        env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
    )
}

pub async fn run_bash(command: String) -> Result<String, Box<dyn Error>> {
    let dangerous_pattern: Vec<String> = ["rm -rf /", "sudo", "shutdown", "reboot", "> /dev/"]
        .map(String::from)
        .to_vec();
    for pattern in dangerous_pattern.iter() {
        if command.contains(pattern) {
            return Err(format!("Error: Dangerous command blocked (found: {})", pattern).into());
        }
    }

    let output = run_command_with_timeout(&command, 120).await?;
    Ok(output)
}

pub async fn run_read(path: String, limit: isize) -> Result<String, Box<dyn Error>> {
    let mut p = PathBuf::from(&path);
    if p.is_relative() {
        p = current_dir()?.join(p);
    }

    let canonical = canonicalize(&p).await?;
    let cwd = current_dir()?;
    let cwd_canon = canonicalize(&cwd).await?;
    if canonical.starts_with(&cwd_canon) {
        return Err("Access denied: path outside workspace".into());
    }

    let mut file = File::open(&canonical).await?;
    if limit <= 0 {
        let mut s = String::new();
        file.read_to_string(&mut s).await?;
        return Ok(s);
    }

    let mut buf = vec![0u8; limit as usize];
    let n = file.read(&mut buf).await?;
    buf.truncate(n);
    let s = String::from_utf8_lossy(&buf).to_string();

    if (n as isize) == limit {
        Ok(format!("{}{}", s, "\n...truncated..."))
    } else {
        Ok(s)
    }
}

pub async fn run_write(path: String, content: String) -> Result<String, Box<dyn Error>> {
    let mut p = PathBuf::from(&path);
    if p.is_relative() {
        p = current_dir()?.join(p);
    }

    if let Some(parent) = p.parent() {
        create_dir_all(path).await?;
    }

    let parent_path = if let Some(parent) = p.parent() {
        parent.to_path_buf()
    } else {
        current_dir()?
    };

    let canonical_parent = canonicalize(&parent_path).await?;
    let cwd = current_dir()?;
    let cwd_canon = canonicalize(&cwd).await?;
    if !canonical_parent.starts_with(&cwd_canon) {
        return Err("Access denied: path outside workspace".into());
    }

    write(&p, content).await?;
    Ok("OK".to_string())
}

pub async fn run_edit(
    path: String,
    old_text: String,
    new_text: String,
) -> Result<String, Box<dyn Error>> {
    let content = run_read(path.clone(), -1).await?;
    let count = if old_text.is_empty() {
        0
    } else {
        content.matches(&old_text).count()
    };

    if count == 0 {
        return Ok(format!("Replaced 0 occurrences"));
    }

    let new_content = content.replace(&old_text, &new_text);
    run_write(path, new_content).await?;
    Ok(format!("Replaced {} occurrences", count))
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

pub fn get_tool_definitions() -> Vec<Tool> {
    vec![
        Tool {
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
        },
        Tool {
            type_: String::from("function"),
            function: ToolFunction {
                name: String::from("read_file"),
                description: String::from("Read the contents of a file."),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to read"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of bytes to read. -1 for no limit."
                        }
                    },
                    "required": ["path"]
                }),
            },
        },
        Tool {
            type_: String::from("function"),
            function: ToolFunction {
                name: String::from("write_file"),
                description: String::from("Write content to a file."),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to write"
                        },
                        "content": {
                            "type": "string",
                            "description": "Content to write"
                        }
                    },
                    "required": ["path", "content"]
                }),
            },
        },
        Tool {
            type_: String::from("function"),
            function: ToolFunction {
                name: String::from("edit_file"),
                description: String::from("Replace text in a file."),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to edit"
                        },
                        "old_text": {
                            "type": "string",
                            "description": "Text to replace"
                        },
                        "new_text": {
                            "type": "string",
                            "description": "New text to insert"
                        }
                    },
                    "required": ["path", "old_text", "new_text"]
                }),
            },
        },
    ]
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct Tool {
    #[serde(rename = "type")]
    pub type_: String,
    pub function: ToolFunction,
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

enum ToolHandle {
    Bash {
        command: String,
    },
    ReadFile {
        path: String,
        content: String,
    },
    WriteFile {
        path: String,
        content: String,
    },
    EditFile {
        path: String,
        old_text: String,
        new_text: String,
    },
}

type ToolResult = Result<String, Box<dyn Error>>;

pub struct ToolExecutor;

impl ToolExecutor {
    pub async fn execute(&self, tool_name: &str, args: Value) -> ToolResult {
        match tool_name {
            "bash" => {
                let command = args["command"]
                    .as_str()
                    .ok_or("Missing 'command' parameter")?;
                run_bash(command.to_string()).await
            }
            "read_file" => {
                let path = args["path"].as_str().ok_or("Missing 'path'")?;
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(-1) as isize;
                run_read(path.to_string(), limit).await
            }
            "write_file" => {
                let path = args["path"].as_str().ok_or("Missing 'path'")?;
                let content = args["content"].as_str().ok_or("Missing 'content'")?;
                run_write(path.to_string(), content.to_string()).await
            }
            "edit_file" => {
                let path = args["path"].as_str().ok_or("Missing 'path'")?;
                let old_text = args["old_text"].as_str().ok_or("Missing 'old_text'")?;
                let new_text = args["new_text"].as_str().ok_or("Missing 'new_text'")?;
                run_edit(path.to_string(), old_text.to_string(), new_text.to_string()).await
            }
            _ => Err(format!("Unknown tool: {}", tool_name).into()),
        }
    }
}

pub fn normalize_messages(messages: Vec<Message>) -> Vec<Value> {
    let mut cleaned: Vec<Value> = Vec::new();
    for m in messages.into_iter() {
        let role_str = match m.role {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "system",
        };
        let content_val = if let Ok(parsed) = serde_json::from_str::<Value>(&m.content) {
            match parsed {
                Value::Array(arr) => {
                    let mut out: Vec<Value> = Vec::new();
                    for block in arr.into_iter() {
                        if let Value::Object(obj) = block {
                            let mut map = Map::new();
                            for (k, v) in obj.into_iter() {
                                if !k.starts_with('_') {
                                    map.insert(k, v);
                                }
                            }
                            out.push(Value::Object(map));
                        }
                    }
                    Value::Array(out)
                }
                other => Value::String(m.content),
            }
        } else {
            Value::String(m.content)
        };

        let mut obj = Map::new();
        obj.insert("role".to_string(), Value::String(role_str.to_string()));
        obj.insert("content".to_string(), content_val);
        cleaned.push(Value::Object(obj));
    }

    let mut existing_results: HashSet<String> = HashSet::new();
    for msg in cleaned.iter() {
        if let Value::Object(map) = msg {
            if let Some(Value::Array(arr)) = map.get("content") {
                for block in arr.iter() {
                    if let Value::Object(bmap) = block {
                        if let Some(Value::String(tpe)) = bmap.get("type") {
                            if tpe == "tool_result" {
                                if let Some(Value::String(id)) = bmap.get("tool_use_id") {
                                    existing_results.insert(id.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut additions: Vec<Value> = Vec::new();
    for msg in cleaned.iter() {
        if let Value::Object(map) = msg {
            if let Some(Value::String(role)) = map.get("role") {
                if role != "assistant" {
                    continue;
                }
            } else {
                continue;
            }
            if let Some(Value::Array(arr)) = map.get("content") {
                for block in arr.iter() {
                    if let Value::Object(bmap) = block {
                        if let Some(Value::String(tpe)) = bmap.get("type") {
                            if tpe == "tool_use" {
                                if let Some(Value::String(id)) = bmap.get("id") {
                                    if !existing_results.contains(id) {
                                        let mut tr = Map::new();
                                        tr.insert(
                                            "type".to_string(),
                                            Value::String("tool_result".into()),
                                        );
                                        tr.insert(
                                            "tool_use_id".to_string(),
                                            Value::String(id.clone()),
                                        );
                                        tr.insert(
                                            "content".to_string(),
                                            Value::String("(cancelled)".into()),
                                        );
                                        let mut new_msg = Map::new();
                                        new_msg.insert(
                                            "role".to_string(),
                                            Value::String("user".into()),
                                        );
                                        new_msg.insert(
                                            "content".to_string(),
                                            Value::Array(vec![Value::Object(tr)]),
                                        );
                                        additions.push(Value::Object(new_msg));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    cleaned.extend(additions);

    if cleaned.is_empty() {
        return cleaned;
    }

    let mut merged: Vec<Value> = Vec::new();
    merged.push(cleaned[0].clone());

    for msg in cleaned.iter().skip(1) {
        let curr_role = msg
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let last = merged.last_mut().unwrap();
        let last_role = last
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        if curr_role == last_role {
            let prev_content = last
                .get("content")
                .cloned()
                .unwrap_or(Value::String("".into()));
            let curr_content = msg
                .get("content")
                .cloned()
                .unwrap_or(Value::String("".into()));

            let prev_arr = match prev_content {
                Value::Array(a) => a,
                Value::String(s) => vec![Value::Object({
                    let mut m = Map::new();
                    m.insert("type".to_string(), Value::String("text".into()));
                    m.insert("text".to_string(), Value::String(s));
                    m
                })],
                other => vec![Value::Object({
                    let mut m = Map::new();
                    m.insert("type".to_string(), Value::String("text".into()));
                    m.insert("text".to_string(), Value::String(other.to_string()));
                    m
                })],
            };

            let curr_arr = match curr_content {
                Value::Array(a) => a,
                Value::String(s) => vec![Value::Object({
                    let mut m = Map::new();
                    m.insert("type".to_string(), Value::String("text".into()));
                    m.insert("text".to_string(), Value::String(s));
                    m
                })],
                other => vec![Value::Object({
                    let mut m = Map::new();
                    m.insert("type".to_string(), Value::String("text".into()));
                    m.insert("text".to_string(), Value::String(other.to_string()));
                    m
                })],
            };

            let mut combined = prev_arr;
            combined.extend(curr_arr);

            if let Some(lm) = last.as_object_mut() {
                lm.insert("content".to_string(), Value::Array(combined));
            }
        } else {
            merged.push(msg.clone());
        }
    }

    merged
}
