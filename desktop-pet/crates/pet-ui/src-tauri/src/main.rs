//! 二进制入口：用于单独运行 UI 壳进行测试。
//! 默认走 Mock 模式。生产环境入口请看 `apps/pet-app/src/main.rs`。

use std::sync::Arc;

use pet_core::mocks::{InMemoryPetStateStore, RecordingSpeechSink};
use pet_core::{ChatProvider, PetStateStore, SpeechSink};

fn main() {
    let store: Arc<dyn PetStateStore> = Arc::new(InMemoryPetStateStore::new());
    let speech: Arc<dyn SpeechSink> = Arc::new(RecordingSpeechSink::new());

    // 用 pet-ai 的 MockProvider
    let chat: Arc<dyn ChatProvider> = Arc::new(pet_ai::MockChatProvider::new(
        "你好，我是你的桌面宠物～(来自 pet-ui 调试入口)",
    ));

    // 第 4 个参数传 true，表示是 mock 模式
    pet_ui::run(store, speech, chat, true);
}
