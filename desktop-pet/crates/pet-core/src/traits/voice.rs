//! 语音播报抽象。

use crate::PetResult;

/// 语音输出抽象。
///
/// 同步接口：实现内部将文本交给系统 TTS 后立即返回；播放生命周期由实现
/// 自行维护。
pub trait SpeechSink: Send + Sync {
    /// 播报文本。若正在播报，行为由实现定义（排队或打断）。
    fn speak(&self, text: &str) -> PetResult<()>;

    /// 停止当前播报并清空队列。
    fn stop(&self) -> PetResult<()>;
}
