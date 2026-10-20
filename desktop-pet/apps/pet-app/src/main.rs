//! 组合根：唯一知道“用什么实现”的 crate。
//!
//! 阶段 2 用 `pet-core::mocks` 里的内存实现；阶段 3 在此替换为
//! `pet_ai::LlamaChatEngine` / `pet_memory::SqlitePetStateStore` /
//! `pet_voice::SystemTts`，`pet-ui` 不需要改动。

use std::sync::Arc;

use pet_core::mocks::{InMemoryPetStateStore, MockChatProvider, RecordingSpeechSink};
use pet_core::{ChatProvider, PetStateStore, SpeechSink};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let store: Arc<dyn PetStateStore> = Arc::new(InMemoryPetStateStore::new());
    let speech: Arc<dyn SpeechSink> = Arc::new(RecordingSpeechSink::new());

    let mock = MockChatProvider::new();
    mock.set_fallback("你好，我是你的桌面宠物～");
    let chat: Arc<dyn ChatProvider> = Arc::new(mock);

    tracing::info!("starting desktop-pet");
    pet_ui::run(store, speech, chat);
    Ok(())
}
