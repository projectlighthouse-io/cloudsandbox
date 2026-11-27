---
builds_on: The RustCrypto `argon2` crate
phase: 3
---

# password

Hash and verify passwords.

## Introduction

You should never store a user's password as it was typed. `password` hashes it
with Argon2id, using the parameters OWASP recommends, and stores those
parameters inside the hash string. Later it verifies a login attempt against
that stored hash, and tells you when a hash was made with older settings and
should be replaced.

The module stores nothing and knows nothing about users. Password strength
rules are not its concern either: enforce those with validation in your app.
If you use [auth](auth.md), you rarely call `password` yourself:
`auth.attempt` verifies and rehashes, and `auth::hash_password` hashes for
registration.

`password` depends only on the RustCrypto `argon2` crate.

## Installation

Add the module from git:

```toml
[dependencies]
password = { git = "https://github.com/projectlighthouse-io/cloudfoundry" }
```

## Hashing and verifying passwords

Hash a password when a user signs up or changes it, and verify it when they log
in:

```rust
let hash = password::hash(&input.password)?;              // "$argon2id$v=19$m=19456,t=2,p=1$..."

password::verify(&input.password, &user.password_hash)?;  // Err(Error::Mismatch) on a wrong password
```

`verify` returns `Error::Mismatch` for a wrong password and `Error::Hash` for a
stored hash it cannot read.

## Upgrading old hashes

The parameters are read from each stored hash, so old hashes keep verifying
after the defaults change. To move users onto the new settings, check after a
successful `verify` and store a fresh hash:

```rust
if password::needs_rehash(&user.password_hash) {
    users.set_password_hash(user.id, password::hash(&input.password)?).await?;
}
```

`auth.attempt` does this for you.

## Not revealing who has an account

When nobody has the email address someone tried to log in with, call
`verify_none`. It spends the same time as a real `verify`, so the response time
does not reveal which accounts exist:

```rust
let Some(user) = users.by_email(&input.email).await? else {
    password::verify_none(&input.password);
    return Err(failed());
};
```

## Performance

Hashing and verifying take tens of milliseconds of CPU by design. In an async
handler under load, run them on the blocking pool with
`tokio::task::spawn_blocking`. `auth.attempt` and `auth::hash_password` already
do.
