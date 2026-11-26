//! The Inertia lab: one handler per protocol feature.

use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use inertia::{Defer, Merge, Optional, Page, Visit};
use route::{Body, Form, Json, Query, Redirect, http::Request};
use serde::{Deserialize, Serialize};
use session::Session;
use validate::{Errors, Valid, Validate, email, max_len, min_len, required};

use crate::responses::lab::Basics;

/// How often expensive props were actually computed, to show what the server skipped.
static RENDERS: AtomicU64 = AtomicU64::new(0);
static PLAN_LOADS: AtomicU64 = AtomicU64::new(0);

fn clock() -> String {
    let ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    format!("{}.{:03}s", ms / 1000 % 100_000, ms % 1000)
}

fn flash(session: &Session, message: &str) {
    session.flash("status", message).expect("a string serializes");
}

// ---- index ----

#[derive(Serialize)]
pub struct Experiment {
    href: &'static str,
    name: &'static str,
    what: &'static str,
}

#[derive(Serialize)]
pub struct Index {
    experiments: Vec<Experiment>,
}

pub async fn index(_: ()) -> Page<Index> {
    let experiments = vec![
        Experiment { href: "/lab/basics", name: "Props & partial reloads", what: "Typed props (ts-rs), shared props, reloading one prop" },
        Experiment { href: "/lab/lazy", name: "Optional & deferred props", what: "Props computed only on request, or after first paint" },
        Experiment { href: "/lab/merge", name: "Merge props", what: "Infinite list: pages append, reset starts over" },
        Experiment { href: "/lab/once", name: "Once props", what: "Computed once, remembered across pages" },
        Experiment { href: "/lab/form", name: "Forms & validation", what: "useForm, field errors, error bags, flash, plain HTML form with _token" },
        Experiment { href: "/lab/redirects", name: "Redirects", what: "PUT / PATCH / DELETE come back as 303" },
        Experiment { href: "/lab/broken", name: "Missing component", what: "A typo in a component name is a clear error in dev (expect a 500)" },
    ];
    inertia::render("lab/index", Index { experiments })
}

// ---- basics ----

pub async fn basics(_: ()) -> Page<Basics> {
    let renders = RENDERS.fetch_add(1, Ordering::SeqCst) + 1;
    inertia::render("lab/basics", Basics { server_time: clock(), renders, tags: vec!["rust".into(), "inertia".into(), "vue".into()] })
}

// ---- optional & deferred ----

#[derive(Serialize)]
pub struct Stats {
    users: u32,
    jobs: u32,
    computed_at: String,
}

#[derive(Serialize)]
pub struct ReportRow {
    name: &'static str,
    value: u32,
}

#[derive(Serialize)]
pub struct Lazy {
    stats: Optional<Stats>,
    report: Defer<Vec<ReportRow>>,
    activity: Defer<Vec<String>>,
}

pub async fn lazy(visit: Visit) -> Page<Lazy> {
    let stats = visit.optional("stats", async { Stats { users: 42, jobs: 7, computed_at: clock() } }).await;
    let report = visit
        .defer("report", async {
            tokio::time::sleep(Duration::from_millis(800)).await; // a slow query
            vec![ReportRow { name: "builds", value: 120 }, ReportRow { name: "failures", value: 3 }]
        })
        .await;
    // A second group loads in its own request, in parallel with `report`.
    let activity = visit
        .defer("activity", async {
            tokio::time::sleep(Duration::from_millis(1500)).await;
            vec![format!("deploy finished at {}", clock()), "ada pushed to main".into()]
        })
        .await
        .group("activity");
    inertia::render("lab/lazy", Lazy { stats, report, activity })
}

// ---- merge ----

#[derive(Deserialize)]
pub struct PageQuery {
    page: Option<u32>,
}

#[derive(Serialize)]
pub struct Item {
    id: u32,
    text: String,
}

#[derive(Serialize)]
pub struct Feed {
    page: u32,
    has_more: bool,
    items: Merge<Vec<Item>>,
}

pub async fn merge(Query(q): Query<PageQuery>) -> Page<Feed> {
    let page = q.page.unwrap_or(1).clamp(1, 5);
    let items = ((page - 1) * 5 + 1..=page * 5).map(|id| Item { id, text: format!("Item {id}, loaded with page {page}") }).collect();
    inertia::render("lab/merge", Feed { page, has_more: page < 5, items: Merge::new(items).match_on("id") })
}

// ---- once ----

#[derive(Serialize)]
pub struct Plan {
    name: &'static str,
    price: u32,
}

#[derive(Serialize)]
pub struct OncePage {
    title: &'static str,
    plans: inertia::Once<Vec<Plan>>,
    plan_loads: u64,
}

async fn once_page(visit: Visit, title: &'static str) -> Page<OncePage> {
    let plans = visit
        .once("plans", async {
            PLAN_LOADS.fetch_add(1, Ordering::SeqCst);
            vec![Plan { name: "Hobby", price: 0 }, Plan { name: "Team", price: 20 }]
        })
        .await;
    inertia::render("lab/once", OncePage { title, plans, plan_loads: PLAN_LOADS.load(Ordering::SeqCst) })
}

pub async fn once(visit: Visit) -> Page<OncePage> {
    once_page(visit, "Pricing").await
}

pub async fn once_again(visit: Visit) -> Page<OncePage> {
    once_page(visit, "Checkout").await
}

// ---- forms ----

#[derive(Deserialize)]
pub struct Contact {
    name: String,
    email: String,
    message: String,
}

impl Validate for Contact {
    fn validate(&self, e: &mut Errors) {
        e.check("name", required(&self.name));
        e.check("email", email(&self.email));
        e.check("message", min_len(&self.message, 10)).check("message", max_len(&self.message, 500));
    }
}

#[derive(Deserialize)]
pub struct Newsletter {
    email: String,
}

impl Validate for Newsletter {
    fn validate(&self, e: &mut Errors) {
        e.check("email", email(&self.email));
    }
}

#[derive(Deserialize)]
pub struct Plain {
    name: String,
}

pub async fn form(_: ()) -> Page<()> {
    inertia::render("lab/form", ())
}

pub async fn contact((session, input): (Session, Valid<Json<Contact>>)) -> Redirect {
    flash(&session, &format!("Thanks {}, your message was received.", input.name));
    Redirect::to_route("lab.form")
}

pub async fn newsletter((session, input): (Session, Valid<Json<Newsletter>>)) -> Redirect {
    flash(&session, &format!("{} subscribed.", input.email));
    Redirect::to_route("lab.form")
}

/// A plain HTML form post: no JavaScript, CSRF token in the `_token` field.
pub async fn plain((session, Form(input)): (Session, Form<Plain>)) -> Redirect {
    flash(&session, &format!("Plain form posted by {} (CSRF via _token).", input.name));
    Redirect::to_route("lab.form")
}

// ---- redirects ----

pub async fn redirects(_: ()) -> Page<()> {
    inertia::render("lab/redirects", ())
}

pub async fn mutate((session, req): (Session, Request<Body>)) -> Redirect {
    flash(&session, &format!("{} handled at {}; you were sent back with a 303.", req.method(), clock()));
    Redirect::to_route("lab.redirects")
}

// ---- missing component ----

pub async fn broken(_: ()) -> Page<()> {
    inertia::render("lab/does-not-exist", ())
}
