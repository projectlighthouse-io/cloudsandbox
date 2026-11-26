use std::sync::Arc;

use db::Db;
use event::Events;
use queue::{PostgresStore, Queue};

use crate::config::Config;

db::handle!(pub Primary: Postgres);

/// Everything a handler or command needs. Cheap to clone.
#[derive(Clone)]
pub struct App {
    pub cfg: Arc<Config>,
    pub db: Db<Primary>,
    // Read once the app pushes jobs or emits events.
    #[allow(dead_code)]
    pub queue: Queue<PostgresStore>,
    #[allow(dead_code)]
    pub events: Arc<Events<App>>,
    /// Slipway's in-memory mock data.
    pub slipway: crate::slipway::Store,
}

impl App {
    pub fn new(cfg: Config, db: Db<Primary>, queue: Queue<PostgresStore>, events: Arc<Events<App>>) -> Self {
        Self { cfg: Arc::new(cfg), db, queue, events, slipway: crate::slipway::Store::seeded() }
    }
}
