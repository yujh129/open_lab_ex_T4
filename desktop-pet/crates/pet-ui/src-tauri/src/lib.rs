//! Tauri 2 应用壳

use std::sync::Arc;
use pet_core::{ChatMessage, ChatProvider, PetState, PetStateStore, Role, SpeechSink};
use serde::Serialize;
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

pub(crate) async fn run_chat(chat: &dyn ChatProvider, text: String) -> Result<String, String> {
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
) {
    tauri::Builder::default()
        .setup(move |app| {
            app.manage(AppState {
                store,
                speech,
                chat_provider,
            });

            // 确保窗口始终置顶
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_always_on_top(true);
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
