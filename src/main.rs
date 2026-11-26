use std::sync::Arc;

use cloudsandbox::{
    commands,
    config::Config,
    error::AppError,
    events,
    state::{App, Primary},
};
use db::Db;
use event::Events;
use queue::Queue;
use securecookie::{Codec, Keys};
use session::PostgresStore;
use tracing_subscriber::EnvFilter;
use vite::Vite;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cfg = Config::from_env()?;
    let codec = Codec::new(Keys::parse(&cfg.app_key).map_err(|e| AppError::Config(format!("APP_KEY: {e}")))?);
    let vite = Vite::load("public", &["resources/js/app.ts"]).map_err(|e| AppError::Config(e.to_string()))?;

    let db: Db<Primary> = Db::connect(&cfg.database_url).await?;
    let sessions = PostgresStore::new(db.pool().clone());

    let queue = Queue::new(queue::PostgresStore::new(db.pool().clone()));
    let events = Arc::new(events::register(Events::builder().queue(&queue)).build());

    let app = App::new(cfg, db, queue.clone(), events.clone());
    let router = cloudsandbox::router(&app, sessions.clone(), codec, vite)?;

    commands::register(&router, sessions, queue, events).run(app, std::env::args()).await;
    Ok(())
}
