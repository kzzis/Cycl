//! 年月("2026-07")の取得と加減算。フロント側はchronoを持たないので、
//! 現在時刻だけJSのDateから取り、残りは純粋な整数演算で済ませる。

const MONTH_NAMES: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// 現在のローカル年月を "YYYY-MM" で返す。
pub fn current() -> String {
    let now = js_sys::Date::new_0();
    format!(
        "{:04}-{:02}",
        now.get_full_year(),
        now.get_month() as i32 + 1
    )
}

fn parse(year_month: &str) -> (i32, i32) {
    let mut parts = year_month.split('-');
    let year = parts.next().and_then(|y| y.parse().ok()).unwrap_or(1970);
    let month = parts.next().and_then(|m| m.parse().ok()).unwrap_or(1);
    (year, month)
}

/// "YYYY-MM" を`months`ヶ月ずらす(負値で過去)。
pub fn shift(year_month: &str, months: i32) -> String {
    let (year, month) = parse(year_month);
    let total = year * 12 + (month - 1) + months;
    format!(
        "{:04}-{:02}",
        total.div_euclid(12),
        total.rem_euclid(12) + 1
    )
}

/// 表示用の短いラベル("2026-07" -> "Jul 2026")。
pub fn label(year_month: &str) -> String {
    let (year, month) = parse(year_month);
    let name = MONTH_NAMES
        .get((month - 1).clamp(0, 11) as usize)
        .copied()
        .unwrap_or("???");
    format!("{name} {year}")
}

/// 短縮ラベル("2026-07" -> "Jul")。積み上げグラフの軸用。
pub fn short_label(year_month: &str) -> String {
    let (_, month) = parse(year_month);
    MONTH_NAMES
        .get((month - 1).clamp(0, 11) as usize)
        .copied()
        .unwrap_or("???")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_moves_within_a_year() {
        assert_eq!(shift("2026-07", 1), "2026-08");
        assert_eq!(shift("2026-07", -2), "2026-05");
    }

    #[test]
    fn shift_crosses_year_boundaries() {
        assert_eq!(shift("2026-01", -1), "2025-12");
        assert_eq!(shift("2026-12", 1), "2027-01");
        assert_eq!(shift("2026-03", -6), "2025-09");
    }

    #[test]
    fn label_renders_a_readable_month() {
        assert_eq!(label("2026-07"), "Jul 2026");
        assert_eq!(short_label("2026-01"), "Jan");
    }
}
