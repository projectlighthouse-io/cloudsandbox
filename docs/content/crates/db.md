---
builds_on: `sqlx`, with one database feature
version: 0.2.0
---

# db

Typed pool handles and explicit transactions over `sqlx`.

## Introduction

The db crate is a thin layer over `sqlx`. It gives each connection pool its own
type, runs a closure inside a transaction, and can optionally hold handles in
process-wide slots. Everything else is `sqlx`: your app writes queries with it
directly.

The crate does not wrap, build or abstract queries. Migrations are plain `sqlx`
migrations, run by `art migrate`, and pagination is left to your app for now.

### Why queries stay concrete

Strict types and one query for every database pull against each other:
compile-time checked queries are checked against one database. So the toolkit
does not wrap queries. Support for several databases lives in the crates that
store data (`session`, `queue`), as drivers.

## Installation

Add the crate from git, with the feature for your database:

```toml
[dependencies]
db = { git = "https://github.com/projectlighthouse-io/toolkit" }
```

The available features are `postgres`, `mysql`, `sqlite` and `singleton`.
Turn on one database feature, and add `singleton` if you want process-wide
handles.

## Declaring handles

Declare a handle type for each pool with `db::handle!`, connect it, and pass
its pool to `sqlx`:

```rust
db::handle!(Primary: Postgres);
db::handle!(Analytics: Postgres);

let primary: Db<Primary> = Db::connect(&cfg.database_url).await?;
let analytics: Db<Analytics> = Db::connect(&cfg.analytics_url).await?;

let run = sqlx::query_as!(Run, "select * from runs where id = $1", id)
    .fetch_one(primary.pool())
    .await?;

primary.tx(|tx| async move {
    /* queries on tx */
    Ok(())
}).await?;
```

A single-database app declares one handle. With several, each is its own type,
so passing `Db<Analytics>` where `Db<Primary>` is expected is a compile error.

## Transactions

Call `tx` on a handle and run your queries on the transaction it gives you, as
in the example above.

There is no transaction-per-request middleware. It would hold a connection for
the whole request and hide the cost, so you open a transaction where you need
one.

## Sharing handles

By default you pass a handle explicitly, through app state or an action's
fields. That keeps tests isolated.

### Singleton and multiton

The `singleton` feature adds one process-wide slot per handle type:

```rust
Db::<Primary>::set_singleton(primary)?; // once, at startup
let db = Db::<Primary>::singleton()?;    // anywhere
```

With one handle, that slot is a singleton. With several handles it is a
multiton, still type-checked per handle.

Both return a `SingletonError` that names the handle instead of panicking.
`singleton()` on a slot that was never set returns `NotSet`. Calling
`set_singleton` twice returns `AlreadySet` and keeps the first handle, because a
singleton is set once and never replaced.

## Pool settings

`Db::connect_with(url, &cfg)` builds the pool from a `db::PoolConfig`, whose
defaults are sqlx's: 10 connections at most, none kept open, a 30 second
acquire timeout, 10 minutes idle and 30 minutes lifetime. An impossible config
(no connections, a minimum above the maximum, zero timeouts) is a
`sqlx::Error::Configuration` and opens nothing. With the `configured` feature it
loads with [configured](configured.md):

```toml
[db]
max_connections = 20
acquire_timeout = "5s"
max_lifetime = "never"   # no limit
```

```rust
let primary = Db::<Primary>::connect_with(&cfg.database_url, &cfg.db).await?;
```

## Things to know

- Declare a handle per database with `db::handle!(pub Primary: Postgres)`; `Db<Primary>` and `Db<Analytics>` are different types, so they cannot be mixed up.
- `tx(async |tx| ...)` commits when the closure returns `Ok` and rolls back otherwise.
