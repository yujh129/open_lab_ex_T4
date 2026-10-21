//! 组合根：唯一知道“用什么实现”的 crate。
use std::sync::Arc;
use std::time::Duration;
use pet_core::mocks::{InMemoryPetStateStore, RecordingSpeechSink};
use pet_core::{ChatProvider, PetStateStore, SpeechSink};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 强制指定 GTK 输入法模块（根据你的环境二选一，fcitx 或 ibus）
    if std::env::var("GTK_IM_MODULE").is_err() {
        std::env::set_var("GTK_IM_MODULE", "fcitx"); 
        std::env::set_var("QT_IM_MODULE", "fcitx");
        std::env::set_var("XMODIFIERS", "@im=fcitx");
    }

    tracing_subscriber::fmt().init();

    let port: u16 = std::env::var("LLAMA_SERVER_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(9931);

    // 直接使用 tokio 的 timeout
    let is_mock_mode = tokio::time::timeout(
        Duration::from_millis(500),
        TcpStream::connect(format!("127.0.0.1:{}", port)),
    )
    .await
    .is_err();

    let chat_provider: Arc<dyn ChatProvider> = if is_mock_mode {
        tracing::warn!("llama-server 不可达，降级为 Mock 模式");
        Arc::new(pet_ai::MockChatProvider::new(
            "当前处于 Mock 模式，请启动 llama-server。",
        ))
    } else {
        tracing::info!("成功连接 llama-server，使用真实 AI");
        Arc::new(pet_ai::LlamaServerChatProvider::new(port))
    };

    let store: Arc<dyn PetStateStore> = Arc::new(InMemoryPetStateStore::new());
    let speech: Arc<dyn SpeechSink> = Arc::new(RecordingSpeechSink::new());

    // run 内部会阻塞进入 Tauri 的事件循环
    pet_ui::run(store, speech, chat_provider, is_mock_mode);
    Ok(())
}
