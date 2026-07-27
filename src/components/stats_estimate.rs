use dioxus::prelude::*;
use shared::AccuracyEntry;

use crate::tauri_api::stats as api;

/// 棒グラフに出す直近の件数。多すぎると1本ずつが読めなくなるので絞る。
const RECENT_LIMIT: usize = 8;

const LINE_W: f64 = 320.0;
const LINE_H: f64 = 130.0;
const PAD_X: f64 = 28.0;
const PAD_Y: f64 = 18.0;

/// 見積もり vs 実績の棒グラフと、予測精度スコアの推移。
#[component]
pub fn StatsEstimate() -> Element {
    let mut data =
        use_resource(move || async move { api::accuracy(None).await.unwrap_or_default() });

    use_effect(move || {
        crate::tauri_api::listen::<()>("todos:changed", move |_| data.restart());
    });

    let entries: Vec<AccuracyEntry> = data.value().read().clone().unwrap_or_default();

    if entries.is_empty() {
        return rsx! {
            div { class: "stats-view",
                p { class: "muted stats__empty",
                    "Complete a task that has a target to see estimate accuracy."
                }
            }
        };
    }

    // 棒グラフは直近を上に並べる。軸は見積もり・実績の最大値に合わせる。
    let recent: Vec<AccuracyEntry> = entries.iter().rev().take(RECENT_LIMIT).cloned().collect();
    let max = recent
        .iter()
        .map(|e| e.estimated_count.max(e.actual_count))
        .max()
        .unwrap_or(1)
        .max(1) as f64;

    // 精度推移の折れ線(古い順)。単一系列なので凡例は置かない。
    let count = entries.len();
    let x_of = |i: usize| {
        if count <= 1 {
            LINE_W / 2.0
        } else {
            PAD_X + (LINE_W - PAD_X * 2.0) * (i as f64 / (count - 1) as f64)
        }
    };
    let y_of = |score: f64| LINE_H - PAD_Y - (LINE_H - PAD_Y * 2.0) * score.clamp(0.0, 1.0);
    let points = entries
        .iter()
        .enumerate()
        .map(|(i, e)| format!("{:.1},{:.1}", x_of(i), y_of(e.accuracy_score)))
        .collect::<Vec<_>>()
        .join(" ");
    let last = entries.last().cloned().unwrap();
    let last_x = x_of(count - 1);
    let last_y = y_of(last.accuracy_score);
    let avg = entries.iter().map(|e| e.accuracy_score).sum::<f64>() / count as f64;

    rsx! {
        div { class: "stats-view",
            section { class: "stats-card",
                h3 { class: "stats-card__title", "Estimated vs actual" }
                div { class: "stats-keys",
                    span { class: "stats-key",
                        span { class: "stats-key__swatch stats-key__swatch--est" }
                        "Estimated"
                    }
                    span { class: "stats-key",
                        span { class: "stats-key__swatch stats-key__swatch--act" }
                        "Actual"
                    }
                }
                ul { class: "ebars",
                    for e in recent.iter().cloned() {
                        li { class: "ebar",
                            span { class: "ebar__label", title: "{e.todo_title}", "{e.todo_title}" }
                            div { class: "ebar__bars",
                                div { class: "ebar__row",
                                    div {
                                        class: "ebar__fill ebar__fill--est",
                                        style: "width: {e.estimated_count as f64 / max * 100.0}%",
                                    }
                                    span { class: "ebar__value", "{e.estimated_count}" }
                                }
                                div { class: "ebar__row",
                                    div {
                                        class: "ebar__fill ebar__fill--act",
                                        style: "width: {e.actual_count as f64 / max * 100.0}%",
                                    }
                                    span { class: "ebar__value", "{e.actual_count}" }
                                }
                            }
                        }
                    }
                }
            }

            section { class: "stats-card",
                h3 { class: "stats-card__title", "Accuracy trend" }
                p { class: "stats-card__note muted",
                    "Average {avg:.2} over {count} completed task(s). 1.00 means the estimate matched exactly."
                }
                svg {
                    class: "linechart",
                    view_box: "0 0 {LINE_W} {LINE_H}",
                    preserve_aspect_ratio: "xMidYMid meet",
                    // 目盛りは 0 / 0.5 / 1.0 の3本だけ引く。
                    for (score, label) in [(1.0_f64, "1.0"), (0.5, "0.5"), (0.0, "0")] {
                        line {
                            class: "linechart__grid",
                            x1: "{PAD_X}",
                            y1: "{y_of(score)}",
                            x2: "{LINE_W - PAD_X}",
                            y2: "{y_of(score)}",
                        }
                        text {
                            class: "linechart__tick",
                            x: "{PAD_X - 6.0}",
                            y: "{y_of(score)}",
                            text_anchor: "end",
                            dominant_baseline: "central",
                            "{label}"
                        }
                    }
                    polyline {
                        class: "linechart__line",
                        points: "{points}",
                        fill: "none",
                    }
                    for (i, e) in entries.iter().enumerate() {
                        circle {
                            class: "linechart__dot",
                            cx: "{x_of(i)}",
                            cy: "{y_of(e.accuracy_score)}",
                            r: "4",
                        }
                    }
                    // 直近の値だけ直接ラベルする。
                    text {
                        class: "linechart__endlabel",
                        x: "{last_x}",
                        y: "{last_y - 10.0}",
                        text_anchor: "end",
                        "{last.accuracy_score:.2}"
                    }
                }
            }
        }
    }
}
