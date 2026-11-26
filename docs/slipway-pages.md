# Slipway: pages and props

What each Slipway page receives from the server and what it lets the user do.
Design against this; the backend already serves it (`src/slipway/handlers.rs`,
checked by `tests/slipway.rs`). TypeScript types: `resources/js/types/slipway.ts`.

Each page demonstrates one Inertia feature. The design should make that feature
visible (the original brief puts a small "Inertia: …" badge in each page header),
and give every list an empty, loading and error state.

## On every page (shared props)

| Prop | Shape | Notes |
| --- | --- | --- |
| `auth.user` | `{ id, name, email, avatar }` | `avatar` is always `null` in the mock: design an initials fallback |
| `team` | `{ id, name }` | The current team |
| `teams` | `Team[]` | For the team switcher (2 teams) |
| `flash` | `{ success, error }` | Strings or `null`, for toasts. Present for one request after an action |
| `errors` | `{ field: message }` | Validation errors after a failed form (see Sites, Settings) |

Shell actions: switch team (`POST /slipway/teams/{id}`), sign out (`POST /slipway/logout`),
command palette (⌘K) backed by `GET /slipway/palette`, which returns
`{ kind: "server" | "site", label, hint, href }[]`.

### Status values

| Entity | Values |
| --- | --- |
| Server | `active` (ok), `provisioning` (pending), `stopped` (neutral), `failed` |
| Site | `deployed` (ok), `deploying` (pending), `suspended` (neutral), `failed` |
| Deployment | `queued`, `running` (pending), `finished` (ok), `failed`, `cancelled` (neutral) |

Times are Unix milliseconds. IPs, commit hashes, commands and logs are monospace.

## 1. Overview — `/slipway`

**Feature: deferred props and polling.** The page paints with the tiles and the list;
the sparklines arrive about 0.4 s later (design a skeleton), then refresh every 5 s.

| Prop | Shape |
| --- | --- |
| `tiles` | `{ servers, sites, deploysToday, failed }` (numbers) |
| `recent` | `Deployment[]` (8 newest) |
| `sparklines` | deferred: `{ serverId, name, status, cpu: number[24], memory: number[24] }[]`, values 1–99 (%) |

## 2. Servers — `/slipway/servers`

**Feature: partial reloads.** Searching, filtering, sorting and paging reload only the
table; the URL query string follows (`?search=&provider=&status=&sort=&dir=&page=`),
and scroll position and input focus are kept.

| Prop | Shape |
| --- | --- |
| `servers` | `{ data: Server[], meta: { page, perPage: 10, total } }` |
| `filters` | `{ search, provider, status, sort, dir }` (current values) |
| `providers` | `string[]` (Hetzner, DigitalOcean, AWS, Vultr, Linode) |
| `statuses` | `string[]` |

`Server`: `{ id, name, provider, region, ip, status, cpu, memory, disk, sitesCount }`
(cpu/memory/disk in %). Sortable: `name`, `region`, `cpu`, `memory`, `sitesCount`.
Design: a table (sticky header), stacked cards on phones, a "no servers match" state.

## 3. Server — `/slipway/servers/{id}/{tab}`

**Feature: nested persistent layout, a page per tab, lazy props, prefetch on hover.**
A server header and tab bar stay put while the tab content changes. Tabs: `sites`,
`metrics`, `scheduler`, `workers`, `logs` (each its own URL). Every tab gets `server`.

| Tab | Props |
| --- | --- |
| Sites | `sites: Site[]` |
| Metrics | `history` (deferred, ~0.3 s): `{ cpu, memory, disk: number[60], load: number[60] (0–4), bandwidth: number[60] (Mbps) }`, one point per minute |
| Scheduler | `tasks: { id, command, frequency, nextRunAt }[]` |
| Workers | `workers: { id, connection, queue, processes, status: "running" \| "stopped" }[]` |
| Logs | `lines` (deferred, ~0.5 s): `string[40]`, each `timestamp LEVEL host message` (INFO/WARN/ERROR) |

Header actions: New site (link), Reboot (confirm dialog, `POST …/reboot`, then a toast).
An unknown or other-team server shows the 404 page.

## 4. Site form — `/slipway/sites/create`, `/slipway/sites/{id}/edit`

**Feature: forms with server validation, a dirty-state guard and upload progress.**

| Prop | Shape |
| --- | --- |
| `site` | `null` (create) or `{ id, serverId, domain, repository, branch, runtime, env: {key, value}[], hasCertificate }` |
| `servers` | `{ id, name }[]` |
| `runtimes` | `string[]` (Node 22, Rust, Go 1.23, Python 3.12, Static) |

Fields: server (select), domain, repository (`owner/name`), branch, runtime (select),
environment variables (add/remove key/value rows), SSL certificate (optional `.pem`
file, up to 1 MB). Submitted as `multipart/form-data`: create `POST /slipway/sites`,
edit `PUT /slipway/sites/{id}`.

Errors come back per field (`errors.domain`, `errors["env.2.key"]`, `errors.certificate`, …):

| Field | Possible messages |
| --- | --- |
| `serverId` | Pick a server. |
| `domain` | This field is required. · Must be a domain like app.example.com. · This domain is already taken. |
| `repository` | Must look like owner/name. |
| `branch` | No spaces. · Must be at most 100 characters. |
| `runtime` | Must be one of the allowed values. |
| `env.N.key` | Use UPPER_SNAKE_CASE. · Duplicate key. |
| `certificate` | Must be a PEM certificate. · Must be at most 1 MB. |

Design: an upload progress bar, a "leave with unsaved changes?" prompt, an edit state
showing a certificate is already installed. Success returns to the server's Sites tab
with a toast.

## 5. Deployments — `/slipway/deployments`

**Feature: infinite scroll and optimistic updates.** The feed loads 15 at a time as the
user scrolls. "Deploy now" and "Cancel" update the list instantly and roll back if the
server refuses. A new deploy moves `queued` → `running` (1.5 s) → `finished`/`failed`
(3–6 s later).

| Prop | Shape |
| --- | --- |
| `deployments` | `Deployment[]` (15 per page, newest first) |
| `sites` | `{ id, domain, status }[]` for the "Deploy now" picker |
| `log` | only on request (`?log={id}`): `{ id, lines: string[] }` for an expanded row |

`Deployment`: `{ id, siteId, site, commit, message, author, branch, status, startedAt, durationMs }`
(`durationMs` is `null` while running).

Actions: deploy (`POST /slipway/deployments` `{ siteId }`), cancel
(`POST /slipway/deployments/{id}/cancel`), expand a row's log. Refusals to design for:
a suspended site ("… is suspended; deploys are blocked.") and cancelling a finished
deploy ("Only queued or running deployments can be cancelled.").

## 6. Activity — `/slipway/activity`

**Feature: load when visible, scroll restoration.** The last three days render at once;
older days load when the user scrolls near the end. Coming back to the page restores
the scroll position.

| Prop | Shape |
| --- | --- |
| `days` | `{ date: "YYYY-MM-DD", label: "Today" \| "Yesterday" \| date, entries: Activity[] }[]` (3) |
| `older` | loaded on scroll: the same shape, the remaining ~11 days |

`Activity`: `{ id, at, actor, action, target }`, read as "actor action target"
("Grace Hopper restarted atlas-web-1").

## 7. Settings — `/slipway/settings`

**Feature: shared props, flash messages, confirm dialogs, history encryption.** Every
Slipway page is encrypted in the browser history; signing out clears it, so the back
button can't reveal earlier pages.

| Prop | Shape |
| --- | --- |
| `members` | `{ id, name, email, role }[]` (5) |
| `tokens` | `{ id, name, lastUsedAt, createdAt }[]` |
| `roles` | `["owner", "admin", "developer", "viewer"]` |

Sections and actions:
- Profile: name and email (`PUT /slipway/settings/profile`; errors on `name`, `email`). The
  sidebar's user block updates after saving (shared props).
- Members: change role (`PATCH …/members/{id}` `{ role }`), remove (confirm,
  `DELETE …/members/{id}`; the owner can't be removed: "The owner cannot be removed.").
- API tokens: create `{ name }` (the new secret appears once, in the success toast:
  "Token created: slp_…"), revoke (confirm).
- Danger zone: sign out.

## 8. Error pages — 403, 404, 419, 500, 503

Rendered as Inertia pages with `status` as the only prop; the HTTP status matches.
Preview them at `/slipway/errors/{status}`. Unknown URLs under `/slipway` show the 404.
Suggested copy: 403 forbidden, 404 not found, 419 page expired (session timed out),
500 server error, 503 down for maintenance.
