use serde::Serialize;
use ts_rs::TS;

/// Props on every page (see `share` in main.rs).
#[derive(Serialize)]
pub struct Shared {
    pub app_name: String,
    /// One-request flash message, set by handlers with `session.flash("status", ..)`.
    pub flash: Option<String>,
    /// For plain HTML forms, which send it back as `_token`.
    pub csrf_token: Option<String>,
}

/// Props for `resources/js/pages/lab/basics.vue`; its TypeScript type is generated.
#[derive(Serialize, TS)]
#[ts(export)]
pub struct Basics {
    pub server_time: String,
    pub renders: u64,
    pub tags: Vec<String>,
}
