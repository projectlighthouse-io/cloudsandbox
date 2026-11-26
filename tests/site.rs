//! The docs site is served from `docs/dist`. Build it first:
//! `cargo run --manifest-path docs/Cargo.toml`.

use std::sync::Arc;

use cloudsandbox::{config::Config, state::App};
use db::Db;
use event::Events;
use http_body_util::BodyExt;
use queue::Queue;
use route::http::{Request, StatusCode};
use securecookie::{Codec, Keys};
use session::MemoryStore;

async fn get(uri: &str) -> (StatusCode, String) {
    let key = Keys::generate();
    let cfg = Config { app_name: "cloudsandbox".into(), app_key: key.clone(), database_url: "postgres://unused@127.0.0.1:1/unused".into() };
    let pool = sqlx::postgres::PgPoolOptions::new().connect_lazy(&cfg.database_url).unwrap();
    let app = App::new(cfg, Db::from_pool(pool.clone()), Queue::new(queue::PostgresStore::new(pool)), Arc::new(Events::builder().build()));
    let routes = cloudsandbox::router(&app, MemoryStore::new(), Codec::new(Keys::parse(&key).unwrap())).unwrap();
    let res = routes.handle(Request::get(uri).body(route::empty()).unwrap()).await;
    let status = res.status();
    (status, String::from_utf8(res.into_body().collect().await.unwrap().to_bytes().to_vec()).unwrap())
}

#[tokio::test]
async fn serves_the_docs_site() {
    assert!(std::path::Path::new(cloudsandbox::SITE_DIR).join("index.html").exists(), "docs/dist is missing: build the docs first");
    let (status, body) = get("/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Opinionated Rust batteries."));
    assert_eq!(get("/docs/crates/route/").await.0, StatusCode::OK);
    assert_eq!(get("/site.css").await.0, StatusCode::OK);
    assert_eq!(get("/servers").await.0, StatusCode::NOT_FOUND);
}
