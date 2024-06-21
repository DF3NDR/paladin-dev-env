// src/adapters/secondary/sql_content_repository.rs
use rusqlite::{params, Connection, Result};
use crate::domain::entities::content_item::ContentItem;
use crate::domain::entities::content_list::ContentList;
use crate::domain::services::content_fetching_service::{
    ContentFetchingService, 
    ContentListFetchingService
};

pub struct SqlContentRepository;

impl ContentListFetchingService for SqlContentRepository {
    fn fetch_content_list(&self, url: &str) -> Result<ContentList, String> {
        // Implementation here
        Ok(ContentList {
            uuid: todo!(),
            name: todo!(),
            url: todo!(),
            created: todo!(),
            modified: todo!(),
            list_items: todo!(),
        })
    }
}

impl ContentFetchingService for SqlContentRepository {
    fn fetch_content(&self, url: &str) -> Result<ContentItem, String> {
        // Implementation here
        Ok(ContentItem {
            metadata: todo!(),
            body: todo!(),
        })
    }
}

impl SqlContentRepository {
    pub fn init_db() -> Result<Connection> {
        let conn = Connection::open("data.db")?;
        conn.execute(
            // ToDo This needs to be updated to match the actual schema
            "CREATE TABLE IF NOT EXISTS content (
                uuid INTEGER PRIMARY KEY,
                source TEXT NOT NULL,
                title TEXT NOT NULL,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_content() {
        let fetcher = SqlContentRepository;
        let result = fetcher.fetch_content("database_entry_id");
        // Validate the result
        assert!(result.is_ok());
    }
}
