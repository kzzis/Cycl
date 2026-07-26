use serde::{Deserialize, Serialize};

/// タグ別の作業時間集計(統計グラフ用)。タグ無しは name="Untagged"。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagFocus {
    pub name: String,
    pub color: String,
    pub secs: i64,
}
