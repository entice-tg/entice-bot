use schema::chats;
use chrono::{DateTime, Utc};

#[derive(Queryable)]
pub struct Chat {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub last_updated: DateTime<Utc>,
}

#[derive(Insertable)]
#[table_name = "chats"]
pub struct NewChat<'a> {
    pub id: i64,
    pub title: &'a str,
    pub description: &'a str,
}
