use rusqlite::{params, Connection, OptionalExtension};

pub fn get(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| format!("读取设置失败: {e}"))
}

pub fn set(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .map_err(|e| format!("写入设置失败: {e}"))?;
    Ok(())
}

pub fn get_all(conn: &Connection) -> Result<std::collections::HashMap<String, String>, String> {
    let mut stmt = conn
        .prepare("SELECT key, value FROM settings")
        .map_err(|e| format!("查询设置失败: {e}"))?;

    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| format!("遍历设置失败: {e}"))?;

    let mut map = std::collections::HashMap::new();
    for r in rows {
        let (k, v) = r.map_err(|e| format!("读取行失败: {e}"))?;
        map.insert(k, v);
    }
    Ok(map)
}
