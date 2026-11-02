use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Note {
    pub id: i64,
    pub body: String,
    pub created_at: DateTime<Utc>,
}
