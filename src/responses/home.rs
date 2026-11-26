use serde::Serialize;
use ts_rs::TS;

/// Props for `resources/js/pages/home.vue`.
#[derive(Serialize, TS)]
#[ts(export)]
pub struct Home {
    pub app_name: String,
}
