use serde::{Deserialize, Serialize};

/// タグ別の作業時間集計(統計グラフ用)。タグ無しは name="Untagged"。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagFocus {
    pub name: String,
    pub color: String,
    pub secs: i64,
}

/// 月次サマリー。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlyStats {
    pub total_focus_secs: i64,
    pub completed_sessions: i64,
    /// 完了セッションあたりの平均中断回数。
    pub avg_interruptions: f64,
    /// その月に完了したタスクの平均予測精度スコア(0.0〜1.0)。記録が無ければ0。
    pub avg_accuracy: f64,
}

/// 予測精度ログ1件(見積もり vs 実績)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccuracyEntry {
    pub todo_title: String,
    pub estimated_count: i64,
    pub actual_count: i64,
    pub accuracy_score: f64,
    pub recorded_at: String,
}

/// 曜日×時間帯の集中度(ヒートマップ用)。weekday は 0=月曜 .. 6=日曜。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HourFocus {
    pub weekday: i64,
    pub hour: i64,
    pub secs: i64,
}

/// タグ別の月次サマリー。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagSummary {
    pub name: String,
    pub color: String,
    pub secs: i64,
    /// このタグが付いた完了タスクの平均実績ポモドーロ数。
    pub avg_pomodoros: f64,
    /// このタグが付いた完了タスクの平均予測精度スコア。
    pub avg_accuracy: f64,
}

/// 見積もりと実績の乖離から精度スコアを算出(0.0〜1.0)。
/// 一致: 1.0, 1個ずれ: 0.67, 2個ずれ: 0.5 と逓減する。
pub fn calc_accuracy_score(estimated: i64, actual: i64) -> f64 {
    let diff = (estimated - actual).unsigned_abs() as f64;
    (1.0 / (1.0 + diff * 0.5)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_estimate_scores_one() {
        assert_eq!(calc_accuracy_score(4, 4), 1.0);
    }

    #[test]
    fn score_decays_as_the_gap_widens() {
        let one_off = calc_accuracy_score(4, 5);
        let two_off = calc_accuracy_score(4, 6);
        assert!(one_off < 1.0 && two_off < one_off);
        assert_eq!(two_off, 0.5);
    }

    #[test]
    fn score_is_symmetric_and_stays_in_range() {
        assert_eq!(calc_accuracy_score(2, 5), calc_accuracy_score(5, 2));
        let far_off = calc_accuracy_score(1, 100);
        assert!(far_off > 0.0 && far_off < 0.05);
    }
}
