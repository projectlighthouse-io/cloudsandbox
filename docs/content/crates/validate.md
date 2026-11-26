---
builds_on: `serde` for serialising `Errors`
phase: 2
---

# validate

Prove that input was validated.

## Introduction

Your handlers should never have to wonder whether the data they were given has
been checked. This crate gives you a `Validate` trait to implement on your input
structs, and a `Valid<T>` wrapper that can only be built by running that
validation. If a handler receives a `Valid<T>`, the input passed.

Failures are collected in `Errors`, a bag of messages keyed by field name, and
the crate ships a small set of rule functions to get you started.

The crate does not parse requests: it wraps an input that has already been
parsed. It also does not decide what a validation failure looks like as a
response; `inertia` and your app's error type do that. And it never queries the
database, so a rule such as "unique email" belongs in an action.

## Installation

Add the crate from git:

```toml
[dependencies]
validate = { git = "https://github.com/projectlighthouse-io/cloudfoundry" }
```

## Validating input

Implement `Validate` on your input struct, checking each field with a rule.
Then ask for `Valid<...>` in your handler instead of the bare input:

```rust
#[derive(Deserialize)]
struct CreatePipeline { name: String, repo: String }

impl Validate for CreatePipeline {
    fn validate(&self, e: &mut Errors) {
        e.check("name", required(&self.name));
        e.check("name", max_len(&self.name, 80));
        e.check("repo", repo_slug(&self.repo));
    }
}

async fn store(cx: Cx<App>, input: Valid<Form<CreatePipeline>>) -> Result<Redirect, AppError> {
    let input = input.into_inner(); // CreatePipeline, known valid
    ...
}
```

`Valid<T>` has no public constructor, so there is no way to get one without
validation running and passing. Call `into_inner()` to take the checked value
out.

## Handling errors

Every rule runs. `Errors` collects every failure, not only the first, so your
user sees all of their mistakes at once.

The field names in `Errors` are the names the frontend sent. That lets a form
attach each message to the input it belongs to.

## Writing rules

A rule is a plain function that returns `Result<(), Message>`. The built-in
rules work this way, and you add your own the same way: write a function and
pass its result to `e.check`.
