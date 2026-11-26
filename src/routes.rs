use route::Router;

use crate::{handlers, state::App};

/// Every route in one place. Everything else is served from the docs site.
pub fn register(mut r: Router<App>) -> Router<App> {
    r.get("/health", handlers::health).name("health");
    r
}
