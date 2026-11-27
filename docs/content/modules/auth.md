---
builds_on: `session`, `route`, `password`; `throttle` for login limits
version: 0.1.0
---

# auth

Know who the current user is, log them in, and guard routes.

## Introduction

Most apps need to know who is making a request. `auth` logs users in and out of
the session, checks passwords, resolves the current user for each request, and
gives you middleware to guard routes. It also covers the flows around a login:
"remember me", API tokens, registration and password reset.

`auth` does not define or store users. Your app owns its user type and its
queries, and tells `auth` how to load and save what it needs through a few small
traits. Talking to OAuth providers is the job of [socialite](socialite.md).

The handlers and pages do not run from inside the module either. They are app
code, copied into your app by `art publish auth`, so you can change them
freely: the logic lives in the module, and your handler calls it.

## Installation

Add the module from git. The `throttle` feature limits failed logins:

```toml
[dependencies]
auth = { git = "https://github.com/projectlighthouse-io/toolkit", features = ["throttle"] }
throttle = { git = "https://github.com/projectlighthouse-io/toolkit" }
```

Then add the layer after the session and CSRF layers:

```rust
let router = routes::register(Router::new())
    .layer(session::layer(sessions, codec))
    .layer(csrf::layer::<Session>())
    .layer(auth::layer(Users { db: app.db.clone() })
        .throttle(Limiter::memory(FixedWindow::new(5, Duration::from_secs(60))))  // 5 failed logins a minute
        .remember(remember_codec)                                                 // "remember me"
        .tokens());                                                               // API tokens
```

### Publishing the auth files

Run `art publish auth` once after adding the dependency. It copies the handlers,
requests, a `Users` provider and the migration into your app, and prints the
routes to add:

| Command | Publishes |
| --- | --- |
| `art publish auth` | JSON handlers for your own frontend: login, logout, register, forgot, reset and change password |
| `art publish auth --with-ui` | The same flows as Inertia handlers, plus Vue pages for each |

Add `--driver sqlite` or `--driver mysql` for those databases. Published files
are plain code with no link back to the module, so upgrading the module never
touches them, and `art publish` never overwrites a file unless you pass
`--force`.

| Published to | Contents |
| --- | --- |
| `src/users.rs` | `User`, `Users` and the `auth` traits implemented with `sqlx` |
| `src/handlers/{login,register,password,password_reset}.rs` | The handlers |
| `src/requests/{login,register,password,password_reset}.rs` | Inputs and their validation |
| `migrations/<timestamp>_create_users.*.sql` | `users`, `api_tokens` and `password_resets` |
| `resources/js/pages/{auth,settings}/*.vue` | Login, register, forgot and reset pages, and the change-password page (`--with-ui`) |

## Loading users

Your app tells `auth` how to load a user by id by implementing `Provider`:

```rust
impl auth::Provider for Users {
    type User = User;
    type Id = i64;

    async fn find(&self, id: i64) -> Result<Option<User>, auth::Error> {
        Ok(sqlx::query_as("select * from users where id = $1").bind(id).fetch_optional(self.db.pool()).await?)
    }

    fn id(&self, user: &User) -> i64 {
        user.id
    }
}
```

Only the user id is stored in the session, never the user itself. The user is
loaded at most once per request, and only if something asks for it.

## Logging in

Implement `Credentials` as well, so `auth` can find a user by login and read
and update the password hash. The hash is an `Option`: accounts made through
[social login](socialite.md) have none, and password login refuses them until
they set one through "forgot password". Then `auth.attempt` does the whole login:

```rust
pub async fn login(cx: Cx<App>, (auth, input): (Auth<User>, Valid<Json<Login>>)) -> Result<Redirect, Response<Body>> {
    let users = Users { db: cx.db.clone() };
    match auth.attempt(&users, &input.email, &input.password, input.remember).await {
        Ok(_user) => Ok(Redirect::to_route("home")),
        Err(AttemptError::Failed) => Err(field_error("email", "These credentials do not match our records.")),
        Err(AttemptError::Throttled(wait)) => Err(field_error("email", &format!("Try again in {} seconds.", wait.as_secs()))),
        Err(AttemptError::Other(e)) => Err(server_error(e)),
    }
}
```

`attempt` takes these steps:

1. With `.throttle(limiter)`, it refuses when this login and IP have failed too
   often.
2. It looks the user up and verifies the password on the blocking pool. An
   unknown login costs the same time as a wrong password.
3. On failure, it counts the attempt. On success, it clears the count.
4. When the stored hash uses old settings, it stores a fresh one.
5. It logs the user in, regenerating the session id, and sets the remember
   cookie when asked.

Because the handler is yours, add your own checks around it: refuse banned
users, write an audit log, redirect somewhere else.

To log in a user you already have, for example right after registration, call
`auth.login(&user)`. `auth.logout()` invalidates the session and, with
"remember me", clears the stored token and the cookie.

## Registering users

The published `register` handler validates the input, checks the email is not
taken, hashes the password with `auth::hash_password` (off the async runtime),
inserts the user and logs them in. Add whatever your app needs on sign-up, such
as a welcome mail or a default team, to that handler.

The published `users` table has an optional `name`, and the register request
accepts it without requiring it. If your app needs a name, make it required in
`src/requests/register.rs`; that one line is all it takes.

## Changing passwords

A logged-in user changes their password with `auth.change_password`. It checks
the current password, stores the new hash, gives the session a new id, and
clears the "remember me" token so remembered devices have to log in again. The
user stays logged in on the device they used:

```rust
match auth.change_password(&users, &input.current_password, &input.password).await {
    Ok(()) => Ok(StatusCode::NO_CONTENT),
    Err(ChangeError::WrongPassword) => Err(field_error("current_password", "The current password is incorrect.")),
    Err(ChangeError::NoPassword) => Err(field_error("current_password", "Set a password with “forgot password” first.")),
    Err(ChangeError::NotLoggedIn) => Err(StatusCode::UNAUTHORIZED.into_response()),
    Err(ChangeError::Other(e)) => Err(server_error(e)),
}
```

Put the route behind `auth::require::<User>()`.

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
and a `401` for JSON requests. `auth::guest::<User>()` does the opposite: it
sends logged-in users to `home`, for pages like the login form.

`Authed<U>` can only be built for a logged-in user. A handler that takes one
cannot run for an anonymous request, even if the route forgot the `require`
layer.

When a user is optional, take `Auth<U>` instead. `auth.user()` returns
`Option<&U>`, so the same handler can serve guests and logged-in users.

## Remembering users

Implement `Remember` (read and store a token per user) and add
`.remember(codec)` to the layer. `auth.login_remember(&user)` then sets a
signed cookie that logs the user back in after the session ends. Logging out
clears the token, so old cookies stop working.

## API tokens

Implement `Tokens` and add `.tokens()` to the layer. Requests with
`Authorization: Bearer <token>` are then authenticated by the token alone,
without a session. Issue tokens with an optional lifetime:

```rust
let token = auth::issue_token(&users, &user, "cli", Some(Duration::from_secs(30 * 86400))).await?;
// show `token` once; only its SHA-256 hash is stored, with its expiry

auth::revoke_token(&users, auth.bearer_token().unwrap()).await?;  // API logout
```

An expired token is treated as no token. Exempt token-authenticated routes from
CSRF, which guards cookie-based requests.

## Resetting passwords

Implement `Resets` (one hashed token per user, with its creation time). The
published `password_reset` handlers use two functions:

```rust
let (user, token) = auth::reset_link_token(&users, &input.email).await?;   // at most one link a minute
send_reset_link(&user, &token);                                            // your mailer

let user = auth::reset_password(&users, &input.email, &input.token, &input.password).await?;  // valid for an hour
```

The forgot-password form answers the same way whether or not the email has an
account, so it does not reveal who is registered. A successful reset signs out
"remember me" devices and logs the user in. Until you connect a mailer, the
published `send_reset_link` logs the link; set `APP_URL` so it points at your
app.
