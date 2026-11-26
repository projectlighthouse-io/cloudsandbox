use event::EventsBuilder;

use crate::state::App;

/// Every listener. Add `.on(my_event::on_my_event)` or
/// `.on_queued("name", my_event::on_my_event)` per listener.
pub fn register(b: EventsBuilder<App>) -> EventsBuilder<App> {
    b
}
