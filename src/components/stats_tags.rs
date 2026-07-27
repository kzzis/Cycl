use dioxus::prelude::*;
use shared::{format_focus, TagSummary};

use crate::month;
use crate::tauri_api::stats as api;

/// 積み上げグラフに並べる月数。
const MONTHS: i32 = 6;

/// タグ別の月次積み上げ棒グラフと、直近月のタグ別実績。
#[component]
pub fn StatsTags() -> Element {
    let mut data = use_resource(move || async move {
        let now = month::current();
        let mut months: Vec<(String, Vec<TagSummary>)> = Vec::new();
        for back in (0..MONTHS).rev() {
            let ym = month::shift(&now, -back);
            let tags = api::tag_summary(&ym).await.unwrap_or_default();
            months.push((ym, tags));
        }
        months
    });

    use_effect(move || {
        crate::tauri_api::listen::<()>("todos:changed", move |_| data.restart());
    });

    let months: Vec<(String, Vec<TagSummary>)> = data.value().read().clone().unwrap_or_default();
    let peak = months
        .iter()
        .map(|(_, tags)| tags.iter().map(|t| t.secs).sum::<i64>())
        .max()
        .unwrap_or(0);

    if peak == 0 {
        return rsx! {
            div { class: "stats-view",
                p { class: "muted stats__empty", "No tagged focus time in the last 6 months." }
            }
        };
    }

    // 凡例は期間中に登場したタグをまとめて出す(色はタグ自身の色)。
    let mut legend: Vec<TagSummary> = Vec::new();
    for (_, tags) in &months {
        for tag in tags {
            if !legend.iter().any(|l| l.name == tag.name) {
                legend.push(tag.clone());
            }
        }
    }

    let latest = months.last().cloned().unwrap_or_default();

    rsx! {
        div { class: "stats-view",
            section { class: "stats-card",
                h3 { class: "stats-card__title", "Focus by tag, last {MONTHS} months" }
                div { class: "mstack",
                    for (ym, tags) in months.iter().cloned() {
                        {
                            let total: i64 = tags.iter().map(|t| t.secs).sum();
                            let col_pct = total as f64 / peak as f64 * 100.0;
                            rsx! {
                                div { class: "mstack__slot",
                                    div { class: "mstack__plot",
                                        div {
                                            class: "mstack__col",
                                            style: "height: {col_pct}%",
                                            title: "{month::label(&ym)} — {format_focus(total)}",
                                            for tag in tags.iter().cloned() {
                                                div {
                                                    class: "mstack__seg",
                                                    style: "height: {tag.secs as f64 / total.max(1) as f64 * 100.0}%; background-color: {tag.color}",
                                                }
                                            }
                                        }
                                    }
                                    span { class: "mstack__label", "{month::short_label(&ym)}" }
                                }
                            }
                        }
                    }
                }
                ul { class: "stats-legend",
                    for tag in legend.iter().cloned() {
                        li { class: "stats-legend-row",
                            span {
                                class: "stats-legend-dot",
                                style: "background-color: {tag.color}",
                            }
                            span { class: "stats-legend-name", "{tag.name}" }
                        }
                    }
                }
            }

            section { class: "stats-card",
                h3 { class: "stats-card__title", "This month by tag" }
                if latest.1.is_empty() {
                    p { class: "muted", "Nothing recorded this month yet." }
                } else {
                    table { class: "stats-table",
                        thead {
                            tr {
                                th { "Tag" }
                                th { "Focus" }
                                th { "Avg 🍅" }
                                th { "Accuracy" }
                            }
                        }
                        tbody {
                            for tag in latest.1.iter().cloned() {
                                tr {
                                    td {
                                        span {
                                            class: "stats-legend-dot",
                                            style: "background-color: {tag.color}",
                                        }
                                        "{tag.name}"
                                    }
                                    td { "{format_focus(tag.secs)}" }
                                    td {
                                        if tag.avg_pomodoros > 0.0 { "{tag.avg_pomodoros:.1}" } else { "—" }
                                    }
                                    td {
                                        if tag.avg_accuracy > 0.0 { "{tag.avg_accuracy:.2}" } else { "—" }
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
