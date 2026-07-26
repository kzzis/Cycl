use crate::error::AppResult;
use rusqlite::Connection;
use shared::Tag;

fn tag_from_row(row: &rusqlite::Row) -> rusqlite::Result<Tag> {
    Ok(Tag {
        id: row.get("id")?,
        name: row.get("name")?,
        color: row.get("color")?,
    })
}

pub fn list(conn: &Connection) -> AppResult<Vec<Tag>> {
    let mut stmt = conn.prepare("SELECT id, name, color FROM tag ORDER BY name ASC")?;
    let tags = stmt
        .query_map([], tag_from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tags)
}

pub fn create(conn: &Connection, name: &str, color: &str) -> AppResult<Tag> {
    conn.execute(
        "INSERT INTO tag (name, color) VALUES (?1, ?2)",
        (name, color),
    )?;
    let id = conn.last_insert_rowid();
    Ok(conn.query_row(
        "SELECT id, name, color FROM tag WHERE id = ?1",
        [id],
        tag_from_row,
    )?)
}

pub fn delete(conn: &Connection, id: i64) -> AppResult<()> {
    conn.execute("DELETE FROM tag WHERE id = ?1", [id])?;
    Ok(())
}

/// TodoにTagを付与する。既に付いている場合は何もしない。
pub fn add_to_todo(conn: &Connection, todo_id: i64, tag_id: i64) -> AppResult<()> {
    conn.execute(
        "INSERT OR IGNORE INTO todo_tag (todo_id, tag_id) VALUES (?1, ?2)",
        (todo_id, tag_id),
    )?;
    Ok(())
}

pub fn remove_from_todo(conn: &Connection, todo_id: i64, tag_id: i64) -> AppResult<()> {
    conn.execute(
        "DELETE FROM todo_tag WHERE todo_id = ?1 AND tag_id = ?2",
        (todo_id, tag_id),
    )?;
    Ok(())
}

/// 指定したTodoに付与されているTag一覧を取得する。
pub fn tags_for_todo(conn: &Connection, todo_id: i64) -> AppResult<Vec<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, t.color FROM tag t
         JOIN todo_tag tt ON tt.tag_id = t.id
         WHERE tt.todo_id = ?1
         ORDER BY t.name ASC",
    )?;
    let tags = stmt
        .query_map([todo_id], tag_from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tags)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{migrations, todo_queries};

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    #[test]
    fn create_and_list_tags() {
        let conn = setup_conn();
        create(&conn, "work", "#ff0000").unwrap();
        create(&conn, "home", "#00ff00").unwrap();
        let tags = list(&conn).unwrap();
        assert_eq!(tags.len(), 2);
        // name順なので home が先
        assert_eq!(tags[0].name, "home");
    }

    #[test]
    fn add_and_remove_tag_on_todo() {
        let conn = setup_conn();
        let todo = todo_queries::create(&conn, "作業", None).unwrap();
        let tag = create(&conn, "urgent", "#ff0000").unwrap();

        add_to_todo(&conn, todo.id, tag.id).unwrap();
        assert_eq!(tags_for_todo(&conn, todo.id).unwrap().len(), 1);

        // 二重付与しても増えない
        add_to_todo(&conn, todo.id, tag.id).unwrap();
        assert_eq!(tags_for_todo(&conn, todo.id).unwrap().len(), 1);

        remove_from_todo(&conn, todo.id, tag.id).unwrap();
        assert!(tags_for_todo(&conn, todo.id).unwrap().is_empty());
    }

    #[test]
    fn deleting_tag_removes_it_from_todos() {
        let conn = setup_conn();
        let todo = todo_queries::create(&conn, "作業", None).unwrap();
        let tag = create(&conn, "temp", "#123456").unwrap();
        add_to_todo(&conn, todo.id, tag.id).unwrap();

        delete(&conn, tag.id).unwrap();

        assert!(list(&conn).unwrap().is_empty());
        assert!(tags_for_todo(&conn, todo.id).unwrap().is_empty());
    }
}
