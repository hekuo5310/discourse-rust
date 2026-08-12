use std::iter::once;

use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use forum_core::{
    hash_password, new_session_token, normalize_email, normalize_slug, normalize_username,
    token_digest, validate_body, validate_title, verify_password, Category, CategoryInput,
    IdResponse, ListResponse, LoginInput, Post, PostInput, RegisterInput, SessionResponse, Topic,
    TopicDetail, TopicInput, User,
};
use redis::AsyncTypedCommands;
use serde::de::DeserializeOwned;
use serde_json::json;
use sqlx::Error as SqlxError;
use tower::ServiceBuilder;
use tower_http::{sensitive_headers::SetSensitiveHeadersLayer, trace::TraceLayer};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    rows::{CategoryRow, PostRow, TopicRow, UserRow, UserWithPasswordRow},
    AppState,
};

const SESSION_TTL_SECONDS: u64 = 60 * 60 * 24 * 7;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/me", get(me))
        .route("/api/v1/categories", get(list_categories).post(create_category))
        .route("/api/v1/topics", get(list_topics).post(create_topic))
        .route("/api/v1/topics/{id}", get(topic_detail))
        .route("/api/v1/topics/{id}/posts", post(create_post))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(SetSensitiveHeadersLayer::new(once(AUTHORIZATION)))
                .layer(TraceLayer::new_for_http()),
        )
}

async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

fn payload<T: DeserializeOwned>(payload: Result<Json<T>, JsonRejection>) -> ApiResult<T> {
    payload
        .map(|Json(value)| value)
        .map_err(|_| ApiError::bad_request("invalid JSON"))
}

fn bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .map(str::to_owned)
}

fn parse_id(value: &str, message: &'static str) -> ApiResult<Uuid> {
    Uuid::parse_str(value).map_err(|_| ApiError::not_found(message))
}

fn unique_violation(error: &SqlxError) -> bool {
    error
        .as_database_error()
        .and_then(|error| error.code())
        .is_some_and(|code| code == "23505")
}

async fn current_user(state: &AppState, headers: &HeaderMap) -> ApiResult<Option<User>> {
    let Some(token) = bearer(headers) else {
        return Ok(None);
    };
    let digest = token_digest(&token);

    // Redis is only a latency hint. PostgreSQL remains authoritative for
    // session revocation and expiration.
    let mut cache = state.cache.clone();
    if let Err(error) = cache
        .get(&format!("session:{digest}"))
        .await
        .map(|_: Option<String>| ())
    {
        tracing::warn!(%error, "Redis session hint lookup failed");
    }

    sqlx::query_as::<_, UserRow>(
        "SELECT u.id, u.username, u.email, u.role, u.created_at \
         FROM sessions s JOIN users u ON u.id = s.user_id \
         WHERE s.token_hash = $1 AND s.expires_at > CURRENT_TIMESTAMP",
    )
    .bind(digest)
    .fetch_optional(&state.db)
    .await
    .map(|user| user.map(Into::into))
    .map_err(ApiError::internal)
}

async fn require_user(state: &AppState, headers: &HeaderMap) -> ApiResult<User> {
    current_user(state, headers)
        .await?
        .ok_or_else(|| ApiError::unauthorized("authentication required"))
}

async fn create_session(state: &AppState, user: User) -> ApiResult<SessionResponse> {
    let token = new_session_token();
    let digest = token_digest(&token);
    sqlx::query(
        "INSERT INTO sessions (id, user_id, token_hash, expires_at) \
         VALUES ($1, $2, $3, CURRENT_TIMESTAMP + INTERVAL '7 days')",
    )
    .bind(Uuid::new_v4())
    .bind(parse_id(&user.id, "user not found")?)
    .bind(&digest)
    .execute(&state.db)
    .await
    .map_err(ApiError::internal)?;

    let mut cache = state.cache.clone();
    if let Err(error) = cache
        .set_ex(format!("session:{digest}"), user.id.clone(), SESSION_TTL_SECONDS)
        .await
    {
        tracing::warn!(%error, "Redis session hint write failed");
    }

    Ok(SessionResponse { token, user })
}

async fn register(
    State(state): State<AppState>,
    payload_result: Result<Json<RegisterInput>, JsonRejection>,
) -> ApiResult<impl IntoResponse> {
    let input = payload(payload_result)?;
    let username = normalize_username(&input.username)
        .map_err(|_| ApiError::unprocessable("invalid username"))?;
    let email = normalize_email(&input.email)
        .map_err(|_| ApiError::unprocessable("invalid email"))?;
    let password_hash = hash_password(&input.password)
        .map_err(|_| ApiError::unprocessable("password must contain 12 to 256 bytes"))?;

    let mut transaction = state.db.begin().await.map_err(ApiError::internal)?;
    sqlx::query("LOCK TABLE users IN SHARE ROW EXCLUSIVE MODE")
        .execute(&mut *transaction)
        .await
        .map_err(ApiError::internal)?;
    let first_user = sqlx::query_scalar::<_, bool>("SELECT NOT EXISTS (SELECT 1 FROM users)")
        .fetch_one(&mut *transaction)
        .await
        .map_err(ApiError::internal)?;
    let role = if first_user { "admin" } else { "member" };
    let inserted = sqlx::query_as::<_, UserRow>(
        "INSERT INTO users (id, username, email, password_hash, role) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING id, username, email, role, created_at",
    )
    .bind(Uuid::new_v4())
    .bind(username)
    .bind(email)
    .bind(password_hash)
    .bind(role)
    .fetch_one(&mut *transaction)
    .await;
    let user: User = match inserted {
        Ok(user) => user.into(),
        Err(error) if unique_violation(&error) => {
            return Err(ApiError::conflict("username or email already exists"));
        }
        Err(error) => return Err(ApiError::internal(error)),
    };
    transaction.commit().await.map_err(ApiError::internal)?;

    Ok((StatusCode::CREATED, Json(create_session(&state, user).await?)))
}

async fn login(
    State(state): State<AppState>,
    payload_result: Result<Json<LoginInput>, JsonRejection>,
) -> ApiResult<impl IntoResponse> {
    let input = payload(payload_result)?;
    let login = input.login.trim().to_ascii_lowercase();
    let user = sqlx::query_as::<_, UserWithPasswordRow>(
        "SELECT id, username, email, role, password_hash, created_at \
         FROM users WHERE username = $1 OR email = $1",
    )
    .bind(login)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::internal)?;
    let Some(user) = user.filter(|user| verify_password(&input.password, &user.password_hash))
    else {
        return Err(ApiError::unauthorized("invalid credentials"));
    };

    Ok(Json(create_session(&state, user.public()).await?))
}

async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    let token = bearer(&headers)
        .ok_or_else(|| ApiError::unauthorized("authentication required"))?;
    let digest = token_digest(&token);
    sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
        .bind(&digest)
        .execute(&state.db)
        .await
        .map_err(ApiError::internal)?;

    let mut cache = state.cache.clone();
    if let Err(error) = cache.del(format!("session:{digest}")).await {
        tracing::warn!(%error, "Redis session hint deletion failed");
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn me(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<User>> {
    Ok(Json(require_user(&state, &headers).await?))
}

async fn list_categories(
    State(state): State<AppState>,
) -> ApiResult<Json<ListResponse<Category>>> {
    let items = sqlx::query_as::<_, CategoryRow>(
        "SELECT id, name, slug, created_at FROM categories ORDER BY name",
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::internal)?
    .into_iter()
    .map(Into::into)
    .collect();
    Ok(Json(ListResponse { items }))
}

async fn create_category(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload_result: Result<Json<CategoryInput>, JsonRejection>,
) -> ApiResult<impl IntoResponse> {
    let user = require_user(&state, &headers).await?;
    if user.role != "admin" {
        return Err(ApiError::forbidden("administrator required"));
    }
    let input = payload(payload_result)?;
    let name = input.name.trim().to_string();
    if !(2..=80).contains(&name.chars().count()) {
        return Err(ApiError::unprocessable("invalid category name"));
    }
    let slug = normalize_slug(&input.slug)
        .map_err(|_| ApiError::unprocessable("invalid category slug"))?;
    let id = Uuid::new_v4();
    let result = sqlx::query(
        "INSERT INTO categories (id, name, slug, creator_id) VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(name)
    .bind(slug)
    .bind(parse_id(&user.id, "user not found")?)
    .execute(&state.db)
    .await;
    match result {
        Ok(_) => Ok((
            StatusCode::CREATED,
            Json(IdResponse { id: id.to_string() }),
        )),
        Err(error) if unique_violation(&error) => {
            Err(ApiError::conflict("category slug already exists"))
        }
        Err(error) => Err(ApiError::internal(error)),
    }
}

async fn list_topics(State(state): State<AppState>) -> ApiResult<Json<ListResponse<Topic>>> {
    let items = sqlx::query_as::<_, TopicRow>(
        "SELECT t.id, t.category_id, t.author_id, u.username AS author_username, \
         t.title, t.created_at, t.updated_at, \
         CAST((SELECT COUNT(*) FROM posts p WHERE p.topic_id = t.id) AS INTEGER) AS post_count \
         FROM topics t JOIN users u ON u.id = t.author_id \
         ORDER BY t.updated_at DESC LIMIT 50",
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::internal)?
    .into_iter()
    .map(Into::into)
    .collect();
    Ok(Json(ListResponse { items }))
}

async fn create_topic(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload_result: Result<Json<TopicInput>, JsonRejection>,
) -> ApiResult<impl IntoResponse> {
    let user = require_user(&state, &headers).await?;
    let input = payload(payload_result)?;
    let title = validate_title(&input.title)
        .map_err(|_| ApiError::unprocessable("invalid title"))?;
    let body = validate_body(&input.body)
        .map_err(|_| ApiError::unprocessable("invalid body"))?;
    let category_id = parse_id(&input.category_id, "category not found")?;
    let category_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM categories WHERE id = $1)",
    )
    .bind(category_id)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::internal)?;
    if !category_exists {
        return Err(ApiError::not_found("category not found"));
    }

    let topic_id = Uuid::new_v4();
    let author_id = parse_id(&user.id, "user not found")?;
    let mut transaction = state.db.begin().await.map_err(ApiError::internal)?;
    sqlx::query(
        "INSERT INTO topics (id, category_id, author_id, title) VALUES ($1, $2, $3, $4)",
    )
    .bind(topic_id)
    .bind(category_id)
    .bind(author_id)
    .bind(title)
    .execute(&mut *transaction)
    .await
    .map_err(ApiError::internal)?;
    sqlx::query(
        "INSERT INTO posts (id, topic_id, author_id, body, position) VALUES ($1, $2, $3, $4, 1)",
    )
    .bind(Uuid::new_v4())
    .bind(topic_id)
    .bind(author_id)
    .bind(body)
    .execute(&mut *transaction)
    .await
    .map_err(ApiError::internal)?;
    transaction.commit().await.map_err(ApiError::internal)?;

    Ok((
        StatusCode::CREATED,
        Json(IdResponse {
            id: topic_id.to_string(),
        }),
    ))
}

async fn topic_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<TopicDetail>> {
    let id = parse_id(&id, "topic not found")?;
    let topic = sqlx::query_as::<_, TopicRow>(
        "SELECT t.id, t.category_id, t.author_id, u.username AS author_username, \
         t.title, t.created_at, t.updated_at, \
         CAST((SELECT COUNT(*) FROM posts p WHERE p.topic_id = t.id) AS INTEGER) AS post_count \
         FROM topics t JOIN users u ON u.id = t.author_id WHERE t.id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::not_found("topic not found"))?
    .into();
    let posts = sqlx::query_as::<_, PostRow>(
        "SELECT p.id, p.topic_id, p.author_id, u.username AS author_username, \
         p.body, p.position, p.created_at, p.updated_at \
         FROM posts p JOIN users u ON u.id = p.author_id \
         WHERE p.topic_id = $1 ORDER BY p.position",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::internal)?
    .into_iter()
    .map(Into::into)
    .collect::<Vec<Post>>();
    Ok(Json(TopicDetail { topic, posts }))
}

async fn create_post(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    payload_result: Result<Json<PostInput>, JsonRejection>,
) -> ApiResult<impl IntoResponse> {
    let user = require_user(&state, &headers).await?;
    let topic_id = parse_id(&id, "topic not found")?;
    let input = payload(payload_result)?;
    let body = validate_body(&input.body)
        .map_err(|_| ApiError::unprocessable("invalid body"))?;

    let mut transaction = state.db.begin().await.map_err(ApiError::internal)?;
    let topic_exists = sqlx::query_scalar::<_, Uuid>("SELECT id FROM topics WHERE id = $1 FOR UPDATE")
        .bind(topic_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(ApiError::internal)?
        .is_some();
    if !topic_exists {
        return Err(ApiError::not_found("topic not found"));
    }
    let position = sqlx::query_scalar::<_, i32>(
        "SELECT COALESCE(MAX(position), 0) + 1 FROM posts WHERE topic_id = $1",
    )
    .bind(topic_id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(ApiError::internal)?;
    let post_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts (id, topic_id, author_id, body, position) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(post_id)
    .bind(topic_id)
    .bind(parse_id(&user.id, "user not found")?)
    .bind(body)
    .bind(position)
    .execute(&mut *transaction)
    .await
    .map_err(ApiError::internal)?;
    sqlx::query("UPDATE topics SET updated_at = CURRENT_TIMESTAMP WHERE id = $1")
        .bind(topic_id)
        .execute(&mut *transaction)
        .await
        .map_err(ApiError::internal)?;
    transaction.commit().await.map_err(ApiError::internal)?;

    Ok((
        StatusCode::CREATED,
        Json(IdResponse {
            id: post_id.to_string(),
        }),
    ))
}
