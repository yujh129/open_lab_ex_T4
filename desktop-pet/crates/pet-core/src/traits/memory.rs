//! 持久记忆与提醒调度抽象。

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{MemoryEntry, PetResult, Reminder, Role};

/// 持久记忆存储抽象。
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// 写入一条记忆并返回落库后的条目。
    async fn remember(&self, role: Role, content: &str) -> PetResult<MemoryEntry>;

    /// 按查询串检索最近的若干条记忆，按时间倒序。
    async fn recall(&self, query: &str, limit: usize) -> PetResult<Vec<MemoryEntry>>;

    /// 返回最近的若干条记忆，按时间倒序。
    async fn all_recent(&self, limit: usize) -> PetResult<Vec<MemoryEntry>>;
}

/// 提醒调度抽象。
///
/// 同步接口：调度与轮询都是短操作，异步等待由调用方（pet-app 的 tokio
/// 定时器）负责。
pub trait ReminderScheduler: Send + Sync {
    /// 在 `at` 时刻安排一条提醒，返回落库后的条目。
    fn schedule(&self, at: DateTime<Utc>, message: String) -> PetResult<Reminder>;

    /// 取出所有已到期的提醒，并将其标记为已触发。
    fn poll_due(&self) -> PetResult<Vec<Reminder>>;
}
