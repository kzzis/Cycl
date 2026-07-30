//! MCP(Model Context Protocol)のうち、ツール提供サーバーに必要な最小限。
//! トランスポートはJSON-RPC 2.0 over HTTP。

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// 実装しているMCPのバージョン。
pub const PROTOCOL_VERSION: &str = "2024-11-05";

#[derive(Debug, Deserialize)]
pub struct Request {
    #[allow(dead_code)]
    pub jsonrpc: Option<String>,
    /// 通知(notification)にはidが無い。その場合レスポンスを返さない。
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct Response {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorObject>,
}

#[derive(Debug, Serialize)]
pub struct ErrorObject {
    pub code: i32,
    pub message: String,
}

impl Response {
    pub fn ok(id: Value, result: Value) -> Self {
        Response {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn err(id: Value, code: i32, message: impl Into<String>) -> Self {
        Response {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(ErrorObject {
                code,
                message: message.into(),
            }),
        }
    }
}

/// `initialize`への応答。
pub fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "cycl", "version": env!("CARGO_PKG_VERSION") },
    })
}

/// `tools/call`の成功応答(テキスト1件)。
pub fn text_result(text: impl Into<String>) -> Value {
    json!({ "content": [{ "type": "text", "text": text.into() }] })
}

/// `tools/call`の失敗応答。プロトコル上はエラーではなく`isError`で返す。
pub fn tool_error(text: impl Into<String>) -> Value {
    json!({
        "content": [{ "type": "text", "text": text.into() }],
        "isError": true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_advertises_tools_capability() {
        let result = initialize_result();
        assert_eq!(result["protocolVersion"], PROTOCOL_VERSION);
        assert!(result["capabilities"]["tools"].is_object());
        assert_eq!(result["serverInfo"]["name"], "cycl");
    }

    #[test]
    fn notifications_have_no_id() {
        let notification: Request =
            serde_json::from_str(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
                .unwrap();
        assert!(notification.id.is_none());
        assert_eq!(notification.method, "notifications/initialized");
    }

    #[test]
    fn responses_carry_either_a_result_or_an_error() {
        let ok = serde_json::to_value(Response::ok(json!(1), json!({"a": 1}))).unwrap();
        assert_eq!(ok["jsonrpc"], "2.0");
        assert!(ok.get("error").is_none());

        let err = serde_json::to_value(Response::err(json!(1), -32601, "nope")).unwrap();
        assert!(err.get("result").is_none());
        assert_eq!(err["error"]["code"], -32601);
    }

    #[test]
    fn tool_errors_are_flagged_in_the_payload() {
        // MCPではツール失敗はJSON-RPCエラーではなくisErrorで返す。
        let ok = text_result("done");
        assert!(ok.get("isError").is_none());
        assert_eq!(ok["content"][0]["type"], "text");

        let bad = tool_error("boom");
        assert_eq!(bad["isError"], true);
        assert_eq!(bad["content"][0]["text"], "boom");
    }
}
