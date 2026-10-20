//! 无外部依赖的 Mock 实现，供下游 crate 在开发 / 测试时使用。
//!
//! 仅在 `feature = "mocks"` 或 crate 内部测试时编译。

use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{
    ChatMessage, ChatProvider, MemoryEntry, MemoryId, MemoryStore, PetResult, Reminder,
    ReminderId, ReminderScheduler, Role, SpeechSink,
};

/// 可编程的对话引擎 Mock。
pub struct MockChatProvider {
    responses: Mutex<Vec<String>>,
    fallback: Mutex<String>,
    calls: Mutex<Vec<Vec<ChatMessage>>>,
}

impl MockChatProvider {
    /// 创建 Mock，默认 fallback 为 `"ok"`。
    pub fn new() -> Self {
        Self {
            responses: Mutex::new(Vec::new()),
            fallback: Mutex::new("ok".to_string()),
            calls: Mutex::new(Vec::new()),
        }
    }

    /// 追加一条预设回复，按 FIFO 消费。
    pub fn push_response(&self, text: impl Into<String>) {
        self.responses.lock().unwrap().push(text.into());
    }

    /// 设置响应队列耗尽时的固定回复。
    pub fn set_fallback(&self, text: impl Into<String>) {
        *self.fallback.lock().unwrap() = text.into();
    }

    /// 返回到目前为止 `chat` 收到的所有消息快照。
    pub fn calls(&self) -> Vec<Vec<ChatMessage>> {
        self.calls.lock().unwrap().clone()
    }
}

impl Default for MockChatProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChatProvider for MockChatProvider {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        on_token: &(dyn for<'a> Fn(&'a str) + Send + Sync),
    ) -> PetResult<String> {
        self.calls.lock().unwrap().push(messages.to_vec());
        let text = {
            let mut q = self.responses.lock().unwrap();
            if q.is_empty() {
                self.fallback.lock().unwrap().clone()
            } else {
                q.remove(0)
            }
        };
        // 按空白切分，模拟流式 token。`text` 的局部借用只活到循环结束，
        // 因为 `for<'a>` 让回调不再捕获具体生命周期。
        for tok in text.split_inclusive(char::is_whitespace) {
            on_token(tok);
        }
        Ok(text)
    }
}

/// 内存版 [`MemoryStore`]。
pub struct InMemoryMemoryStore {
    entries: Mutex<Vec<MemoryEntry>>,
}

impl InMemoryMemoryStore {
    /// 创建空存储。
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
        }
    }

    /// 返回当前条目数。
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }
}

impl Default for InMemoryMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MemoryStore for InMemoryMemoryStore {
    async fn remember(&self, role: Role, content: &str) -> PetResult<MemoryEntry> {
        let entry = MemoryEntry {
            id: MemoryId::new(),
            role,
            content: content.to_string(),
            created_at: Utc::now(),
        };
        self.entries.lock().unwrap().push(entry.clone());
        Ok(entry)
    }

    async fn recall(&self, query: &str, limit: usize) -> PetResult<Vec<MemoryEntry>> {
        let q = query.to_lowercase();
        let mut out: Vec<_> = self
            .entries
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.content.to_lowercase().contains(&q))
            .cloned()
            .collect();
        out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        out.truncate(limit);
        Ok(out)
    }

    async fn all_recent(&self, limit: usize) -> PetResult<Vec<MemoryEntry>> {
        let mut out: Vec<_> = self.entries.lock().unwrap().clone();
        out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        out.truncate(limit);
        Ok(out)
    }
}

/// 记录型 [`SpeechSink`]，不真正发声。
pub struct RecordingSpeechSink {
    spoken: Mutex<Vec<String>>,
    speaking: Mutex<bool>,
}

impl RecordingSpeechSink {
    /// 创建新实例。
    pub fn new() -> Self {
        Self {
            spoken: Mutex::new(Vec::new()),
            speaking: Mutex::new(false),
        }
    }

    /// 返回所有被播报过的文本快照。
    pub fn spoken(&self) -> Vec<String> {
        self.spoken.lock().unwrap().clone()
    }

    /// 返回当前是否处于“播放中”状态。
    pub fn is_speaking(&self) -> bool {
        *self.speaking.lock().unwrap()
    }
}

impl Default for RecordingSpeechSink {
    fn default() -> Self {
        Self::new()
    }
}

impl SpeechSink for RecordingSpeechSink {
    fn speak(&self, text: &str) -> PetResult<()> {
        self.spoken.lock().unwrap().push(text.to_string());
        *self.speaking.lock().unwrap() = true;
        Ok(())
    }

    fn stop(&self) -> PetResult<()> {
        *self.speaking.lock().unwrap() = false;
        Ok(())
    }
}

/// 内存版 [`ReminderScheduler`]。
pub struct InMemoryReminderScheduler {
    reminders: Mutex<Vec<Reminder>>,
}

impl InMemoryReminderScheduler {
    /// 创建空调度器。
    pub fn new() -> Self {
        Self {
            reminders: Mutex::new(Vec::new()),
        }
    }

    /// 返回尚未触发的提醒数量。
    pub fn pending(&self) -> usize {
        self.reminders
            .lock()
            .unwrap()
            .iter()
            .filter(|r| !r.fired)
            .count()
    }
}

impl Default for InMemoryReminderScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl ReminderScheduler for InMemoryReminderScheduler {
    fn schedule(&self, at: DateTime<Utc>, message: String) -> PetResult<Reminder> {
        let r = Reminder {
            id: ReminderId::new(),
            at,
            message,
            fired: false,
        };
        self.reminders.lock().unwrap().push(r.clone());
        Ok(r)
    }

    fn poll_due(&self) -> PetResult<Vec<Reminder>> {
        let now = Utc::now();
        let mut due = Vec::new();
        let mut guard = self.reminders.lock().unwrap();
        for r in guard.iter_mut() {
            if !r.fired && r.at <= now {
                r.fired = true;
                due.push(r.clone());
            }
        }
        Ok(due)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[tokio::test]
    async fn chat_streams_tokens_and_records_calls() {
        let mock = MockChatProvider::new();
        mock.push_response("hello world");
        // 闭包按值借用 `collected`（&Mutex<String> 是 Send + Sync），
        // 因为 on_token 是内联 dyn 类型，闭包不需 'static。
        let collected = Mutex::new(String::new());
        let cb = |tok: &str| collected.lock().unwrap().push_str(tok);
        let out = mock
            .chat(
                &[ChatMessage {
                    role: Role::User,
                    content: "hi".into(),
                }],
                &cb,
            )
            .await
            .unwrap();
        assert_eq!(out, "hello world");
        assert_eq!(*collected.lock().unwrap(), "hello world");
        assert_eq!(mock.calls().len(), 1);
    }

    #[tokio::test]
    async fn memory_recall_filters_by_substring() {
        let store = InMemoryMemoryStore::new();
        store.remember(Role::User, "I like cats").await.unwrap();
        store.remember(Role::User, "I dislike rain").await.unwrap();
        let hits = store.recall("cat", 10).await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(store.all_recent(10).await.unwrap().len(), 2);
    }

    #[test]
    fn speech_records_and_stops() {
        let sink = RecordingSpeechSink::new();
        sink.speak("hi").unwrap();
        assert!(sink.is_speaking());
        assert_eq!(sink.spoken(), vec!["hi".to_string()]);
        sink.stop().unwrap();
        assert!(!sink.is_speaking());
    }

    #[test]
    fn reminder_poll_returns_due_and_marks_fired() {
        let sched = InMemoryReminderScheduler::new();
        sched
            .schedule(Utc::now() - Duration::seconds(1), "past".into())
            .unwrap();
        sched
            .schedule(Utc::now() + Duration::hours(1), "future".into())
            .unwrap();
        let due = sched.poll_due().unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].message, "past");
        assert_eq!(sched.pending(), 1);
    }
}
