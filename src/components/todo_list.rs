use dioxus::prelude::*;
use shared::DEFAULT_CATEGORY;

use super::{CategoryTabs, TagBar, TodoForm, TodoItem};
use crate::hooks::use_tags::use_tags;
use crate::hooks::use_todos::use_todos;

#[component]
pub fn TodoList() -> Element {
    let todos = use_todos();
    let tags = use_tags();
    let mut dragging_id = use_signal(|| None::<i64>);
    let mut filter_tag = use_signal(|| None::<i64>);
    let mut filter_category = use_signal(|| None::<String>);

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
    let category_filter = filter_category.read().clone();
    // フィルタが有効なときはドラッグ並び替えを無効化する(部分集合の並び替えは混乱を招くため)。
    let reorder_enabled = filter.is_none() && category_filter.is_none();
    let all_tags = tags.items.read().clone();
    let visible: Vec<_> = todos
        .items
        .read()
        .iter()
        .filter(|todo| match filter {
            Some(tag_id) => todo.tags.iter().any(|t| t.id == tag_id),
            None => true,
        })
        .filter(|todo| match &category_filter {
            Some(category) => &todo.category == category,
            None => true,
        })
        .cloned()
        .collect();
    // 新規Todoは、カテゴリで絞り込み中ならそのカテゴリに、そうでなければ既定に作る。
    let new_todo_category = category_filter
        .clone()
        .unwrap_or(DEFAULT_CATEGORY.to_string());

    rsx! {
        div { class: "todo-list",
            CategoryTabs {
                active: category_filter.clone(),
                on_select: move |c| filter_category.set(c),
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
                    todos.add(title, target_count, new_todo_category.clone());
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
                    }
                }
            }
        }
    }
}
