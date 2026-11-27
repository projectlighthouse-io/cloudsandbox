---
builds_on: `session` (to hold `state` and the PKCE verifier between redirect and callback), `reqwest`
version: 0.1.0
---

# socialite

OAuth login with third-party providers.

## Introduction

Letting users sign in with an account they already have, such as Google or
GitHub, means speaking OAuth. `socialite` handles the protocol for you: it
builds the redirect to the provider with a `state` value and PKCE, and when the
provider sends the user back, it checks `state`, exchanges the code and fetches
the profile. You get back the provider's own user type, or one `SocialUser`
shape that looks the same for every provider.

What happens next is up to your app. `socialite` does not log anyone in and
does not store accounts: you map the user to your own and call `auth.login`.
Provider tokens are handed to your app, which decides what to do with them.

`socialite` builds on `session`, which holds `state` and the PKCE verifier
between the redirect and the callback. It has no dependency on `auth`.

## Installation

Add the module from git with a feature for each provider you use. Only those
providers are compiled:

```toml
[dependencies]
socialite = { git = "https://github.com/projectlighthouse-io/toolkit", features = ["google", "github"] }
```

| Feature | Provider | Email |
| --- | --- | --- |
| `github` | `GitHub` | Verified primary email with the `user:email` scope; otherwise the public profile email, unverified |
| `gitlab` | `GitLab`, on gitlab.com or self-hosted | Yes, with a verified flag |
| `google` | `Google` | Yes, with a verified flag |
| `reddit` | `Reddit` | No |
| `twitter` | `Twitter` (X) | No |
| `oidc` | `Oidc`: any OpenID Connect provider (Okta, Keycloak, Azure AD, ...) | Yes, with a verified flag |
| `auth0` | `Auth0` (turns on `oidc`) | Yes, with a verified flag |

The `route` feature adds ready-made routes; see
[Ready-made routes](#ready-made-routes).

## Configuring a provider

Create the provider once at startup with your client credentials and the URL
of your callback route, and keep it in your app state:

```rust
use socialite::{GitHub, Google};

let google = Google::new(&cfg.google_client_id, &cfg.google_client_secret, "https://myapp.com/auth/google/callback");
let github = GitHub::new(&cfg.github_client_id, &cfg.github_client_secret, "https://myapp.com/auth/github/callback")
    .scopes(&["read:user"]);
```

Each provider starts with sensible scopes (`openid email profile` for Google,
GitLab and OpenID Connect; `identity` for Reddit; `users.read tweet.read` for
X), and `.scopes(...)` replaces them.

Some providers take a few extra options:

```rust
Google::new(id, secret, callback)
    .hosted_domain("acme.com")      // only accounts of this Google Workspace domain
    .offline()                       // ask for a refresh token
    .prompt("select_account");       // always show the account picker

GitLab::new(id, secret, callback).base_url("https://gitlab.example.com");     // self-hosted
Reddit::new(id, secret, callback).user_agent("web:myapp:v1.0 (by /u/me)");   // Reddit asks for one

let okta = Oidc::discover("okta", "https://acme.okta.com", id, secret, callback).await?;  // reads the discovery document
let auth0 = Auth0::new("acme.eu.auth0.com", id, secret, callback);
```

`hosted_domain` is checked against the user Google returns, not just passed to
Google as a hint, because a hint can be removed from the URL.

## Redirecting and handling the callback

Two handlers complete the flow: one sends the user to the provider, and one
receives them when they come back. Both are your app's code. Bring the
`Provider` trait into scope to call the flow methods:

```rust
use socialite::{Callback, Provider};

// GET /auth/google
async fn redirect(cx: Cx<App>, session: Session) -> Redirect {
    Redirect::to(cx.google.authorize_url(&session))
}

// GET /auth/google/callback
async fn callback(cx: Cx<App>, (auth, session, Query(cb)): (Auth<User>, Session, Query<Callback>)) -> Result<Redirect, AppError> {
    let social = cx.google.social_user(&session, cb).await?;
    let user = cx.users.find_or_create_social(&social, auth.user()).await?;
    auth.login(&user);
    Ok(Redirect::to_route("dashboard"))
}
```

`authorize_url` stores a random `state` and a PKCE verifier in the session.
The callback checks `state` in constant time and uses it once, so a forged or
replayed callback fails with `Error::State`. PKCE (`S256`) is on for every
provider that accepts it; Reddit does not, so it is off there.

If the user declines, or the provider reports an error, you get
`Error::Denied`. A network or decoding failure is `Error::Http`.

## Ready-made routes

With the `route` feature, one call registers both routes for a provider:

```rust
socialite::routes(&mut r, google, handlers::social::callback);
// GET /auth/google            redirects to Google           (route name `social.google`)
// GET /auth/google/callback   runs your callback handler    (route name `social.google.callback`)
```

Your callback takes a `SocialCallback`. By the time your handler runs, `state`
has been checked and the code exchanged; `SocialCallback` holds the result:

```rust
async fn callback(cx: Cx<App>, (auth, social): (Auth<User>, SocialCallback)) -> Redirect {
    match social.0 {
        Ok(user) => { /* find or create the account, auth.login, redirect */ }
        Err(e) => { /* declined, forged state, network failure: your choice of page */ }
    }
}
```

Register each provider you configured. The session layer must run before
these routes.

## Publishing the social login files

`art publish socialite` copies the account handling into your app, after
`art publish auth`. Add `--driver sqlite` or `--driver mysql` for those
databases, and `--with-ui` for the Inertia callback and the buttons:

| Published to | Contents |
| --- | --- |
| `src/social.rs` | `Users::find_or_create_social`, the linking rules |
| `src/handlers/social.rs` | The callback for every provider |
| `resources/js/components/SocialButtons.vue` | "Continue with ..." links for the configured providers (`--with-ui`) |
| `migrations/<timestamp>_create_social_accounts.*.sql` | One row per linked provider account |
| `migrations/<timestamp>_allow_null_passwords.*.sql` | Makes `users.password_hash` nullable |

`find_or_create_social` decides who the user is, in this order:

1. An account already linked to this provider account logs in.
2. A logged-in user gets the provider account linked to them.
3. An account with the same **verified** email is the same person: the
   provider account is linked to it.
4. Otherwise a new account is created, with the provider's name and verified
   email and no password.

Steps 3 and 4 need an email the provider has verified (`email_verified` is
`Some(true)`). Without one, the sign-in is refused with a message asking the
person to log in first and connect the provider from there. Reddit and X share
no email address at all, so they can only be connected that way too.

> [!WARNING]
> Linking by email trusts the provider's word that the person owns that
> address. If a provider returns an email it has not verified, anyone could
> sign up there with your user's address and then sign in to your app as that
> user. `src/social.rs` therefore only links or creates by a verified email.
> It has a flag, `TRUST_UNVERIFIED_EMAILS`, which defaults to `false`; leave it
> that way. For GitHub, ask for the `user:email` scope, so that `socialite` can
> read the verified primary email; without it, GitHub's verification is unknown
> and GitHub users can only connect after logging in.

Accounts made this way have no password. Password login refuses them, and they
can set a password through "forgot password". The `allow_null_passwords`
migration makes that column nullable. Postgres and MySQL change the column in
place. SQLite cannot, so its migration rebuilds the `users` table with foreign
keys switched off; if you changed `users`, list your columns in that file.

With `--with-ui`, share the list of configured providers with every page so
the buttons know what to show:

```rust
inertia::layer().share(move |_| serde_json::json!({ "social": enabled.clone() }))
```

## The social user

`user(&session, cb)` returns the provider's own type (`GoogleUser`,
`GitHubUser`, `OidcUser`, ...), with every field that provider reports.
`social_user` returns the same user in one shape for all providers:

```rust
pub struct SocialUser {
    pub provider: &'static str,      // "google", "github", or the name you gave an Oidc provider
    pub id: String,                  // the provider's stable id for this user
    pub name: Option<String>,
    pub email: Option<String>,
    pub email_verified: Option<bool>, // None when the provider does not say
    pub avatar: Option<String>,
    pub token: String,
    pub refresh_token: Option<String>,
}
```

Store `(provider, id)` to recognise a returning user: emails can change, the
provider's id does not. Reddit and X share no email address, so for them
`email` is always `None`.

`OidcUser::claims` keeps every claim the provider returned as JSON, for custom
ones such as groups or roles.

## Several OpenID Connect providers

The first argument to `Oidc::new` and `Oidc::discover` is the provider's name.
It keys the session values and becomes `SocialUser::provider`, so one app can
offer several OpenID Connect providers side by side:

```rust
let okta = Oidc::discover("okta", "https://acme.okta.com", ...).await?;
let keycloak = Oidc::discover("keycloak", "https://sso.acme.com/realms/main", ...).await?;
```

## Adding a provider

A provider is its endpoints plus one profile request. Implement `Provider`
in your app, and the redirect, `state`, PKCE and code exchange come for free:

```rust
pub trait Provider: Send + Sync {
    type User: Into<SocialUser> + Send;

    fn name(&self) -> &'static str;
    fn oauth(&self) -> &OAuth2;                                   // credentials, endpoints, scopes
    fn extra_params(&self) -> Vec<(&'static str, String)> { ... }  // optional authorize parameters
    fn fetch_user(&self, token: Token) -> impl Future<Output = Result<Self::User, Error>> + Send;
}
```

`OAuth2` also chooses how the client secret is sent to the token endpoint
(`ClientAuth::Post` in the form, or `ClientAuth::Basic`) and whether PKCE is
used.

## Things to know

- Enable one Cargo feature per provider (`google`, `github`, ...) plus `route` for the ready-made routes.
- Bring the `Provider` trait into scope to call `authorize_url`, `user` and `social_user`.
- Link accounts by email only when the provider verified it; the published `src/social.rs` enforces this.
