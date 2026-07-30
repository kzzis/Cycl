use rusqlite::{Connection, Result};

const MIGRATIONS: &[(&str, &str)] = &[
    (
        "0001_initial",
        r#"
    CREATE TABLE todo (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        title TEXT NOT NULL,
        is_completed INTEGER NOT NULL DEFAULT 0,
        pomodoro_count INTEGER NOT NULL DEFAULT 0,
        target_count INTEGER,
        is_active INTEGER NOT NULL DEFAULT 0,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
    );

    CREATE TABLE pomodoro_session (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        todo_id INTEGER NOT NULL REFERENCES todo(id) ON DELETE CASCADE,
        started_at TEXT NOT NULL,
        completed INTEGER NOT NULL DEFAULT 0
    );
    "#,
    ),
    (
        "0002_add_todo_sort_order",
        r#"
    ALTER TABLE todo ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;
    UPDATE todo SET sort_order = id;
    "#,
    ),
    (
        "0003_tags",
        r#"
    CREATE TABLE tag (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL UNIQUE,
        color TEXT NOT NULL DEFAULT '#6366f1'
    );

    CREATE TABLE todo_tag (
        todo_id INTEGER NOT NULL REFERENCES todo(id) ON DELETE CASCADE,
        tag_id  INTEGER NOT NULL REFERENCES tag(id)  ON DELETE CASCADE,
        PRIMARY KEY (todo_id, tag_id)
    );
    "#,
    ),
    (
        "0004_category",
        r#"
    -- category: 'today' | 'tomorrow' | 'this_week' | 'planned' | 'someday' | 'event'
    ALTER TABLE todo ADD COLUMN category TEXT NOT NULL DEFAULT 'someday';
    "#,
    ),
    (
        "0005_timings",
        r#"
    CREATE TABLE timing (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        key TEXT NOT NULL UNIQUE,
        name TEXT NOT NULL,
        color TEXT NOT NULL DEFAULT '#6366f1',
        sort_order INTEGER NOT NULL DEFAULT 0,
        is_builtin INTEGER NOT NULL DEFAULT 0
    );

    INSERT INTO timing (key, name, color, sort_order, is_builtin) VALUES
        ('today',     'Today',     '#22c55e', 0, 1),
        ('tomorrow',  'Tomorrow',  '#f97316', 1, 1),
        ('this_week', 'This Week', '#8b5cf6', 2, 1),
        ('planned',   'Planned',   '#3b82f6', 3, 1),
        ('someday',   'Someday',   '#a855f7', 4, 1),
        ('event',     'Event',     '#14b8a6', 5, 1);
    "#,
    ),
    (
        "0006_focus_secs",
        r#"
    -- 累積作業時間(秒)。完了に達しなくても一時停止・タスク切替時に加算される。
    ALTER TABLE todo ADD COLUMN focus_secs INTEGER NOT NULL DEFAULT 0;
    "#,
    ),
    (
        "0007_session_duration",
        r#"
    -- 各作業チャンクの長さ(秒)。統計グラフの時間集計に使う。
    ALTER TABLE pomodoro_session ADD COLUMN duration_secs INTEGER NOT NULL DEFAULT 0;
    "#,
    ),
    (
        "0008_analytics",
        r#"
    -- 1ポモドーロ内で発生した中断(一時停止・タスク切替)の回数。
    ALTER TABLE pomodoro_session ADD COLUMN interruption_count INTEGER NOT NULL DEFAULT 0;

    -- 見積もり(target_count)と実績(pomodoro_count)の乖離をタスク完了時に記録する。
    CREATE TABLE estimation_log (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        todo_id INTEGER NOT NULL REFERENCES todo(id) ON DELETE CASCADE,
        estimated_count INTEGER NOT NULL,
        actual_count INTEGER NOT NULL,
        accuracy_score REAL NOT NULL,
        recorded_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
    );
    "#,
    ),
    (
        "0009_app_setting",
        r#"
    -- アプリ設定の汎用key-value(MCPサーバーのON/OFFなど)。
    CREATE TABLE app_setting (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    "#,
    ),
];

/// SQLiteの `PRAGMA user_version` を使って、未適用のマイグレーションだけを順番に当てる。
pub fn run(conn: &Connection) -> Result<()> {
    let current_version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let current_version = current_version as usize;

    for (i, (_name, sql)) in MIGRATIONS.iter().enumerate() {
        if i < current_version {
            continue;
        }
        conn.execute_batch(sql)?;
        conn.pragma_update(None, "user_version", (i + 1) as i64)?;
    }

    Ok(())
}
