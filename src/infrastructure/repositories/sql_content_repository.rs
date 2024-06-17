use rusqlite::{params, Connection, Result};
use crate::domain::entities::NormalizedData;
use crate::domain::services::ContentFetchingService;

pub struct SqlContentRepository;

impl ContentFetchingService for SqlContentRepository {
    fn fetch_content(&self, url: &str) -> Vec<NormalizedData> {
        // Implementation here
        vec![]
    }
}

impl SqlContentRepository {
    pub fn init_db() -> Result<Connection> {
        let conn = Connection::open("data.db")?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS content (
                id INTEGER PRIMARY KEY,
                source TEXT NOT NULL,
                title TEXT NOT NULL,
                link TEXT NOT NULL,
                description TEXT,
                summary TEXT
            )",
            params![],
        )?;
        Ok(conn)
    }

    pub fn save_content(
        conn: &Connection,
        source: &str,
        title: &str,
        link: &str,
        description: &str,
        summary: &str,
    ) -> Result<()> {
        conn.execute(
            "INSERT INTO content (source, title, link, description, summary) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![source, title, link, description, summary],
        )?;
        Ok(())
    }
}
