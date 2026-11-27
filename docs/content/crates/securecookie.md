---
builds_on: An AEAD crate and an HMAC crate from RustCrypto
phase: 3
---

# securecookie

Sign and encrypt cookie values.

## Introduction

When your app stores something in a cookie, you need to know it comes back
unchanged, and sometimes that nobody can read it. This crate signs a value so
tampering is detected, or encrypts and authenticates it so it can be neither
read nor changed. Decoding verifies the value and enforces a maximum age.

The crate is pure functions over bytes. It does not parse or set `Cookie`
headers, which is the caller's job, and it stores nothing. It has no `http`
dependency.

It is named after Gorilla's `securecookie`. The earlier plan called it
`cookie`, but that name clashes with a very widely used crate.

## Installation

Add the crate from git:

```toml
[dependencies]
securecookie = { git = "https://github.com/projectlighthouse-io/toolkit" }
```

## Encoding and decoding values

Build a `Codec` from your keys, then encrypt or sign values under a cookie
name:

```rust
let codec = Codec::new(Keys::from_env("APP_KEY")?);

let value: String = codec.encrypt("session", &session_id)?;
let id: SessionId = codec.decrypt("session", &value, max_age)?;

let signed = codec.sign("remember", &token)?;
let token: Token = codec.verify("remember", &signed, max_age)?;
```

Use `encrypt` and `decrypt` when the value must stay secret, and `sign` and
`verify` when it only needs to be tamper-proof.

The cookie name is bound into the authentication tag, so a value issued for one
cookie cannot be replayed under another name.

### Decoding failures

Every decoding failure returns the same opaque error. You cannot tell a bad
signature from a bad format, and neither can an attacker.

## Keys

Keys are typed and never implement `Debug` or `Display`, so they cannot end up
in a log line by accident.

### Rotating keys

A codec can hold several keys. It decodes with any of them and always encodes
with the newest, so you can introduce a new key without invalidating cookies
that are already out there.
