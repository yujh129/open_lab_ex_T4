//! 宠物领域状态存储抽象。

use crate::PetState;

/// 宠物状态存储抽象：所有状态变更由后端驱动，前端只读快照。
pub trait PetStateStore: Send + Sync {
    /// 返回当前状态快照。
    fn current(&self) -> PetState;

    /// 记录一次用户交互（点击 / 拖拽结束）。
    fn on_interaction(&self);
}
