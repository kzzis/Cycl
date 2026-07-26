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
