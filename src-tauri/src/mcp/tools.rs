//! MCPで公開するツールの定義と実行。既存のDB関数・TimerEngineをそのまま呼ぶ。

use crate::db::{analytics_queries, tag_queries, todo_queries, AppState};
use crate::mcp::protocol::{text_result, tool_error};
use crate::timer::engine::TimerEngine;
use serde_json::{json, Value};
use shared::DEFAULT_TIMING;
use tauri::{AppHandle, Emitter, Manager};

/// `tools/list`で返すツール定義。
pub fn definitions() -> Value {
    json!([
        {
            "name": "todo_list",
            "description": "List Cycl todos with their tags, timing category, pomodoro count and focus time. Optionally filter by tag name or timing category.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "tag": { "type": "string", "description": "Only todos carrying this tag name." },
                    "category": { "type": "string", "description": "Only todos in this timing category key (e.g. today, someday)." }
                }
            }
        },
        {
            "name": "todo_create",
            "description": "Create a todo. Tags are attached by name and created if they don't exist yet.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": { "type": "string" },
                    "target_count": { "type": "integer", "description": "Estimated number of pomodoros." },
                    "category": { "type": "string", "description": "Timing category key. Defaults to someday." },
                    "tags": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["title"]
            }
        },
        {
            "name": "todo_delete",
            "description": "Delete a todo by id.",
            "inputSchema": {
                "type": "object",
                "properties": { "id": { "type": "integer" } },
                "required": ["id"]
            }
        },
        {
            "name": "todo_set_active",
            "description": "Set which todo the timer is currently working on. Pass null to clear.",
            "inputSchema": {
                "type": "object",
                "properties": { "id": { "type": ["integer", "null"] } }
            }
        },
        {
            "name": "timer_start",
            "description": "Start (or resume) the pomodoro timer.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "timer_pause",
            "description": "Pause the pomodoro timer, recording elapsed focus time.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "timer_reset",
            "description": "Reset the current phase of the pomodoro timer back to its full duration.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "stats_monthly",
            "description": "Monthly focus summary: total focus time, completed sessions, average interruptions per session and average estimate accuracy.",
            "inputSchema": {
                "type": "object",
                "properties": { "year_month": { "type": "string", "description": "Month as YYYY-MM. Defaults to the current month." } }
            }
        },
        {
            "name": "stats_accuracy",
            "description": "Estimate-accuracy log: estimated vs actual pomodoros per completed todo, with a 0..1 accuracy score.",
            "inputSchema": {
                "type": "object",
                "properties": { "todo_id": { "type": "integer", "description": "Limit to one todo." } }
            }
        }
    ])
}

fn str_arg(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn i64_arg(args: &Value, key: &str) -> Option<i64> {
    args.get(key).and_then(|v| v.as_i64())
}

fn as_json(value: &impl serde::Serialize) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|e| format!("failed to serialize: {e}"))
}

/// ツールを実行して`tools/call`の結果を返す。
pub fn call(app: &AppHandle, name: &str, args: &Value) -> Value {
    match dispatch(app, name, args) {
        Ok(value) => value,
        Err(message) => tool_error(message),
    }
}

fn dispatch(app: &AppHandle, name: &str, args: &Value) -> Result<Value, String> {
    match name {
        "todo_list" => {
            let state = app.state::<AppState>();
            let conn = state.db.lock().unwrap();
            let mut todos = todo_queries::list(&conn).map_err(|e| e.to_string())?;
            if let Some(tag) = str_arg(args, "tag") {
                todos.retain(|t| t.tags.iter().any(|x| x.name == tag));
            }
            if let Some(category) = str_arg(args, "category") {
                todos.retain(|t| t.category == category);
            }
            Ok(text_result(as_json(&todos)))
        }

        "todo_create" => {
            let title = str_arg(args, "title").ok_or("title is required")?;
            if title.trim().is_empty() {
                return Err("title must not be empty".into());
            }
            let target = i64_arg(args, "target_count");
            let category = str_arg(args, "category");
            let tag_names: Vec<String> = args
                .get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let created = {
                let state = app.state::<AppState>();
                let conn = state.db.lock().unwrap();
                let todo =
                    todo_queries::create(&conn, title.trim(), target).map_err(|e| e.to_string())?;

                if let Some(category) = category {
                    todo_queries::update_category(&conn, todo.id, &category)
                        .map_err(|e| e.to_string())?;
                }

                // 既存タグは名前で引き当て、無ければ作ってから付与する。
                let existing = tag_queries::list(&conn).map_err(|e| e.to_string())?;
                for name in tag_names {
                    let tag_id = match existing.iter().find(|t| t.name == name) {
                        Some(tag) => tag.id,
                        None => {
                            tag_queries::create(&conn, &name, "#6366f1")
                                .map_err(|e| e.to_string())?
                                .id
                        }
                    };
                    tag_queries::add_to_todo(&conn, todo.id, tag_id).map_err(|e| e.to_string())?;
                }
                todo_queries::get(&conn, todo.id).map_err(|e| e.to_string())?
            };

            let _ = app.emit("todos:changed", ());
            Ok(text_result(as_json(&created)))
        }

        "todo_delete" => {
            let id = i64_arg(args, "id").ok_or("id is required")?;
            {
                let state = app.state::<AppState>();
                let conn = state.db.lock().unwrap();
                todo_queries::delete(&conn, id).map_err(|e| e.to_string())?;
            }
            let _ = app.emit("todos:changed", ());
            Ok(text_result(format!("deleted todo {id}")))
        }

        "todo_set_active" => {
            // 切り替え前に、それまでのタスクへ作業経過を記録する(UI操作と同じ扱い)。
            let engine = app.state::<TimerEngine>();
            engine.flush_focus();
            let id = i64_arg(args, "id");
            {
                let state = app.state::<AppState>();
                let conn = state.db.lock().unwrap();
                todo_queries::set_active(&conn, id).map_err(|e| e.to_string())?;
            }
            let _ = app.emit("todos:changed", ());
            Ok(text_result(match id {
                Some(id) => format!("todo {id} is now active"),
                None => "cleared the active todo".to_string(),
            }))
        }

        "timer_start" => {
            let engine = app.state::<TimerEngine>();
            Ok(text_result(as_json(&engine.start())))
        }
        "timer_pause" => {
            let engine = app.state::<TimerEngine>();
            Ok(text_result(as_json(&engine.pause())))
        }
        "timer_reset" => {
            let engine = app.state::<TimerEngine>();
            Ok(text_result(as_json(&engine.reset())))
        }

        "stats_monthly" => {
            let year_month = str_arg(args, "year_month")
                .unwrap_or_else(|| chrono::Local::now().format("%Y-%m").to_string());
            let state = app.state::<AppState>();
            let conn = state.db.lock().unwrap();
            let stats =
                analytics_queries::monthly_stats(&conn, &year_month).map_err(|e| e.to_string())?;
            Ok(text_result(as_json(&json!({
                "yearMonth": year_month,
                "stats": stats,
            }))))
        }

        "stats_accuracy" => {
            let todo_id = i64_arg(args, "todo_id");
            let state = app.state::<AppState>();
            let conn = state.db.lock().unwrap();
            let entries =
                analytics_queries::accuracy_entries(&conn, todo_id).map_err(|e| e.to_string())?;
            Ok(text_result(as_json(&entries)))
        }

        other => Err(format!("unknown tool: {other}")),
    }
}

/// 新規タスクの既定タイミング(ツール説明に載せる値と揃える)。
#[allow(dead_code)]
pub const DEFAULT_CATEGORY: &str = DEFAULT_TIMING;
