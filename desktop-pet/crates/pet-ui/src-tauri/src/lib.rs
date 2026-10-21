//! Tauri 2 应用壳

use pet_core::{ChatMessage, ChatProvider, PetState, PetStateStore, Role, SpeechSink};
use serde::Serialize;
use std::sync::Arc;
use tauri::{Emitter, Manager, State};

pub struct AppState {
    pub store: Arc<dyn PetStateStore>,
    pub speech: Arc<dyn SpeechSink>,
    pub chat_provider: Arc<dyn ChatProvider>,
}

#[derive(Clone, Serialize)]
pub struct PetSnapshot {
    pub mood: String,
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

#[tauri::command]
fn get_state(state: State<'_, AppState>) -> PetSnapshot {
    snapshot(&state)
}

#[tauri::command]
fn pet_clicked(app: tauri::AppHandle, state: State<'_, AppState>) {
    state.store.on_interaction();
    notify_state(&app, &state);
}

#[tauri::command]
fn speak(app: tauri::AppHandle, state: State<'_, AppState>, text: String) {
    let _ = app.emit("pet://speech", text.clone());
    let _ = state.speech.speak(&text);
    notify_state(&app, &state);
}

/// 核心聊天逻辑：增加了 on_token 回调。
pub(crate) async fn run_chat(
    chat: &dyn ChatProvider,
    text: String,
    on_token: &(dyn for<'a> Fn(&'a str) + Send + Sync),
) -> Result<String, String> {
    // 定义桌宠的系统身份和回复规则
    let system_prompt = "你是一只名叫“芙芙”的可爱桌面宠物。你的性格活泼、温暖、俏皮。请用极简短的、口语化的中文回答主人的话，每次回复不要超过60个字。不要使用 Markdown 格式（如加粗、列表），直接输出纯文本对话。";

    let messages = vec![
        ChatMessage {
            role: Role::System,
            content: system_prompt.to_string(),
        },
        ChatMessage {
            role: Role::User,
            content: text,
        },
    ];
    chat.chat(&messages, on_token)
        .await
        .map_err(|e| format!("抱歉，我刚才走神了：{e}"))
}

#[tauri::command]
async fn chat(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    text: String,
) -> Result<String, String> {
    let _ = app.emit("pet://talking", true);
    let _ = app.emit("pet://chat_start", ()); // 通知前端清空气泡

    // 闭包必须捕获 app 进行实时 emit
    let app_clone = app.clone();
    let on_token = move |token: &str| {
        let _ = app_clone.emit("pet://chat_token", token);
    };
    let cb: &(dyn for<'a> Fn(&'a str) + Send + Sync) = &on_token;

    let result = run_chat(state.chat_provider.as_ref(), text, cb).await;

    match &result {
        Ok(reply) => {
            // 如果全部完成，同时发出完整语音事件
            let _ = app.emit("pet://speech", reply.clone());
            if let Err(e) = state.speech.speak(reply) {
                tracing::warn!("speak failed: {e}");
            }
            let _ = state.speech.stop();
        }
        Err(msg) => {
            let _ = app.emit("pet://chat_error", msg.clone());
        }
    }

    let _ = app.emit("pet://talking", false);
    result
}

pub fn run(
    store: Arc<dyn PetStateStore>,
    speech: Arc<dyn SpeechSink>,
    chat_provider: Arc<dyn ChatProvider>,
    is_mock_mode: bool,
) {
    tauri::Builder::default()
        .setup(move |app| {
            app.manage(AppState {
                store,
                speech,
                chat_provider,
            });
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_always_on_top(true);
            }
            let app_handle = app.handle().clone();
            if is_mock_mode {
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    let _ = app_handle.emit(
                        "pet://chat_error",
                        "当前为 mock 模式（请启动 llama-server）",
                    );
                });
            }
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
        let noop = |_: &str| {};
        let msg = run_chat(&FailingChat, "你好".into(), &noop)
            .await
            .unwrap_err();
        assert!(msg.contains("抱歉"), "got: {msg}");
        assert!(msg.contains("connection refused"), "got: {msg}");
    }
}
