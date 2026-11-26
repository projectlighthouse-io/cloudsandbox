---
builds_on: The RustCrypto `argon2` crate
phase: 3
---

# password

Hash and verify passwords.

## Introduction

You should never store a user's password as it was typed. `password` hashes it
with Argon2id, encoding the parameters in the hash string, and later verifies a
login attempt against that stored hash. It can also tell you when a stored hash
should be rehashed because the parameters have changed.

The module stores nothing and knows nothing about users. Password strength
rules are not its concern either: enforce those with validation in your app.

`password` depends only on the RustCrypto `argon2` crate, with no dependency on
any toolkit crate, so it could equally be listed as a crate. It sits with the
modules because only auth flows use it.

## Installation

Add the module from git:

```toml
[dependencies]
password = { git = "https://github.com/projectlighthouse-io/cloudfoundry" }
```

## Hashing and verifying passwords

Hash a password when a user signs up or changes it, and verify it when they log
in. When a stored hash uses outdated parameters, `needs_rehash` tells you to
hash the password again and store the new hash:

```rust
let hash: PasswordHash = password::hash(&input.password)?;

password::verify(&input.password, &user.password_hash)?; // Err on mismatch

if password::needs_rehash(&user.password_hash) {
    /* hash again and store */
}
```

`verify` returns an error on a mismatch. A malformed hash returns the same
error, so callers cannot tell the two apart. The hash comparison is constant
time.

## Performance

Hashing is CPU-bound, so it runs on the blocking pool and never stalls the
async runtime.

## Handling plain passwords safely

Plain passwords are taken as a `Secret` type. It is zeroed on drop and has no
`Debug` output, so a password does not linger in memory or end up in your logs.
