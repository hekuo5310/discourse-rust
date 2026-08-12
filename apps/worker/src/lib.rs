use forum_core::{
    hash_password, new_id, new_session_token, normalize_email, normalize_slug, normalize_username,
    token_digest, validate_body, validate_title, verify_password, ApiError, Category,
    CategoryInput, LoginInput, Post, PostInput, RegisterInput, SessionResponse, Topic, TopicDetail,
    TopicInput, User,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use worker::*;

const SESSION_TTL_SECONDS: u64 = 60 * 60 * 24 * 7;

#[derive(Debug, Deserialize)]
struct UserWithPassword {
    id: String,
    username: String,
    email: String,
    role: String,
    password_hash: String,
    created_at: String,
}

impl UserWithPassword {
    fn public(&self) -> User {
        User {
            id: self.id.clone(),
            username: self.username.clone(),
            email: self.email.clone(),
            role: self.role.clone(),
            created_at: self.created_at.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
struct IdResponse {
    id: String,
}

#[derive(Debug, Serialize)]
struct ListResponse<T> {
    items: Vec<T>,
}

fn json<T: Serialize>(value: &T, status: u16) -> Result<Response> {
    Ok(Response::from_json(value)?.with_status(status))
}

fn api_error(message: &'static str, status: u16) -> Result<Response> {
    json(&ApiError { error: message }, status)
}

fn binding_values(values: &[&str]) -> Vec<JsValue> {
    values
        .iter()
        .map(|value| JsValue::from_str(value))
        .collect()
}

fn bearer(req: &Request) -> Result<Option<String>> {
    Ok(req
        .headers()
        .get("Authorization")?
        .and_then(|value| value.strip_prefix("Bearer ").map(str::to_owned))
        .filter(|value| value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())))
}

async fn current_user(req: &Request, env: &Env) -> Result<Option<User>> {
    let Some(token) = bearer(req)? else {
        return Ok(None);
    };
    let digest = token_digest(&token);

    // KV is deliberately only a latency hint. Session revocation and expiry are
    // always checked against D1 before a user is authenticated.
    let cache = env.kv("CACHE")?;
    let _hint = cache.get(&format!("session:{digest}")).text().await?;

    env.d1("DB")?
        .prepare(
            "SELECT u.id, u.username, u.email, u.role, u.created_at \
             FROM sessions s JOIN users u ON u.id = s.user_id \
             WHERE s.token_hash = ?1 AND s.expires_at > CURRENT_TIMESTAMP",
        )
        .bind(&binding_values(&[&digest]))?
        .first::<User>(None)
        .await
}

async fn require_user(req: &Request, env: &Env) -> Result<std::result::Result<User, Response>> {
    Ok(match current_user(req, env).await? {
        Some(user) => Ok(user),
        None => Err(api_error("authentication required", 401)?),
    })
}

async fn create_session(env: &Env, user: User) -> Result<SessionResponse> {
    let token = new_session_token();
    let digest = token_digest(&token);
    let id = new_id();
    env.d1("DB")?
        .prepare(
            "INSERT INTO sessions (id, user_id, token_hash, expires_at) \
             VALUES (?1, ?2, ?3, datetime('now', '+7 days'))",
        )
        .bind(&binding_values(&[&id, &user.id, &digest]))?
        .run()
        .await?;
    env.kv("CACHE")?
        .put(&format!("session:{digest}"), &user.id)?
        .expiration_ttl(SESSION_TTL_SECONDS)
        .execute()
        .await?;
    Ok(SessionResponse { token, user })
}

async fn register(mut req: Request, env: Env) -> Result<Response> {
    let input: RegisterInput = match req.json().await {
        Ok(value) => value,
        Err(_) => return api_error("invalid JSON", 400),
    };
    let username = match normalize_username(&input.username) {
        Ok(value) => value,
        Err(_) => return api_error("invalid username", 422),
    };
    let email = match normalize_email(&input.email) {
        Ok(value) => value,
        Err(_) => return api_error("invalid email", 422),
    };
    let password_hash = match hash_password(&input.password) {
        Ok(value) => value,
        Err(_) => return api_error("password must contain 12 to 256 bytes", 422),
    };
    let id = new_id();
    let statement = env
        .d1("DB")?
        .prepare(
            "INSERT INTO users (id, username, email, password_hash, role) \
             VALUES (?1, ?2, ?3, ?4, \
             CASE WHEN EXISTS (SELECT 1 FROM users) THEN 'member' ELSE 'admin' END)",
        )
        .bind(&binding_values(&[&id, &username, &email, &password_hash]))?;
    if statement.run().await.is_err() {
        return api_error("username or email already exists", 409);
    }
    let user = env
        .d1("DB")?
        .prepare("SELECT id, username, email, role, created_at FROM users WHERE id = ?1")
        .bind(&binding_values(&[&id]))?
        .first::<User>(None)
        .await?
        .ok_or_else(|| Error::RustError("new user disappeared".into()))?;
    json(&create_session(&env, user).await?, 201)
}

async fn login(mut req: Request, env: Env) -> Result<Response> {
    let input: LoginInput = match req.json().await {
        Ok(value) => value,
        Err(_) => return api_error("invalid JSON", 400),
    };
    let login = input.login.trim().to_ascii_lowercase();
    let user = env
        .d1("DB")?
        .prepare(
            "SELECT id, username, email, role, password_hash, created_at \
             FROM users WHERE username = ?1 OR email = ?1",
        )
        .bind(&binding_values(&[&login]))?
        .first::<UserWithPassword>(None)
        .await?;
    let Some(user) = user.filter(|user| verify_password(&input.password, &user.password_hash))
    else {
        return api_error("invalid credentials", 401);
    };
    json(&create_session(&env, user.public()).await?, 200)
}

async fn logout(req: Request, env: Env) -> Result<Response> {
    let Some(token) = bearer(&req)? else {
        return api_error("authentication required", 401);
    };
    let digest = token_digest(&token);
    env.d1("DB")?
        .prepare("DELETE FROM sessions WHERE token_hash = ?1")
        .bind(&binding_values(&[&digest]))?
        .run()
        .await?;
    env.kv("CACHE")?
        .delete(&format!("session:{digest}"))
        .await?;
    Response::empty().map(|response| response.with_status(204))
}

async fn me(req: Request, env: Env) -> Result<Response> {
    match require_user(&req, &env).await? {
        Ok(user) => json(&user, 200),
        Err(response) => Ok(response),
    }
}

async fn list_categories(env: Env) -> Result<Response> {
    let result = env
        .d1("DB")?
        .prepare("SELECT id, name, slug, created_at FROM categories ORDER BY name")
        .all()
        .await?;
    json(
        &ListResponse {
            items: result.results::<Category>()?,
        },
        200,
    )
}

async fn create_category(mut req: Request, env: Env) -> Result<Response> {
    let user = match require_user(&req, &env).await? {
        Ok(user) if user.role == "admin" => user,
        Ok(_) => return api_error("administrator required", 403),
        Err(response) => return Ok(response),
    };
    let input: CategoryInput = match req.json().await {
        Ok(value) => value,
        Err(_) => return api_error("invalid JSON", 400),
    };
    let name = input.name.trim().to_string();
    if !(2..=80).contains(&name.chars().count()) {
        return api_error("invalid category name", 422);
    }
    let slug = match normalize_slug(&input.slug) {
        Ok(value) => value,
        Err(_) => return api_error("invalid category slug", 422),
    };
    let id = new_id();
    let statement = env
        .d1("DB")?
        .prepare("INSERT INTO categories (id, name, slug, creator_id) VALUES (?1, ?2, ?3, ?4)")
        .bind(&binding_values(&[&id, &name, &slug, &user.id]))?;
    if statement.run().await.is_err() {
        return api_error("category slug already exists", 409);
    }
    json(&IdResponse { id }, 201)
}

async fn list_topics(env: Env) -> Result<Response> {
    let result = env
        .d1("DB")?
        .prepare(
            "SELECT t.id, t.category_id, t.author_id, u.username AS author_username, \
             t.title, t.created_at, t.updated_at, \
             CAST((SELECT COUNT(*) FROM posts p WHERE p.topic_id = t.id) AS INTEGER) AS post_count \
             FROM topics t JOIN users u ON u.id = t.author_id \
             ORDER BY t.updated_at DESC LIMIT 50",
        )
        .all()
        .await?;
    json(
        &ListResponse {
            items: result.results::<Topic>()?,
        },
        200,
    )
}

async fn create_topic(mut req: Request, env: Env) -> Result<Response> {
    let user = match require_user(&req, &env).await? {
        Ok(user) => user,
        Err(response) => return Ok(response),
    };
    let input: TopicInput = match req.json().await {
        Ok(value) => value,
        Err(_) => return api_error("invalid JSON", 400),
    };
    let title = match validate_title(&input.title) {
        Ok(value) => value,
        Err(_) => return api_error("invalid title", 422),
    };
    let body = match validate_body(&input.body) {
        Ok(value) => value,
        Err(_) => return api_error("invalid body", 422),
    };
    let category_exists = env
        .d1("DB")?
        .prepare("SELECT id FROM categories WHERE id = ?1")
        .bind(&binding_values(&[&input.category_id]))?
        .first::<String>(Some("id"))
        .await?
        .is_some();
    if !category_exists {
        return api_error("category not found", 404);
    }
    let topic_id = new_id();
    let post_id = new_id();
    let db = env.d1("DB")?;
    db.batch(vec![
        db.prepare("INSERT INTO topics (id, category_id, author_id, title) VALUES (?1, ?2, ?3, ?4)")
            .bind(&binding_values(&[&topic_id, &input.category_id, &user.id, &title]))?,
        db.prepare("INSERT INTO posts (id, topic_id, author_id, body, position) VALUES (?1, ?2, ?3, ?4, 1)")
            .bind(&binding_values(&[&post_id, &topic_id, &user.id, &body]))?,
    ])
    .await?;
    json(&IdResponse { id: topic_id }, 201)
}

async fn topic_detail(id: &str, env: Env) -> Result<Response> {
    let db = env.d1("DB")?;
    let topic = db
        .prepare(
            "SELECT t.id, t.category_id, t.author_id, u.username AS author_username, \
             t.title, t.created_at, t.updated_at, \
             CAST((SELECT COUNT(*) FROM posts p WHERE p.topic_id = t.id) AS INTEGER) AS post_count \
             FROM topics t JOIN users u ON u.id = t.author_id WHERE t.id = ?1",
        )
        .bind(&binding_values(&[id]))?
        .first::<Topic>(None)
        .await?;
    let Some(topic) = topic else {
        return api_error("topic not found", 404);
    };
    let posts = db
        .prepare(
            "SELECT p.id, p.topic_id, p.author_id, u.username AS author_username, \
             p.body, p.position, p.created_at, p.updated_at \
             FROM posts p JOIN users u ON u.id = p.author_id \
             WHERE p.topic_id = ?1 ORDER BY p.position",
        )
        .bind(&binding_values(&[id]))?
        .all()
        .await?
        .results::<Post>()?;
    json(&TopicDetail { topic, posts }, 200)
}

async fn create_post(id: &str, mut req: Request, env: Env) -> Result<Response> {
    let user = match require_user(&req, &env).await? {
        Ok(user) => user,
        Err(response) => return Ok(response),
    };
    let input: PostInput = match req.json().await {
        Ok(value) => value,
        Err(_) => return api_error("invalid JSON", 400),
    };
    let body = match validate_body(&input.body) {
        Ok(value) => value,
        Err(_) => return api_error("invalid body", 422),
    };
    let post_id = new_id();
    let db = env.d1("DB")?;
    let result = db
        .prepare(
            "INSERT INTO posts (id, topic_id, author_id, body, position) \
             SELECT ?1, ?2, ?3, ?4, COALESCE(MAX(position), 0) + 1 \
             FROM posts WHERE topic_id = ?2 HAVING EXISTS (SELECT 1 FROM topics WHERE id = ?2)",
        )
        .bind(&binding_values(&[&post_id, id, &user.id, &body]))?
        .run()
        .await?;
    let changed = result.meta()?.and_then(|meta| meta.changes).unwrap_or(0);
    if changed == 0 {
        return api_error("topic not found", 404);
    }
    db.prepare("UPDATE topics SET updated_at = CURRENT_TIMESTAMP WHERE id = ?1")
        .bind(&binding_values(&[id]))?
        .run()
        .await?;
    json(&IdResponse { id: post_id }, 201)
}

#[event(fetch, respond_with_errors)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    Router::new()
        .get("/api/v1/health", |_, _| {
            json(&serde_json::json!({ "status": "ok" }), 200)
        })
        .post_async("/api/v1/auth/register", |req, ctx| {
            register(req, ctx.env).await
        })
        .post_async("/api/v1/auth/login", |req, ctx| login(req, ctx.env).await)
        .post_async("/api/v1/auth/logout", |req, ctx| logout(req, ctx.env).await)
        .get_async("/api/v1/me", |req, ctx| me(req, ctx.env).await)
        .get_async("/api/v1/categories", |_, ctx| {
            list_categories(ctx.env).await
        })
        .post_async("/api/v1/categories", |req, ctx| {
            create_category(req, ctx.env).await
        })
        .get_async("/api/v1/topics", |_, ctx| list_topics(ctx.env).await)
        .post_async("/api/v1/topics", |req, ctx| {
            create_topic(req, ctx.env).await
        })
        .get_async("/api/v1/topics/:id", |_, ctx| {
            let id = ctx.param("id").cloned().unwrap_or_default();
            async move { topic_detail(&id, ctx.env).await }
        })
        .post_async("/api/v1/topics/:id/posts", |req, ctx| {
            let id = ctx.param("id").cloned().unwrap_or_default();
            async move { create_post(&id, req, ctx.env).await }
        })
        .run(req, env)
        .await
}
