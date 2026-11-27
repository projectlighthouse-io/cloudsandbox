---
builds_on: `serde` for reading field names and serialising `Errors`; `validate-derive` for `#[derive(Validate)]`
version: 0.2.0
---

# validate

Prove that input was validated.

## Introduction

Your handlers should never have to wonder whether the data they were given has
been checked. With `validate`, you write the rules as attributes on your input
struct, and a handler takes `Valid<T>`: a value that can only exist once every
rule has passed. If the input is wrong, the handler never runs.

Failures are collected in `Errors`, keyed by the field names the client sent,
with one message per field. There are a dozen built-in rules for the common
cases, and writing your own is a small trait.

The crate does not parse requests or query the database. It validates an input
that has already been parsed, and a rule such as "the email is not taken yet"
belongs in your handler.

## Installation

Add the crate from git:

```toml
[dependencies]
validate = { git = "https://github.com/projectlighthouse-io/toolkit", features = ["route"] }
```

| Feature | Adds |
| --- | --- |
| `derive` (on by default) | `#[derive(Validate)]` |
| `route` | `Valid<Form<T>>`, `Valid<Json<T>>`, `Valid<Query<T>>` and `Valid<Path<T>>` as handler inputs; `Errors` as a `422` response |

## Validating input

Derive `Validate` next to `Deserialize` and put the rules on the fields:

```rust
use serde::Deserialize;
use validate::Validate;

#[derive(Deserialize, Validate)]
pub struct NewPipeline {
    #[validate(required, max = 80)]
    pub name: String,

    #[validate(required, url)]
    pub repository: String,

    #[validate(nullable, max = 500)]
    pub description: Option<String>,

    #[validate(one_of("main", "nightly"))]
    pub channel: String,
}
```

Then take it in a handler. The rules run before the handler does:

```rust
async fn store(cx: Cx<App>, input: Valid<Json<NewPipeline>>) -> Result<Redirect, AppError> {
    let Json(pipeline) = input.into_inner();
    // every rule passed
}
```

Rules run in the order you write them, and a field stops at its first failure,
so an empty name reports "This field is required." and nothing else.

An `Option` field may be left out: `None` passes every rule except `required`.
`nullable` changes nothing; it just makes the attribute read like the intent.

## Handling errors

When a rule fails, the client gets a `422` with every field's message:

```json
{ "message": "The given data was invalid.", "errors": { "repository": ["Must be a valid URL."] } }
```

Under Inertia the visit is redirected back instead, and the page receives the
messages as `props.errors`, ready to show under each input.

The keys are the names the client sent. If a field is renamed with
`#[serde(rename = "repo")]` or the struct uses `#[serde(rename_all = "camelCase")]`,
the errors use those names too, so they line up with your form.

## Available rules

| Attribute | Checks |
| --- | --- |
| `required` | Text is not blank, a list is not empty, an `Option` is `Some` |
| `nullable` | Marks an `Option` field: `None` passes |
| `min = n`, `max = n` | Characters for text, items for lists, the value for numbers |
| `between(a, b)` | Within `a` to `b`, inclusive, measured the same way |
| `email`, `url`, `uuid` | Text shaped like an email address, an `http(s)` URL, a UUID |
| `date` | A `YYYY-MM-DD` date that exists |
| `integer`, `numeric` | A number written as text, such as a query parameter |
| `one_of("a", "b")` | One of a fixed set of values, text or numbers |
| `same = other_field` | Equal to another field, e.g. a password confirmation |
| `accepted` | `true`, for "I accept the terms" |
| `rule = MyRule` | Your own rule |
| `with = my_function` | A function that can read other fields |

Lengths count characters, not bytes, and `String` fields reach every rule as
`&str`.

## Writing rules

A rule is a type that implements `Rule`. Give it a name (used for custom
messages) and a check:

```rust
use validate::{Message, Rule, rule};

pub struct RepoSlug;

impl Rule<str> for RepoSlug {
    fn name(&self) -> &str { "repo_slug" }
    fn check(&self, value: &str) -> Result<(), Message> {
        rule(value.split_once('/').is_some_and(|(o, n)| !o.is_empty() && !n.is_empty()), "Use owner/name.")
    }
}

#[derive(Deserialize, Validate)]
pub struct NewPipeline {
    #[validate(required, rule = RepoSlug)]
    pub repo: String,
}
```

A rule can carry settings, like the built-in `max(80)`: make the type a struct
with fields and build it in the attribute, `rule = Prefix("acme/")`.

### Rules that read other fields

`with` on a field calls your function with the whole struct and the field's
value, so a rule can depend on the rest of the input:

```rust
#[derive(Deserialize, Validate)]
pub struct NewUser {
    pub admin: bool,
    #[validate(required, with = team_rules)]
    pub team: String,
}

fn team_rules(user: &NewUser, team: &str) -> Result<(), Message> {
    if user.admin { rule(team == "staff", "Admins belong to the staff team.") } else { Ok(()) }
}
```

`with` on the struct runs after every field rule and can report on any field,
which suits whole groups of rules that switch on or off:

```rust
#[derive(Deserialize, Validate)]
#[validate(with = admin_rules)]
pub struct NewUser {
    pub admin: bool,
    pub permissions: Vec<String>,
}

fn admin_rules(user: &NewUser, e: &mut Errors) {
    if user.admin {
        e.field("permissions", &user.permissions).rule(min(1));
    }
}
```

### Checks that need the database

Validation is synchronous and only sees the input. A check that needs the
database or the logged-in user, such as "this email is already taken", goes in
the handler after `Valid<T>`, and reports the same way:

```rust
if cx.users.by_email(&input.email).await?.is_some() {
    let mut errors = Errors::new();
    errors.add("email", "The email has already been taken.");
    return Err(errors.into_response());
}
```

## Custom messages

Every rule has a default message. To word one your way, list it in a table keyed
`field.rule`, where the rule part is the attribute's name or your rule's `name()`:

```rust
#[derive(Deserialize, Validate)]
#[validate(messages = messages)]
pub struct NewPipeline {
    #[validate(required, max = 80)]
    pub name: String,
    #[validate(required, rule = RepoSlug)]
    pub repo: String,
}

fn messages() -> Messages {
    Messages::new()
        .set("name.required", "Name must be provided for this action.")
        .set("repo.repo_slug", "Which repository should this pipeline build?")
}
```

Rules you do not list keep their default message.

## Without the derive

The attributes write ordinary code, and you can write it yourself: implement
`Validate` and check each field with `e.field(...)`.

```rust
impl Validate for NewPipeline {
    fn validate(&self, e: &mut Errors) {
        e.field("name", self.name.as_str()).rule(required()).rule(max(80));
    }
}
```

## Things to know

- `Valid<T>` can only be built by running the rules, so a handler taking `Valid<Json<T>>` never sees unchecked data.
- Each field reports its first failure; the rules after it do not run.
- Error keys follow `#[serde(rename)]` and `#[serde(rename_all)]`, so they match the names the client sent.
- With the `route` feature, `Errors` is a response: `422` with every field's message. Under Inertia it becomes a redirect back with the errors on the form.
- Database and session checks belong in the handler, after `Valid<T>`.
