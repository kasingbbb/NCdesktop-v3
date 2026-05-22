use crate::db::{self, Database};
use tauri::State;

#[tauri::command]
pub fn get_setting(
    database: State<'_, Database>,
    key: String,
) -> Result<Option<String>, String> {
    let conn = database.conn()?;
    db::settings::get(&conn, &key)
}

#[tauri::command]
pub fn set_setting(
    database: State<'_, Database>,
    key: String,
    value: String,
) -> Result<(), String> {
    let conn = database.conn()?;
    db::settings::set(&conn, &key, &value)
}

#[tauri::command]
pub fn get_all_settings(
    database: State<'_, Database>,
) -> Result<std::collections::HashMap<String, String>, String> {
    let conn = database.conn()?;
    db::settings::get_all(&conn)
}
