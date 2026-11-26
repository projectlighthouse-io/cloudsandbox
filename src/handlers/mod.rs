use inertia::Page;
use route::Cx;

use crate::{error::AppError, responses::home::Home, state::App};

pub async fn home(cx: Cx<App>) -> Page<Home> {
    inertia::render("home", Home { app_name: cx.cfg.app_name.clone() })
}

pub async fn health(cx: Cx<App>) -> Result<&'static str, AppError> {
    sqlx::query("select 1").execute(cx.db.pool()).await?;
    Ok("ok")
}
pub mod lab;
