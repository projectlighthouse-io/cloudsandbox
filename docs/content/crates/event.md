---
builds_on: Nothing by default
version: 0.1.0
---

# event

Dispatch typed events to listeners, in process.

## Introduction

Events let one part of your app announce that something happened without
knowing who cares. You define an event type, register the listeners that should
react to it, and emit it. Some listeners run straight away inside `emit`;
others are turned into jobs and run later by a queue worker.

Everything happens inside your process. The event bus is not a message broker,
so events do not cross process boundaries, and it does not persist events.
When you need durability, use a queued listener.

## Installation

Add the crate from git:

```toml
[dependencies]
event = { git = "https://github.com/projectlighthouse-io/toolkit" }
```

Two optional features connect it to other crates:

| Feature | Adds |
| --- | --- |
| `queue` | `on_queued` |
| `action` | An `Action` whose `Input: From<Event>` can be a listener |

Turn on `queue` to register queued listeners; it pulls in the `queue` crate.
Turn on `action` if you want to register an `Action` as a listener.

## Defining events

An event is any `Clone + Send + 'static` struct:

```rust
#[derive(Clone)]
struct RunFinished { run_id: RunId, status: RunStatus }

let events = Events::builder()
    .on::<RunFinished>(update_commit_status)     // inline
    .on_queued::<RunFinished>(send_notification) // becomes a job
    .build();

events.emit(RunFinished { run_id, status }).await?;
```

Listeners are looked up by the event's type, so there are no string names to
mistype.

## Registering listeners

Build the bus once at startup with `Events::builder()`, adding a listener per
event type with `.on` or `.on_queued`, as shown above. After `build()` the bus
is immutable.

### Inline listeners

Listeners added with `.on` run inside `emit`, in the order you registered them.
`emit` returns the first error and stops, as a function call would. Inline
listeners give no durability.

### Queued listeners

Listeners added with `.on_queued` become jobs. `emit` writes one job per queued
listener and returns without waiting for them. Retries and failure handling are
then the queue's responsibility. Anything that must survive a crash should go
through a queued listener.

## Dispatching events

Call `events.emit(event).await` wherever the thing happened. Inline listeners
have run by the time it returns, and queued listeners have been enqueued.

## Listing events

The event-to-listener table is exported, so you can run `art event:list` to see
which listeners react to each event.
