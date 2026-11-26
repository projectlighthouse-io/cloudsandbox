---
builds_on: `http`, `tower`
phase: 3
---

# csrf

Issue and verify CSRF tokens.

## Introduction

Cross-site request forgery is when another site tricks a signed-in user's
browser into submitting a request to your app. This crate guards against it by
issuing one token per session and checking that every unsafe request (`POST`,
`PUT`, `PATCH` or `DELETE`) carries it back.

The crate does not store the token itself. It reads and writes the token
through a small trait that `session` implements, so it has no hard dependency on
`session`.

## Installation

Add the crate from git:

```toml
[dependencies]
csrf = { git = "https://github.com/projectlighthouse-io/cloudfoundry" }
```

## Protecting your routes

Add the layer to your router. Every unsafe request that passes through it must
carry a valid token:

```rust
let csrf = csrf::layer()
    .except(["webhooks.github"]); // route names
```

The layer accepts the token from either a header or a form field. When a check
fails, your app responds with `419` and no detail about why. Tokens are compared
in constant time.

### Excluding routes

Some routes cannot carry a token, such as incoming webhooks from another
service. Pass their route names to `.except(...)` and the layer lets them
through without a check.

## Sending the token from JavaScript

The layer also sets a readable `XSRF-TOKEN` cookie. A JavaScript client can read
it and echo the token back in a header, so your frontend does not have to embed
the token in each page.

## Storing the token

The token lives wherever your app keeps per-session data. The layer finds it
through the `TokenStore` trait:

```rust
pub trait TokenStore {
    fn token(&self) -> Option<Token>;
    fn set_token(&mut self, token: Token);
}
```

`session` implements this trait, so with sessions in place there is nothing more
to write. The token is rotated whenever the session id is regenerated.
