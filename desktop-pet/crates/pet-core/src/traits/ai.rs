//! AI 对话抽象。

use async_trait::async_trait;

use crate::{ChatMessage, PetResult};

/// 对话引擎抽象。
///
/// 屏蔽 llama-server、mock 以及未来云端模型的差异。
///
/// `on_token` 的 trait object 类型**必须内联书写**，不能放进 `type` 别名：
/// 别名里 `dyn` 的隐式生命周期默认是 `'static`，会把回调闭包强制要求成
/// `'static`，导致借用局部变量（如 `Mutex<String>`）的闭包无法强转。
/// 内联时 `&'x (dyn ...)` 的 trait object 生命周期自动绑定到 `'x`。
///
/// 同时必须显式写 `for<'a>` HRTB：否则 `Fn(&str)` 的 `&str` 会被
/// `#[async_trait]` 展开后的方法生命周期（`'life3`）捕获，令内部
/// `String::split_inclusive` 产生的短借用无法满足。
///
/// 使用 `#[async_trait]` 而非原生 `async fn in trait`，是因为后者（Rust 1.75
/// 起稳定）不满足 dyn-compatibility，而本架构要求 `pet-ui` 持有
/// `Arc<dyn ChatProvider>` 并注入具体实现。
#[async_trait]
pub trait ChatProvider: Send + Sync {
    /// 发送对话历史并返回完整回复。
    ///
    /// `on_token` 在每个流式 token 到达时被同步调用；非流式实现可仅在结束时
    /// 调用一次。返回值为完整回复文本。
    async fn chat(
        &self,
        messages: &[ChatMessage],
        on_token: &(dyn for<'a> Fn(&'a str) + Send + Sync),
    ) -> PetResult<String>;
}
