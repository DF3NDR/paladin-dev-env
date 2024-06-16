// src/preferences/mod.rs
use serde::{Deserialize, Serialize};
use rusqlite::{params, Connection, Result};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserPreferences {
    pub theme: String,
    pub notifications_enabled: bool,
    pub summary_format: String,
}

pub fn load_user_preferences(conn: &Connection) -> Result<UserPreferences> {
    let mut stmt = conn.prepare("SELECT theme, notifications_enabled, summary_format FROM user_preferences")?;
    let user_prefs = stmt.query_row(params![], |row| {
        Ok(UserPreferences {
            theme: row.get(0)?,
            notifications_enabled: row.get(1)?,
            summary_format: row.get(2)?,
        })
    })?;
    Ok(user_prefs)
}

pub fn save_user_preferences(conn: &Connection, prefs: &UserPreferences) -> Result<()> {
    conn.execute(
        "INSERT INTO user_preferences (theme, notifications_enabled, summary_format) VALUES (?1, ?2, ?3)",
        params![prefs.theme, prefs.notifications_enabled, prefs.summary_format],
    )?;
    Ok(())
}
