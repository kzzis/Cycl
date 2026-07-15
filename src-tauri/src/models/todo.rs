use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Todo {
    pub id: i64,
    pub title: String,
    pub is_completed: bool,
    pub pomodoro_count: i64,
    pub target_count: Option<i64>,
    pub is_active: bool,
    pub created_at: String,
}

impl Todo {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Todo {
            id: row.get("id")?,
            title: row.get("title")?,
            is_completed: row.get::<_, i64>("is_completed")? != 0,
            pomodoro_count: row.get("pomodoro_count")?,
            target_count: row.get("target_count")?,
            is_active: row.get::<_, i64>("is_active")? != 0,
            created_at: row.get("created_at")?,
        })
    }
}
