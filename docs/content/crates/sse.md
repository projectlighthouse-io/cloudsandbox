---
builds_on: `http`, `http-body`, `tokio`, `futures-core`
phase: 5
---

# sse

Server-sent event responses.

## Introduction

Server-sent events let your app push updates to the browser over a plain HTTP
response, which suits live logs, progress and notifications. This crate turns a
`Stream` of events into a streaming response. It sends keep-alive comments at an
interval you choose, and supports event ids so a reconnecting client can resume
using `Last-Event-ID`.

The crate does not fan out events to subscribers; a broadcast channel in your
app does that. It is not a replacement for WebSockets either: the connection is
one-way, from server to browser.

## Installation

Add the crate from git:

```toml
[dependencies]
sse = { git = "https://github.com/projectlighthouse-io/cloudfoundry" }
```

## Streaming events

Return `Sse` from a handler, built from any stream of events:

```rust
async fn logs(cx: Cx<App>, Path(id): Path<RunId>) -> Result<Sse, AppError> {
    let stream = cx.logs.subscribe(id).map(|line| Event::new("log").json(&line));
    Ok(Sse::new(stream).keep_alive(Duration::from_secs(15)))
}
```

Each `Event` has a name, and `.json(...)` serialises its data. If serialisation
fails, the stream ends; the error is not swallowed.

### Keeping the connection alive

`.keep_alive(...)` sends a comment at the given interval, so proxies and
browsers do not close an idle connection.

## Handling disconnects

When the client disconnects, the stream is dropped and the subscription ends
with it. No task outlives its connection, so you do not need to clean up
yourself.
