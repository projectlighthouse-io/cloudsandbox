# App layout

What `art new myapp` produces: one Cargo package with one binary that serves
HTTP and runs commands.

## Directory structure

The directories are a starting convention, not a requirement. The CLI relies
on only two of them: `migrations/` and `resources/js/pages/`. Everything else
you can rearrange to suit your app.

```text
myapp/
├── Cargo.toml
├── .env.example
├── src/
│   ├── main.rs        # explicit wiring: config, db, queue, events, router
│   ├── state.rs       # App struct: handles and actions
│   ├── routes.rs      # every route in one place
│   ├── config.rs      # typed config from the environment
│   ├── error.rs       # AppError -> response
│   ├── handlers/      # grouped however the app likes
│   ├── actions/
│   ├── requests/      # input structs and their validation
│   ├── responses/     # view structs and page props
│   ├── middleware/
│   ├── jobs/
│   ├── events/        # event structs and listeners
│   └── commands/
├── migrations/        # sqlx migrations
├── .sqlx/             # offline query cache, committed
├── resources/
│   ├── js/
│   │   ├── app.ts
│   │   └── pages/     # Inertia pages (.vue)
│   └── views/app.html # root HTML template
├── public/
├── tests/
├── package.json
└── vite.config.ts
```

You will notice there is no `models/` directory, and no generator for one.
Where your row structs and queries live is your decision.

## Wiring the app together

There is no glue crate tying the pieces together. Instead, `main.rs` is
generated once and then belongs to your app. It reads the config, connects to
the database, and builds the queue, events, router and commands in plain
sight:

```rust
#[tokio::main]
async fn main() -> Result<(), AppError> {
    let cfg = Config::from_env()?;

    let db: Db<Primary> = Db::connect(&cfg.database_url).await?;
    let queue = Queue::new(queue::PostgresStore::new(db.pool()));
    let codec = securecookie::Codec::new(cfg.app_keys()?);
    let vite = Vite::load("public/build")?;

    let app = App::new(cfg, db.clone(), queue.clone());

    let events = events::register(&app, &queue).build();
    let router = routes::register(Router::new())
        .layer(session::layer(session::PostgresStore::new(db.pool()), codec))
        .layer(csrf::layer().except(["webhooks.github"]))
        .layer(inertia::layer().version(vite.manifest_hash()))
        .build(app.clone())?;

    commands::register(&app, &router, &queue)
        .run(app, std::env::args())
        .await
}
```

Every dependency is visible here. If you want to remove a feature, delete its
line.

## The life of a request

When a request arrives, `route` matches its path and picks the handler along
with that route's middleware stack. The layers then run in order: the session
is loaded, the CSRF token is checked, and the auth guard decides whether the
request may continue.

Next, the handler's input type parses the request, whether that is a `Path`,
a `Form`, a `Valid<...>` or an `Authed<User>`. The handler runs, usually by
calling an action, and its output type becomes a response: a `Page<P>`, a
`Json<T>` or a `Redirect`.

On the way out, the layers finish their work. Inertia turns a `Page` into HTML
or JSON, and the session is saved if it changed.

## Shaping responses

You do not need a crate to keep database rows out of your responses. A
response transformer is a `Serialize` struct with a `From<&Domain>`
implementation:

```rust
#[derive(Serialize)]
struct RunView { id: RunId, status: RunStatus, started_at: Timestamp }

impl From<&Run> for RunView { /* ... */ }
```

This keeps your rows out of responses without adding any new abstraction.

## Working with AI agents

The docs are written to be read by coding agents as well as people:

- [llms.txt](/llms.txt) describes the toolkit and links every page as Markdown;
  [llms-full.txt](/llms-full.txt) is all of it in one file.
- Every page has a Markdown version: add `.md` to its address, for example
  `/docs/crates/route.md`, or use the **Markdown** and **Open in Claude** buttons at the top
  of the page.
- `art new` writes an `AGENTS.md` into your app, with a `CLAUDE.md` that imports it. It
  describes the layout, the `art` commands and the rules above, and links the docs.
  `art publish <crate>` adds that crate's notes. Codex, Cursor and Claude Code read it
  automatically, so an agent working in your app starts from the same conventions you do.
