//! Tauri 2 应用壳：只依赖 `pet-core` 的 trait 对象，不依赖任何具体实现。

use std::sync::Arc;

use pet_core::{ChatMessage, ChatProvider, PetState, PetStateStore, Role, SpeechSink};
use serde::Serialize;
use tauri::{Emitter, Manager, State};

/// 全局运行态：由 `pet-app` 在启动时注入具体实现。
pub struct AppState {
    /// 宠物领域状态的唯一事实源。
    pub store: Arc<dyn PetStateStore>,
    /// 语音输出。
    pub speech: Arc<dyn SpeechSink>,
    /// 对话引擎（阶段 2 用 MockChatProvider）。
    ///
    /// 字段名刻意不用 `chat`：`#[tauri::command] async fn chat` 与字段同名时，
    /// `generate_handler!` 展开会在 `run` 内把标识符 `chat` 解析到字段而非
    /// 命令函数，触发 E0618。
    pub chat_provider: Arc<dyn ChatProvider>,
}

/// 前端可渲染的宠物状态快照。
#[derive(Clone, Serialize)]
pub struct PetSnapshot {
    /// 当前情绪：`"idle"` / `"happy"` / `"talking"`。
    pub mood: String,
    /// 当前是否正在说话。
    pub talking: bool,
}

fn snapshot(state: &AppState) -> PetSnapshot {
    let s: PetState = state.store.current();
    let talking = state.speech.is_speaking();
    let mood = if talking {
        "talking"
    } else if s.mood == pet_core::Mood::Happy {
        "happy"
    } else {
        "idle"
    };
    PetSnapshot {
        mood: mood.to_string(),
        talking,
    }
}

fn notify_state(app: &tauri::AppHandle, state: &AppState) {
    let _ = app.emit("pet://state", snapshot(state));
}

/// 获取当前状态快照。
#[tauri::command]
fn get_state(state: State<'_, AppState>) -> PetSnapshot {
    snapshot(&state)
}

/// 宠物区域被单击：记录交互，发出 `Clicked`，并让前端切到 `happy`。
#[tauri::command]
fn pet_clicked(app: tauri::AppHandle, state: State<'_, AppState>) {
    state.store.on_interaction();
    notify_state(&app, &state);
}

/// 让宠物直接说一句话（保留旧接口，供内部或其他命令复用）。
#[tauri::command]
fn speak(app: tauri::AppHandle, state: State<'_, AppState>, text: String) {
    let _ = app.emit("pet://speech", text.clone());
    let _ = state.speech.speak(&text);
    notify_state(&app, &state);
}

/// 核心聊天逻辑：**不依赖 Tauri**，便于单测。
///
/// 成功返回完整回复；失败把 `PetError` 转成用户可见的中文文案。
pub(crate) async fn run_chat(
    chat: &dyn ChatProvider,
    text: String,
) -> Result<String, String> {
    let messages = vec![ChatMessage {
        role: Role::User,
        content: text,
    }];
    let noop = |_: &str| {};
    let cb: &(dyn for<'a> Fn(&'a str) + Send + Sync) = &noop;
    chat.chat(&messages, cb)
        .await
        .map_err(|e| format!("抱歉，我刚才走神了：{e}"))
}

/// 聊天 command：前端输入文本 → 调 `ChatProvider` → 事件 + 语音。
///
/// 事件顺序：
/// 1. `pet://talking` true
/// 2. `pet://speech` (text) 或 `pet://chat_error` (msg)
/// 3. `pet://talking` false
///
/// 无论成功还是失败都会回到 `talking=false`，前端不会卡在 talking 状态。
#[tauri::command]
async fn chat(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    text: String,
) -> Result<String, String> {
    let _ = app.emit("pet://talking", true);
    let result = run_chat(state.chat_provider.as_ref(), text).await;

    match &result {
        Ok(reply) => {
            let _ = app.emit("pet://speech", reply.clone());
            if let Err(e) = state.speech.speak(reply) {
                tracing::warn!("speak failed: {e}");
            }
            // 立即 stop：MockSpeechSink 同步完成，避免 is_speaking 卡在 true。
            let _ = state.speech.stop();
        }
        Err(msg) => {
            let _ = app.emit("pet://chat_error", msg.clone());
        }
    }

    let _ = app.emit("pet://talking", false);
    result
}

/// 构造并运行 Tauri 应用。
///
/// 三个依赖由 `pet-app` 注入，`pet-ui` 不知道具体实现。
///
/// 参数名 `chat_provider` 而非 `chat`：避免与 `#[tauri::command] async fn chat`
/// 同名，后者会让 `generate_handler!` 展开时解析到局部变量。
pub fn run(
    store: Arc<dyn PetStateStore>,
    speech: Arc<dyn SpeechSink>,
    chat_provider: Arc<dyn ChatProvider>,
) {
    tauri::Builder::default()
        .setup(move |app| {
            app.manage(AppState {
                store,
                speech,
                chat_provider,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            pet_clicked,
            speak,
            chat
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use pet_core::{PetError, PetResult};

    /// 始终返回错误的 provider，用来验证错误转换路径。
    struct FailingChat;

    #[async_trait]
    impl ChatProvider for FailingChat {
        async fn chat(
            &self,
            _messages: &[ChatMessage],
            _on_token: &(dyn for<'a> Fn(&'a str) + Send + Sync),
        ) -> PetResult<String> {
            Err(PetError::Ai("connection refused".into()))
        }
    }

    #[tokio::test]
    async fn run_chat_maps_provider_error_to_user_message() {
        let msg = run_chat(&FailingChat, "你好".into()).await.unwrap_err();
        assert!(msg.contains("抱歉"), "got: {msg}");
        assert!(msg.contains("connection refused"), "got: {msg}");
    }

    #[tokio::test]
    async fn run_chat_returns_reply_on_success() {
        use pet_core::mocks::MockChatProvider;
        let mock = MockChatProvider::new();
        mock.push_response("echo: hi");
        let reply = run_chat(&mock, "hi".into()).await.unwrap();
        assert_eq!(reply, "echo: hi");
    }
}
