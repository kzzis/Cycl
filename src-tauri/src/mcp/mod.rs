//! Cycl内蔵のMCPサーバー。Claude DesktopなどのMCPクライアントから
//! Todo・タイマー・分析データを操作できるようにする。
//!
//! セキュリティ上の前提:
//! - 待ち受けは127.0.0.1のみ(LANへは開かない)
//! - `Authorization: Bearer <token>`必須。トークンは起動ごとに生成する
//! - `Origin`が付いていればlocalhost系だけ許可(ブラウザ経由のDNSリバインディング対策)
//! - 既定は無効。設定画面のトグルで明示的に有効化したときだけポートを開く

pub mod protocol;
pub mod tools;

use crate::mcp::protocol::{initialize_result, Request, Response};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use hyper_util::rt::TokioIo;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::AppHandle;
use tokio::net::TcpListener;
use tower::Service;

pub const PORT: u16 = 3737;
/// 有効/無効を覚えておく設定キー。
pub const SETTING_KEY: &str = "mcp_enabled";
/// 停止フラグを見に戻る間隔。停止操作の反映がこの時間だけ遅れる。
const ACCEPT_POLL: Duration = Duration::from_millis(500);

#[derive(Clone)]
struct HandlerState {
    token: String,
    app: AppHandle,
}

/// MCPサーバーの起動状態。Tauriのmanaged stateとして持つ。
pub struct McpServer {
    token: String,
    running: Arc<AtomicBool>,
}

impl McpServer {
    pub fn new() -> Self {
        McpServer {
            token: random_token(),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// 待ち受けを開始する。すでに動いていれば何もしない。
    pub async fn start(&self, app: AppHandle) -> Result<(), String> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let listener = match TcpListener::bind(("127.0.0.1", PORT)).await {
            Ok(listener) => listener,
            Err(e) => {
                self.running.store(false, Ordering::SeqCst);
                return Err(format!("could not listen on 127.0.0.1:{PORT}: {e}"));
            }
        };

        let router = Router::new()
            .route("/mcp", post(handle))
            .with_state(HandlerState {
                token: self.token.clone(),
                app,
            });

        let running = self.running.clone();
        tauri::async_runtime::spawn(accept_loop(listener, running, router));
        Ok(())
    }

    /// 待ち受けを止める。acceptループが次にフラグを見た時点で終了し、ポートが解放される。
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

/// 16バイトの乱数を16進で。OSの乱数が読めない場合だけ時刻由来にフォールバックする。
fn random_token() -> String {
    let mut bytes = [0u8; 16];
    if getrandom::fill(&mut bytes).is_err() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        bytes.copy_from_slice(&nanos.to_le_bytes());
    }
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

async fn accept_loop(listener: TcpListener, running: Arc<AtomicBool>, router: Router) {
    while running.load(Ordering::SeqCst) {
        // acceptを待ち続けると停止に気づけないので、一定時間で切り上げてフラグを見る。
        match tokio::time::timeout(ACCEPT_POLL, listener.accept()).await {
            Ok(Ok((stream, _addr))) => {
                let router = router.clone();
                tauri::async_runtime::spawn(async move {
                    let io = TokioIo::new(stream);
                    let service = hyper::service::service_fn(move |req| {
                        let mut router = router.clone();
                        async move { router.call(req).await }
                    });
                    let _ = hyper::server::conn::http1::Builder::new()
                        .serve_connection(io, service)
                        .await;
                });
            }
            // listenerが壊れた場合は諦めて終了する。
            Ok(Err(_)) => break,
            Err(_timeout) => continue,
        }
    }
}

/// `Origin`が付いている場合はlocalhost由来だけ通す。
fn origin_allowed(headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get("origin").and_then(|v| v.to_str().ok()) else {
        // CLIやMCPクライアントからの直接呼び出しにはOriginが無い。
        return true;
    };
    let host = origin
        .split("://")
        .nth(1)
        .unwrap_or("")
        .split('/')
        .next()
        .unwrap_or("");
    let host = host.split(':').next().unwrap_or("");
    matches!(host, "localhost" | "127.0.0.1" | "[::1]" | "::1")
}

fn bearer_ok(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|token| token.trim() == expected)
        .unwrap_or(false)
}

async fn handle(
    State(state): State<HandlerState>,
    headers: HeaderMap,
    body: String,
) -> impl IntoResponse {
    if !origin_allowed(&headers) {
        return (StatusCode::FORBIDDEN, "origin not allowed").into_response();
    }
    if !bearer_ok(&headers, &state.token) {
        return (StatusCode::UNAUTHORIZED, "invalid or missing bearer token").into_response();
    }

    let request: Request = match serde_json::from_str(&body) {
        Ok(request) => request,
        Err(e) => {
            let error = Response::err(Value::Null, -32700, format!("parse error: {e}"));
            return (StatusCode::OK, Json(error)).into_response();
        }
    };

    // 通知(idなし)には本文を返さない。
    let Some(id) = request.id.clone() else {
        return StatusCode::ACCEPTED.into_response();
    };

    let response = match request.method.as_str() {
        "initialize" => {
            // 相手が話せる版に合わせる。合わせられなければこちらの最新を名乗る。
            let requested = request
                .params
                .get("protocolVersion")
                .and_then(|v| v.as_str());
            Response::ok(id, initialize_result(requested))
        }
        "ping" => Response::ok(id, json!({})),
        "tools/list" => Response::ok(id, json!({ "tools": tools::definitions() })),
        "tools/call" => {
            let name = request
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let args = request
                .params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let app = state.app.clone();
            // DBアクセスは同期なので、非同期ランタイムのスレッドを塞がないよう外へ出す。
            match tokio::task::spawn_blocking(move || tools::call(&app, &name, &args)).await {
                Ok(result) => Response::ok(id, result),
                Err(e) => Response::err(id, -32603, format!("tool panicked: {e}")),
            }
        }
        other => Response::err(id, -32601, format!("method not found: {other}")),
    };

    (StatusCode::OK, Json(response)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderName, HeaderValue};

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.insert(
                HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        headers
    }

    #[test]
    fn requests_without_an_origin_are_allowed() {
        // MCPクライアントからの直接呼び出しにはOriginが付かない。
        assert!(origin_allowed(&headers(&[])));
    }

    #[test]
    fn localhost_origins_are_allowed() {
        assert!(origin_allowed(&headers(&[(
            "origin",
            "http://localhost:5173"
        )])));
        assert!(origin_allowed(&headers(&[(
            "origin",
            "http://127.0.0.1:3737"
        )])));
    }

    #[test]
    fn remote_origins_are_rejected() {
        // DNSリバインディングで外部サイトから叩かれるのを防ぐ。
        assert!(!origin_allowed(&headers(&[(
            "origin",
            "http://evil.example.com"
        )])));
        assert!(!origin_allowed(&headers(&[(
            "origin",
            "https://localhost.evil.example.com"
        )])));
    }

    #[test]
    fn bearer_token_must_match_exactly() {
        let expected = "abc123";
        assert!(bearer_ok(
            &headers(&[("authorization", "Bearer abc123")]),
            expected
        ));
        assert!(!bearer_ok(
            &headers(&[("authorization", "Bearer nope")]),
            expected
        ));
        assert!(!bearer_ok(
            &headers(&[("authorization", "abc123")]),
            expected
        ));
        assert!(!bearer_ok(&headers(&[]), expected));
    }

    #[test]
    fn tokens_are_long_and_unique() {
        let a = random_token();
        let b = random_token();
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }
}
