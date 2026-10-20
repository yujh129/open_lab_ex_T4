//! 所有对外 trait 定义。

pub mod ai;
pub mod memory;
pub mod state;
pub mod voice;

pub use ai::ChatProvider;
pub use memory::{MemoryStore, ReminderScheduler};
pub use state::PetStateStore;
pub use voice::SpeechSink;
