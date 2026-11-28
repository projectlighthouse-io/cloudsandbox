---
builds_on: RustCrypto `chacha20poly1305` and `hmac`; `sqlx` (feature) for the column type and `crypt:rotate`
version: 0.1.0
---

# crypt

Encrypt values at rest, with keys that rotate.

## Introduction

Some columns should never be readable from a database dump: two-factor
secrets, API credentials of your users' integrations, national ID numbers.
`crypt` seals them with XChaCha20-Poly1305 under keys derived from your
`APP_KEY`, and opens them again in your code.

Every sealed value names the key it was made with. `APP_KEY` can list several
keys, newest first: new values use the first, old values open with whichever
key made them. To rotate, put a new key in front, re-encrypt the columns with
`crypt:rotate`, then drop the old key.

## Installation

```toml
[dependencies]
crypt = { git = "https://github.com/projectlighthouse-io/toolkit", features = ["sqlite", "command"] }
```

Pick the feature of your database (`postgres`, `sqlite`, `mysql`); it brings
the column type and the rotation. Without one, `sqlx` gives just the column type.

## Sealing values

```rust
let crypt = crypt::Encrypter::new(crypt::Keys::parse(app.cfg.app_key.expose_secret())?);

let sealed = crypt.encrypt(b"factor2/totp:42", secret)?;      // bytes
let secret = crypt.decrypt(b"factor2/totp:42", &sealed)?;
```

The first argument is the *purpose*. A value only opens with the purpose it
was sealed for, so put the column, and the row when it matters, in it: a
secret copied to another user's row then does not open there.

## Encrypted columns

`Encrypted<T>` holds any serde value, sealed as JSON, and is a column type
over `bytea` / `blob` / `varbinary`:

```rust
#[derive(sqlx::FromRow)]
struct Integration {
    id: i64,
    credentials: crypt::Encrypted<Credentials>,
}

let sealed = Encrypted::seal(&crypt, format!("integrations:{id}").as_bytes(), &credentials)?;
sqlx::query("update integrations set credentials = $1 where id = $2").bind(sealed).bind(id).execute(pool).await?;

let creds: Credentials = row.credentials.open(&crypt, format!("integrations:{}", row.id).as_bytes())?;
```

`Debug` prints `…`: neither the value nor the ciphertext ends up in logs.

## Rotating keys

1. Generate a key (`art key:generate`) and put it first: `APP_KEY="new,old"`.
2. Re-encrypt each encrypted column:

```sh
art crypt:rotate --table totp_secrets --column secret_encrypted --id user_id --purpose 'factor2/totp:{id}' --dry-run
art crypt:rotate --table totp_secrets --column secret_encrypted --id user_id --purpose 'factor2/totp:{id}'
# totp_secrets.secret_encrypted: 1200 re-encrypted, 0 already current, 0 could not be opened
```

3. Once every column reports `0 re-encrypted`, drop the old key: `APP_KEY="new"`.

`{id}` in `--purpose` is each row's id. The command pages through the table by
`--id` (an integer column), skips values already on the newest key, and
updates each row only if it still holds the value it read, so it is safe
while the app runs and safe to run again. Values that do not open (a wrong
`--purpose`, a key no longer listed) are counted and left alone, and the
command fails so you notice. Register it in `src/commands/mod.rs`:

```rust
.add("crypt:rotate", crypt::command::rotate(|app: &App| {
    Ok((app.db.pool().clone(), crypt::Encrypter::new(crypt::Keys::parse(app.cfg.app_key.expose_secret())?)))
}))
```

## Things to know

- `APP_KEY` also signs cookies (`securecookie`); each crate derives its own key
  from it, so one secret serves both, and both accept the key list.
- Errors say nothing about why a value did not open: wrong key, wrong purpose
  and tampering look the same.
- Table and column names for `crypt:rotate` must be plain identifiers.
