---
builds_on: `route` for the inputs, `auth` (feature) for the current user, `session` (feature) for the current team, `sqlx` for the stores
version: 0.2.1
---

# teams

Organizations and teams, with typed roles and permissions.

## Introduction

Most apps that sell to companies end up with teams: people work together on
shared data, some of them can invite others, and one of them owns it. Bigger
customers want organizations on top, with several teams under one roof.

`teams` covers both. A team either belongs to one organization or stands on its
own. Roles are enums you define, and each permission is a type you define, so a
handler says in its signature which permission it needs:

```rust
async fn invite(can: Can<InviteMembers, TeamRole>, input: Valid<Json<Invite>>) -> ... {
    // runs only for a member of {team} whose role grants InviteMembers
}
```

If no rule says which roles have `InviteMembers`, that handler does not
compile. If the user is not on the team, they get a `404`. If their role lacks
the permission, they get a `403`.

`teams` stores memberships and enforces the rules that keep them consistent. It
does not send email or decide what your teams own: projects, billing and the
rest stay in your app, keyed by the team's id.

## Installation

Add the crate with a database driver, plus `auth` and `session`:

```toml
[dependencies]
teams = { git = "https://github.com/projectlighthouse-io/toolkit", features = ["sqlite", "auth", "session"] }
```

Then publish the app code and the tables:

```bash
art publish teams --driver sqlite   # or --driver mysql; Postgres needs no flag
art publish teams --with-ui         # Inertia handlers and Vue pages instead of JSON
art migrate
```

`art publish` prints the lines to add to `src/main.rs` and `src/routes.rs`. The
layer goes after `auth::layer`:

```rust
.layer(auth::layer(users::Users { db: app.db.clone() }))
.layer(teams::layer(access::teams(&app.db), teams::auth_user(|u: &users::User| u.id)))
```

| Feature | Adds |
| --- | --- |
| `sqlite`, `postgres`, `mysql` | `SqliteStore`, `PostgresStore`, `MysqlStore` |
| `auth` | `teams::auth_user`: the user id from `auth`'s logged-in user |
| `session` | `CurrentTeam`, the team a user picked, and `require_current_team` |

## Roles and permissions

The published `src/access.rs` is yours to edit. Roles are enums:

```rust
pub enum TeamRole { Owner, Admin, Member }
pub enum OrgRole { Owner, Admin, Member }
```

Each one implements `teams::Role`: which variant is the owner, and the key it
is stored under. `OrgRole` also says which roles manage every team in the
organization.

Permissions are types. Declare them, then say which roles have each one:

```rust
pub struct InviteMembers;
impl teams::Permission for InviteMembers {}

impl teams::Grants<InviteMembers> for TeamRole {
    fn grants(&self) -> bool {
        matches!(self, Self::Owner | Self::Admin)
    }
}
```

The stub's `permissions!` macro declares several at once. To add a permission,
declare it, write its `Grants` impl and take `Can<YourPermission, TeamRole>` in
the handler.

## Routes and inputs

Routes name their team or organization by id: `/teams/{team}/...` and
`/organizations/{org}/...`. The inputs read that parameter and check access
before the handler runs.

| Input | Gives | Fails with |
| --- | --- | --- |
| `Membership<TeamRole>` | `team`, `role`, `via_org`, `user_id` | `404` |
| `Can<P, TeamRole>` | a `Membership` whose role grants `P` | `404`, `403` |
| `OrgMembership<OrgRole>` | `org`, `role`, `user_id` | `404` |
| `OrgCan<P, OrgRole>` | an `OrgMembership` whose role grants `P` | `404`, `403` |
| `CurrentTeam<TeamRole>` | the current team, or `None` | — |

A non-member gets `404` rather than `403`, so the response does not reveal that
the team exists. A request with no logged-in user gets whatever `auth` answers:
a redirect to the login page for a browser, `401` for an API client.

Inside a handler, `membership.can::<DeleteTeam>()` checks a permission without
failing the request, which suits pages that show or hide a button.

### The current team

There are no personal teams: a user creates a team or joins one through an
invitation. Which team they work in is their explicit choice. `CurrentTeam` is
the team they picked, cached in the session and stored in
`users.current_team_id` so a new login lands on it. It never falls back to
"the first team": until the user picks one, it is `None`.

```rust
async fn switch(m: Membership<TeamRole>, mut current: CurrentTeam<TeamRole>, session: Session) -> Result<Redirect, AppError> {
    current.set(&m).await?;
    Ok(Redirect::to(teams::intended(&session, &format!("/teams/{}", m.team.uuid))))
}
```

`set` takes a `Membership`, so a user can only pick a team they can open. The
published `store` handler picks a team as soon as the user creates it. If the
picked team is deleted, or the user is removed from it, the next request clears
the choice.

### Routes that need a team

Put them in a group behind `require_current_team`:

```rust
r.group("/app", |r| {
    r.layer(auth::require::<User>());
    r.layer(teams::require_current_team::<TeamRole>());
    r.get("/dashboard", handlers::dashboard);   // CurrentTeam<TeamRole> is Some here
});
```

With a current team the request passes and `CurrentTeam` is `Some` (resolved
once, by the layer). Without one:

| Request | Answer |
| --- | --- |
| Page visit, user has teams | Redirect to the picker, `/teams/select` |
| Page visit, user has no team | Redirect to "create a team", `/teams/create` |
| JSON | `409 Conflict`, `{"error": "team_required"}` |

The page they wanted is kept: after picking or creating a team, the published
handlers return there with `teams::intended(&session, default)`. Change the
paths with `.select_path("/choose")` and `.create_path("/start")`.

Leave the picker, "create a team", invitation links, account settings and
logout outside the group: they must work without a team.

### Your own columns on teams

`Team` has the columns the module needs. For more (a slug, a plan, a billing
id), add them in a migration of your own and read and write them with your own
queries, keyed by the team's id. The store never selects or writes columns it
does not know, so they survive every update of the module:

```sql
-- migrations/<timestamp>_add_slug_to_teams.up.sql
alter table teams add column slug text unique;
```

```rust
// src/team_slugs.rs: app code
pub async fn slug(db: &Db<Primary>, team: i64) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("select slug from teams where id = $1").bind(team).fetch_one(db.pool()).await
}

pub async fn set_slug(db: &Db<Primary>, team: i64, slug: &str) -> Result<(), sqlx::Error> {
    sqlx::query("update teams set slug = $1 where id = $2").bind(slug).bind(team).execute(db.pool()).await.map(drop)
}

// In a handler: the input already checked access.
let slug = team_slugs::slug(&cx.db, membership.team.id).await?;
```

Make such columns nullable or give them a default: `create_team` inserts only
its own columns. For something per member rather than per team, use a table of
your own keyed by `(team_id, user_id)` the same way.

## Organizations

An organization groups teams. Its owners and admins (whatever your
`manages_teams` returns) act as owners of every team in it without joining
them. For such a user, `Membership::via_org` is `true`.

- A team in an organization only takes members of that organization.
- Leaving an organization, or being removed from it, removes you from its teams.
- Deleting an organization deletes its teams.
- Team names are unique within an organization. Teams of their own may share names.

## Changing teams

Changes go through `Teams`, which enforces the rules. The published
`access::teams(&cx.db)` builds one:

```rust
let teams = access::teams(&cx.db);
let team = teams.create_team(user.id, None, "Platform").await?;   // the creator is the owner
teams.add_member(&team, bob, TeamRole::Member).await?;
teams.change_role(&team, bob, TeamRole::Admin).await?;
teams.remove_member(&team, bob).await?;
```

A team and an organization always keep at least one owner. Demoting or
removing the last one is `Error::LastOwner`, so ownership is handed over by
making someone else an owner first.

The published handlers add one more rule: only owners make owners, and only
owners change an owner's role.

## Invitations

Invite by email to a team or an organization, with a role:

```rust
let target = Target::Team { team: can.team.id, role: TeamRole::Member };
let new = teams.invite(target, "ada@example.com", can.user_id, Duration::from_secs(7 * 86400)).await?;
// send a link with new.token
```

The token is returned once and only its hash is stored. Accepting checks that
the logged-in user's email is the invited one, joins with the invited role and
uses up the invitation:

```rust
teams.accept(&token, user.id, &user.email).await?;
```

An expired invitation is deleted when someone tries it. An invitation to an
organization's team needs the user to be in the organization already.

The published `send_invitation` in `src/access.rs` mails the link with
[mail](../crates/mail.md), from the template `resources/mail/invitation.md`
(it names the team or organization). Publish `mail` first.

## Testing

`MemoryStore` keeps everything in memory, so rules and handlers can be tested
without a database. For the inputs, pass any `Fn(&Parts) -> Option<i64>` as the
user, for example one that reads a test header:

```rust
let teams = Teams::new(MemoryStore::<TeamRole, OrgRole>::new());
router.layer(teams::layer(teams.clone(), |parts: &Parts| {
    parts.headers.get("x-user")?.to_str().ok()?.parse().ok()
}));
```

## Things to know

- Roles are stored by `Role::key()`. Renaming a variant is free; changing a key needs a data migration. A stored key that no longer matches a role grants nothing, and is logged.
- `Can<P, R>` does not compile unless `R: Grants<P>`, so a permission cannot be checked without a rule for it.
- Non-members get `404`, never `403`.
- An organization's owners and admins see all of its teams, with `via_org` set.
- Invitation links are mailed by the stub ([mail](../crates/mail.md)), from `resources/mail/invitation.md`.
- No personal teams and no "last team" rule: a user may leave or delete all their teams; `require_current_team` then sends them to create one.
