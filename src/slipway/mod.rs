//! Slipway: a server-management dashboard demonstrating Inertia, one feature per page.

pub mod data;
pub mod handlers;

use route::Router;

use crate::state::App;

pub use data::Store;

/// Slipway's routes, at the root of the app.
pub fn register(r: &mut Router<App>) {
    use handlers as h;
    r.get("/", h::overview).name("overview");
    r.get("/servers", h::servers).name("servers");
    r.get("/servers/{id}", h::server).name("server");
    r.get("/servers/{id}/sites", h::server_sites).name("server.sites");
    r.get("/servers/{id}/metrics", h::server_metrics).name("server.metrics");
    r.get("/servers/{id}/scheduler", h::server_scheduler).name("server.scheduler");
    r.get("/servers/{id}/workers", h::server_workers).name("server.workers");
    r.get("/servers/{id}/logs", h::server_logs).name("server.logs");
    r.post("/servers/{id}/reboot", h::reboot).name("server.reboot");
    r.get("/sites", h::sites).name("sites");
    r.get("/sites/create", h::create_site).name("sites.create");
    // Room for a certificate upload, to watch the progress bar.
    r.post("/sites", h::store_site).name("sites.store").body_limit(25 * 1024 * 1024);
    r.get("/sites/{id}/edit", h::edit_site).name("sites.edit");
    r.put("/sites/{id}", h::update_site).name("sites.update").body_limit(25 * 1024 * 1024);
    r.post("/sites/{id}", h::update_site).body_limit(25 * 1024 * 1024);
    r.get("/deployments", h::deployments).name("deployments");
    r.post("/deployments", h::deploy).name("deployments.store");
    r.post("/deployments/{id}/cancel", h::cancel).name("deployments.cancel");
    r.get("/activity", h::activity).name("activity");
    r.get("/settings", h::settings).name("settings");
    r.get("/settings/profile", h::settings_profile).name("settings.profile");
    r.put("/settings/profile", h::update_profile).name("settings.profile.update");
    r.patch("/settings/members/{id}", h::update_member).name("settings.members.update");
    r.delete("/settings/members/{id}", h::remove_member).name("settings.members.remove");
    r.post("/settings/tokens", h::create_token).name("settings.tokens.store");
    r.delete("/settings/tokens/{id}", h::revoke_token).name("settings.tokens.revoke");
    r.post("/teams/{id}", h::switch_team).name("teams.switch");
    r.post("/logout", h::logout).name("logout");
    r.get("/palette", h::palette).name("palette");
    r.get("/errors/{status}", h::error_page).name("errors");
}
