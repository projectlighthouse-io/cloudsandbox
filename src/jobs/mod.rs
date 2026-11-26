use queue::{PostgresStore, Worker};

use crate::state::App;

/// Every job `queue:work` runs. Add `.register::<my_job::MyJob>()` per job.
pub fn register(w: Worker<PostgresStore, App>) -> Worker<PostgresStore, App> {
    w
}
