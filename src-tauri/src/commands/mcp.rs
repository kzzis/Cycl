use crate::db::{setting_queries, AppState};
use crate::error::AppResult;
use crate::mcp::{McpServer, PORT, SETTING_KEY};
use serde::Serialize;
use tauri::{AppHandle, State};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStatus {
    pub enabled: bool,
    pub port: u16,
    /// 接続に必要なトークン。起動ごとに変わる。
    pub token: String,
    /// 直近の起動失敗の理由(ポート使用中など)。
    pub error: Option<String>,
}

#[tauri::command]
pub fn mcp_status(server: State<McpServer>) -> McpStatus {
    McpStatus {
        enabled: server.is_running(),
        port: PORT,
        token: server.token().to_string(),
        error: None,
    }
}

#[tauri::command]
pub async fn mcp_set_enabled(
    app: AppHandle,
    state: State<'_, AppState>,
    server: State<'_, McpServer>,
    enabled: bool,
) -> AppResult<McpStatus> {
    let mut error = None;
    if enabled {
        if let Err(message) = server.start(app.clone()).await {
            error = Some(message);
        }
    } else {
        server.stop();
    }

    // 実際に起動できた場合だけ有効として覚える。
    let running = server.is_running();
    {
        let conn = state.db.lock().unwrap();
        setting_queries::set_bool(&conn, SETTING_KEY, running)?;
    }

    Ok(McpStatus {
        enabled: running,
        port: PORT,
        token: server.token().to_string(),
        error,
    })
}
