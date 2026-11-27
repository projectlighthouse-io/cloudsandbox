---
builds_on: Nothing
phase: 2
---

# action

One trait for a unit of application logic.

## Introduction

The same idea as [Laravel Actions](https://www.laravelactions.com/): you write
a task once, then run it from a handler, a job, a listener or a command.
Actions are optional. A handler can always do its work inline.

This crate only defines the `Action` trait. It knows nothing about HTTP,
queues, events or commands. Each of those crates ships its own adapter behind
an `action` feature, so you turn on that feature in the crate you want to run
your action from.

## Installation

Add the crate from git:

```toml
[dependencies]
action = { git = "https://github.com/projectlighthouse-io/toolkit" }
```

## The trait

The trait has two types and one method:

```rust
pub trait Action {
    type Input;
    type Output;

    fn run(&self, input: Self::Input) -> impl Future<Output = Self::Output> + Send;
}
```

An action that can fail sets `Output` to a `Result`.

## Writing actions

The `impl Future + Send` line is written once, in the trait. When you implement
an action, `run` is a plain `async fn`:

```rust
struct StartRun { db: Db<Primary>, queue: Queue }

impl Action for StartRun {
    type Input = StartRunInput;
    type Output = Result<Run, AppError>;

    async fn run(&self, input: StartRunInput) -> Result<Run, AppError> {
        /* ... */
    }
}
```

Your action's dependencies are fields on the struct, set once at startup.

You never write `Send` or `Sync` yourself. They are auto traits, so an action
whose fields are thread-safe has them with nothing written. The compiler checks
this where the action is registered.

## Running actions

You can call an action directly, or hand it to one of the adapters:

| Run as | Call | The adapter requires |
| --- | --- | --- |
| Plain call | `start_run.run(input).await` | nothing |
| Handler | `r.post("/runs", route::action(start_run))` | `Input: FromRequest`, `Output: IntoResponse` |
| Job | `queue.push(&start_run, input)` | `Input: Serialize + DeserializeOwned`, `Output = Result<_, _>` |
| Listener | `events.on::<PushReceived>(start_run)` | `Input: From<PushReceived>` |
| Command | `commands.add("runs:start", start_run)` | `Input: clap::Args` |

An action that does not meet a requirement cannot be registered that way, and
the compiler names the missing bound.
