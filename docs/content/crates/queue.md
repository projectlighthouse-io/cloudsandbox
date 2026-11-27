---
builds_on: `serde`, `serde_json`, `tokio`
phase: 5
---

# queue

Persist, claim, run and retry jobs.

## Introduction

Some work does not belong inside a request: sending a notification, running a
build, calling a slow external service. The queue lets you push that work as a
job, store it in your database, and have a worker pick it up and run it in the
background.

You can push a job straight away or with a delay. Workers claim jobs from named
queues, run them with a concurrency limit you choose, and record the result.
Failed jobs are retried with backoff up to a maximum number of attempts, and
jobs that run out of attempts land in a failed-jobs table, where you can retry
them by id. When a worker shuts down, it stops claiming new jobs, finishes the
ones it is running, and then exits.

The queue does not schedule recurring work; that would be a separate
`schedule` crate. It also does not guarantee exactly-once delivery. Delivery is
at-least-once, so your jobs must be idempotent: running the same job twice
should leave your app in the same state as running it once.

## Installation

Add the crate from git:

```toml
[dependencies]
queue = { git = "https://github.com/projectlighthouse-io/toolkit" }
```

The queue stores jobs through `sqlx`, so turn on the feature for your database.
Each driver claims jobs in the way that suits its database:

| Feature | Adds |
| --- | --- |
| `postgres` | Store using `FOR UPDATE SKIP LOCKED` |
| `mysql` | Store using `FOR UPDATE SKIP LOCKED` |
| `sqlite` | Store using a single-writer claim |
| `action` | `queue.push(&action, input)` |

The `action` feature lets you push an `Action` and its input as a job.

## Defining jobs

A job is a serialisable struct that implements `Job`. Give it a `NAME`, the
context type your app passes to workers, and a `run` method:

```rust
#[derive(Serialize, Deserialize)]
struct ExecuteRun { run_id: RunId }

impl Job for ExecuteRun {
    const NAME: &'static str = "execute_run";
    type Context = App;

    async fn run(self, cx: &App) -> Result<(), JobError> { /* ... */ }
}

queue.push(ExecuteRun { run_id }).await?;
queue.push_in(Duration::from_secs(30), ExecuteRun { run_id }).await?;
```

`Job::NAME` is the stable identifier stored alongside the payload. Because the
queue looks jobs up by that name rather than by the Rust type, you can rename
the type without stranding jobs that are already queued.

## Dispatching jobs

Call `queue.push(job)` to run a job as soon as a worker is free, or
`queue.push_in(delay, job)` to hold it back for a while, as shown above.

### Dispatching inside a transaction

When the queue and your app share a database, you can enqueue a job inside
your own transaction with `queue.push_in_tx(&mut tx, job)`. The job is only
stored if the transaction commits, so a job is never queued for data that was
rolled back.

## Running workers

A worker claims jobs and runs them against your app context. Register every
job type it may receive, choose the queues it listens on, and set how many jobs
it runs at once:

```rust
let worker = Worker::new(queue, app)
    .register::<ExecuteRun>()
    .queues(["default", "runs"])
    .concurrency(8);

worker.run(shutdown_signal()).await?;
```

Every job a worker can receive must be registered at startup. If a worker
claims a job whose name it does not know, the job goes to failed jobs with a
clear error instead of being dropped.

When the shutdown signal fires, the worker stops claiming, waits for the jobs
it is running to finish, and returns.

### Crashed workers

A claimed job has a visibility timeout. If the worker that claimed it dies
before recording a result, the job becomes claimable again once the timeout
passes, and another worker picks it up. This is one reason jobs need to be
idempotent.

## Writing a store

Storage sits behind the `Store` trait. The bundled drivers implement it, and
you can implement it yourself to keep jobs somewhere else:

```rust
pub trait Store: Send + Sync + 'static {
    fn push(&self, job: NewJob) -> impl Future<Output = Result<JobId>> + Send;
    fn claim(&self, queues: &[&str], limit: u32) -> impl Future<Output = Result<Vec<Claimed>>> + Send;
    fn complete(&self, id: JobId) -> impl Future<Output = Result<()>> + Send;
    fn fail(&self, id: JobId, error: &str, retry_at: Option<Timestamp>) -> impl Future<Output = Result<()>> + Send;
}
```
