//! MCP(Model Context Protocol)のうち、ツール提供サーバーに必要な最小限。
//! トランスポートはJSON-RPC 2.0 over HTTP。

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// クライアントが版を送ってこない/こちらが知らない版だったときに名乗る版。
pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// 話せるMCPのバージョン。新しい順。
///
/// このサーバーはPOST単発のJSON応答しか返さないが、それはStreamable HTTPでも
/// 認められた応答形なので2025系とも握手できる。SSEストリームとセッションIDは
/// どちらも任意で、`GET /mcp`にルートを張っていない結果405が返るのも仕様どおり。
pub const SUPPORTED_VERSIONS: &[&str] = &[PROTOCOL_VERSION, "2025-03-26", "2024-11-05"];

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

/// クライアントが要求した版を、こちらが話せるならそのまま返す。
/// 知らない版(将来の改訂など)ならこちらの最新を返し、続けるかはクライアントに委ねる。
pub fn negotiate_version(requested: Option<&str>) -> &'static str {
    requested
        .and_then(|requested| {
            SUPPORTED_VERSIONS
                .iter()
                .find(|version| **version == requested)
                .copied()
        })
        .unwrap_or(PROTOCOL_VERSION)
}

/// `initialize`への応答。`requested`は`params.protocolVersion`。
pub fn initialize_result(requested: Option<&str>) -> Value {
    json!({
        "protocolVersion": negotiate_version(requested),
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
        let result = initialize_result(None);
        assert_eq!(result["protocolVersion"], PROTOCOL_VERSION);
        assert!(result["capabilities"]["tools"].is_object());
        assert_eq!(result["serverInfo"]["name"], "cycl");
    }

    #[test]
    fn known_versions_are_echoed_back() {
        // 相手が話せる版に合わせる。古いクライアントを切らないための肝。
        for version in SUPPORTED_VERSIONS {
            assert_eq!(negotiate_version(Some(version)), *version);
            assert_eq!(
                initialize_result(Some(version))["protocolVersion"],
                *version
            );
        }
    }

    #[test]
    fn unknown_versions_fall_back_to_ours() {
        // 未知の新しい改訂。こちらの最新を名乗り、続けるかは相手に任せる。
        assert_eq!(negotiate_version(Some("2027-01-01")), PROTOCOL_VERSION);
        // 版を送ってこないクライアントも同じ扱い。
        assert_eq!(negotiate_version(None), PROTOCOL_VERSION);
    }

    #[test]
    fn the_default_version_is_the_newest_we_speak() {
        assert_eq!(SUPPORTED_VERSIONS[0], PROTOCOL_VERSION);
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
