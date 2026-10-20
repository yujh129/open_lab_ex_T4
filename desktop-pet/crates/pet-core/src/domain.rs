//! 领域类型定义：宠物状态、对话消息、记忆条目、提醒、行为事件。
//!
//! 本模块不引入任何 GUI 句柄、数据库连接或网络客户端。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 宠物情绪状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mood {
    /// 平静。
    Neutral,
    /// 高兴。
    Happy,
    /// 疲惫。
    Tired,
    /// 好奇。
    Curious,
    /// 困倦。
    Sleepy,
    /// 烦躁。
    Annoyed,
}

/// 屏幕坐标（逻辑像素）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    /// 水平坐标。
    pub x: f32,
    /// 垂直坐标。
    pub y: f32,
}

impl Default for Position {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// 宠物的领域状态，不含 GUI 或 IO 资源。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PetState {
    /// 当前情绪。
    pub mood: Mood,
    /// 当前屏幕位置。
    pub position: Position,
    /// 精力值，约定范围 0.0–1.0。
    pub energy: f32,
    /// 最近一次用户交互时间（UTC）。
    pub last_interaction_at: DateTime<Utc>,
}

/// 对话角色。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// 系统提示。
    System,
    /// 用户消息。
    User,
    /// 助手回复。
    Assistant,
}

/// 单条对话消息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    /// 发言角色。
    pub role: Role,
    /// 文本内容。
    pub content: String,
}

/// 记忆条目 ID。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MemoryId(pub Uuid);

impl MemoryId {
    /// 生成新的随机 ID。
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MemoryId {
    fn default() -> Self {
        Self::new()
    }
}

/// 一条持久化记忆。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// 条目 ID。
    pub id: MemoryId,
    /// 原始发言角色。
    pub role: Role,
    /// 文本内容。
    pub content: String,
    /// 写入时间（UTC）。
    pub created_at: DateTime<Utc>,
}

/// 提醒 ID。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReminderId(pub Uuid);

impl ReminderId {
    /// 生成新的随机 ID。
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ReminderId {
    fn default() -> Self {
        Self::new()
    }
}

/// 一条定时提醒。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reminder {
    /// 提醒 ID。
    pub id: ReminderId,
    /// 触发时间（UTC）。
    pub at: DateTime<Utc>,
    /// 提醒文本。
    pub message: String,
    /// 是否已触发。
    pub fired: bool,
}

/// 宠物行为事件，用于内核 → UI 的单向通知。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PetEvent {
    /// 宠物被点击。
    Clicked {
        /// 点击位置。
        at: Position,
    },
    /// 宠物被拖拽。
    Dragged {
        /// 起点。
        from: Position,
        /// 终点。
        to: Position,
    },
    /// 宠物说出了文本。
    Spoke {
        /// 说出的文本。
        text: String,
    },
    /// 宠物移动到新位置。
    Walked {
        /// 起点。
        from: Position,
        /// 终点。
        to: Position,
    },
    /// 情绪发生变化。
    MoodChanged {
        /// 旧情绪。
        from: Mood,
        /// 新情绪。
        to: Mood,
    },
    /// AI 回复到达。
    AiReplyArrived {
        /// 回复文本。
        text: String,
    },
}
