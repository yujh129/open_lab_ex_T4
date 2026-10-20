//! `pet-cli`：最小命令行入口，用于验证 `pet-core` 的 trait 契约。
//!
//! 子命令：
//! - `chat "<message>"`：走 `MockChatProvider`，返回 `echo: <message>`。
//! - `memory list`：用 `InMemoryMemoryStore` 预置两条示例后列出。
//! - `state`：打印默认 `PetState`。
//!
//! 退出码：0 成功，1 用法错误，2 运行时错误。
//! 所有输出均为格式化 JSON，便于后续被 Tauri 后端复用。

use chrono::Utc;
use pet_core::mocks::{InMemoryMemoryStore, MockChatProvider};
use pet_core::{
    ChatMessage, ChatProvider, MemoryEntry, MemoryStore, Mood, PetState, Position, Role,
};
use serde::Serialize;
use std::process::ExitCode;

/// 命令行错误分类，映射到退出码。
#[derive(Debug)]
enum CliError {
    /// 用法错误 → 退出码 1。
    Usage(String),
    /// 运行时错误 → 退出码 2。
    Runtime(String),
}

/// 已解析的子命令。
enum Command {
    /// `chat "<message>"`。
    Chat { text: String },
    /// `memory list`。
    MemoryList,
    /// `state`。
    State,
}

/// `chat` 子命令的 JSON 输出结构。
#[derive(Serialize)]
struct ChatOutput {
    /// Mock 引擎返回的完整回复。
    reply: String,
}

/// `memory list` 子命令的 JSON 输出结构。
#[derive(Serialize)]
struct MemoryListOutput {
    /// 按时间倒序排列的记忆条目。
    entries: Vec<MemoryEntry>,
}

/// `state` 子命令的 JSON 输出结构。
#[derive(Serialize)]
struct StateOutput {
    /// 当前宠物领域状态。
    state: PetState,
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(CliError::Usage(msg)) => {
            eprintln!("usage error: {msg}");
            ExitCode::from(1)
        }
        Err(CliError::Runtime(msg)) => {
            eprintln!("runtime error: {msg}");
            ExitCode::from(2)
        }
    }
}

async fn run() -> Result<(), CliError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_command(&args)? {
        Command::Chat { text } => print_json(&build_chat_output(text).await?),
        Command::MemoryList => print_json(&build_memory_output().await?),
        Command::State => print_json(&build_state_output()),
    }
}

/// 解析子命令。所有非法组合统一返回 [`CliError::Usage`]，最终退出码 1。
fn parse_command(args: &[String]) -> Result<Command, CliError> {
    match args.first().map(String::as_str) {
        Some("chat") => {
            if args.len() != 2 {
                return Err(CliError::Usage("usage: pet-cli chat \"<message>\"".into()));
            }
            Ok(Command::Chat {
                text: args[1].clone(),
            })
        }
        Some("memory") => match (args.len(), args.get(1).map(String::as_str)) {
            (2, Some("list")) => Ok(Command::MemoryList),
            _ => Err(CliError::Usage("usage: pet-cli memory list".into())),
        },
        Some("state") => {
            if args.len() != 1 {
                return Err(CliError::Usage("usage: pet-cli state".into()));
            }
            Ok(Command::State)
        }
        Some(other) => Err(CliError::Usage(format!("unknown command: {other}"))),
        None => Err(CliError::Usage(
            "usage: pet-cli <chat|memory|state> ...".into(),
        )),
    }
}

/// 构造 `chat` 输出：向 Mock 注册一条 `echo: <text>` 回复后发起对话。
async fn build_chat_output(text: String) -> Result<ChatOutput, CliError> {
    let provider = MockChatProvider::new();
    provider.push_response(format!("echo: {text}"));
    let messages = vec![ChatMessage {
        role: Role::User,
        content: text,
    }];
    // 空闭包作为 token 回调；显式标注为内联 dyn 类型，避免类型别名带来 'static 推断。
    let noop = |_: &str| {};
    let cb: &(dyn for<'a> Fn(&'a str) + Send + Sync) = &noop;
    let reply = provider
        .chat(&messages, cb)
        .await
        .map_err(|e| CliError::Runtime(format!("chat failed: {e}")))?;
    Ok(ChatOutput { reply })
}

/// 构造 `memory list` 输出：内存 store 预置两条示例后读取最近条目。
async fn build_memory_output() -> Result<MemoryListOutput, CliError> {
    let store = InMemoryMemoryStore::new();
    store
        .remember(Role::User, "hello")
        .await
        .map_err(|e| CliError::Runtime(format!("remember failed: {e}")))?;
    store
        .remember(Role::Assistant, "hi there")
        .await
        .map_err(|e| CliError::Runtime(format!("remember failed: {e}")))?;
    let entries = store
        .all_recent(100)
        .await
        .map_err(|e| CliError::Runtime(format!("all_recent failed: {e}")))?;
    Ok(MemoryListOutput { entries })
}

/// 构造 `state` 输出：默认情绪、原点位置、满精力。
fn build_state_output() -> StateOutput {
    let state = PetState {
        mood: Mood::Neutral,
        position: Position::default(),
        energy: 1.0,
        last_interaction_at: Utc::now(),
    };
    StateOutput { state }
}

/// 将任意 `Serialize` 结构体以 pretty JSON 打印到 stdout。
fn print_json<T: Serialize>(value: &T) -> Result<(), CliError> {
    let s = serde_json::to_string_pretty(value)
        .map_err(|e| CliError::Runtime(format!("json serialize failed: {e}")))?;
    println!("{s}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn chat_output_contains_echo_reply() {
        let out = build_chat_output("你好".to_string()).await.unwrap();
        assert_eq!(out.reply, "echo: 你好");
    }

    #[tokio::test]
    async fn memory_output_has_two_seeded_entries() {
        let out = build_memory_output().await.unwrap();
        assert_eq!(out.entries.len(), 2);
        assert!(out.entries.iter().any(|e| e.content == "hello"));
        assert!(out.entries.iter().any(|e| e.content == "hi there"));
    }

    #[test]
    fn state_output_serializes_with_mood_and_energy() {
        let out = build_state_output();
        let s = serde_json::to_string(&out).unwrap();
        assert!(s.contains("\"mood\""));
        assert!(s.contains("\"neutral\""));
        assert!(s.contains("\"energy\""));
        assert!(s.contains("\"last_interaction_at\""));
    }
}
