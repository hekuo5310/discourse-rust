use anyhow::Context;
use redis::aio::ConnectionManager;
use sqlx::{migrate::Migrator, postgres::PgPoolOptions, PgPool};

static MIGRATOR: Migrator = sqlx::migrate!("../../migrations/postgres");

#[derive(Clone)]
pub struct AppState {
    pub(crate) db: PgPool,
    pub(crate) cache: ConnectionManager,
}

impl AppState {
    pub async fn connect(database_url: &str, redis_url: &str) -> anyhow::Result<Self> {
        let db = PgPoolOptions::new()
            .max_connections(20)
            .connect(database_url)
            .await
            .context("failed to connect to PostgreSQL")?;
        MIGRATOR
            .run(&db)
            .await
            .context("failed to migrate PostgreSQL")?;

        let client = redis::Client::open(redis_url).context("invalid Redis URL")?;
        let cache = ConnectionManager::new(client)
            .await
            .context("failed to connect to Redis")?;

        Ok(Self { db, cache })
    }

    pub async fn reset_for_test(&self) -> anyhow::Result<()> {
        sqlx::query("TRUNCATE TABLE posts, topics, categories, sessions, users CASCADE")
            .execute(&self.db)
            .await
            .context("failed to reset PostgreSQL test data")?;
        Ok(())
    }
}
