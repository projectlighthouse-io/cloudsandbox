---
builds_on: `tokio` for `wait`; `route` and `tower` for the middleware; `sqlx` for the SQL stores
version: 0.1.0
---

# throttle

Rate limit anything, per key.

## Introduction

Sooner or later something in your app has to slow down: a login form that
should not take a thousand guesses a minute, an API that allows each client so
many requests, a third-party service that bans you if you call it too often.
`throttle` handles all of these with one type, the `Limiter`.

A limiter pairs an algorithm (how hits are counted) with a store (where the
counts live), and keeps a separate count for every key you give it: an IP
address, an email address, a user id, or the name of an external service.

## Installation

Add the crate from git:

```toml
[dependencies]
throttle = { git = "https://github.com/projectlighthouse-io/toolkit" }
```

Everything beyond the in-memory limiter is behind a feature:

| Feature | Adds |
| --- | --- |
| `route` | `throttle::layer`, route middleware answering `429` |
| `postgres`, `sqlite`, `mysql` | SQL stores, so several servers share one limit |
| `command` | `throttle::command::prune` for `throttle:prune` |

## Limiting anything

Create a limiter with an algorithm, then count a hit whenever the thing you
are limiting happens:

```rust
use std::time::Duration;
use throttle::{FixedWindow, Limiter};

let exports = Limiter::memory(FixedWindow::new(5, Duration::from_secs(60)));

let d = exports.hit(&user_id).await?;
if !d.allowed {
    return Err(format!("try again in {} seconds", d.retry_after.as_secs()));
}
```

`hit` returns a `Decision` with `allowed`, `remaining` and `retry_after`. A
denied hit is not counted, so a client that keeps knocking is not locked out
any longer than the limit says.

| Method | Does |
| --- | --- |
| `hit(key)` | Count one hit |
| `hit_n(key, cost)` | Count a hit worth several, such as a batch call |
| `check(key)` | Say what a hit would get, without counting it |
| `clear(key)` | Forget a key's hits |
| `wait(key)` | Sleep until a hit is allowed, then count it |

`check` and `clear` suit the "count only failures" pattern: check before doing
the work, hit when it fails, clear when it succeeds. That is how
[auth](auth.md) throttles logins.

## Choosing an algorithm

Each algorithm takes the parameters that make sense for it:

| Algorithm | Parameters | Behaviour |
| --- | --- | --- |
| `FixedWindow` | `max`, `window` | The window starts at a key's first hit. Cheapest; allows up to `2 * max` across a window boundary |
| `SlidingWindow` | `max`, `window` | Any `window`-long span, estimated from the current and previous windows. Smooth at boundaries |
| `TokenBucket` | `capacity`, `refill`, `every` | Bursts up to `capacity`, then a steady rate |

```rust
Limiter::memory(SlidingWindow::new(100, Duration::from_secs(60)));        // 100 per minute, smoothly
Limiter::memory(TokenBucket::new(10, 1, Duration::from_secs(1)));         // bursts of 10, then 1 a second
```

To count some other way, implement `Algorithm`. It receives the state it stored
last time and returns a decision plus the new state.

## Calling external services

For a third-party API with a rate limit of its own, use `wait`. It sleeps until
the next call is allowed instead of failing:

```rust
let github = Limiter::memory(TokenBucket::new(10, 1, Duration::from_secs(1)));

github.wait("github").await?;
let repo = client.get("https://api.github.com/repos/acme/shop").send().await?;
```

## Limiting routes

With the `route` feature, `throttle::layer` limits every route in a router or
group. Over the limit, the client gets `429 Too Many Requests` with a
`Retry-After` header:

```rust
r.group("/api", |r| {
    r.layer(throttle::layer("api", Limiter::memory(SlidingWindow::new(60, Duration::from_secs(60))), throttle::by_ip));
    r.get("/runs", runs::index);
});
```

The first argument names the limit, so two layers can share a store without
sharing counts. The last picks the key from the request. `throttle::by_ip` uses
the client address that `route::serve` records; behind a proxy, key by a header
your proxy sets instead:

```rust
fn by_forwarded_for(parts: &Parts) -> Option<String> {
    parts.headers.get("x-forwarded-for")?.to_str().ok().map(str::to_owned)
}
```

A request whose key function returns `None` is not counted.

## Sharing limits across servers

`Limiter::memory` keeps counts in the process, which is right for one server
and for tests. When several servers must share a limit, use a SQL store:

```bash
art publish throttle            # or --driver sqlite / --driver mysql
art migrate
```

```rust
let logins = Limiter::new(throttle::PostgresStore::new(pool.clone()), FixedWindow::new(5, Duration::from_secs(60)));
```

Each hit runs in a short transaction that locks the key's row, so concurrent
hits on one key are all counted. Expired keys are deleted by `throttle:prune`;
schedule it, or run it now and then.
