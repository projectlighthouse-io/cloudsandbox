---
builds_on: `config-rs` for files, `secrecy` for secrets, `dotenvy` for `.env`
version: 0.2.0
---

# configured

Load typed app config from files, with env vars and secrets resolved.

## Introduction

Your app reads its config once, at startup, into one struct. `configured`
reads TOML, YAML or JSON files with `config-rs`, replaces `env(...)` values with
environment variables, and deserializes the result into your struct. If anything
is missing or malformed, startup fails with every problem listed at once.

The other toolkit crates do not read files or env vars. Each one that has
settings exposes a plain `Config` struct with defaults. With the crate's
`configured` feature, that struct deserializes, so you can nest it in your own
config:

```toml
# config/default.toml
app_name     = "env(APP_NAME, my-app)"
database_url = "required_env(DATABASE_URL)"
app_key      = "required_secret_env(APP_KEY)"

[session]
lifetime = "2h"

[db]
max_connections = 20
```

```rust
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub app_name: String,
    pub database_url: String,
    pub app_key: configured::SecretString,

    #[serde(default)]
    pub session: session::Config,

    #[serde(default)]
    pub db: db::PoolConfig,
}

let Loaded { value: cfg, listing } = configured::load::<Config>("config")?;

let codec = Codec::new(Keys::parse(cfg.app_key.expose_secret())?);
let primary = Db::<Primary>::connect_with(&cfg.database_url, &cfg.db).await?;
let sessions = session::layer(store, codec).config(cfg.session)?;
```

## Installation

Add the crate from git in your app:

```toml
[dependencies]
configured = { git = "https://github.com/projectlighthouse-io/toolkit" }
session = { git = "https://github.com/projectlighthouse-io/toolkit", features = ["configured"] }
```

Turn on the `configured` feature of each crate whose `Config` you load. Those
crates depend on `configured` with `default-features = false`, which brings in
only `serde`, `secrecy` and `humantime-serde`, not the file loader.

| Feature | Adds |
| --- | --- |
| `load` (default) | `configured::load`, with `config-rs` and `dotenvy` |

## Files

`configured::load::<T>(dir)` reads, in order:

1. `.env` in the working directory, if there is one. It never overrides a variable
   that is already set, and a `.env` that does not parse is an error.
2. `<dir>/default.toml` (or `.yaml`, `.yml`, `.json`). It is required.
3. `<dir>/<APP_ENV>.toml`, when `APP_ENV` is set. It is optional, and its keys
   override the default file key by key.

Each name may have only one file: `default.toml` next to `default.yaml` is an
error, because only one of them would be read. `APP_ENV` must be a plain name
such as `production` or `staging`.

## Reading env vars

A value that is exactly one of these forms is replaced with an env var:

| Form | When the var is unset or empty | Redacted in `show()` |
| --- | --- | --- |
| `env(NAME)` | The key is left out, so the field's default applies | No |
| `env(NAME, fallback)` | `fallback` is used | No |
| `required_env(NAME)` | Startup fails | No |
| `secret_env(NAME)` | The key is left out | Yes |
| `required_secret_env(NAME)` | Startup fails | Yes |

An empty variable counts as unset, so `APP_KEY=` fails a required key, and an
unset optional key gives the struct's default rather than an empty string.

Env vars are text, but they fill typed fields: `port = "env(PORT)"` works for a
`u16` and `debug = "env(DEBUG, false)"` for a `bool`. Durations are written as
`"90s"`, `"2h"` or `"14d"`.

A value that starts like a form but is malformed, such as `env(APP_KEY` or
`required_env(X, default)`, is an error, never a literal string. In an array,
an unset optional item is dropped.

All problems are reported together:

```text
config: 2 env problem(s)
  app_key: APP_KEY is not set (`required_secret_env(APP_KEY)`)
  database_url: DATABASE_URL is not set (`required_env(DATABASE_URL)`)
```

## Secrets

Declare secret fields as `configured::SecretString`. Its `Debug` prints
`[REDACTED]`, and reading it takes an explicit `expose_secret()`. The `secret_`
forms also mark the key, so `show()` prints `<redacted>` for it and error
messages never contain the value.

`loaded.listing.show()` lists every key set in a file or by an env var, for a
`config:show` command. Keys you leave out keep their defaults and are not
listed. It prints `<redacted>` for:

- keys loaded with a `secret_` form;
- keys named like a secret (`*_key`, `*_secret`, `*_password`, `*_token`), even
  when the value is written in the file;
- the password in a URL (`postgres://user:<redacted>@host/db`).

`loaded.listing.show_secret_values()` prints the same list with the real
values. Call it only on an explicit request; the generated app uses
`config:show --print-secret-values`, which also prints a warning.

Keep the listing from the startup load and hand it to the command, so
`config:show` prints exactly what the app loaded without reading the files
again:

```rust
let Loaded { value: cfg, listing } = configured::load::<Config>("config")?;
// ...
.add("config:show", config::show(listing))
```

## Things to know

- Settings come only from files and the `env(...)` forms. There is no blanket
  environment source, so a stray variable cannot override a key, and your top
  level struct can use `deny_unknown_fields`.
- Each crate's `Config` uses `deny_unknown_fields`: a typo such as `cookie_nam`
  fails at startup.
- `env(...)` replaces a whole value. `"postgres://${USER}@host"` is not
  supported.
- An `Option<Duration>` whose default is set, such as `db.idle_timeout`, is
  turned off with `"never"`, because TOML has no null.
