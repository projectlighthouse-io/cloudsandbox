---
builds_on: `webauthn-rs` 0.6, `session` for ceremony state, `db` (uuid), `sqlx` for the stores; `throttle` and `event` (features)
version: 0.1.0
---

# passkeys

Sign in with passkeys, with or without a password.

## Introduction

A passkey is a key pair kept by the user's device or password manager (iCloud
Keychain, Google Password Manager, 1Password, a YubiKey). Signing in means the
device signs a challenge from your server after the user unlocks it with a
fingerprint, their face or a PIN. Nothing secret is stored on the server, and a
phishing site cannot use the passkey: it is bound to your domain.

`passkeys` wraps [webauthn-rs](https://github.com/kanidm/webauthn-rs) and stores
one row per passkey. It supports three things:

- **Adding passkeys** to a signed-in account, then listing, renaming and
  removing them.
- **Passwordless sign-in.** The login page has a "Sign in with a passkey"
  button; the browser lists the passkeys it has for your site and no username
  is typed.
- **A passkey as the second factor.** After a correct password, the
  two-factor (`factor2`) challenge accepts a passkey instead of a code.

Every ceremony is two requests. The first returns options for
`navigator.credentials.create` or `.get` and keeps the challenge in the
session. The second posts the browser's answer, which is checked against it.
`passkeys` never logs anyone in: the published handlers call `auth.login`.

## Installation

Add the module from git, with your database driver:

```toml
[dependencies]
passkeys = { git = "https://github.com/projectlighthouse-io/toolkit", features = ["sqlite", "event", "throttle", "ts"] }
```

Publish the table, handlers, settings page and browser helper, after `auth` (and
`factor2`, for the second factor):

```sh
art publish auth --with-ui --driver sqlite
art publish factor2 --with-ui --driver sqlite
art publish passkeys --with-ui --driver sqlite
art migrate
```

`art publish` prints the `mod` lines and routes to add. The app gets
`src/passkey.rs` (the app's one `Passkeys`), `src/handlers/passkey.rs`,
`src/requests/passkey.rs`, `resources/js/pages/settings/passkeys.vue` and
`resources/js/passkeys.ts`. The files are named `passkey`, not `passkeys`, so
they do not shadow the crate.

`Passkeys::new` takes the app URL and name. The URL's host is the *relying
party id* every passkey is bound to:

```rust
let pk = Passkeys::new(passkeys::SqliteStore::new(pool), &cfg.app_url, &cfg.app_name)?;
```

## Adding a passkey

A signed-in user adds a passkey from the settings page:

```rust
// POST /settings/passkeys/options
let options = pk.start_registration(&session, user.id, &user.email, &user.email).await?;

// POST /settings/passkeys, with the browser's credential.toJSON()
let added = pk.finish_registration(&session, user.id, &input.name, &input.credential).await?;
```

The options ask for a discoverable passkey (one that can sign in without a
username) and exclude the passkeys the user already has. A credential that is
already registered to any account is refused with `Error::AlreadyRegistered`.

In the browser, `resources/js/passkeys.ts` does the other half:

```ts
import { register } from '../../passkeys'

await register('MacBook')
```

## Passwordless sign-in

Add a button to the login page:

```vue
<script setup lang="ts">
import { signIn } from '../../passkeys'
</script>

<button type="button" @click="signIn">Sign in with a passkey</button>
```

The handlers behind it:

```rust
// POST /passkeys/login/options
let options = pk.start_login(&session)?;

// POST /passkeys/login
let used = pk.finish_login(&session, Some(ip), &input.credential).await?;
auth.login(&users.find(used.user_id).await?.unwrap());
```

The passkey verified the user itself (the device asked for a fingerprint or
PIN), so this login does not go through the two-factor challenge.

## Passkeys as a second factor

When two-factor is on, the challenge page can offer a passkey next to the code
field. The handler checks the passkey for the user whose password was just
checked, then finishes the pending login with `TwoFactor::pass`:

```rust
let Some(user) = tf.pending(&session) else { return Ok(Redirect::to_route("login")) };  // the password was correct
let options = pk.start_authentication(&session, user).await?;        // POST .../passkey/options

pk.finish_authentication(&session, Some(ip), user, &input.credential).await?;
let passed = tf.pass(&session)?;
auth.login(&users.find(passed.user_id).await?.unwrap());
```

On the page, pass the "Trust this browser" checkbox along; the handler then sets
the same trusted-browser cookie a code would:

```vue
<button type="button" @click="secondFactor(form.trust)">Use a passkey</button>
```

## Managing passkeys

```rust
let list = pk.passkeys(user.id).await?;               // newest first, with name, created_at, last_used_at
pk.rename(user.id, passkey, "Work laptop").await?;    // false if it is not theirs
pk.remove(user.id, passkey).await?;                    // behind auth::PasswordConfirmed
```

Passkeys are named in URLs by `public_id` (`db::Uuid<Passkey>`); the internal
id, credential and user handle are not serialized. Removing a passkey takes
`auth::PasswordConfirmed<User>`, so the user confirms their password first.

## Rate limiting

With feature `throttle`, give `Passkeys` a limiter. Failed passwordless sign-ins
count per IP, failed second factors per user and IP; a success clears the count:

```rust
let limiter = Arc::new(throttle::Limiter::memory(throttle::FixedWindow::new(5, Duration::from_secs(60))));
let pk = Passkeys::new(store, &cfg.app_url, &cfg.app_name)?.throttle(limiter);
```

Over the limit, `finish_login` and `finish_authentication` return
`Error::Throttled(retry_after)` without checking the answer. Keep the limiter
for the life of the app: a memory limiter made per request forgets every attempt.

## Events

With feature `event`, `Passkeys` emits its events on your bus itself:

```rust
let pk = Passkeys::new(store, &cfg.app_url, &cfg.app_name)?.events(app.events.clone(), app.clone());
```

| Event | When |
| --- | --- |
| `PasskeyAdded { user_id, passkey, name }` | `finish_registration` stored a passkey |
| `PasskeyRemoved { user_id, passkey }` | `remove` deleted one |
| `PasskeyUsed { user_id, passkey, passwordless }` | a sign-in or second factor passed |

A failing listener is logged, not returned: the change it reports has already
happened. Without the feature the structs in `passkeys::events` are there for
your handlers to emit.

## Things to know

- **Keep the session server side.** Ceremony state is serialised into the
  session. With the memory, SQL or Redis session store that is safe; a session
  kept in a client-side cookie would let a browser replay an old challenge.
- **The host is permanent.** Passkeys are bound to the host of `app_url`.
  Moving the app to another host orphans every passkey.
- **Each ceremony works once,** and for five minutes. A second answer, or an
  answer for another user, is `Error::NoCeremony`: start again.
- **Security keys without storage** still register (`residentKey: preferred`),
  but only work as a second factor, not for passwordless sign-in.
- **Browsers:** the published helper uses `PublicKeyCredential.parseCreationOptionsFromJSON`
  and `toJSON()` (Chrome 129, Safari 18, Firefox 119).
- **Deleting a user:** call `delete_all(user_id)` first. There are no cascades.
- Stores: `MemoryStore`, and `SqliteStore`, `PostgresStore`, `MysqlStore`
  behind the driver features. The credential is stored as JSON (`jsonb`,
  `text`, `json`).
