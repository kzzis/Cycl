// Phase 3のタイマーエンジンから呼ばれるまではTauriコマンドとして未公開・未使用
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PomodoroSession {
    pub id: i64,
    pub todo_id: i64,
    pub started_at: String,
    pub completed: bool,
}

impl PomodoroSession {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(PomodoroSession {
            id: row.get("id")?,
            todo_id: row.get("todo_id")?,
            started_at: row.get("started_at")?,
            completed: row.get::<_, i64>("completed")? != 0,
        })
    }
}
