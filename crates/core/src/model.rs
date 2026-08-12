use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub role: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Topic {
    pub id: String,
    pub category_id: String,
    pub author_id: String,
    pub author_username: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub post_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Post {
    pub id: String,
    pub topic_id: String,
    pub author_id: String,
    pub author_username: String,
    pub body: String,
    pub position: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterInput {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginInput {
    pub login: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryInput {
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopicInput {
    pub category_id: String,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostInput {
    pub body: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionResponse {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopicDetail {
    pub topic: Topic,
    pub posts: Vec<Post>,
}

#[derive(Debug, Serialize)]
pub struct ApiError<'a> {
    pub error: &'a str,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdResponse {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListResponse<T> {
    pub items: Vec<T>,
}
