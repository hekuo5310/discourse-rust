use std::env;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use forum_core::{IdResponse, SessionResponse, TopicDetail};
use forum_native::{router, AppState};
use http_body_util::BodyExt;
use serde::{de::DeserializeOwned, Serialize};
use tower::ServiceExt;

#[tokio::test]
async fn complete_forum_flow() {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set for this test");
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set for this test");
    let state = AppState::connect(&database_url, &redis_url)
        .await
        .expect("test services must be reachable");
    state
        .reset_for_test()
        .await
        .expect("test data should reset");
    let app = router(state);

    let (status, session): (_, SessionResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/auth/register",
        None,
        Some(&serde_json::json!({
            "username": "founder",
            "email": "founder@example.com",
            "password": "correct horse battery staple"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(session.user.role, "admin");

    let (status, category): (_, IdResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/categories",
        Some(&session.token),
        Some(&serde_json::json!({ "name": "General", "slug": "general" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, topic): (_, IdResponse) = request_json(
        &app,
        Method::POST,
        "/api/v1/topics",
        Some(&session.token),
        Some(&serde_json::json!({
            "category_id": category.id,
            "title": "The first topic",
            "body": "A clean-room Rust forum is taking shape."
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, detail): (_, TopicDetail) = request_json(
        &app,
        Method::GET,
        &format!("/api/v1/topics/{}", topic.id),
        None,
        None::<&serde_json::Value>,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail.posts.len(), 1);

    let (status, _post): (_, IdResponse) = request_json(
        &app,
        Method::POST,
        &format!("/api/v1/topics/{}/posts", topic.id),
        Some(&session.token),
        Some(&serde_json::json!({ "body": "This is the first reply." })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (_, detail): (_, TopicDetail) = request_json(
        &app,
        Method::GET,
        &format!("/api/v1/topics/{}", topic.id),
        None,
        None::<&serde_json::Value>,
    )
    .await;
    assert_eq!(detail.topic.post_count, 2);
    assert_eq!(detail.posts.len(), 2);
    assert_eq!(detail.posts[1].position, 2);

    let status = request_empty(
        &app,
        Method::POST,
        "/api/v1/auth/logout",
        Some(&session.token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let status = request_empty(&app, Method::GET, "/api/v1/me", Some(&session.token)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

async fn request_json<T: DeserializeOwned, B: Serialize + ?Sized>(
    app: &Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Option<&B>,
) -> (StatusCode, T) {
    let request = build_request(method, uri, token, body);
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("request succeeds");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("response body is readable")
        .to_bytes();
    let decoded = serde_json::from_slice(&bytes).expect("response contains expected JSON");
    (status, decoded)
}

async fn request_empty(app: &Router, method: Method, uri: &str, token: Option<&str>) -> StatusCode {
    app.clone()
        .oneshot(build_request(
            method,
            uri,
            token,
            None::<&serde_json::Value>,
        ))
        .await
        .expect("request succeeds")
        .status()
}

fn build_request<B: Serialize + ?Sized>(
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Option<&B>,
) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let bytes = match body {
        Some(body) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            serde_json::to_vec(body).expect("test body serializes")
        }
        None => Vec::new(),
    };
    builder.body(Body::from(bytes)).expect("valid test request")
}
