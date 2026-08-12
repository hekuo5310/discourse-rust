use chrono::{DateTime, SecondsFormat, Utc};
use forum_core::{Category, Post, Topic, User};
use sqlx::FromRow;
use uuid::Uuid;

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[derive(Debug, FromRow)]
pub(crate) struct UserRow {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

impl From<UserRow> for User {
    fn from(value: UserRow) -> Self {
        Self {
            id: value.id.to_string(),
            username: value.username,
            email: value.email,
            role: value.role,
            created_at: timestamp(value.created_at),
        }
    }
}

#[derive(Debug, FromRow)]
pub(crate) struct UserWithPasswordRow {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

impl UserWithPasswordRow {
    pub fn public(self) -> User {
        UserRow {
            id: self.id,
            username: self.username,
            email: self.email,
            role: self.role,
            created_at: self.created_at,
        }
        .into()
    }
}

#[derive(Debug, FromRow)]
pub(crate) struct CategoryRow {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
}

impl From<CategoryRow> for Category {
    fn from(value: CategoryRow) -> Self {
        Self {
            id: value.id.to_string(),
            name: value.name,
            slug: value.slug,
            created_at: timestamp(value.created_at),
        }
    }
}

#[derive(Debug, FromRow)]
pub(crate) struct TopicRow {
    pub id: Uuid,
    pub category_id: Uuid,
    pub author_id: Uuid,
    pub author_username: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub post_count: i32,
}

impl From<TopicRow> for Topic {
    fn from(value: TopicRow) -> Self {
        Self {
            id: value.id.to_string(),
            category_id: value.category_id.to_string(),
            author_id: value.author_id.to_string(),
            author_username: value.author_username,
            title: value.title,
            created_at: timestamp(value.created_at),
            updated_at: timestamp(value.updated_at),
            post_count: value.post_count,
        }
    }
}

#[derive(Debug, FromRow)]
pub(crate) struct PostRow {
    pub id: Uuid,
    pub topic_id: Uuid,
    pub author_id: Uuid,
    pub author_username: String,
    pub body: String,
    pub position: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<PostRow> for Post {
    fn from(value: PostRow) -> Self {
        Self {
            id: value.id.to_string(),
            topic_id: value.topic_id.to_string(),
            author_id: value.author_id.to_string(),
            author_username: value.author_username,
            body: value.body,
            position: value.position,
            created_at: timestamp(value.created_at),
            updated_at: timestamp(value.updated_at),
        }
    }
}
