---
builds_on: `session` (to hold `state` and the PKCE verifier between redirect and callback)
phase: 3
---

# socialite

OAuth login with third-party providers.

## Introduction

Letting users sign in with an account they already have, such as GitHub, means
speaking OAuth. `socialite` handles the protocol for you: it builds the
redirect to the provider with `state` and PKCE, and when the provider sends the
user back, it checks `state`, exchanges the code and fetches the profile. You
get back a normalised `SocialUser`. Each provider is a driver behind a feature.

What happens next is up to your app. `socialite` does not log anyone in: you
map the `SocialUser` to your own user and call `auth.login`. It does not create
or link accounts, because that policy belongs to you. Provider tokens are not
stored either; they are returned to your app, which decides what to do with
them.

`socialite` builds on `session`, which holds `state` and the PKCE verifier
between the redirect and the callback. It uses the `oauth2` crate for the
protocol and an HTTP client for the profile request. It has no dependency on
`auth`.

## Installation

Add the module from git, with the driver for each provider you need:

```toml
[dependencies]
socialite = { git = "https://github.com/projectlighthouse-io/cloudfoundry" }
```

| Feature | Adds |
| --- | --- |
| `github` | GitHub driver |
| `google` | Google driver |
| `gitlab` | GitLab driver |

GitHub is the only driver written first, since the CI app needs it.

The redirect and callback handlers are app code. Run `art publish socialite`
to copy them into `src/handlers/social.rs`, where the account-linking policy
and the redirect target are yours to change.

## Configuring a provider

Create a driver with your client credentials and the URL of your callback
route, then choose the scopes you need:

```rust
use socialite::github::{GitHub, Scope};

let github = GitHub::new(
    cfg.github_client_id,     // ClientId
    cfg.github_client_secret, // ClientSecret
    app.url("auth.github.callback", &[])?,
).scopes([Scope::ReadUser, Scope::UserEmail]);
```

`ClientId` and `ClientSecret` are distinct types, so the two arguments cannot
be swapped. Your config produces them when it reads the environment.
`ClientSecret` has no `Debug` output.

### Scopes

Scopes are an enum per provider, so a GitHub scope cannot be passed to Google
and a typo does not compile:

```rust
pub enum Scope {
    ReadUser,
    UserEmail,
    ReadOrg,
    Repo,
    /// Escape hatch for a scope this enum does not list yet.
    Other(&'static str),
}
```

Each scope enum keeps an `Other` variant. Providers add scopes faster than a
library releases, and your app should never be blocked waiting for one.

## Redirecting and handling the callback

Two handlers complete the flow: one sends the user to the provider, and one
receives them when they come back. These are the published handlers in
`src/handlers/social.rs`:

```rust
// src/handlers/social.rs (published, then owned by the app)
async fn redirect(cx: Cx<App>, session: Session) -> Result<Redirect, AppError> {
    Ok(cx.github.redirect(&session)?)
}

async fn callback(
    cx: Cx<App>,
    (session, auth, query): (Session, Auth<User>, Query<Callback>),
) -> Result<Redirect, AppError> {
    let social: SocialUser<GitHub> = cx.github.user(&session, query.0).await?;

    let user = match cx.users.find_by_social(&social).await? {
        Some(user) => user,
        None => cx.users.create(NewUser::from(&social)).await?,
    };
    auth.login(&user);

    Ok(Redirect::to_route("dashboard"))
}
```

The `state` value is random, single use, and compared in constant time. A
callback without a matching `state` in the session is rejected. PKCE is always
on for providers that support it.

## The social user

The callback gives you a `SocialUser`:

```rust
pub struct SocialUser<P: Provider> {
    pub id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub email_verified: bool,
    pub avatar: Option<String>,
    pub token: Token, // access token, optional refresh token, expiry
    provider: PhantomData<P>,
}

impl<P: Provider> SocialUser<P> {
    /// The provider's stable name, for storing in a database column.
    pub fn provider(&self) -> &'static str { P::NAME }
}
```

`email_verified` is reported exactly as the provider states it. Never link
accounts by an unverified email address. Tokens have no `Debug` output.

The provider is part of the type: `SocialUser<GitHub>` and `SocialUser<Google>`
are different types. A lookup written for GitHub accounts cannot be handed a
Google user:

```rust
async fn find_by_social(&self, social: &SocialUser<GitHub>) -> Result<Option<User>, AppError>
```

## Finding and creating users

Finding and creating users stay separate, and neither belongs to this module.
`find_by_social` is a query in your app, and it only reads. `create` is your
app's ordinary database function for inserting a user: it takes a `NewUser`
and knows nothing about OAuth. The mapping between the two is one
`From<&SocialUser<GitHub>> for NewUser` in your app. Your callback handler
decides whether a missing user is created, rejected or sent to a sign-up page.

### Working with any provider

Code that handles any provider is generic, and reads the provider's name from
the type:

```rust
async fn link<P: Provider>(&self, user: &User, social: &SocialUser<P>) -> Result<(), AppError> {
    // insert (user.id, social.provider(), social.id)
}
```

This is a type parameter rather than a `ProviderKind` enum on purpose. An enum
would have to list every provider inside this module, so a third-party driver
could not exist without an `Other` variant, which is a string again.

## Adding a provider

A driver is the provider's endpoints plus one profile request. You can add a
provider without touching this module by implementing `Provider`:

```rust
pub trait Provider: Send + Sync + 'static {
    const NAME: &'static str;
    type Scope: AsRef<str>;

    fn endpoints(&self) -> Endpoints;
    fn fetch_user(&self, token: &Token) -> impl Future<Output = Result<SocialUser<Self>>> + Send
    where
        Self: Sized;
}
```
