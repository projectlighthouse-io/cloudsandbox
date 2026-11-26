//! Slipway's handlers. Each page names the Inertia feature it demonstrates.

use std::{collections::BTreeMap, time::Duration};

use inertia::{Defer, Optional, Page, Scroll, Visit};
use route::{
    Body, Cx, IntoResponse, Json, Multipart, MultipartError, Path, Query, Redirect,
    http::{Response, StatusCode},
};
use serde::{Deserialize, Serialize};
use session::Session;
use validate::{Errors, alpha_dash, max_len, one_of, required, rule};

use super::data::{
    ApiToken, DAY, Data, Deployment, EnvVar, Member, RUNTIMES, SERVER_STATUSES, Server, Site, Store, Team, User, now_ms,
};
use crate::state::App;

const TEAM_KEY: &str = "slipway_team";

/// Every Slipway page is encrypted in the browser's history; logout clears it.
fn page<P: Serialize>(component: &'static str, props: P) -> Page<P> {
    inertia::render(component, props).encrypt_history(true)
}

pub fn team_id(session: Option<&Session>) -> u64 {
    session.and_then(|s| s.get::<u64>(TEAM_KEY).ok().flatten()).unwrap_or(1)
}

pub fn flash_success(session: &Session, message: &str) {
    session.flash("flash_success", message).expect("a string serializes");
}

pub fn flash_error(session: &Session, message: &str) {
    session.flash("flash_error", message).expect("a string serializes");
}

fn errors(field: &str, message: &str) -> Response<Body> {
    let mut e = Errors::new();
    e.add(field, message.to_owned());
    e.into_response()
}

#[derive(Serialize)]
pub struct Auth {
    pub user: User,
}

#[derive(Serialize)]
pub struct Flash {
    pub success: Option<String>,
    pub error: Option<String>,
}

/// Props on every page: `auth.user`, `team`, `teams`, `flash`.
#[derive(Serialize)]
pub struct Shared {
    pub app_name: String,
    pub auth: Auth,
    pub team: Team,
    pub teams: Vec<Team>,
    pub flash: Flash,
    pub csrf_token: Option<String>,
}

pub fn shared(store: &Store, app_name: &str, session: Option<&Session>) -> Shared {
    let data = store.lock();
    let team_id = team_id(session);
    let get = |key: &str| session.and_then(|s| s.get::<String>(key).ok().flatten());
    Shared {
        app_name: app_name.to_owned(),
        auth: Auth { user: data.user.clone() },
        team: data.teams.iter().find(|t| t.id == team_id).unwrap_or(&data.teams[0]).clone(),
        teams: data.teams.clone(),
        flash: Flash { success: get("flash_success"), error: get("flash_error") },
        csrf_token: get("_token"),
    }
}

fn team_servers(data: &Data, team: u64) -> Vec<&Server> {
    data.servers.iter().filter(|s| s.team_id == team).collect()
}

fn team_sites(data: &Data, team: u64) -> Vec<u64> {
    let servers: Vec<u64> = team_servers(data, team).iter().map(|s| s.id).collect();
    data.sites.iter().filter(|s| servers.contains(&s.server_id)).map(|s| s.id).collect()
}

/// A metric that moves with time, so every poll shows fresh points.
fn metric(seed: u64, at_ms: u64, base: u32) -> u32 {
    let t = at_ms as f64 / 1000.0;
    let s = seed as f64;
    let v = base as f64 + 9.0 * (t / 40.0 + s).sin() + 4.0 * (t / 9.0 + s * 1.7).sin();
    v.clamp(1.0, 99.0) as u32
}

fn series(seed: u64, base: u32, points: u64, step_ms: u64) -> Vec<u32> {
    let now = now_ms();
    (0..points).rev().map(|i| metric(seed, now - i * step_ms, base)).collect()
}

// Overview: deferred props, refreshed by polling.

#[derive(Serialize)]
pub struct Delta {
    text: String,
    good: bool,
}

#[derive(Serialize)]
pub struct Note {
    text: String,
    /// `ok`, `warn`, `bad` or `idle`: the dot's colour.
    tone: &'static str,
    /// Shown as " at HH:MM" in the viewer's time zone.
    at: Option<u64>,
}

#[derive(Serialize)]
pub struct Tile {
    label: &'static str,
    value: u64,
    delta: Delta,
    /// The last seven days, today last.
    series: Vec<u32>,
    /// `sky`, `indigo` or `red`: the trend line's colour.
    color: &'static str,
    note: Note,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    server_id: u64,
    name: String,
    cpu: Option<u32>,
    memory: Option<u32>,
    /// CPU over the last 40 seconds, oldest first.
    trend: Vec<u32>,
}

#[derive(Serialize)]
pub struct Overview {
    summary: String,
    tiles: Vec<Tile>,
    recent: Vec<Deployment>,
    /// The first servers' usage; arrives just after first paint.
    usage: Defer<Vec<Usage>>,
    /// The rest, from a slower source, in its own request.
    more_usage: Defer<Vec<Usage>>,
}

fn usage(servers: Vec<Server>) -> Vec<Usage> {
    servers
        .into_iter()
        .map(|s| Usage {
            server_id: s.id,
            trend: s.cpu.map(|cpu| series(s.id, cpu, 9, 5_000)).unwrap_or_default(),
            cpu: s.cpu.map(|cpu| metric(s.id, now_ms(), cpu)),
            memory: s.memory,
            name: s.name,
        })
        .collect()
}

fn median(mut values: Vec<u64>) -> u64 {
    values.sort_unstable();
    values.get(values.len() / 2).copied().unwrap_or(0)
}

pub async fn overview(cx: Cx<App>, (visit, session): (Visit, Session)) -> Page<Overview> {
    let team = team_id(Some(&session));
    let (summary, tiles, recent, first, rest) = {
        let data = cx.slipway.lock();
        let servers = team_servers(&data, team);
        let site_ids = team_sites(&data, team);
        let now = now_ms();
        let today_start = now - now % DAY;
        let week_ago = now - 7 * DAY;
        let history = data.history.iter().find(|(t, _)| *t == team).map(|(_, h)| h.clone()).expect("seeded history");
        let today: Vec<&Deployment> = data.deployments.iter().filter(|d| site_ids.contains(&d.site_id) && d.started_at >= today_start).collect();
        let failed: Vec<&&Deployment> = today.iter().filter(|d| d.status == "failed").collect();

        let mut providers: Vec<&str> = servers.iter().map(|s| s.provider.as_str()).collect();
        providers.sort_unstable();
        providers.dedup();
        let summary = format!("{} servers across {} providers", servers.len(), providers.len());

        let degraded = servers.iter().filter(|s| s.status == "degraded" || s.status == "offline").count();
        let new_servers = servers.iter().filter(|s| s.created_at >= week_ago).count();
        let new_sites = data.sites.iter().filter(|s| site_ids.contains(&s.id) && s.created_at >= week_ago).count();
        let failing_sites = data.sites.iter().filter(|s| site_ids.contains(&s.id) && s.status == "failed").count();

        let mut deploys = history.deploys.to_vec();
        *deploys.last_mut().unwrap() = today.len() as u32;
        let previous = deploys[..6].iter().sum::<u32>() as f64 / 6.0;
        let vs_avg = if previous > 0.0 { ((today.len() as f64 / previous - 1.0) * 100.0).round() as i64 } else { 0 };
        let mut failures = history.failures.to_vec();
        *failures.last_mut().unwrap() = failed.len() as u32;
        let vs_yesterday = failed.len() as i64 - failures[5] as i64;
        let durations = today.iter().filter_map(|d| d.duration_ms).collect();

        let tiles = vec![
            Tile {
                label: "Servers",
                value: servers.len() as u64,
                delta: Delta { text: format!("+{new_servers} this week"), good: true },
                series: history.servers.to_vec(),
                color: "sky",
                note: Note { text: format!("{} healthy, {degraded} degraded", servers.len() - degraded), tone: if degraded > 0 { "warn" } else { "ok" }, at: None },
            },
            Tile {
                label: "Sites",
                value: site_ids.len() as u64,
                delta: Delta { text: format!("+{new_sites} this week"), good: true },
                series: history.sites.to_vec(),
                color: "sky",
                note: match failing_sites {
                    0 => Note { text: "All certificates valid".into(), tone: "ok", at: None },
                    n => Note { text: format!("{n} sites failing"), tone: "bad", at: None },
                },
            },
            Tile {
                label: "Deploys today",
                value: today.len() as u64,
                delta: Delta { text: format!("{vs_avg:+}% vs avg"), good: vs_avg >= 0 },
                series: deploys,
                color: "indigo",
                note: Note { text: format!("Median {}s", median(durations) / 1000), tone: "idle", at: None },
            },
            Tile {
                label: "Failed today",
                value: failed.len() as u64,
                delta: Delta { text: format!("{vs_yesterday:+} vs yesterday"), good: vs_yesterday <= 0 },
                series: failures,
                color: "red",
                note: match failed.first() {
                    Some(d) => Note { text: d.site.clone(), tone: "bad", at: Some(d.started_at) },
                    None => Note { text: "None today".into(), tone: "ok", at: None },
                },
            },
        ];
        let recent = data.deployments.iter().filter(|d| site_ids.contains(&d.site_id)).take(5).cloned().collect();
        let all: Vec<Server> = servers.into_iter().cloned().collect();
        let (first, rest) = all.split_at(all.len().min(4));
        (summary, tiles, recent, first.to_vec(), rest.to_vec())
    };
    let usage_now = visit
        .defer("usage", async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            usage(first)
        })
        .await;
    let more_usage = visit
        .defer("more_usage", async move {
            tokio::time::sleep(Duration::from_millis(1500)).await;
            usage(rest)
        })
        .await
        .group("slow");
    page("Overview", Overview { summary, tiles, recent, usage: usage_now, more_usage })
}

// Servers index: partial reloads that keep the query string in sync.

#[derive(Deserialize, Default)]
pub struct ServerQuery {
    q: Option<String>,
    provider: Option<String>,
    status: Option<String>,
    sort: Option<String>,
    dir: Option<String>,
    page: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    page: usize,
    per_page: usize,
    total: usize,
}

#[derive(Serialize)]
pub struct Paginated<T> {
    data: Vec<T>,
    meta: Meta,
}

#[derive(Serialize)]
pub struct Filters {
    q: String,
    provider: String,
    status: String,
    sort: String,
    dir: String,
}

#[derive(Serialize)]
pub struct ServersIndex {
    servers: Paginated<Server>,
    filters: Filters,
    providers: Vec<String>,
    statuses: Vec<&'static str>,
}

pub async fn servers(cx: Cx<App>, (session, Query(q)): (Session, Query<ServerQuery>)) -> Page<ServersIndex> {
    let team = team_id(Some(&session));
    let search = q.q.unwrap_or_default().trim().to_lowercase();
    let provider = q.provider.unwrap_or_default();
    let status = q.status.unwrap_or_default();
    let sort = q.sort.filter(|s| ["name", "region", "cpu", "memory", "sitesCount"].contains(&s.as_str())).unwrap_or_else(|| "name".into());
    let dir = if q.dir.as_deref() == Some("desc") { "desc" } else { "asc" }.to_owned();
    let per_page = 10;

    let (mut rows, mut providers) = {
        let data = cx.slipway.lock();
        let all = team_servers(&data, team);
        let providers: Vec<String> = all.iter().map(|s| s.provider.clone()).collect();
        let rows: Vec<Server> = all
            .into_iter()
            .filter(|s| search.is_empty() || [&s.name, &s.ip, &s.region, &s.provider].iter().any(|f| f.to_lowercase().contains(&search)))
            .filter(|s| provider.is_empty() || s.provider == provider)
            .filter(|s| status.is_empty() || s.status == status)
            .cloned()
            .collect();
        (rows, providers)
    };
    providers.sort_unstable();
    providers.dedup();
    // Servers without metrics sort last either way.
    let metric_key = |v: Option<u32>| v.map_or(-1, i64::from);
    rows.sort_by(|a, b| match sort.as_str() {
        "region" => a.region.cmp(&b.region),
        "cpu" => metric_key(a.cpu).cmp(&metric_key(b.cpu)),
        "memory" => metric_key(a.memory).cmp(&metric_key(b.memory)),
        "sitesCount" => a.sites_count.cmp(&b.sites_count),
        _ => a.name.cmp(&b.name),
    });
    if dir == "desc" {
        rows.reverse();
    }
    let total = rows.len();
    let pages = total.div_ceil(per_page).max(1);
    let page_no = q.page.unwrap_or(1).clamp(1, pages);
    let data = rows.into_iter().skip((page_no - 1) * per_page).take(per_page).collect();

    page(
        "servers/Index",
        ServersIndex {
            servers: Paginated { data, meta: Meta { page: page_no, per_page, total } },
            filters: Filters { q: search, provider, status, sort, dir },
            providers,
            statuses: SERVER_STATUSES.to_vec(),
        },
    )
}

// Server show: a page per tab inside a nested layout, heavy props deferred.

fn find_server(cx: &App, team: u64, id: u64) -> Option<Server> {
    cx.slipway.lock().servers.iter().find(|s| s.id == id && s.team_id == team).cloned()
}

#[derive(Serialize)]
pub struct ErrorPage {
    status: u16,
}

fn not_found() -> Response<Body> {
    (StatusCode::NOT_FOUND, page("errors/NotFound", ErrorPage { status: 404 })).into_response()
}

#[derive(Serialize)]
pub struct ServerSites {
    server: Server,
    sites: Vec<Site>,
}

#[derive(Serialize)]
pub struct History {
    cpu: Vec<u32>,
    memory: Vec<u32>,
    disk: Vec<u32>,
    load: Vec<f32>,
    bandwidth: Vec<u32>,
}

#[derive(Serialize)]
pub struct ServerMetrics {
    server: Server,
    history: Defer<History>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    id: u64,
    command: String,
    frequency: &'static str,
    next_run_at: u64,
}

#[derive(Serialize)]
pub struct ServerScheduler {
    server: Server,
    tasks: Vec<Task>,
}

#[derive(Serialize)]
pub struct Worker {
    id: u64,
    connection: &'static str,
    queue: String,
    processes: u32,
    status: &'static str,
}

#[derive(Serialize)]
pub struct ServerWorkers {
    server: Server,
    workers: Vec<Worker>,
}

#[derive(Serialize)]
pub struct ServerLogs {
    server: Server,
    lines: Defer<Vec<String>>,
}

pub async fn server(Path(id): Path<u64>) -> Redirect {
    Redirect::to(format!("/servers/{id}/metrics"))
}

pub async fn server_sites(cx: Cx<App>, (session, Path(id)): (Session, Path<u64>)) -> Response<Body> {
    let Some(server) = find_server(&cx, team_id(Some(&session)), id) else { return not_found() };
    let sites = cx.slipway.lock().sites.iter().filter(|s| s.server_id == id).cloned().collect();
    page("servers/Sites", ServerSites { server, sites }).into_response()
}

pub async fn server_metrics(cx: Cx<App>, (session, visit, Path(id)): (Session, Visit, Path<u64>)) -> Response<Body> {
    let Some(server) = find_server(&cx, team_id(Some(&session)), id) else { return not_found() };
    let (cpu, mem, disk) = (server.cpu.unwrap_or(5), server.memory.unwrap_or(10), server.disk);
    let history = visit
        .defer("history", async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            History {
                cpu: series(id, cpu, 60, 60_000),
                memory: series(id * 7, mem, 60, 60_000),
                disk: series(id * 13, disk, 60, 60_000),
                load: series(id * 3, 30, 60, 60_000).into_iter().map(|v| v as f32 / 25.0).collect(),
                bandwidth: series(id * 5, 40, 60, 60_000).into_iter().map(|v| v * 12).collect(),
            }
        })
        .await;
    page("servers/Metrics", ServerMetrics { server, history }).into_response()
}

pub async fn server_scheduler(cx: Cx<App>, (session, Path(id)): (Session, Path<u64>)) -> Response<Body> {
    let Some(server) = find_server(&cx, team_id(Some(&session)), id) else { return not_found() };
    let now = now_ms();
    let tasks = vec![
        Task { id: 1, command: "art queue:prune".into(), frequency: "Hourly", next_run_at: now + 1_800_000 },
        Task { id: 2, command: "art backup:run --compress".into(), frequency: "Daily at 03:00", next_run_at: now + 9 * 3_600_000 },
        Task { id: 3, command: format!("certbot renew --cert-name {}", server.name), frequency: "Weekly", next_run_at: now + 3 * DAY },
    ];
    page("servers/Scheduler", ServerScheduler { server, tasks }).into_response()
}

pub async fn server_workers(cx: Cx<App>, (session, Path(id)): (Session, Path<u64>)) -> Response<Body> {
    let Some(server) = find_server(&cx, team_id(Some(&session)), id) else { return not_found() };
    let running = if server.status == "healthy" || server.status == "degraded" { "running" } else { "stopped" };
    let workers = vec![
        Worker { id: 1, connection: "postgres", queue: "default".into(), processes: 4, status: running },
        Worker { id: 2, connection: "postgres", queue: "emails".into(), processes: 2, status: running },
        Worker { id: 3, connection: "postgres", queue: "media".into(), processes: 1, status: "stopped" },
    ];
    page("servers/Workers", ServerWorkers { server, workers }).into_response()
}

pub async fn server_logs(cx: Cx<App>, (session, visit, Path(id)): (Session, Visit, Path<u64>)) -> Response<Body> {
    let Some(server) = find_server(&cx, team_id(Some(&session)), id) else { return not_found() };
    let name = server.name.clone();
    let lines = visit
        .defer("lines", async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let levels = ["INFO", "INFO", "INFO", "WARN", "INFO", "ERROR"];
            (0..40)
                .map(|i| {
                    let level = levels[(i * 7 + id as usize) % levels.len()];
                    let message = match level {
                        "ERROR" => "upstream timed out after 30s (GET /api/search)",
                        "WARN" => "slow query: 1.9s on deployments",
                        _ => "request completed in 42ms",
                    };
                    format!("2026-11-26T{:02}:{:02}:{:02}Z {level:5} {name} {message}", 10 + i / 30, (i * 3) % 60, (i * 17) % 60)
                })
                .collect()
        })
        .await;
    page("servers/Logs", ServerLogs { server, lines }).into_response()
}

pub async fn reboot(cx: Cx<App>, (session, Path(id)): (Session, Path<u64>)) -> Response<Body> {
    let Some(server) = find_server(&cx, team_id(Some(&session)), id) else { return not_found() };
    cx.slipway.lock().log("restarted", &server.name);
    flash_success(&session, &format!("{} is restarting.", server.name));
    Redirect::to(format!("/servers/{id}/metrics")).into_response()
}

// Sites: a multipart form with validation and a certificate upload.

#[derive(Serialize)]
pub struct ServerOption {
    id: u64,
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteFormSite {
    id: u64,
    server_id: u64,
    domain: String,
    repository: String,
    branch: String,
    runtime: String,
    env: Vec<EnvVar>,
    has_certificate: bool,
}

#[derive(Serialize)]
pub struct SiteForm {
    site: Option<SiteFormSite>,
    servers: Vec<ServerOption>,
    runtimes: Vec<&'static str>,
}

fn site_form(cx: &App, team: u64, site: Option<&Site>) -> Page<SiteForm> {
    let servers = team_servers(&cx.slipway.lock(), team).iter().map(|s| ServerOption { id: s.id, name: s.name.clone() }).collect();
    let component = if site.is_some() { "sites/Edit" } else { "sites/Create" };
    let site = site.map(|s| SiteFormSite {
        id: s.id,
        server_id: s.server_id,
        domain: s.domain.clone(),
        repository: s.repository.clone(),
        branch: s.branch.clone(),
        runtime: s.runtime.clone(),
        env: s.env.clone(),
        has_certificate: s.certificate.is_some(),
    });
    page(component, SiteForm { site, servers, runtimes: RUNTIMES.to_vec() })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteRow {
    #[serde(flatten)]
    site: Site,
    server: String,
}

#[derive(Serialize)]
pub struct SitesIndex {
    sites: Vec<SiteRow>,
}

pub async fn sites(cx: Cx<App>, session: Session) -> Page<SitesIndex> {
    let team = team_id(Some(&session));
    let data = cx.slipway.lock();
    let ids = team_sites(&data, team);
    let sites = data
        .sites
        .iter()
        .filter(|s| ids.contains(&s.id))
        .map(|s| SiteRow {
            server: data.servers.iter().find(|v| v.id == s.server_id).map(|v| v.name.clone()).unwrap_or_default(),
            site: s.clone(),
        })
        .collect();
    page("sites/Index", SitesIndex { sites })
}

pub async fn create_site(cx: Cx<App>, session: Session) -> Page<SiteForm> {
    site_form(&cx, team_id(Some(&session)), None)
}

pub async fn edit_site(cx: Cx<App>, (session, Path(id)): (Session, Path<u64>)) -> Response<Body> {
    let team = team_id(Some(&session));
    let site = cx.slipway.lock().sites.iter().find(|s| s.id == id).cloned();
    match site.filter(|s| find_server(&cx, team, s.server_id).is_some()) {
        Some(site) => site_form(&cx, team, Some(&site)).into_response(),
        None => not_found(),
    }
}

/// The site form, read from `multipart/form-data` (it may carry a certificate file).
#[derive(Default)]
struct SiteInput {
    server_id: String,
    domain: String,
    repository: String,
    branch: String,
    runtime: String,
    env: BTreeMap<usize, EnvVar>,
    certificate: Option<Vec<u8>>,
}

const CERT_MAX: usize = 1024 * 1024;

async fn read_site(mut form: Multipart) -> Result<SiteInput, MultipartError> {
    let mut input = SiteInput::default();
    while let Some(field) = form.next_field().await? {
        let name = field.name().unwrap_or_default().to_owned();
        if name == "certificate" {
            let bytes = field.bytes().await?;
            if !bytes.is_empty() {
                input.certificate = Some(bytes.to_vec());
            }
            continue;
        }
        let text = field.text().await?;
        match name.as_str() {
            "serverId" => input.server_id = text,
            "domain" => input.domain = text.trim().to_lowercase(),
            "repository" => input.repository = text.trim().to_owned(),
            "branch" => input.branch = text.trim().to_owned(),
            "runtime" => input.runtime = text,
            // env[0][key], env[0][value]
            n if n.starts_with("env[") => {
                let mut parts = n.trim_start_matches("env[").split("][");
                let (Some(i), Some(part)) = (parts.next().and_then(|i| i.parse().ok()), parts.next()) else { continue };
                let entry = input.env.entry(i).or_insert(EnvVar { key: String::new(), value: String::new() });
                match part.trim_end_matches(']') {
                    "key" => entry.key = text.trim().to_owned(),
                    "value" => entry.value = text,
                    _ => {}
                }
            }
            _ => {}
        }
    }
    Ok(input)
}

fn domain_shape(s: &str) -> Result<(), validate::Message> {
    let labels: Vec<&str> = s.split('.').collect();
    rule(labels.len() >= 2 && labels.iter().all(|l| !l.is_empty() && alpha_dash(l).is_ok() && !l.contains('_')), "Must be a domain like app.example.com.")
}

fn repo_shape(s: &str) -> Result<(), validate::Message> {
    let parts: Vec<&str> = s.split('/').collect();
    rule(parts.len() == 2 && parts.iter().all(|p| alpha_dash(p).is_ok()), "Must look like owner/name.")
}

fn env_key(s: &str) -> Result<(), validate::Message> {
    let ok = s.starts_with(|c: char| c.is_ascii_uppercase() || c == '_') && s.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
    rule(ok, "Use UPPER_SNAKE_CASE.")
}

async fn save_site(cx: Cx<App>, session: Session, form: Multipart, id: Option<u64>) -> Response<Body> {
    let input = match read_site(form).await {
        Ok(i) => i,
        Err(e) => return e.into_response(),
    };
    let team = team_id(Some(&session));
    let mut e = Errors::new();
    let server = input.server_id.parse::<u64>().ok().and_then(|sid| find_server(&cx, team, sid));
    e.check("serverId", rule(server.is_some(), "Pick a server."));
    e.check("domain", required(&input.domain)).check("domain", domain_shape(&input.domain));
    let taken = {
        let data = cx.slipway.lock();
        data.sites
            .iter()
            .find(|s| s.domain == input.domain && Some(s.id) != id)
            .map(|s| data.servers.iter().find(|v| v.id == s.server_id).map(|v| v.name.clone()).unwrap_or_default())
    };
    if let Some(on) = taken {
        e.add("domain", format!("This domain is already used by another site on {on}."));
    }
    e.check("repository", required(&input.repository)).check("repository", repo_shape(&input.repository));
    e.check("branch", required(&input.branch)).check("branch", max_len(&input.branch, 100)).check("branch", rule(!input.branch.contains(' '), "No spaces."));
    e.check("runtime", one_of(input.runtime.as_str(), &RUNTIMES));
    let mut seen = vec![];
    for (i, var) in &input.env {
        e.check(&format!("env.{i}.key"), required(&var.key)).check(&format!("env.{i}.key"), env_key(&var.key));
        e.check(&format!("env.{i}.key"), rule(!seen.contains(&var.key), "Duplicate key."));
        seen.push(var.key.clone());
    }
    if let Some(cert) = &input.certificate {
        e.check("certificate", rule(cert.len() <= CERT_MAX, "Must be at most 1 MB."));
        e.check("certificate", rule(String::from_utf8_lossy(cert).contains("-----BEGIN CERTIFICATE-----"), "Must be a PEM certificate."));
    }
    if !e.is_empty() {
        return e.into_response();
    }

    let server = server.expect("validated");
    let mut data = cx.slipway.lock();
    let certificate = input.certificate.map(|c| String::from_utf8_lossy(&c).into_owned());
    let env: Vec<EnvVar> = input.env.into_values().collect();
    match id {
        Some(id) => {
            let Some(site) = data.sites.iter_mut().find(|s| s.id == id) else { return not_found() };
            site.server_id = server.id;
            site.domain = input.domain.clone();
            site.repository = input.repository;
            site.branch = input.branch;
            site.runtime = input.runtime;
            site.env = env;
            if certificate.is_some() {
                site.certificate = certificate;
            }
        }
        None => {
            let id = data.next_id();
            data.sites.push(Site {
                id,
                server_id: server.id,
                domain: input.domain.clone(),
                repository: input.repository,
                branch: input.branch,
                runtime: input.runtime,
                status: "deployed".into(),
                last_deployed_at: None,
                created_at: now_ms(),
                env,
                certificate,
            });
            if let Some(s) = data.servers.iter_mut().find(|s| s.id == server.id) {
                s.sites_count += 1;
            }
        }
    }
    data.log(if id.is_some() { "updated the site" } else { "added the site" }, &input.domain);
    drop(data);
    flash_success(&session, &format!("{} saved.", input.domain));
    Redirect::to(format!("/servers/{}/sites", server.id)).into_response()
}

pub async fn store_site(cx: Cx<App>, (session, form): (Session, Multipart)) -> Response<Body> {
    save_site(cx, session, form, None).await
}

pub async fn update_site(cx: Cx<App>, (session, Path(id), form): (Session, Path<u64>, Multipart)) -> Response<Body> {
    save_site(cx, session, form, Some(id)).await
}

// Deployments: infinite scroll through scroll props; the client updates optimistically.

#[derive(Deserialize)]
pub struct DeploymentQuery {
    page: Option<u64>,
    log: Option<u64>,
}

#[derive(Serialize)]
pub struct DeploymentLog {
    id: u64,
    lines: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteOption {
    id: u64,
    domain: String,
    status: String,
}

#[derive(Serialize)]
pub struct Deployments {
    deployments: Scroll<Vec<Deployment>>,
    sites: Vec<SiteOption>,
    /// The expanded log, for `?log=<id>`; `null` for an unknown id.
    log: Optional<Option<DeploymentLog>>,
}

const PER_PAGE: u64 = 15;

pub async fn deployments(cx: Cx<App>, (session, visit, Query(q)): (Session, Visit, Query<DeploymentQuery>)) -> Page<Deployments> {
    let team = team_id(Some(&session));
    let (rows, sites, wanted) = {
        let data = cx.slipway.lock();
        let ids = team_sites(&data, team);
        let rows: Vec<Deployment> = data.deployments.iter().filter(|d| ids.contains(&d.site_id)).cloned().collect();
        let sites = data
            .sites
            .iter()
            .filter(|s| ids.contains(&s.id))
            .map(|s| SiteOption { id: s.id, domain: s.domain.clone(), status: s.status.clone() })
            .collect();
        let wanted = q.log.and_then(|id| rows.iter().find(|d| d.id == id).cloned());
        (rows, sites, wanted)
    };
    let pages = (rows.len() as u64).div_ceil(PER_PAGE).max(1);
    let current = q.page.unwrap_or(1).clamp(1, pages);
    let items = rows.into_iter().skip(((current - 1) * PER_PAGE) as usize).take(PER_PAGE as usize).collect();
    let deployments = Scroll::new(items, current)
        .previous((current > 1).then(|| current - 1))
        .next((current < pages).then(|| current + 1))
        .match_on("id");
    let log = visit
        .optional("log", async move {
            wanted.map(|d| DeploymentLog {
                id: d.id,
                lines: vec![
                    format!("git fetch origin {}", d.branch),
                    "cargo build --release".into(),
                    "art migrate".into(),
                    match d.status.as_str() {
                        "failed" => "error: relation \"jobs\" already exists".into(),
                        "cancelled" => "cancelled by Elizabeth Lane".into(),
                        "finished" => "release is live, traffic switched".into(),
                        other => format!("{other}…"),
                    },
                ],
            })
        })
        .await;
    page("deployments/Index", Deployments { deployments, sites, log })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployInput {
    site_id: u64,
}

pub async fn deploy(cx: Cx<App>, (session, Json(input)): (Session, Json<DeployInput>)) -> Response<Body> {
    let team = team_id(Some(&session));
    let deployment = {
        let mut data = cx.slipway.lock();
        let ids = team_sites(&data, team);
        let Some(site) = data.sites.iter().find(|s| s.id == input.site_id && ids.contains(&s.id)).cloned() else {
            return errors("siteId", "Pick a site.");
        };
        if site.status == "suspended" {
            return errors("siteId", &format!("{} is suspended; deploys are blocked.", site.domain));
        }
        let id = data.next_id();
        let d = Deployment {
            id,
            site_id: site.id,
            site: site.domain.clone(),
            commit: format!("{:07x}", (now_ms() ^ id.wrapping_mul(2_654_435_761)) & 0xfff_ffff),
            message: "Deploy requested by you".into(),
            author: data.user.name.clone(),
            branch: site.branch.clone(),
            status: "queued".into(),
            started_at: now_ms(),
            duration_ms: None,
        };
        data.deployments.insert(0, d.clone());
        data.log("requested a deployment of", &site.domain);
        d
    };
    simulate(cx.slipway.clone(), deployment.id);
    flash_success(&session, &format!("Deploying {} ({}).", deployment.site, deployment.commit));
    Redirect::to("/deployments").into_response()
}

/// queued -> running -> finished or failed, unless cancelled on the way.
fn simulate(store: Store, id: u64) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(1500)).await;
        if !advance(&store, id, "queued", "running") {
            return;
        }
        let fails = id % 5 == 0;
        tokio::time::sleep(Duration::from_millis(3000 + (id % 4) * 1000)).await;
        advance(&store, id, "running", if fails { "failed" } else { "finished" });
    });
}

fn advance(store: &Store, id: u64, from: &str, to: &str) -> bool {
    let mut data = store.lock();
    let Some(d) = data.deployments.iter_mut().find(|d| d.id == id && d.status == from) else { return false };
    d.status = to.into();
    if to != "running" {
        d.duration_ms = Some(now_ms() - d.started_at);
    }
    let (site_id, status) = (d.site_id, d.status.clone());
    if let Some(site) = data.sites.iter_mut().find(|s| s.id == site_id) {
        match status.as_str() {
            "running" => site.status = "deploying".into(),
            "finished" => {
                site.status = "deployed".into();
                site.last_deployed_at = Some(now_ms());
            }
            "failed" => site.status = "failed".into(),
            _ => {}
        }
    }
    true
}

pub async fn cancel(cx: Cx<App>, (session, Path(id)): (Session, Path<u64>)) -> Response<Body> {
    let cancelled = {
        let mut data = cx.slipway.lock();
        match data.deployments.iter_mut().find(|d| d.id == id) {
            Some(d) if d.status == "queued" || d.status == "running" => {
                d.status = "cancelled".into();
                d.duration_ms = Some(now_ms() - d.started_at);
                Some(d.site.clone())
            }
            _ => None,
        }
    };
    match cancelled {
        Some(site) => {
            flash_success(&session, &format!("Cancelled the deploy of {site}."));
            Redirect::to("/deployments").into_response()
        }
        None => errors("deployment", "Only queued or running deployments can be cancelled."),
    }
}

// Activity: older days load when they scroll into view.

#[derive(Serialize)]
pub struct Day {
    date: String,
    label: String,
    entries: Vec<super::data::Activity>,
}

#[derive(Serialize)]
pub struct ActivityPage {
    days: Vec<Day>,
    older: Optional<Vec<Day>>,
}

/// `YYYY-MM-DD` (UTC) for a Unix millisecond timestamp.
pub fn date_of(ms: u64) -> String {
    let days = (ms / DAY) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

fn days_of(entries: Vec<super::data::Activity>) -> Vec<Day> {
    let today = date_of(now_ms());
    let yesterday = date_of(now_ms() - DAY);
    let mut days: Vec<Day> = vec![];
    for e in entries {
        let date = date_of(e.at);
        match days.last_mut() {
            Some(d) if d.date == date => d.entries.push(e),
            _ => {
                let label = if date == today { "Today".into() } else if date == yesterday { "Yesterday".into() } else { date.clone() };
                days.push(Day { date, label, entries: vec![e] });
            }
        }
    }
    days
}

pub async fn activity(cx: Cx<App>, visit: Visit) -> Page<ActivityPage> {
    let all = days_of(cx.slipway.lock().activity.clone());
    let mut all = all.into_iter();
    let days: Vec<Day> = all.by_ref().take(3).collect();
    let rest: Vec<Day> = all.collect();
    let older = visit.optional("older", async move { rest }).await;
    page("Activity", ActivityPage { days, older })
}

// Settings: shared props, flash messages, and clearing history on logout.

#[derive(Serialize)]
pub struct Settings {
    members: Vec<Member>,
    tokens: Vec<ApiToken>,
    roles: Vec<&'static str>,
}

const MEMBER_ROLES: [&str; 3] = ["owner", "admin", "member"];

pub async fn settings(_: ()) -> Redirect {
    Redirect::to("/settings/profile")
}

pub async fn settings_profile(cx: Cx<App>) -> Page<Settings> {
    let data = cx.slipway.lock();
    page("settings/Profile", Settings { members: data.members.clone(), tokens: data.tokens.clone(), roles: MEMBER_ROLES.to_vec() })
}

#[derive(Deserialize)]
pub struct ProfileInput {
    name: String,
    email: String,
}

pub async fn update_profile(cx: Cx<App>, (session, Json(input)): (Session, Json<ProfileInput>)) -> Response<Body> {
    let mut e = Errors::new();
    e.check("name", required(&input.name)).check("name", max_len(&input.name, 80));
    e.check("email", validate::email(&input.email));
    if !e.is_empty() {
        return e.into_response();
    }
    {
        let mut data = cx.slipway.lock();
        data.user.name = input.name.trim().to_owned();
        data.user.email = input.email.trim().to_owned();
    }
    flash_success(&session, "Profile saved.");
    Redirect::to("/settings/profile").into_response()
}

#[derive(Deserialize)]
pub struct RoleInput {
    role: String,
}

pub async fn update_member(cx: Cx<App>, (session, Path(id), Json(input)): (Session, Path<u64>, Json<RoleInput>)) -> Response<Body> {
    if one_of(input.role.as_str(), &MEMBER_ROLES).is_err() {
        return errors("role", "Pick a role.");
    }
    let name = {
        let mut data = cx.slipway.lock();
        let Some(m) = data.members.iter_mut().find(|m| m.id == id) else { return not_found() };
        m.role = input.role.clone();
        m.name.clone()
    };
    flash_success(&session, &format!("{name} is now {}.", input.role));
    Redirect::to("/settings/profile").into_response()
}

pub async fn remove_member(cx: Cx<App>, (session, Path(id)): (Session, Path<u64>)) -> Response<Body> {
    let removed = {
        let mut data = cx.slipway.lock();
        if data.members.iter().any(|m| m.id == id && m.role == "owner") {
            return errors("member", "The owner cannot be removed.");
        }
        let before = data.members.len();
        data.members.retain(|m| m.id != id);
        before != data.members.len()
    };
    if !removed {
        return not_found();
    }
    flash_success(&session, "Member removed.");
    Redirect::to("/settings/profile").into_response()
}

#[derive(Deserialize)]
pub struct TokenInput {
    name: String,
}

pub async fn create_token(cx: Cx<App>, (session, Json(input)): (Session, Json<TokenInput>)) -> Response<Body> {
    if let Err(m) = required(&input.name).and(max_len(&input.name, 40)) {
        return errors("name", &m);
    }
    let secret = {
        let mut data = cx.slipway.lock();
        let id = data.next_id();
        data.tokens.push(ApiToken { id, name: input.name.trim().to_owned(), last_used_at: None, created_at: now_ms() });
        format!("slp_{:016x}", (now_ms() ^ id.wrapping_mul(0x9E37_79B9_7F4A_7C15)).rotate_left(17))
    };
    flash_success(&session, &format!("Token created: {secret}. Copy it now; it will not be shown again."));
    Redirect::to("/settings/profile").into_response()
}

pub async fn revoke_token(cx: Cx<App>, (session, Path(id)): (Session, Path<u64>)) -> Response<Body> {
    cx.slipway.lock().tokens.retain(|t| t.id != id);
    flash_success(&session, "Token revoked.");
    Redirect::to("/settings/profile").into_response()
}

pub async fn switch_team(cx: Cx<App>, (session, Path(id)): (Session, Path<u64>)) -> Response<Body> {
    let Some(team) = cx.slipway.lock().teams.iter().find(|t| t.id == id).cloned() else { return not_found() };
    session.insert(TEAM_KEY, &team.id).expect("a number serializes");
    flash_success(&session, &format!("Switched to {}.", team.name));
    Redirect::to("/").into_response()
}

/// "Sign out all sessions" (mock): the session stays, but the browser's
/// encrypted history is cleared, so Back cannot reveal cached pages.
pub async fn logout(session: Session) -> Redirect {
    inertia::clear_history(&session);
    flash_success(&session, "Signed out of every session. History cleared: Back cannot reveal earlier pages.");
    Redirect::to("/")
}

// Command palette data: a plain JSON endpoint the palette fetches.

#[derive(Serialize)]
pub struct PaletteItem {
    kind: &'static str,
    label: String,
    hint: String,
    href: String,
}

pub async fn palette(cx: Cx<App>, session: Session) -> Json<Vec<PaletteItem>> {
    let team = team_id(Some(&session));
    let data = cx.slipway.lock();
    let mut items: Vec<PaletteItem> = team_servers(&data, team)
        .iter()
        .map(|s| PaletteItem { kind: "server", label: s.name.clone(), hint: format!("{} · {}", s.provider, s.region), href: format!("/servers/{}/metrics", s.id) })
        .collect();
    let ids = team_sites(&data, team);
    items.extend(data.sites.iter().filter(|s| ids.contains(&s.id)).map(|s| PaletteItem {
        kind: "site",
        label: s.domain.clone(),
        hint: format!("on {}", data.servers.iter().find(|v| v.id == s.server_id).map_or("", |v| v.name.as_str())),
        href: format!("/sites/{}/edit", s.id),
    }));
    Json(items)
}

pub async fn error_page(Path(status): Path<u16>) -> Response<Body> {
    match status {
        403 | 419 | 500 | 503 => (StatusCode::from_u16(status).expect("known status"), page("errors/Error", ErrorPage { status })).into_response(),
        _ => not_found(),
    }
}

/// Unknown URLs (served from `public/` misses).
pub async fn missing(_: ()) -> Response<Body> {
    not_found()
}
