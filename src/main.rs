mod actions;
mod commands;
mod config;
mod error;
mod events;
mod handlers;
mod jobs;
mod middleware;
mod requests;
mod responses;
mod routes;
mod state;

use std::sync::Arc;

use db::Db;
use event::Events;
use queue::Queue;
use route::Router;
use securecookie::{Codec, Keys};
use session::{PostgresStore, Session};
use tower_http::services::ServeDir;
use vite::Vite;
use tracing_subscriber::EnvFilter;

use crate::{config::Config, error::AppError, state::{App, Primary}};

#[tokio::main]
async fn main() -> Result<(), AppError> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cfg = Config::from_env()?;
    let codec = Codec::new(Keys::parse(&cfg.app_key).map_err(|e| AppError::Config(format!("APP_KEY: {e}")))?);

    let vite = Vite::load("public", &["resources/js/app.ts"]).map_err(|e| AppError::Config(e.to_string()))?;
    let asset_version = vite.manifest_hash().to_owned();

    let db: Db<Primary> = Db::connect(&cfg.database_url).await?;
    let sessions = PostgresStore::new(db.pool().clone());

    let queue = Queue::new(queue::PostgresStore::new(db.pool().clone()));
    let events = Arc::new(events::register(Events::builder().queue(&queue)).build());

    let app = App::new(cfg, db, queue.clone(), events.clone());

    let mut pages = inertia::layer()
        .root_template("resources/views/app.html")
        .pages_dir("resources/js/pages")
        .version(&asset_version)
        .head(move || vite.tags())
        .share({
            let app_name = app.cfg.app_name.clone();
            move |ext| {
                let session = ext.get::<Session>();
                responses::lab::Shared {
                    app_name: app_name.clone(),
                    flash: session.and_then(|s| s.get::<String>("status").ok().flatten()),
                    csrf_token: session.and_then(|s| s.get::<String>("_token").ok().flatten()),
                }
            }
        });
    if let Some(url) = &app.cfg.ssr_url {
        pages = pages.ssr(url);
    }

    let router = routes::register(Router::new())
        .layer(session::layer(sessions.clone(), codec))
        .layer(csrf::layer::<Session>())
        .layer(pages)
        .fallback(ServeDir::new("public"))
        .build(app.clone())?;

    commands::register(&router, sessions, queue, events).run(app, std::env::args()).await;
    Ok(())
}
