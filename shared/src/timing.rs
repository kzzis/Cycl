use serde::{Deserialize, Serialize};

/// Todoを「いつやるか」で分類するタイミング。組み込み6種に加えユーザーが追加できる。
/// `todo.category` は `Timing::key` を指す。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Timing {
    pub id: i64,
    pub key: String,
    pub name: String,
    pub color: String,
    pub is_builtin: bool,
}

/// 新規Todoの既定タイミング(DBのDEFAULTと組み込みシードに一致させる)。
pub const DEFAULT_TIMING: &str = "someday";
