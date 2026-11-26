use std::sync::Arc;

use command::Commands;
use event::Events;
use queue::Queue;
use route::Routes;

use crate::state::App;

/// Every command the binary runs. `serve` runs when none is given.
pub fn register(
    routes: &Routes,
    sessions: session::PostgresStore,
    queue: Queue<queue::PostgresStore>,
    events: Arc<Events<App>>,
) -> Commands<App> {
    Commands::new()
        .default("serve", route::command::serve(routes))
        .add("route:list", route::command::list(routes))
        .add("config:show", crate::config::show)
        .add("session:prune", session::command::prune(sessions))
        .add("event:list", event::command::list(&events))
        .add("queue:retry", queue::command::retry(queue.clone()))
        .add("queue:work", queue::command::work(queue, move |w| crate::jobs::register(events.register_jobs(w))))
}
