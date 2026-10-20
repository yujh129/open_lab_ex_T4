//! 宠物桌面应用的领域契约层。
//!
//! 本 crate 定义所有对外 trait 与领域类型，不包含任何 GUI、IO 或平台相关代码。
//! 上层 crate（pet-ai / pet-memory / pet-voice / pet-ui）只依赖本 crate 的抽象。
//!
//! 异步 trait 使用 `#[async_trait]`，以保证 `Arc<dyn ChatProvider>` 等 trait
//! object 在 dyn 分发下可编译。

pub mod domain;
pub mod error;
pub mod traits;

#[cfg(any(test, feature = "mocks"))]
pub mod mocks;

pub use domain::{
    ChatMessage, MemoryEntry, MemoryId, Mood, PetEvent, PetState, Position, Reminder,
    ReminderId, Role,
};
pub use error::{PetError, PetResult};
pub use traits::{ChatProvider, MemoryStore, ReminderScheduler, SpeechSink};
