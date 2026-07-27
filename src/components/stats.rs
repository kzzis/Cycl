use dioxus::prelude::*;

use super::{StatsEstimate, StatsFocus, StatsMonth, StatsTags};

#[derive(Clone, Copy, PartialEq)]
enum View {
    Focus,
    Estimate,
    Month,
    Tags,
}

/// Statsタブ本体。分析ビューを切り替える。各ビューが自分の期間フィルタを持つ。
#[component]
pub fn Stats() -> Element {
    let mut view = use_signal(|| View::Focus);
    let active = *view.read();

    let nav = |target: View, label: &'static str| {
        rsx! {
            button {
                class: if active == target { "stats-nav__item stats-nav__item--active" } else { "stats-nav__item" },
                onclick: move |_| view.set(target),
                "{label}"
            }
        }
    };

    rsx! {
        div { class: "stats",
            nav { class: "stats-nav",
                {nav(View::Focus, "Focus")}
                {nav(View::Estimate, "Estimate")}
                {nav(View::Month, "Month")}
                {nav(View::Tags, "Tags")}
            }
            match active {
                View::Focus => rsx! { StatsFocus {} },
                View::Estimate => rsx! { StatsEstimate {} },
                View::Month => rsx! { StatsMonth {} },
                View::Tags => rsx! { StatsTags {} },
            }
        }
    }
}
