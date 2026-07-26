use dioxus::prelude::*;
use shared::Timing;

use super::{TagBar, TodoForm, TodoItem};
use crate::hooks::use_tags::UseTags;
use crate::hooks::use_timer::UseTimer;
use crate::hooks::use_timings::UseTimings;
use crate::hooks::use_todos::UseTodos;

/// 単一タイミングのTodo詳細ビュー。マスター(タイミング一覧)から遷移してくる。
#[component]
pub fn TodoList(timing: Timing, on_back: EventHandler<()>) -> Element {
    let todos = use_context::<UseTodos>();
    let tags = use_context::<UseTags>();
    let timings = use_context::<UseTimings>();
    let timer = use_context::<UseTimer>();
    let mut dragging_id = use_signal(|| None::<i64>);
    let mut filter_tag = use_signal(|| None::<i64>);

    let timer_running = timer
        .state
        .read()
        .as_ref()
        .map(|s| s.is_running)
        .unwrap_or(false);

    // 再生/一時停止ボタン。動作中の取り組みタスクなら一時停止、
    // 取り組み中で停止中なら再開、別タスクなら切り替えて新規開始する。
    let toggle_timer = move |id: i64| {
        let is_active = todos.items.read().iter().any(|t| t.id == id && t.is_active);
        if is_active {
            if timer_running {
                timer.pause();
            } else {
                timer.start();
            }
        } else {
            todos.select_active(id);
            timer.start_fresh();
        }
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
                        is_running: todo.is_active && timer_running,
                        on_toggle_complete: move |id| todos.toggle_complete(id),
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
                        on_toggle_timer: toggle_timer,
                        on_rename: move |(todo_id, title): (i64, String)| {
                            let target = todos
                                .items
                                .read()
                                .iter()
                                .find(|t| t.id == todo_id)
                                .and_then(|t| t.target_count);
                            todos.rename(todo_id, title, target);
                        },
                    }
                }
            }
        }
    }
}
