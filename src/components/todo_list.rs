use dioxus::prelude::*;
use shared::Timing;

use super::{TagBar, TodoForm, TodoItem};
use crate::app::Tab;
use crate::hooks::use_tags::use_tags;
use crate::hooks::use_timer::UseTimer;
use crate::hooks::use_timings::use_timings;
use crate::hooks::use_todos::UseTodos;

/// 単一タイミングのTodo詳細ビュー。マスター(タイミング一覧)から遷移してくる。
#[component]
pub fn TodoList(timing: Timing, on_back: EventHandler<()>) -> Element {
    let todos = use_context::<UseTodos>();
    let tags = use_tags();
    let timings = use_timings();
    let timer = use_context::<UseTimer>();
    let mut tab = use_context::<Signal<Tab>>();
    let mut dragging_id = use_signal(|| None::<i64>);
    let mut filter_tag = use_signal(|| None::<i64>);

    // 再生ボタン: そのタスクを取り組み中にし、タイマーを新しく開始してTimerタブへ移る。
    let start_task = move |id: i64| {
        todos.select_active(id);
        timer.start_fresh();
        tab.set(Tab::Timer);
    };

    if *todos.is_loading.read() {
        return rsx! { p { class: "muted", "Loading..." } };
    }

    // ドラッグ終了: 現在のリスト順(ホバー中にライブで並び替え済み)をDBへ保存する。
    let end_drag = move |_| {
        if dragging_id.take().is_some() {
            let ids: Vec<i64> = todos.items.read().iter().map(|t| t.id).collect();
            todos.reorder(ids);
        }
    };

    // ホバー中のライブ並び替え: ドラッグ中の行を、通過した行の隣へその場で移動する。
    // 下方向へ動かしていればホバー行の下、上方向なら上へ挿入する。
    let reorder_on_hover = move |hover_over_id: i64| {
        let Some(dragged_id) = *dragging_id.read() else {
            return;
        };
        if dragged_id == hover_over_id {
            return;
        }
        let mut items_sig = todos.items;
        let mut items = items_sig.write();
        let (Some(from), Some(hovered)) = (
            items.iter().position(|t| t.id == dragged_id),
            items.iter().position(|t| t.id == hover_over_id),
        ) else {
            return;
        };
        let dragging_down = from < hovered;
        let moved = items.remove(from);
        let mut to = items
            .iter()
            .position(|t| t.id == hover_over_id)
            .unwrap_or(items.len());
        if dragging_down {
            to += 1;
        }
        items.insert(to, moved);
    };

    let filter = *filter_tag.read();
    // タグで絞り込み中はドラッグ並び替えを無効化する(部分集合の並び替えは混乱を招くため)。
    let reorder_enabled = filter.is_none();
    let all_tags = tags.items.read().clone();
    let all_timings = timings.items.read().clone();
    let timing_key = timing.key.clone();
    let visible: Vec<_> = todos
        .items
        .read()
        .iter()
        .filter(|todo| todo.category == timing_key)
        .filter(|todo| match filter {
            Some(tag_id) => todo.tags.iter().any(|t| t.id == tag_id),
            None => true,
        })
        .cloned()
        .collect();
    let new_todo_timing = timing.key.clone();

    rsx! {
        div { class: "todo-list",
            div { class: "todo-list__header",
                button {
                    class: "todo-list__back",
                    aria_label: "Back to timings",
                    onclick: move |_| on_back.call(()),
                    "←"
                }
                span {
                    class: "timing-row__dot",
                    style: "background-color: {timing.color}",
                }
                h2 { class: "todo-list__title", "{timing.name}" }
            }
            TagBar {
                tags: all_tags.clone(),
                active_filter: filter,
                on_filter: move |f| filter_tag.set(f),
                on_create: move |(name, color): (String, String)| tags.add(name, color),
                on_delete: move |id| tags.remove(id),
            }
            TodoForm {
                on_submit: move |(title, target_count): (String, Option<i64>)| {
                    todos.add(title, target_count, new_todo_timing.clone());
                }
            }
            ul {
                onmouseup: end_drag,
                onmouseleave: end_drag,
                for todo in visible {
                    TodoItem {
                        key: "{todo.id}",
                        todo: todo.clone(),
                        all_tags: all_tags.clone(),
                        all_timings: all_timings.clone(),
                        is_dragging: reorder_enabled && *dragging_id.read() == Some(todo.id),
                        on_toggle_complete: move |id| todos.toggle_complete(id),
                        on_select_active: move |id| todos.select_active(id),
                        on_delete: move |id| todos.remove(id),
                        on_drag_start: move |id| {
                            if reorder_enabled {
                                dragging_id.set(Some(id));
                            }
                        },
                        on_hover: reorder_on_hover,
                        on_add_tag: move |(todo_id, tag_id)| todos.add_tag(todo_id, tag_id),
                        on_remove_tag: move |(todo_id, tag_id)| todos.remove_tag(todo_id, tag_id),
                        on_change_category: move |(todo_id, category)| todos.update_category(todo_id, category),
                        on_start: start_task,
                    }
                }
            }
        }
    }
}
