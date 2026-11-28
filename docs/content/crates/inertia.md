---
builds_on: `http`, `tower`, `serde`, `serde_json`
version: 0.1.0
---

# inertia

Speak the Inertia protocol (v3) for a `tower` service.

## Introduction

Inertia lets you build a single-page frontend while keeping routing, data
loading and validation on the server. Your handlers return a page component
name and its props; the Inertia client in the browser renders them.

This crate speaks the Inertia v3 protocol for you. A layer turns each page into
full HTML on a first visit or JSON on an Inertia visit. It also handles asset
version checks, `303` redirects after `PUT`, `PATCH` and `DELETE`, partial
reloads, lazy and deferred props, and shared props added to every page.

It does not route, validate, authenticate or bundle assets; other crates do
those. It does not render JavaScript on the server either: there is no SSR for
now.

## Installation

Add the crate from git:

```toml
[dependencies]
inertia = { git = "https://github.com/projectlighthouse-io/toolkit" }
```

Two optional features connect it to other crates:

| Feature | Adds |
| --- | --- |
| `validate` | A validation failure on an Inertia visit redirects back with `Errors` as `props.errors` |
| `session` | Flash storage for those errors across the redirect |

## Rendering pages

Call `inertia::render` with a component name and a props struct. It returns a
`Page<P>`:

```rust
#[derive(Serialize)]
struct RunShow { run: RunView }

async fn show(cx: Cx<App>, Path(id): Path<RunId>) -> Result<Page<RunShow>, AppError> {
    let run = cx.runs.find(id).await?;
    Ok(inertia::render("runs/show", RunShow { run: RunView::from(&run) }))
}
```

Returning `Page<P>` is what makes a handler an Inertia endpoint. There is no
derive and no attribute. Handlers that return `Json<T>`, `Redirect` or a plain
response pass through the layer untouched.

Props are serialised once, from your typed struct straight to bytes. TypeScript
types for your props are generated in your app, not by this crate.

The component name is a string. In dev, a missing component is reported at
startup by checking the pages directory.

## Configuring the layer

The layer does all of the protocol work. Give it your root template, the asset
version and any props to share with every page:

```rust
let inertia = inertia::layer()
    .root_template("resources/views/app.html")
    .version(vite.manifest_hash())
    .share(|req| Shared { flash: ..., auth: ... });
```

The split between the two is strict. `Page<P>` is plain data and knows nothing
about the request; only the layer sees the request headers. A `Page` returned on
a route without the layer cannot be rendered, so the router build detects this
and fails startup.

### Asset versioning

When the version a client sends does not match the current one, the layer
responds with `409` and an `X-Inertia-Location` header, and the client reloads
the page to pick up the new assets.

### From a config file

`inertia::Config` covers the root template, the pages directory, history
encryption and SSR. The asset version, `head` and `share` stay methods, since
they are code. `.config(cfg)` reads the template right away and fails instead
of falling back: a missing template, one without `{{ inertia }}`, an SSR URL
that is not `http://`, or `ssr_url` without the `ssr` feature is a
`ConfigError`. An empty string means unset, so `INERTIA_SSR_URL=` turns SSR
off.

```toml
[inertia]
root_template = "resources/views/app.html"
pages_dir = "resources/js/pages"
ssr_url = "env(INERTIA_SSR_URL)"
ssr_timeout = "2s"
```

```rust
inertia::layer().config(cfg.inertia)?.version(&vite.manifest_hash())
```

Turn on the `configured` feature to load it with
[configured](configured.md).

## Expensive props

Some props are costly to compute or not needed on the first render. Wrap them
in `Lazy` or `Defer`:

```rust
struct RunShow {
    run: RunView,
    logs: Lazy<Vec<LogLine>>,       // computed only when asked for
    artifacts: Defer<Vec<Artifact>> // fetched after first render
}
```

A `Lazy` prop is computed only when a partial reload asks for it. A `Defer` prop
is left out of the first render and fetched afterwards.

## Validation errors

With the `validate` feature, a validation failure on an Inertia visit redirects
back with the `Errors` available as `props.errors`. Add the `session` feature to
flash those errors across the redirect.

## Things to know

- Layer order: session, CSRF, auth, then Inertia (first added is outermost).
- Props are any `Serialize` type; derive `ts_rs::TS` with `#[ts(export)]` to generate TypeScript types for the page.
- Validation errors on an Inertia request become a redirect back with the errors flashed for the next page; a changed asset version answers `409` so the client reloads.
