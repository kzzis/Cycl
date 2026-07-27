use dioxus::prelude::*;
use shared::{format_focus, HourFocus, MonthlyStats};

use crate::month;
use crate::tauri_api::stats as api;

const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
/// 朝/昼/夜の区切り(時)。夜は18時〜翌4時。
const PARTS: [(&str, &str, i64, i64); 3] = [
    ("Morning", "5–11", 5, 11),
    ("Afternoon", "12–17", 12, 17),
    ("Evening", "18–4", 18, 4),
];

/// 月次サマリーと、曜日×時間帯の集中度ヒートマップ。
#[component]
pub fn StatsMonth() -> Element {
    let mut ym = use_signal(month::current);

    let mut summary = use_resource(move || {
        let m = ym.read().clone();
        async move { api::monthly(&m).await.ok() }
    });
    let mut hours = use_resource(move || {
        let m = ym.read().clone();
        async move { api::focus_hours(&m).await.unwrap_or_default() }
    });

    use_effect(move || {
        crate::tauri_api::listen::<()>("todos:changed", move |_| {
            summary.restart();
            hours.restart();
        });
    });

    let stats: MonthlyStats = summary
        .value()
        .read()
        .clone()
        .flatten()
        .unwrap_or(MonthlyStats {
            total_focus_secs: 0,
            completed_sessions: 0,
            avg_interruptions: 0.0,
            avg_accuracy: 0.0,
        });
    let cells: Vec<HourFocus> = hours.value().read().clone().unwrap_or_default();
    let peak = cells.iter().map(|c| c.secs).max().unwrap_or(0);

    // (曜日, 時)から秒を引くための索引。
    let lookup = |weekday: i64, hour: i64| -> i64 {
        cells
            .iter()
            .find(|c| c.weekday == weekday && c.hour == hour)
            .map(|c| c.secs)
            .unwrap_or(0)
    };
    // 0=空、1〜4=濃さ。単一色相の濃淡だけで大小を表す。
    let level = move |secs: i64| -> i64 {
        if secs == 0 || peak == 0 {
            return 0;
        }
        ((secs as f64 / peak as f64 * 4.0).ceil() as i64).clamp(1, 4)
    };

    let accuracy_label = if stats.avg_accuracy > 0.0 {
        format!("{:.2}", stats.avg_accuracy)
    } else {
        "—".to_string()
    };

    rsx! {
        div { class: "stats-view",
            div { class: "stats-filters",
                button {
                    class: "stats-nav__arrow",
                    aria_label: "Previous month",
                    onclick: move |_| {
                        let prev = month::shift(&ym.read(), -1);
                        ym.set(prev);
                    },
                    "‹"
                }
                span { class: "stats-filters__label", "{month::label(&ym.read())}" }
                button {
                    class: "stats-nav__arrow",
                    aria_label: "Next month",
                    onclick: move |_| {
                        let next = month::shift(&ym.read(), 1);
                        ym.set(next);
                    },
                    "›"
                }
            }

            div { class: "kpis",
                div { class: "kpi",
                    span { class: "kpi__label", "Focus time" }
                    span { class: "kpi__value", "{format_focus(stats.total_focus_secs)}" }
                }
                div { class: "kpi",
                    span { class: "kpi__label", "Sessions" }
                    span { class: "kpi__value", "{stats.completed_sessions}" }
                }
                div { class: "kpi",
                    span { class: "kpi__label", "Interruptions / session" }
                    span { class: "kpi__value", "{stats.avg_interruptions:.1}" }
                }
                div { class: "kpi",
                    span { class: "kpi__label", "Estimate accuracy" }
                    span { class: "kpi__value", "{accuracy_label}" }
                }
            }

            section { class: "stats-card",
                h3 { class: "stats-card__title", "When you focus" }
                if peak == 0 {
                    p { class: "muted", "No focus time recorded this month." }
                } else {
                    div { class: "heatmap",
                        for (row, day) in WEEKDAYS.iter().enumerate() {
                            div { class: "heatmap__row",
                                span { class: "heatmap__day", "{day}" }
                                for hour in 0..24_i64 {
                                    {
                                        let secs = lookup(row as i64, hour);
                                        rsx! {
                                            span {
                                                class: "heatmap__cell heatmap__cell--l{level(secs)}",
                                                title: "{day} {hour}:00 — {format_focus(secs)}",
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "heatmap__axis",
                            span { class: "heatmap__day" }
                            for hour in 0..24_i64 {
                                span { class: "heatmap__tick",
                                    if hour % 6 == 0 { "{hour}" }
                                }
                            }
                        }
                    }
                    div { class: "heatmap__scale",
                        span { class: "muted", "Less" }
                        for l in 0..5_i64 {
                            span { class: "heatmap__cell heatmap__cell--l{l}" }
                        }
                        span { class: "muted", "More" }
                    }
                    // 色に頼らず読めるよう、時間帯ごとの合計も並べる。
                    ul { class: "parts",
                        for (name, range, from, to) in PARTS {
                            {
                                let secs: i64 = cells
                                    .iter()
                                    .filter(|c| {
                                        if from <= to {
                                            c.hour >= from && c.hour <= to
                                        } else {
                                            c.hour >= from || c.hour <= to
                                        }
                                    })
                                    .map(|c| c.secs)
                                    .sum();
                                rsx! {
                                    li { class: "parts__row",
                                        span { class: "parts__name", "{name}" }
                                        span { class: "parts__range muted", "{range}" }
                                        span { class: "parts__value", "{format_focus(secs)}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
