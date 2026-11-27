//! The landing page (`index.html`).

use crate::mocks::{chk, mock};
use crate::{CATS, CTA, esc, foot, head, ic, icon, nav, tail};

/// The `cargo tree` branch shown in each section strip.
fn tree(key: &str) -> &'static str {
    match key {
        "compose" => "toolkit ├── route ├── db └── inertia",
        "requests" => "route ── tower ── http ── matchit",
        "inertia" => "inertia ── serde_json ── vite",
        "background" => "queue ── sqlx ── tokio",
        "auth" => "auth ── session ── securecookie",
        "rules" => "0 hard dependencies between crates",
        "faq" => "6 questions, honest answers",
        _ => unreachable!(),
    }
}

fn strip(num: &str, name: &str, key: &str, dark: bool) -> String {
    format!(
        r#"<div class="strip{}"><div class="rail-box strip-in"><span class="snum mono">{num}</span><span class="slabel mono">{name}</span><span class="stree mono">{}</span></div></div>"#,
        if dark { " dark" } else { "" },
        esc(tree(key))
    )
}

fn feature(id: &str, title: &str, body: &str) -> String {
    format!(
        r#"<a class="cell fcell reveal" href="{}"><div class="ftext"><p class="flabel mono">{id}</p><h3>{title}</h3><p>{body}</p></div><div class="fmock">{}</div></a>"#,
        crate::href(id),
        mock(id)
    )
}

fn text_cell(label: &str, title: &str, body: &str, icon: Option<&str>) -> String {
    let tile = icon.map_or(String::new(), |i| format!(r#"<span class="itile">{}</span>"#, ic(i)));
    format!(r#"<div class="cell tcell reveal">{tile}<p class="flabel mono">{label}</p><h3>{title}</h3><p>{body}</p></div>"#)
}

fn t(label: &str, title: &str, body: &str) -> String {
    text_cell(label, title, body, None)
}

fn hero_window() -> String {
    let code = [
        r#"<span class="k">async fn</span> <span class="f">show</span>(cx: <span class="t">Cx</span>&lt;<span class="t">App</span>&gt;, <span class="t">Path</span>(id): <span class="t">Path</span>&lt;<span class="t">RunId</span>&gt;)"#,
        r#"    -&gt; <span class="t">Result</span>&lt;<span class="t">Page</span>&lt;<span class="t">RunShow</span>&gt;, <span class="t">AppError</span>&gt;"#,
        "{",
        r#"    <span class="k">let</span> run = cx.runs.<span class="f">find</span>(id).<span class="k">await</span>?;"#,
        "",
        r#"    <span class="t">Ok</span>(inertia::<span class="f">render</span>(<span class="s">"runs/show"</span>, <span class="t">RunShow</span> {"#,
        r#"        run: <span class="t">RunView</span>::<span class="f">from</span>(&amp;run),"#,
        r#"        logs: <span class="t">Lazy</span>::<span class="f">new</span>(|| cx.logs.<span class="f">tail</span>(id)),"#,
        "    }))",
        "}",
    ];
    let lines: String = code
        .iter()
        .enumerate()
        .map(|(i, c)| format!(r#"<div class="cl" data-s><span class="ln">{}</span><span class="cc">{}</span></div>"#, i + 1, if c.is_empty() { "&nbsp;" } else { c }))
        .collect();
    let resp = [
        "{",
        r#"  <span class="p">"component"</span>: <span class="s">"runs/show"</span>,"#,
        r#"  <span class="p">"props"</span>: {"#,
        r#"    <span class="p">"run"</span>: { <span class="p">"id"</span>: <span class="n">42</span>, <span class="p">"status"</span>: <span class="s">"running"</span> },"#,
        r#"    <span class="p">"errors"</span>: {}"#,
        "  },",
        r#"  <span class="p">"url"</span>: <span class="s">"/runs/42"</span>,"#,
        r#"  <span class="p">"version"</span>: <span class="s">"9f1c2e"</span>"#,
        "}",
    ];
    let rlines: String = resp.iter().map(|c| format!(r#"<div class="cl" data-s><span class="cc">{c}</span></div>"#)).collect();
    format!(
        r#"<div class="win reveal" data-live="seq" data-step="260" role="img" aria-label="A Rust handler returning an Inertia page, and the JSON response it produces">
<div class="win-in">
<div class="win-code"><div class="wbar"><span class="wtab on mono">src/runs.rs</span><span class="wtab mono">routes.rs</span><span class="wtab mono">Cargo.toml</span></div>
<div class="code mono">{lines}<span class="caret" aria-hidden="true"></span></div></div>
<div class="win-resp"><div class="wbar"><span class="meth">GET</span><span class="mono muted">/runs/42</span><span class="st st-ok wbadge"><i></i>200 · X-Inertia</span></div>
<div class="code mono resp">{rlines}</div>
<p class="wnote mono">logs is lazy: sent only when a partial reload asks for it</p></div>
</div></div>"#
    )
}

fn compose() -> String {
    let mut groups = String::new();
    for (cat, ids) in CATS {
        let boxes: String = ids
            .iter()
            .map(|i| {
                let on = if ["route", "db", "inertia"].contains(i) { " checked" } else { "" };
                format!(r#"<label class="pick"><input type="checkbox" value="{i}"{on}><span class="pbox">{}</span><span class="mono">{i}</span></label>"#, chk())
            })
            .collect();
        let short = match *cat {
            "Sessions and security" => "Security",
            "Background work" => "Background",
            c => c,
        };
        groups += &format!(r#"<fieldset class="pgroup"><legend class="mono">{short}</legend>{boxes}</fieldset>"#);
    }
    format!(
        r##"<div class="cell compose-l reveal"><div class="ftext"><p class="flabel mono">try it</p><h3>Tick what your app needs</h3>
<p>The manifest on the right is all you add. Nothing you leave unticked is compiled, downloaded or linked.</p></div>
<form class="picks" aria-label="Choose components">{groups}</form></div>
<div class="cell compose-r reveal"><div class="mock mock-plain"><div class="mock-in">
<div class="mbar"><span class="mt mono">Cargo.toml</span><button type="button" class="copy-btn" data-copy-target="#cargo">{}<span>Copy</span></button></div>
<pre class="toml mono" id="cargo" aria-live="polite"></pre>
<p class="tsum mono" id="cargoSum"></p></div></div></div>"##,
        ic("copy")
    )
}

const FAQ: &[(&str, &str)] = &[
    ("Can I use it today?", "Not yet. These pages document a design draft. The crates are being built in phases alongside a real app, a microVM-based CI service, and each one ships when that app needs it."),
    ("Do I have to use all of it?", "No. Every component is its own crate. None of them depends on another unless you switch on a feature, so a single route or queue in an existing project is a normal use."),
    ("How is this different from axum?", "axum is a router with extractors. This is a set of crates for the rest of a server-driven app: sessions, validation, Inertia, queues, events and auth. Because everything speaks tower, the crates also work inside an axum app."),
    ("Is there an ORM?", "No, on purpose. You write sqlx queries. The db crate only gives each pool its own type and makes transactions explicit, so mixing up two databases fails to compile."),
    ("Which databases are supported?", "Postgres first. The session and queue crates define small store traits so MySQL and SQLite drivers can follow without changing the API. Your own queries use whatever sqlx supports."),
    ("Does the Inertia adapter do server-side rendering?", "Not yet. Rendering Vue on the server needs a JavaScript runtime beside the Rust binary. It is planned for when an app needs it, not before."),
];

fn faq() -> String {
    let rows: String = FAQ
        .iter()
        .enumerate()
        .map(|(i, (q, a))| {
            format!(
                r#"<div class="qa"><h3><button type="button" class="qbtn" aria-expanded="false" aria-controls="qa{i}" id="qb{i}"><span>{q}</span>{}</button></h3><div class="qbody" id="qa{i}" role="region" aria-labelledby="qb{i}"><div><p>{a}</p></div></div></div>"#,
                icon("plus", "ic plus")
            )
        })
        .collect();
    format!(r#"<div class="cell faq">{rows}</div>"#)
}

pub fn page(index_json: &str) -> String {
    let body = format!(
        r##"
<section class="hero"><div class="hero-tex" aria-hidden="true"></div><div class="rail-box hero-in">
<a class="pill reveal" href="#parts"><span class="pill-ic">{layers}</span>17 crates, none required together<span class="pill-arrow">{arrow}</span></a>
<h1 class="reveal">Opinionated Rust batteries.</h1>
<p class="sub reveal">Routing, sessions, Inertia pages, queues and auth as small crates on plain <code>http</code> and <code>tower</code> types. Take one, or build a whole app.</p>
<div class="ctas reveal"><a class="btn btn-ink btn-lg" href="/docs/">{CTA}</a><a class="btn btn-line btn-lg" href="#parts">Browse components</a></div>
{win}
</div></section>

<section aria-labelledby="h-compose">{s1}<div class="rail-box grid">
<div class="cell hcell reveal"><h2 id="h-compose">Take one crate. Or all seventeen.</h2><p class="lede">Each component has its own crate and version. Integrations between them are features you switch on, never dependencies you inherit.</p></div>
<div class="row2">{compose}</div></div></section>

<section aria-labelledby="h-req" id="parts">{s2}<div class="rail-box grid">
<div class="cell hcell reveal"><h2 id="h-req">From socket to handler, typed the whole way.</h2><p class="lede">A request is matched, checked and validated before your code runs. What reaches the handler is already the type you asked for.</p></div>
<div class="row2">{f_route}{f_validate}</div><div class="row2">{f_action}{f_session}</div>
<div class="row3">{t_cookie}{t_csrf}{t_errors}</div></div></section>

<section class="band" aria-labelledby="h-inertia">{s3}<div class="rail-box grid">
<div class="cell hcell reveal"><h2 id="h-inertia">Return a page. Get an Inertia app.</h2><p class="lede">A handler that returns <code>Page&lt;P&gt;</code> is an Inertia endpoint. The layer answers first visits, JSON visits, partial reloads and version changes for you.</p></div>
<div class="row2">{f_inertia}{f_vite}</div>
<div class="row3">{t_lazy}{t_defer}{t_ts}</div></div></section>

<section aria-labelledby="h-bg">{s4}<div class="rail-box grid">
<div class="cell hcell reveal"><h2 id="h-bg">Work that happens after the response.</h2><p class="lede">A database-backed queue, a typed event bus whose listeners can become jobs, and commands that ship inside the same binary.</p></div>
<div class="row2">{f_queue}{f_event}</div><div class="row2">{f_command}{f_sse}</div>
<div class="row3">{t_once}{t_tx}{t_shut}</div></div></section>

<section aria-labelledby="h-auth">{s5}<div class="rail-box grid">
<div class="cell hcell reveal"><h2 id="h-auth">Know who is asking. Trust what you stored.</h2><p class="lede">Guards and login on top of sessions, Argon2id hashing, OAuth with typed providers, and database handles that cannot be mixed up.</p></div>
<div class="row2">{f_auth}{f_socialite}</div><div class="row2">{f_password}{f_db}</div>
<div class="row3">{t_publish}{t_scopes}{t_single}</div></div></section>

<section aria-labelledby="h-rules">{s6}<div class="rail-box grid">
<div class="cell hcell reveal"><h2 id="h-rules">Three rules every crate keeps.</h2><p class="lede">They decide what goes in, what stays out, and why the request path never waits on a lock.</p></div>
<div class="row3">{r1}{r2}{r3}</div></div></section>

<section aria-labelledby="h-faq" id="faq">{s7}<div class="rail-box grid">
<div class="cell hcell reveal"><h2 id="h-faq">Questions you would ask first.</h2><p class="lede">Including the ones about what is not done yet.</p></div>
{faq}</div></section>

<section class="closing"><div class="hero-tex" aria-hidden="true"></div><div class="rail-box grid">
<div class="cell hcell center reveal"><h2>Start with route and db.</h2><p class="lede">Most apps begin with two crates and add the rest only when they need them.</p>
<div class="ctas"><a class="btn btn-ink btn-lg" href="/docs/">{CTA}</a><a class="btn btn-line btn-lg" href="#parts">Browse components</a></div></div></div></section>
"##,
        layers = ic("layers"),
        arrow = ic("arrow"),
        win = hero_window(),
        s1 = strip("01", "Compose", "compose", false),
        compose = compose(),
        s2 = strip("02", "Requests", "requests", false),
        f_route = feature("route", "Routes with names", "Groups, per-route middleware and URL generation. The table is frozen and checked at startup."),
        f_validate = feature("validate", "Valid input or no call", "A handler asks for Valid&lt;T&gt;. If validation fails, your code never runs, and every field error is kept."),
        f_action = feature("action", "Write it once, run it anywhere", "One Action becomes a handler, a job, a listener or a command. The compiler names any missing bound."),
        f_session = feature("session", "Sessions that stay lazy", "Nothing is written unless something changed. Flash data lives for exactly one following request."),
        t_cookie = t("securecookie", "Encrypted and bound", "Values are sealed with an AEAD and bound to the cookie name, so they cannot be replayed elsewhere."),
        t_csrf = t("csrf", "Tokens Inertia expects", "One token per session, checked in constant time on unsafe methods. Webhooks are exempted by route name."),
        t_errors = t("validate + inertia", "Errors become props", "On an Inertia visit, a failed check redirects back and the field errors arrive as props.errors."),
        s3 = strip("03", "Inertia", "inertia", true),
        f_inertia = feature("inertia", "The protocol, handled", "First visits, JSON visits, partial reloads and version changes are all answered by one tower layer."),
        f_vite = feature("vite", "Assets from Vite", "Dev server tags while you work, hashed manifest tags in production. A missing entry fails startup."),
        t_lazy = t("Lazy&lt;T&gt;", "Computed when asked", "Expensive props are skipped on normal visits and run only when a partial reload names them."),
        t_defer = t("Defer&lt;T&gt;", "Painted, then filled", "Slow props load after the first render, so the page appears before every query has finished."),
        t_ts = t("ts-rs", "Props typed in Vue", "Props are plain Serialize structs. The app derives TypeScript types from them and art dev keeps them fresh."),
        s4 = strip("04", "Background", "background", false),
        f_queue = feature("queue", "Jobs with retries", "Workers claim jobs with SKIP LOCKED, retry with backoff and keep failures where you can retry them."),
        f_event = feature("event", "Typed events, no strings", "Listeners are found by event type. Inline ones run now; queued ones become jobs and survive a crash."),
        f_command = feature("command", "Commands in the binary", "route:list, queue:work and your own commands run from the same binary you deploy."),
        f_sse = feature("sse", "Live updates, one way", "Stream log lines or statuses to the browser. When the tab closes, the subscription ends with it."),
        t_once = t("queue", "At-least-once, honestly", "Delivery is at-least-once, so jobs are written to be idempotent. Stable job names survive type renames."),
        t_tx = t("push_in_tx", "Queued with your data", "Push a job inside the caller's transaction, so nothing is queued for a row that was rolled back."),
        t_shut = t("worker", "Stops without losing work", "On shutdown a worker stops claiming, finishes what it holds, then exits. Dead workers' jobs return."),
        s5 = strip("05", "Auth and data", "auth", false),
        f_auth = feature("auth", "Login that is hard to get wrong", "Logging in always regenerates the session. Authed&lt;User&gt; cannot exist for an anonymous request."),
        f_socialite = feature("socialite", "Sign in with seven providers", "GitHub, GitLab, Google, Reddit, X, Auth0 and any OpenID Connect provider, each behind a feature. State and PKCE are handled for you."),
        f_password = feature("password", "Argon2id by default", "Hashing runs off the async runtime, and old hashes report when they should be upgraded."),
        f_db = feature("db", "Databases you cannot mix up", "Each pool has its own type over sqlx. Passing the wrong one is a compile error, not an incident."),
        t_publish = t("art publish", "Published, then yours", "Login and OAuth handlers are copied into your app once, so redirects and logging are yours to change."),
        t_scopes = t("Scope::ReadUser", "Scopes that compile", "Each provider has its own scope enum, so a GitHub scope cannot be sent to Google by mistake."),
        t_single = t("db::global", "Singleton when you want it", "Pass handles explicitly by default. Turn on one global slot per handle type when that suits you."),
        s6 = strip("06", "Rules", "rules", false),
        r1 = text_cell("principle 1", "One responsibility each", "A crate that grows a second job is split, or the job is cut. You never compile more than you use.", Some("layers")),
        r2 = text_cell("principle 2", "Types carry the rules", "Valid&lt;T&gt;, Authed&lt;User&gt; and Db&lt;Primary&gt; only exist when the check passed.", Some("shield")),
        r3 = text_cell("principle 3", "Frozen at startup", "Routers, middleware and listeners are built once, so the request path never waits on a lock.", Some("snow")),
        s7 = strip("07", "FAQ", "faq", false),
        faq = faq(),
    );
    head(
        "toolkit · Opinionated Rust batteries",
        "Small Rust crates for server-driven web apps: routing, sessions, Inertia, queues and auth.",
        "/",
        "website",
        &format!(
            r#"{{"@context":"https://schema.org","@type":"WebSite","name":"toolkit","url":{},"description":"Small Rust crates for server-driven web apps: routing, sessions, Inertia, queues and auth."}}"#,
            crate::json_str(&format!("{}/", crate::site()))
        ),
    )
        + r##"<body class="home"><a class="skip" href="#main">Skip to content</a>"##
        + &nav("")
        + r#"<main id="main" class="page">"#
        + &body
        + "</main>"
        + &foot()
        + &tail(index_json)
}
