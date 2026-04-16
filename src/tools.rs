use std::{env, error::Error, process::Stdio, time::Duration};

use serde_json::Value;
use tokio::{process::Command, time::timeout};

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
    Ok(String::new())
}

pub async fn run_write(path: String, content: String) -> Result<String, Box<dyn Error>> {
    Ok(String::new())
}

pub async fn run_edit(
    path: String,
    old_text: String,
    new_text: String,
) -> Result<String, Box<dyn Error>> {
    Ok(String::new())
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

#[derive(Debug, Clone)]
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

struct ToolExecutor;

impl ToolExecutor {
    async fn execute(&self, tool_name: &str, args: Value) -> ToolResult {
        match tool_name {
            "bash" => {
                let command = args["command"]
                    .as_str()
                    .ok_or("Missing 'command' parameter")?;
                run_bash(command.to_string()).await
            }
            "read_file" => {
                let path = args["path"].as_str().ok_or("Missing 'path'")?;
                if let Some(limit) = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as isize)
                {
                    run_read(path.to_string(), limit).await
                } else {
                    Err("调用失败".into())
                }
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
