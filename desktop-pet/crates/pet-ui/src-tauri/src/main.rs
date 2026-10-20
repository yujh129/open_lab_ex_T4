//! 二进制入口：阶段 2 用 pet-core 的 mocks，保证无 llama-server 也能跑通 UI。

use std::sync::Arc;

use pet_core::mocks::{InMemoryPetStateStore, MockChatProvider, RecordingSpeechSink};
use pet_core::{ChatProvider, PetStateStore, SpeechSink};

fn main() {
    let store: Arc<dyn PetStateStore> = Arc::new(InMemoryPetStateStore::new());
    let speech: Arc<dyn SpeechSink> = Arc::new(RecordingSpeechSink::new());

    let mock = MockChatProvider::new();
    mock.set_fallback("你好，我是你的桌面宠物～");
    let chat: Arc<dyn ChatProvider> = Arc::new(mock);

    pet_ui::run(store, speech, chat);
}
