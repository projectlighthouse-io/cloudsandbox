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
validate = { git = "https://github.com/projectlighthouse-io/toolkit", features = ["route"] }
```

The `route` feature lets a handler ask for `Valid<Form<T>>`, `Valid<Json<T>>`,
`Valid<Query<T>>` or `Valid<Path<T>>`, and turns `Errors` into a response.

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

With the `route` feature, invalid input never reaches your handler. The request
is answered with a 422 that carries every error:

```json
{ "message": "The given data was invalid.", "errors": { "email": ["Must be a valid email address."] } }
```

On an Inertia visit, the `inertia` crate (feature `validate`) redirects back
instead, and the next page receives the messages as `props.errors`.

## Available rules

| Kind | Rules |
| --- | --- |
| Presence, length | `required`, `min_len`, `max_len`, `exact_len`, `min_items`, `max_items` |
| Numbers | `between`, `at_least`, `at_most`, `integer`, `numeric` |
| Choices, comparisons | `one_of`, `not_one_of`, `same`, `different`, `accepted` |
| Characters | `alpha`, `alpha_num`, `alpha_dash`, `lowercase`, `uppercase`, `starts_with`, `ends_with` |
| Formats | `email`, `url`, `uuid`, `ip`, `ipv4`, `ipv6`, `date` (`YYYY-MM-DD`) |

## Writing rules

A rule is a plain function that returns `Result<(), Message>`. The built-in
rules work this way, and you add your own the same way: write a function and
pass its result to `e.check`. The `rule` helper turns a condition and a message
into that result:

```rust
fn repo_slug(s: &str) -> Result<(), Message> {
    rule(s.split('/').count() == 2, "Must look like owner/name.")
}
```

The same building blocks cover custom messages, fields that depend on each
other, optional fields and lists:

```rust
e.check("repo", repo_slug(&self.repo));                                  // your own rule
e.check("name", required(&self.name).or_message("Tell us your name.")); // your own message
e.check("confirm", same(&self.password, &self.confirm));                 // compare two fields
e.check("bio", nullable(self.bio.as_deref(), |b| max_len(b, 500)));      // optional field
e.check(&format!("tags.{i}"), required(tag));                            // each item in a list
```

### Checks that need the database

Rules never query the database. Run a check such as "this email is taken" in an
action, and report the failure with `Errors::add` so it reaches the user the
same way as every other error.
