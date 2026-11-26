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
pub mod slipway;
pub mod state;

use route::{Router, Routes};
use securecookie::Codec;
use session::Session;
use tower_http::services::ServeDir;
use vite::Vite;

use crate::{error::AppError, state::App};

/// Routes plus the session, CSRF and Inertia layers, and static files from `public/`.
pub fn router<St: session::Store>(app: &App, sessions: St, codec: Codec, vite: Vite) -> Result<Routes, AppError> {
    let asset_version = vite.manifest_hash().to_owned();
    let mut pages = inertia::layer()
        .root_template("resources/views/app.html")
        .pages_dir("resources/js/pages")
        .version(&asset_version)
        .head(move || vite.tags())
        .share({
            let (app_name, store) = (app.cfg.app_name.clone(), app.slipway.clone());
            move |ext| slipway::handlers::shared(&store, &app_name, ext.get::<Session>())
        });
    if let Some(url) = &app.cfg.ssr_url {
        pages = pages.ssr(url);
    }

    Ok(routes::register(Router::new())
        .layer(session::layer(sessions, codec))
        .layer(csrf::layer::<Session>())
        .layer(pages)
        .fallback(ServeDir::new("public"))
        .build(app.clone())?)
}
