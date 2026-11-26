use serde::Serialize;
use ts_rs::TS;

/// Props for `resources/js/pages/lab/basics.vue`; its TypeScript type is generated.
#[derive(Serialize, TS)]
#[ts(export)]
pub struct Basics {
    pub server_time: String,
    pub renders: u64,
    pub tags: Vec<String>,
}
