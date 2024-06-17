use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserPreferences {
    pub theme: String,
    pub notifications_enabled: bool,
    pub summary_format: String,
}
