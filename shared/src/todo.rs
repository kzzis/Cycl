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
