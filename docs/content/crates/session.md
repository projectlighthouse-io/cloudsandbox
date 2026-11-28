---
builds_on: `securecookie`, `http`, `tower`, `serde`
version: 0.1.2
---

# session

Load and save session data for a request.

## Introduction

Sessions let your app remember things about a visitor between requests. This
crate gives you a `tower` layer that loads the session before your handler runs
and saves it afterwards, with typed get, insert and remove in between. It also
handles flash data, which lives for exactly one following request, and lets you
regenerate the session id on login or invalidate the session on logout.

Session data is kept in a store behind a narrow trait, with drivers for each
database.

The session does not know who the user is; that is `auth`. It does not issue
CSRF tokens either; that is `csrf`.

## Installation

Add the crate from git:

```toml
[dependencies]
session = { git = "https://github.com/projectlighthouse-io/toolkit" }
```

Then turn on the feature for the store you want. `sqlx` is only pulled in with
a database feature.

| Feature | Adds |
| --- | --- |
| `cookie-store` | Whole session stored in the encrypted cookie |
| `postgres` | `PostgresStore` |
| `mysql` | `MysqlStore` |
| `sqlite` | `SqliteStore` |
| `configured` | `session::Config` loads from config files |

## Configuring the layer

Create the layer with a store and a `securecookie` codec, then set the cookie
name, lifetime and `SameSite` policy. In a handler, ask for `Session` as an
input:

```rust
let sessions = session::layer(PostgresStore::new(db.pool()), codec)
    .cookie_name("app_session")
    .lifetime(Duration::days(14))
    .same_site(SameSite::Lax);

// in a handler input
async fn store(cx: Cx<App>, session: Session, input: Valid<Form<Login>>) -> ...

session.insert("cart", &cart)?;
let cart: Option<Cart> = session.get("cart")?;
session.flash("status", "Saved");
session.regenerate();
```

The session id is random, never derived from user data, and only ever sent
encrypted.

### From a config file

All cookie settings live in `session::Config`. Pass one with `.config(cfg)`,
which checks it first: an invalid cookie name, a lifetime under a second, or
`SameSite=None` without `Secure` (browsers drop such cookies) is a
`ConfigError` at startup. Setters called after `.config` change single fields.

With the `configured` feature, load it with
[configured](configured.md). Missing keys keep their defaults
(`session`, 14 days, `lax`, `secure = true`); unknown keys are an error:

```toml
[session]
cookie_name = "app_session"
lifetime = "2h"
same_site = "strict"   # strict | lax | none
secure = true
```

```rust
let sessions = session::layer(store, codec).config(cfg.session)?;
```

## Working with session data

Use `insert`, `get` and `remove` to store and read typed values. `get` returns
`None` when the key is not set.

Sessions are lazy: nothing is written unless the session was changed. An
anonymous request that never touches the session creates no row and sets no
cookie.

### Flash data

`flash` stores a value for exactly one following request, which suits status
messages shown after a redirect.

### Regenerating and invalidating

Call `regenerate()` when a user logs in, so the session gets a fresh id.
Invalidate the session when they log out.

## Browsers and devices

A session that belongs to a user (`session.set_user(Some(id))`, which `auth`
calls on login) records its browser's user agent and IP, and when it was last
seen. `last_seen_at` is written at most once a minute, or at once when the
browser or IP changes; anonymous sessions are never touched for it.

The SQL stores and `MemoryStore` implement `Devices`:

```rust
use session::Devices;

let devices = store.devices(user_id).await?;                 // unexpired, most recent first
let current = session.public_id();                            // this browser, to mark in the list
store.revoke(user_id, public_id).await?;                      // end one session
store.revoke_others(user_id, current.unwrap()).await?;        // end the rest
```

Each `Device` is named by a random `public_id` (a UUID); the session id never
leaves the encrypted cookie. The columns come with `art publish session`; apps
that published it earlier get only the `add_session_devices` migration. Times
are unix seconds and `ip` is text. `RedisStore` keeps the device in the session
but cannot list a user's sessions.

## Pruning expired sessions

Expired records are removed by a command, `session:prune`, not on the request
path, so your requests never pay for the cleanup.

## Writing a store

A store implements three methods:

```rust
pub trait Store: Send + Sync + 'static {
    fn load(&self, id: &SessionId) -> impl Future<Output = Result<Option<Record>>> + Send;
    fn save(&self, record: &Record) -> impl Future<Output = Result<()>> + Send;
    fn delete(&self, id: &SessionId) -> impl Future<Output = Result<()>> + Send;
}
```

A driver for a new database is three hand-written queries.

## Things to know

- Add `session::layer(store, codec)` before the CSRF and auth layers; both read the session from the request.
- SQL stores need their table: `art publish session` (add `--driver sqlite` or `--driver mysql`), then `art migrate`.
- `MemoryStore` loses sessions on restart and is per process; use a SQL store in production.
