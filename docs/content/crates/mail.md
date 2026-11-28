---
builds_on: `lettre` for SMTP; `queue` (feature) for sending from a worker; `configured` (feature) for the `[mail]` section
version: 0.1.0
---

# mail

Send mail from templates: over SMTP, to the log, or into memory for tests,
now or from the queue.

## Introduction

Apps send a handful of mails that matter: verify your address, reset your
password, you were invited. `mail` gives you a `Mailer` trait with three
implementations, picked by a URL so each environment differs by one setting:

| URL | Mailer |
| --- | --- |
| `smtp://user:pass@host:587?tls=required` | `SmtpMailer`, STARTTLS (`smtp://host:1025` without TLS, for Mailpit) |
| `smtps://user:pass@host:465` | `SmtpMailer`, TLS from the start |
| `log` | `LogMailer`: every mail goes to the log, body included. For development |
| `memory` | `MemoryMailer`: mail is kept in memory; tests read it back |

`Mailer` is object-safe: the app holds a `Box<dyn Mailer>` chosen by config,
and you can implement it to send through another service. Addresses
(`Ada <ada@acme.test>` or `ada@acme.test`) are checked before anything is sent,
whatever the mailer.

## Installation

```toml
[dependencies]
mail = { git = "https://github.com/projectlighthouse-io/toolkit", features = ["configured", "queue"] }
```

```sh
art publish queue   # the jobs table, for queued mail
art publish mail    # src/mailer.rs: the app's mailer and `deliver`
```

Add the section to `src/config.rs` and `config/default.toml`, and the job to
`src/jobs/mod.rs`:

```rust
#[serde(default)]
pub mail: mail::Config,
```

```toml
[mail]
url = "secret_env(MAIL_URL, log)"
from = "env(MAIL_FROM, hello@example.test)"
queue = true
```

```rust
w.register::<mail::SendMail<App>>()
```

`url` is a secret: it can hold the SMTP password, and `config:show` prints it
as `<redacted>`.

## Templates

A template is a Markdown-ish file in `resources/mail/`: a `Subject:` line, a
blank line, then the body.

```text
Subject: You are invited to join {{name}}

# Join {{name}}

You were invited to join **{{name}}**.

[Accept the invitation]({{link}})

- The link works for a week.
- Ignore this mail if you do not know why you got it.
```

Blocks are separated by blank lines: `# ` and `## ` headings, `- ` lists and
paragraphs. A paragraph that is only a link becomes a button. Inline:
`**bold**`, `` `code` `` and `[text](url)`. Every template renders to a plain
text body and an HTML body.

`{{name}}` is filled in after rendering, HTML-escaped in the HTML body, so a
value never turns into markup. A placeholder without a value is an error, so a
typo never reaches a user:

```rust
let template = mail::Template::parse(include_str!("../resources/mail/invitation.md"))?;
let message = template.message(&email, &[("name", &team.name), ("link", &link)])?;
```

## Sending

The published `deliver` renders a template and sends it:

```rust
crate::mailer::deliver(&app, include_str!("../resources/mail/invitation.md"), &email, &[("name", &team.name), ("link", &link)]).await;
```

With `[mail] queue = true` (the default) it stores a `mail::SendMail` job and
returns; `queue:work` sends it, retrying failures with the queue's backoff
(three attempts, then failed jobs). With `queue = false` it sends during the
request. Errors are logged, so a handler carries on. [auth](../modules/auth.md)
sends reset and verification links this way, and [teams](../modules/teams.md)
sends invitations.

To send directly, without a template or the queue:

```rust
let message = mail::Message::new(&email, "Hello", "Plain text").html("<p>HTML</p>");
crate::mailer::mailer(&app).send(message).await?;
mail::send_later::<App, _>(&app.queue, message).await?;   // or queue it yourself
```

## Testing

Build a `MemoryMailer` and read what was sent:

```rust
let mailer = mail::MemoryMailer::new("hello@acme.test");
send_welcome(&mailer, &user).await?;

let sent = mailer.sent();
assert_eq!(sent[0].to, "ada@acme.test");
assert!(sent[0].text.contains("/verify-email/"));
```

Clones of a `MemoryMailer` share what was sent. To see real mail locally, run
Mailpit and point the app at it:

```sh
docker run -p 1025:1025 -p 8025:8025 axllent/mailpit
MAIL_URL=smtp://127.0.0.1:1025   # then open http://localhost:8025
```

## Things to know

- Queued mail needs `queue:work` running. `art dev` runs only the server: start
  a worker beside it, or set `queue = false` locally.
- SMTP opens one connection per mail (no pool: a pooled transport must be
  dropped inside a Tokio runtime).
- The `log` mailer writes links that work: keep it out of production logs.
- A bad `[mail] url` does not stop startup: `src/mailer.rs` logs the error and
  falls back to the log mailer.
- Errors: `Error::Address`, `Error::Url` (the password is never echoed),
  `Error::Template` (no `Subject:`, or a missing value), `Error::Send`.
