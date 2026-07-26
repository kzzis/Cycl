//! タイミングカテゴリ(Todoを「いつやるか」で分類する軸)の定義。
//! DBには `category` カラムに文字列で保存される。

/// (値, 表示名) の一覧。フロントのタブ表示・セレクトの選択肢に使う。
pub const CATEGORIES: &[(&str, &str)] = &[
    ("today", "Today"),
    ("tomorrow", "Tomorrow"),
    ("this_week", "This Week"),
    ("planned", "Planned"),
    ("someday", "Someday"),
    ("event", "Event"),
];

/// 新規Todoの既定カテゴリ(DBのDEFAULTと一致させる)。
pub const DEFAULT_CATEGORY: &str = "someday";

/// カテゴリ値の表示名を返す。未知の値はそのまま返す。
pub fn category_label(value: &str) -> &str {
    CATEGORIES
        .iter()
        .find(|(v, _)| *v == value)
        .map(|(_, label)| *label)
        .unwrap_or(value)
}
