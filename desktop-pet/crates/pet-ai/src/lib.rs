//! pet-ai: 对接 llama-server 的 OpenAI 兼容接口及 Mock 实现。

use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use pet_core::{ChatMessage, ChatProvider, PetError, PetResult};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

pub mod sse;

/// llama-server 的 ChatProvider 实现。
pub struct LlamaServerChatProvider {
    port: u16,
    client: Client,
}

impl LlamaServerChatProvider {
    /// 创建新的 Provider，超时时间固定 120s。
    pub fn new(port: u16) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap_or_default();
        Self { port, client }
    }

    /// 从环境变量 `LLAMA_SERVER_PORT` 创建，默认 9931。
    pub fn from_env() -> Self {
        let port = std::env::var("LLAMA_SERVER_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(9931);
        Self::new(port)
    }
}

#[async_trait]
impl ChatProvider for LlamaServerChatProvider {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        on_token: &(dyn for<'a> Fn(&'a str) + Send + Sync),
    ) -> PetResult<String> {
        let url = format!("http://127.0.0.1:{}/v1/chat/completions", self.port);

        let req_messages: Vec<_> = messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    pet_core::Role::System => "system",
                    pet_core::Role::User => "user",
                    pet_core::Role::Assistant => "assistant",
                };
                json!({ "role": role, "content": m.content })
            })
            .collect();

        let body = json!({
            "model": "default",
            "messages": req_messages,
            "stream": true,
            "max_tokens": 1024
        });

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| PetError::Ai(format!("请确认 llama-server 已启动在 {}。错误详情：{}", self.port, e)))?;

        if !resp.status().is_success() {
            return Err(PetError::Ai(format!(
                "llama-server 返回 HTTP 错误: {}",
                resp.status()
            )));
        }

        let mut stream = resp.bytes_stream().eventsource();
        let mut full_text = String::new();

        while let Some(event) = stream.next().await {
            match event {
                Ok(ev) => {
                    // eventsource-stream 已经剥离了 "data: " 前缀，直接传 ev.data
                    match sse::parse_sse_line(&ev.data) {
                        Ok(Some(text)) => {
                            on_token(&text);
                            full_text.push_str(&text);
                        }
                        Err(_) => break, // 遇到 [DONE]
                        _ => {}
                    }
                }
                Err(e) => {
                    tracing::warn!("SSE stream error: {}", e);
                    break;
                }
            }
        }

        Ok(full_text)
    }
}

/// 包含多段流式输出的 MockProvider，用于无 llama-server 时的 CI/测试。
pub struct MockChatProvider {
    reply: String,
}

impl MockChatProvider {
    /// 使用固定回复创建 Mock。
    pub fn new(reply: impl Into<String>) -> Self {
        Self {
            reply: reply.into(),
        }
    }
}

#[async_trait]
impl ChatProvider for MockChatProvider {
    async fn chat(
        &self,
        _messages: &[ChatMessage],
        on_token: &(dyn for<'a> Fn(&'a str) + Send + Sync),
    ) -> PetResult<String> {
        // 将回复切成多个单字符块，每块间隔 10ms 模拟真实流式
        let tokens: Vec<char> = self.reply.chars().collect();
        let mut full_text = String::new();

        for c in tokens {
            let s = c.to_string();
            on_token(&s);
            full_text.push_str(&s);
            sleep(Duration::from_millis(10)).await;
        }

        Ok(full_text)
    }
}
