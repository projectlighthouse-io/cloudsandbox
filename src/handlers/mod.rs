use route::Cx;

use crate::{error::AppError, state::App};

pub async fn health(cx: Cx<App>) -> Result<&'static str, AppError> {
    sqlx::query("select 1").execute(cx.db.pool()).await?;
    Ok("ok")
}
