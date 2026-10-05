use chrono::{DateTime, Utc};

pub struct Note{
    id:i64,
    body:String,
    created_at:DateTime<Utc>,
}
