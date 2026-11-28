---
builds_on: `lettre` for SMTP; `configured` (feature) for the `[mail]` section
version: 0.1.0
---

# mail

Send mail over SMTP, to the log, or into memory for tests.

## Introduction

Apps send a handful of mails that matter: verify your address, reset your
password, you were invited. `mail` sends them through one of three drivers,
picked by a URL, so each environment differs by one setting:

| URL | Driver |
| --- | --- |
| `smtp://user:pass@host:587?tls=required` | SMTP with STARTTLS (`smtp://host:1025` without TLS, for Mailpit) |
| `smtps://user:pass@host:465` | SMTP over TLS |
| `log` | Every mail goes to the log, body included. For development |
| `memory` | Mail is kept in memory; tests read it back |

Addresses (`Ada <ada@acme.test>` or `ada@acme.test`) are checked before
anything is sent, whatever the driver, so a test with `memory` catches a bad
address too.

## Installation

```toml
[dependencies]
mail = { git = "https://github.com/projectlighthouse-io/toolkit", features = ["configured"] }
```

```sh
art publish mail   # src/mailer.rs: the app's one Mailer
```

Add the section to `src/config.rs` and `config/default.toml`:

```rust
#[serde(default)]
pub mail: mail::Config,
```

```toml
[mail]
url = "secret_env(MAIL_URL, log)"
from = "env(MAIL_FROM, hello@example.test)"
```

`url` is a secret: it can hold the SMTP password, and `config:show` prints it
as `<redacted>`.

## Sending mail

```rust
use crate::mailer::mailer;

let message = mail::Message::new(&user.email, "Verify your email address", format!("Open {link}"))
    .html(format!("<a href=\"{link}\">Verify</a>"));   // optional HTML part
mailer(&app).send(message).await?;
```

Every mail has a plain-text body; the HTML part is optional. [auth](../modules/auth.md)'s
published handlers send reset and verification links this way.

## Testing

Build a `memory` mailer and read what was sent:

```rust
let mailer = mail::Mailer::memory("hello@acme.test");
send_welcome(&mailer, &user).await?;

let sent = mailer.sent();
assert_eq!(sent[0].to, "ada@acme.test");
assert!(sent[0].text.contains("/verify-email/"));
```

Clones of a `memory` mailer share what was sent. To see real mail locally,
run Mailpit and point the app at it:

```sh
docker run -p 1025:1025 -p 8025:8025 axllent/mailpit
MAIL_URL=smtp://127.0.0.1:1025   # then open http://localhost:8025
```

## Things to know

- Mail is sent during the request. SMTP opens one connection per mail (no
  pool), which is fine for sign-up and reset mail; send bulk mail from a
  [queue](queue.md) job.
- The `log` driver writes links that work: keep it out of production logs.
- A bad `[mail] url` does not stop startup: the published `src/mailer.rs`
  logs the error and falls back to the log driver.
- Errors: `Error::Address` (not an email address), `Error::Url` (no driver
  for the URL; the password is never echoed), `Error::Send` (the server
  refused or could not be reached).
