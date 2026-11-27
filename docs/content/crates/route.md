---
builds_on: `http`, `tower`, `matchit` (radix-tree path matching), `serde` for typed parameters
version: 0.1.0
---

# route

Match a request to a handler.

## Introduction

Every request your app receives starts at the router. You tell it which paths
exist, which handler answers each one, and which middleware sits in front. It
then freezes into a lookup table that is checked once at startup and never
locked again while serving.

The router only routes. It does not parse bodies beyond handing bytes to your
handler's input type, and it knows nothing about sessions, auth, Inertia or
validation. Those live in their own crates and plug in as `tower` layers. There
are no controllers or resource routes: a handler is a plain async function.

## Installation

Add the crate from git:

```toml
[dependencies]
route = { git = "https://github.com/projectlighthouse-io/toolkit" }
```

Everything beyond routing is behind a feature:

| Feature | Adds |
| --- | --- |
| `server` | `route::serve` over hyper (HTTP/1 and HTTP/2) |
| `command` | `route::command::{list, serve}` for `route:list` and `serve` |
| `action` | `route::action(a)`: an `Action` as a handler |
| `multipart` | `Multipart`, a `multipart/form-data` input for file uploads |

## Basic routing

Create a router, register a path per HTTP method, and build it with your app
state:

```rust
let mut r = Router::new();

r.get("/", home::index).name("home");

r.group("/runs", |r| {
    r.layer(auth::require::<User>());
    r.get("/", runs::index).name("runs.index");
    r.get("/{id}", runs::show).name("runs.show");
    r.post("/", runs::store).name("runs.store");
});

let app = r.build(state)?; // freezes the router and validates it
```

Path segments in braces, such as `/runs/{id}`, are parameters. Your handler
receives them already parsed into the type it asks for.

## Writing handlers

A handler is a free async function with one typed input and one typed output:

```rust
async fn show(cx: Cx<App>, Path(id): Path<RunId>) -> Result<Page<RunShow>, AppError>
```

`Cx<S>` gives the handler your app state, and it derefs to `S`. The input is
any type that implements `FromRequest`; the toolkit ships `Path<T>`,
`Query<T>`, `Form<T>`, `Json<T>`, and `()` when there is no input at all. The
output is any type that implements `IntoResponse`.

Handlers can live in whichever module suits your app. How you group them is up
to you.

### Combining inputs

When a handler needs data from several places, take a tuple of up to four
inputs. Inputs that only read the request head, such as `Path<T>` or a
session, go first; at most one input reads the body, and it goes last:

```rust
async fn update(cx: Cx<App>, (Path(id), Json(body)): (Path<RunId>, Json<UpdateRunBody>)) -> Result<Redirect, AppError>
```

## Request bodies

`Form`, `Json` and `Bytes` read at most 2 MiB (`DEFAULT_BODY_LIMIT`). A larger
body is answered with a 413 before your handler runs. Raise the limit for a
whole router or group, or for one route:

```rust
r.body_limit(8 * 1024 * 1024);                                   // every route below, groups inherit it
r.post("/sites", sites::store).body_limit(25 * 1024 * 1024);     // one route
```

### File uploads

With the `multipart` feature, ask for `Multipart` and read the form field by
field as it streams in:

```rust
async fn upload(mut form: Multipart) -> Result<String, MultipartError> {
    while let Some(field) = form.next_field().await? {      // over the limit: 413
        let name = field.name().unwrap_or("").to_owned();
        let bytes = field.bytes().await?;
    }
    Ok("ok".into())
}
```

The whole body counts against the route's body limit, so an upload route
usually raises it.

## Named routes

Giving a route a name lets the rest of your app refer to it without repeating
its path. Change the path later and handlers, redirects and the frontend keep
working.

```rust
r.get("/runs/{id}", runs::show).name("runs.show");

let url = app.url("runs.show", &[("id", "42")])?; // "/runs/42"
Redirect::to_route("home")
Redirect::to_route("runs.show").params(&[("id", "42")])
```

`url()` returns an error if a parameter is missing or extra, so a broken link
shows up in your tests rather than in production.

### Name prefixes

Naming is optional per route. A group can set a name prefix, the same way it
sets a path prefix:

```rust
r.group("/runs", |r| {
    r.name_prefix("runs.");
    r.get("/", runs::index).name("index");     // runs.index
    r.get("/{id}", runs::show).name("show");   // runs.show
});
```

### Typed names

`.name()` and `url()` accept anything that implements `route::Name`. Strings
do, and so can an enum your app defines:

```rust
#[derive(Clone, Copy)]
enum Name { Home, RunsIndex, RunsShow, Dashboard }

impl route::Name for Name {
    const ALL: &'static [Self] = &[Self::Home, Self::RunsIndex, Self::RunsShow, Self::Dashboard];

    fn as_str(&self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::RunsIndex => "runs.index",
            Self::RunsShow => "runs.show",
            Self::Dashboard => "dashboard",
        }
    }
}

r.get("/runs/{id}", runs::show).name(Name::RunsShow);
Redirect::to_route(Name::Dashboard)
```

With an enum, a mistyped name does not compile. Build with
`r.build_named::<Name>(state)` and startup also fails if any variant in `ALL`
was never registered, so a name cannot exist without a route.

### Where names are used

| Use | Call |
| --- | --- |
| Build a URL | `app.url(name, params)` |
| Redirect | `Redirect::to_route(name)`, with `.params(...)` when the path has any |
| Exempt from a layer | `csrf::layer().except([name])` |
| List routes | `art route:list` shows the name column |
| Frontend | The route table is exported, so the app can generate a typed `route()` helper for Vue |

## Route groups

A group shares a path prefix, a name prefix and middleware across its routes.
Middleware is any `tower` layer, added with `r.layer(...)`, and it applies to
every route registered in the group. Layers are composed into each route's
service when the router is built, so a request pays nothing extra to find them.

## Building the router

`build()` checks the whole table before anything is served. Duplicate names,
two routes with the same method and path, and conflicting parameters all fail
startup with an error that names the route. A bad route table never starts
serving.

Once built, the router is immutable and lookups take no lock.

## Listing routes

The route table is exported as data, so other tools can read it without the
router depending on them. Run `art route:list` to see every method, path and
name in your app.

## Things to know

- A handler is an async function taking one input: `fn(I)` or `fn(Cx<App>, I)`. To read several things, take a tuple of up to four inputs; only the last one may read the body (`Json`, `Form`, `Bytes`, `Multipart`).
- Layers added first run outermost. Add the session layer before CSRF and auth, so they can use it.
- `build()` refuses duplicate route names, duplicate method + path pairs and conflicting parameters, so a bad route table fails at startup, not at request time.
- Bodies are limited to 2 MiB unless you raise it with `body_limit`; larger requests get `413` before your handler runs.
- Route names are `&'static str`, or an enum implementing `route::Name` for compile-time checked names.
