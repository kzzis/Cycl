use dioxus::prelude::*;
use shared::{format_focus, TagFocus};

use crate::tauri_api::stats as api;

const RADIUS: f64 = 80.0;
const STROKE: f64 = 28.0;

#[component]
pub fn StatsFocus() -> Element {
    let mut period = use_signal(|| "week".to_string());
    let mut data = use_resource(move || {
        let p = period.read().clone();
        async move { api::focus_by_tag(&p).await.unwrap_or_default() }
    });

    // 作業記録が更新されたらグラフを取り直す。
    use_effect(move || {
        crate::tauri_api::listen::<()>("todos:changed", move |_| data.restart());
    });

    let items: Vec<TagFocus> = data.value().read().clone().unwrap_or_default();
    let total: i64 = items.iter().map(|t| t.secs).sum();

    let active = period.read().clone();
    let tab = |value: &'static str, label: &'static str| {
        let is_active = active == value;
        rsx! {
            button {
                class: if is_active { "stats-tab stats-tab--active" } else { "stats-tab" },
                onclick: move |_| period.set(value.to_string()),
                "{label}"
            }
        }
    };

    rsx! {
        div { class: "stats-view",
            div { class: "stats-filters",
                {tab("week", "Week")}
                {tab("month", "Month")}
                {tab("year", "Year")}
            }

            if total == 0 {
                p { class: "muted stats__empty", "No focus time recorded yet." }
            } else {
                {
                    // ドーナツの各セグメントを事前計算する。
                    let circumference = 2.0 * std::f64::consts::PI * RADIUS;
                    let mut acc = 0.0;
                    let segments: Vec<(String, f64, f64)> = items
                        .iter()
                        .map(|t| {
                            let seg = t.secs as f64 / total as f64 * circumference;
                            let entry = (t.color.clone(), seg, acc);
                            acc += seg;
                            entry
                        })
                        .collect();

                    rsx! {
                        div { class: "stats__chart",
                            svg {
                                class: "donut",
                                width: "220",
                                height: "220",
                                view_box: "0 0 220 220",
                                g { transform: "rotate(-90 110 110)",
                                    for (color, seg, offset) in segments {
                                        circle {
                                            cx: "110",
                                            cy: "110",
                                            r: "{RADIUS}",
                                            fill: "none",
                                            stroke: "{color}",
                                            stroke_width: "{STROKE}",
                                            stroke_dasharray: "{seg} {circumference}",
                                            stroke_dashoffset: "{-offset}",
                                        }
                                    }
                                }
                                text {
                                    x: "110",
                                    y: "110",
                                    class: "donut__total",
                                    text_anchor: "middle",
                                    dominant_baseline: "central",
                                    "{format_focus(total)}"
                                }
                            }
                        }
                        ul { class: "stats__legend",
                            for t in items.iter().cloned() {
                                li { class: "stats__legend-row",
                                    span {
                                        class: "stats__legend-dot",
                                        style: "background-color: {t.color}",
                                    }
                                    span { class: "stats__legend-name", "{t.name}" }
                                    span { class: "stats__legend-value",
                                        "{format_focus(t.secs)} ({t.secs * 100 / total}%)"
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
