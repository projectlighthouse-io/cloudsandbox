---
builds_on: `serde_json`
version: 0.1.0
---

# vite

Resolve asset tags from Vite.

## Introduction

Vite bundles your frontend, and your server-rendered HTML needs to point at the
files it produces. This crate gives you the right `<script>` and `<link>` tags
for an entry point, whether you are running the Vite dev server or serving a
production build.

In production it reads Vite's manifest. In development it detects the running
dev server and returns tags that point at it, with hot reload. It also exposes a
hash of the manifest, which you can use as the Inertia asset version.

The crate does not run Vite or Node; `art dev` and `art build` do that. It does
not serve static files either.

## Installation

Add the crate from git:

```toml
[dependencies]
vite = { git = "https://github.com/projectlighthouse-io/toolkit" }
```

## Loading Vite

Load Vite once at startup, pointing it at your build directory, then ask it for
the tags for your entry points:

```rust
let vite = Vite::load("public/build")?; // manifest, or dev server if the hot file exists

let tags: String = vite.tags(&["resources/js/app.ts"])?;
let version: &str = vite.manifest_hash();
```

Place `tags` in the `<head>` of your root template. Pass `version` to the
Inertia layer so browsers pick up a new build when it changes.

## Production builds

In production the manifest is read once at startup. If an entry point you ask
for is missing from the manifest, startup fails with an error rather than
serving a broken page.

## Development

When the hot file exists, Vite is running in dev mode and the tags point at the
dev server. The hot file is checked on every request, so you can start or stop
Vite without restarting your app.
