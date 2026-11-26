---
builds_on: `session` and `route`
phase: 3
---

# auth

Know who the current user is, and guard routes.

## Introduction

Most apps need to know who is making a request. `auth` logs a user in and out
of the session, resolves the current user for each request, and gives you
middleware to guard routes: `require` for routes that need a logged-in user,
and `guest` for routes that must not have one. It also gives you `Authed<U>`,
an input type that only exists when a user is logged in. An optional "remember
me" cookie and an optional bearer-token guard for APIs are available behind
features.

`auth` does not define or store users. Your app owns its user type and its
queries, and tells `auth` how to load a user by id. Hashing passwords is the
job of [password](password.md), talking to OAuth providers is the job of
[socialite](socialite.md), and deciding what a user may do (authorization)
would be a separate module.

The login handlers and pages do not run from inside the module either. They
are app code, copied into your app by `art publish auth`, so you can change
them freely.

`auth` builds on `session` and `route`, and uses `securecookie` for the
remember token.

## Installation

Add the module from git:

```toml
[dependencies]
auth = { git = "https://github.com/projectlighthouse-io/cloudfoundry" }
```

Two optional features extend it:

| Feature | Adds |
| --- | --- |
| `remember` | Long-lived signed remember cookie |
| `token` | Bearer-token guard with an app-provided token lookup |

### Publishing the auth files

Cargo has no install hook, so publishing is an explicit step. Run
`art publish auth` once after adding the dependency. The module keeps the
primitives (`Auth`, `Authed`, `require`, `guest`, `Provider`); everything you
will want to edit is published into your app instead of being called from
inside the module:

| Published to | Contents |
| --- | --- |
| `src/handlers/auth.rs` | `login`, `logout`, `show_login` handlers |
| `src/requests/login.rs` | `Login` input struct and its validation |
| `src/auth.rs` | `impl auth::Provider for Users` |
| `resources/js/pages/auth/login.vue` | Login page |
| `migrations/<timestamp>_create_users.sql` | Starter users table |

Published files are plain code with no link back to the module, so upgrading
the module never touches them. `art publish` never overwrites an existing file
unless you pass `--force`. It does not edit `routes.rs` either: it prints the
routes for you to add.

None of the published files are required. If you prefer, ignore the stubs and
write your own handlers against the primitives.

## Loading users

Your app tells `auth` how to load a user by id by implementing
`auth::Provider`. This is the only integration point:

```rust
impl auth::Provider for Users {
    type User = User;
    type Id = UserId;

    async fn find(&self, id: UserId) -> Result<Option<User>, auth::Error> {
        sqlx::query_as!(User, "select * from users where id = $1", id)
            .fetch_optional(self.db.pool())
            .await
            .map_err(Into::into)
    }
}
```

Only the user id is stored in the session, never the user itself. The user is
loaded at most once per request, and only if something asks for it.

## Logging in and out

The login and logout handlers are app code, not part of the module.
`art publish auth` copies them into `src/handlers/auth.rs`, and from then on
they are yours to change: redirect somewhere else, add logging, add rate
limits. See [Publishing the auth files](#publishing-the-auth-files).

```rust
// src/handlers/auth.rs (published, then owned by the app)
async fn login(cx: Cx<App>, (auth, input): (Auth<User>, Valid<Form<Login>>)) -> Result<Redirect, AppError> {
    let user = cx.users.by_email(&input.email).await?.ok_or(AppError::BadCredentials)?;
    password::verify(&input.password, &user.password_hash)?;

    auth.login(&user);            // regenerates the session id
    Ok(Redirect::to_route("dashboard"))
}

async fn logout(cx: Cx<App>, auth: Auth<User>) -> Redirect {
    auth.logout();                // invalidates the session
    Redirect::to_route("home")
}
```

Logging in always regenerates the session id, and logging out always
invalidates the session.

## Protecting routes

To require a logged-in user, add the `require` layer to a group, and take
`Authed<User>` as your handler's input:

```rust
r.group("/runs", |r| {
    r.layer(auth::require::<User>());
    r.get("/", runs::index);
});

async fn index(cx: Cx<App>, user: Authed<User>) -> Result<Page<RunIndex>, AppError>
```

An unauthenticated request gets a redirect to the login route for page visits,
and a `401` for JSON requests.

`Authed<U>` can only be built by the guard. A handler that takes one cannot run
for an anonymous request, even if the route forgot the `require` layer: the
input fails first.

## Getting the current user

When a user is optional, take `Auth<U>` instead. `auth.user()` returns
`Option<&U>`, so the same handler can serve guests and logged-in users.
