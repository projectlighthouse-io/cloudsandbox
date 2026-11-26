---
builds_on: `clap`
phase: 1
---

# command

Register and dispatch commands in the app binary.

## Introduction

Your app binary does more than serve HTTP. You also need to run workers, list
routes, and run maintenance tasks against production data. The command crate
gives that binary a registry of named commands, built at startup, and
dispatches to one of them based on the process arguments:
`./myapp queue:work --concurrency 8`. When no command is given, it runs the
default, usually `serve`. Help output lists every command.

The same binary runs in production, so there is no separate tool to deploy.

The crate contains no commands of its own. `serve`, `route:list`, `queue:work`
and the rest are provided by the crates that own that data, behind a `command`
feature on each. It also does not scaffold files or run the dev loop: that is
the job of the `art` CLI, which is a separate binary.

## Installation

Add the crate from git:

```toml
[dependencies]
command = { git = "https://github.com/projectlighthouse-io/cloudfoundry" }
```

Turn on the `action` feature if you want to register an `Action` as a command
with `commands.add("runs:start", action)`. This works when the action's
`Input` implements `clap::Args`.

## Writing commands

A command is an async function that takes your app context and its arguments,
parsed with `clap`. Register it under a name and run the registry with the
process arguments:

```rust
#[derive(clap::Args)]
struct PruneArgs {
    #[arg(long, default_value = "30")]
    days: u32,
}

async fn prune_runs(cx: &App, args: PruneArgs) -> Result<(), AppError> { /* ... */ }

let commands = Commands::new()
    .default("serve", serve)
    .add("runs:prune", prune_runs)
    .add("route:list", route::command::list(&router));

commands.run(app, std::env::args()).await
```

Command names use the `group:verb` form, such as `runs:prune` or `route:list`.

### Return values

A command returns a `Result`. If it returns an error, the error is printed to
stderr and the process exits with a non-zero status, so scripts and CI notice
the failure.

## Using commands from other crates

Crates that own data expose their commands behind a `command` feature. Add them
to your registry alongside your own, as `route::command::list(&router)` does
above.
