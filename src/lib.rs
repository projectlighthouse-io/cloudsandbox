//! cloudsandbox: a sandbox app for the foundry toolkit. `main.rs` wires the
//! process; [`router`] builds the HTTP stack, so tests serve exactly what
//! production serves.

pub mod actions;
pub mod commands;
pub mod config;
pub mod error;
pub mod events;
pub mod handlers;
pub mod jobs;
pub mod middleware;
pub mod requests;
pub mod responses;
pub mod routes;
pub mod state;

use route::Routes;
use securecookie::Codec;
use tower_http::services::{ServeDir, ServeFile};

use crate::{error::AppError, state::App};

/// The docs site, built into `docs/dist` by `cargo run --manifest-path docs/Cargo.toml`.
pub const SITE_DIR: &str = "docs/dist";

/// Routes plus the session layer; any other URL is a file from the docs site,
/// or its 404 page with a 404 status.
pub fn router<St: session::Store>(app: &App, sessions: St, codec: Codec) -> Result<Routes, AppError> {
    Ok(routes::register(route::Router::new())
        .layer(session::layer(sessions, codec))
        .fallback(ServeDir::new(SITE_DIR).not_found_service(ServeFile::new(format!("{SITE_DIR}/404.html"))))
        .build(app.clone())?)
}
