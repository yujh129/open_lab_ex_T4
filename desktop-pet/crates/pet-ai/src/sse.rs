//! SSE 流解析逻辑及单元测试。

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ChatCompletionChunk {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    delta: Delta,
}

#[derive(Debug, Deserialize)]
struct Delta {
    content: Option<String>,
}

/// 处理单行 SSE 数据。
/// 返回值：
/// - `Ok(Some(text))`：成功提取到 token
/// - `Ok(None)`：合法但无文本（如首个包含 role 的 chunk）
/// - `Err(())`：遇到 `[DONE]` 或流结束
pub fn parse_sse_line(data: &str) -> Result<Option<String>, ()> {
    let data = data.trim();
    if data.is_empty() {
        return Ok(None);
    }

    let json_str = if let Some(stripped) = data.strip_prefix("data: ") {
        stripped
    } else {
        data
    };

    if json_str == "[DONE]" {
        return Err(()); 
    }

    match serde_json::from_str::<ChatCompletionChunk>(json_str) {
        Ok(chunk) => {
            if let Some(choice) = chunk.choices.first() {
                if let Some(content) = &choice.delta.content {
                    return Ok(Some(content.clone()));
                }
            }
            Ok(None)
        }
        Err(_) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_line() {
        let normal = r#"data: {"choices":[{"delta":{"content":"你好"}}]}"#;
        assert_eq!(parse_sse_line(normal).unwrap(), Some("你好".to_string()));

        let no_prefix = r#"{"choices":[{"delta":{"content":"世界"}}]}"#;
        assert_eq!(parse_sse_line(no_prefix).unwrap(), Some("世界".to_string()));

        let role_only = r#"data: {"choices":[{"delta":{"role":"assistant"}}]}"#;
        assert_eq!(parse_sse_line(role_only).unwrap(), None);

        let done = "data: [DONE]";
        assert!(parse_sse_line(done).is_err());

        let invalid = r#"data: {"invalid_json": "#;
        assert_eq!(parse_sse_line(invalid).unwrap(), None);
    }

    #[test]
    fn test_special_chars_and_newlines() {
        // token 包含换行符 \n 和双引号 \"
        let input = r#"data: {"choices":[{"delta":{"content":"第一行\n第二行 \"带引号\""}}]}"#;
        let parsed = parse_sse_line(input).unwrap().unwrap();
        assert_eq!(parsed, "第一行\n第二行 \"带引号\"");
    }

    #[test]
    fn test_chunked_bytes_concat() {
        // 模拟 SSE 在分块字节流中被切断（事件源底层处理了 chunk 拼接）
        let chunks = vec![
            "data: {\"choices\":[{\"delta\":{\"content\":\"你\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"好\"}}]}\n\n",
            "data: [DONE]\n\n",
        ];

        let mut result = String::new();
        for chunk in chunks {
            let mut ended = false;
            for line in chunk.lines() {
                match parse_sse_line(line) {
                    Ok(Some(text)) => result.push_str(&text),
                    Err(_) => {
                        ended = true;
                        break;
                    }
                    _ => {}
                }
            }
            if ended { break; }
        }
        assert_eq!(result, "你好");
    }
}
