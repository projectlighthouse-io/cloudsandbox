//! One small, realistic fragment of the product per component.
//!
//! Each mock carries exactly one live behaviour, declared with data-live:
//!   seq    children marked data-s reveal one by one, hold, reset
//!   cycle  children marked data-c take the selected ring in turn
//!   steps  children marked data-st move pending -> current -> done
//!   fill   the bar marked data-f fills, then resets
//! Markup only; site.js drives it, and reduced motion shows the final state.

fn frame(inner: &str, live: &str, step: u32, label: &str) -> String {
    format!(
        "<div class=\"mock \" data-live=\"{live}\" data-step=\"{step}\" role=\"img\" aria-label=\"{label}\"><div class=\"mock-in\">{inner}</div></div>"
    )
}

fn bar(title: &str, right: &str) -> String {
    let right = if right.is_empty() { String::new() } else { format!("<span class=\"mr\">{right}</span>") };
    format!("<div class=\"mbar\"><span class=\"mt\">{title}</span>{right}</div>")
}

pub fn chk() -> &'static str {
    "<svg class=\"ck\" viewBox=\"0 0 16 16\" aria-hidden=\"true\"><path d=\"M3.5 8.5l3 3 6-7\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/></svg>"
}

fn status(word: &str, kind: &str) -> String {
    format!("<span class=\"st st-{kind}\"><i></i>{word}</span>")
}

fn stepper(items: &[(&str, &str)]) -> String {
    let li: String = items
        .iter()
        .map(|(name, detail)| {
            format!("<li class=\"step\" data-st><span class=\"sdot\">{}</span><span class=\"sname\">{name}</span><span class=\"sdet\">{detail}</span></li>", chk())
        })
        .collect();
    format!("<ol class=\"steps\">{li}</ol>")
}

fn term(lines: &str) -> String {
    format!("<div class=\"term mono\">{lines}</div>")
}

pub fn mock(id: &str) -> String {
    match id {
        "route" => {
            let rows: String = [("GET", "/runs", "runs.index", ""), ("POST", "/runs", "runs.store", ""), ("GET", "/runs/{id}", "runs.show", " data-final"), ("DELETE", "/runs/{id}", "runs.destroy", "")]
                .iter()
                .map(|(m, p, n, fin)| format!("<div class=\"row\" data-c{fin}><span class=\"meth\">{m}</span><span class=\"mono grow\">{p}</span><span class=\"mono muted\">{n}</span></div>"))
                .collect();
            frame(&format!("{}<div class=\"rows fade\">{rows}</div>", bar("route:list", "GET /runs/42")), "cycle", 1100, "A route table with a request scanning for its match")
        }
        "validate" => frame(
            &(bar("New pipeline", "POST /pipelines")
                + "<div class=\"form\">"
                + "<label class=\"fl\">Name<span class=\"fi mono\">nightly-build</span></label>"
                + "<label class=\"fl\">Repository<span class=\"fi mono err-ring\" data-s>acme shop</span></label>"
                + "<p class=\"ferr\" data-s>Use owner/name, for example acme/shop.</p>"
                + "<pre class=\"json mono\" data-s>{ \"errors\": { \"repo\": \"Use owner/name…\" } }</pre>"
                + "</div>"),
            "seq",
            1000,
            "A form that shows a field error after validation fails",
        ),
        "action" => {
            let rows: String = [("H", "Handler", "r.post(\"/runs\", action(start_run))"), ("J", "Job", "queue.push(&start_run, input)"), ("L", "Listener", "events.on::<PushReceived>(start_run)"), ("C", "Command", "commands.add(\"runs:start\", start_run)")]
                .iter()
                .map(|(t, n, d)| format!("<div class=\"row\" data-c><span class=\"tile\">{t}</span><span class=\"grow\"><b>{n}</b><small class=\"mono\">{d}</small></span></div>"))
                .collect();
            frame(&format!("{}<div class=\"rows\">{rows}</div>", bar("impl Action for StartRun", "1 trait")), "cycle", 1300, "One action registered as a handler, a job, a listener and a command")
        }
        "securecookie" => frame(
            &(bar("securecookie::Codec", "")
                + &term(concat!(
                    "<div class=\"tl\"><span class=\"g\">encrypt</span> \"session\", 8f3a91c2</div>",
                    "<div class=\"tl\" data-s><span class=\"g\">→</span> v1.Zx9kQ2mB7cWq.p7dF3Q</div>",
                    "<div class=\"tl\" data-s><span class=\"g\">decrypt</span> v1.Zx9kQ2mB7cWq.<u>X</u>7dF3Q</div>",
                    "<div class=\"tl bad\" data-s><span class=\"g\">→</span> rejected: invalid value</div>",
                ))),
            "seq",
            1100,
            "A cookie value is encrypted, then a tampered value is rejected",
        ),
        "session" => frame(
            &(bar("session", "app_session")
                + "<div class=\"rows\">"
                + "<div class=\"row\"><span class=\"mono muted w8\">user_id</span><span class=\"mono grow\">usr_31</span></div>"
                + "<div class=\"row\"><span class=\"mono muted w8\">cart</span><span class=\"mono grow\">[3 items]</span></div>"
                + "<div class=\"row\" data-s><span class=\"mono muted w8\">flash</span><span class=\"mono grow\">\"Profile saved.\"</span>"
                + &status("next request", "acc")
                + "</div>"
                + "<div class=\"row\" data-s><span class=\"mono muted w8\">flash</span><span class=\"mono grow strike\">\"Profile saved.\"</span>"
                + &status("consumed", "idle")
                + "</div>"
                + "</div>"),
            "seq",
            1200,
            "Session values with a flash message that lives for one request",
        ),
        "csrf" => {
            let rows: String = [("/pipelines", "token ok", "ok"), ("/webhooks/github", "exempt", "idle"), ("/settings", "419", "bad")]
                .iter()
                .map(|(p, w, k)| format!("<div class=\"row\" data-s><span class=\"meth\">POST</span><span class=\"mono grow\">{p}</span>{}</div>", status(w, k)))
                .collect();
            frame(&format!("{}<div class=\"rows\">{rows}</div>", bar("csrf::layer()", "")), "seq", 1100, "Requests checked for a CSRF token: one passes, one is exempt, one is rejected")
        }
        "throttle" => {
            let rows: String = [("1", "2 left", "ok"), ("2", "1 left", "ok"), ("3", "0 left", "ok"), ("4", "429 · retry 58s", "bad")]
                .iter()
                .map(|(n, w, k)| format!("<div class=\"row\" data-s><span class=\"meth\">POST</span><span class=\"mono grow\">/login <span class=\"muted\">#{n}</span></span>{}</div>", status(w, k)))
                .collect();
            frame(&format!("{}<div class=\"rows\">{rows}</div>", bar("FixedWindow::new(3, 60s)", "login:ada|10.0.0.1")), "seq", 900, "Login attempts counted against a limit of three a minute, the fourth refused")
        }
        "inertia" => frame(
            &(bar("GET /runs/42", "X-Inertia")
                + &stepper(&[("First visit", "full HTML, page object embedded"), ("Inertia visit", "JSON: component and props"), ("Partial reload", "only logs, other props skipped"), ("Version changed", "409, full reload")])),
            "steps",
            1400,
            "The four kinds of Inertia request and how each is answered",
        ),
        "vite" => frame(
            &(bar("vite.tags()", "")
                + "<div class=\"rows\">"
                + "<div class=\"row\" data-c><span class=\"tag\">dev</span><span class=\"mono grow\">http://localhost:5173/resources/js/app.ts</span></div>"
                + "<div class=\"row\" data-c><span class=\"tag\">prod</span><span class=\"mono grow\">/build/assets/app-4f9c1e.js</span></div>"
                + "</div>"),
            "cycle",
            1600,
            "Asset tags for the dev server and for a production build",
        ),
        "sse" => {
            let lines: String = ["cargo build --release", "Compiling myapp v0.4.0", "Finished in 41.2s", "art migrate", "Release a41c9e2 live"]
                .iter()
                .map(|l| format!("<div class=\"tl\" data-s><span class=\"g\">event:</span> log <span class=\"g\">data:</span> {l}</div>"))
                .collect();
            frame(&(bar("GET /runs/42/logs", "text/event-stream") + &term(&lines)), "seq", 750, "A server-sent event stream appending log lines")
        }
        "queue" => frame(
            &(bar("queue:work", "concurrency 8")
                + "<div class=\"rows fade\">"
                + "<div class=\"row\"><span class=\"mono grow\">ExecuteRun <span class=\"muted\">#4812</span></span><span class=\"prog\"><i data-f></i></span>"
                + &status("running", "acc")
                + "</div>"
                + "<div class=\"row\"><span class=\"mono grow\">SendNotification <span class=\"muted\">#4813</span></span><span class=\"prog\"><i></i></span>"
                + &status("queued", "idle")
                + "</div>"
                + "<div class=\"row\"><span class=\"mono grow\">PruneRuns <span class=\"muted\">#4807</span></span><span class=\"prog\"><i class=\"full\"></i></span>"
                + &status("done", "ok")
                + "</div>"
                + "<div class=\"row\"><span class=\"mono grow\">ExecuteRun <span class=\"muted\">#4805</span></span><span class=\"prog\"><i class=\"full\"></i></span>"
                + &status("retried", "warn")
                + "</div>"
                + "</div>"),
            "fill",
            4200,
            "A job queue with one job in progress",
        ),
        "event" => frame(
            &(bar("events.emit(RunFinished)", "") + &stepper(&[("update_commit_status", "inline, ran in 3 ms"), ("send_notification", "queued as job #4813"), ("record_metrics", "inline, ran in 1 ms")])),
            "steps",
            1000,
            "One event reaching three listeners, one of them queued",
        ),
        "command" => {
            let lines: String = [("GET   ", "/runs           runs.index"), ("GET   ", "/runs/{id}      runs.show"), ("POST  ", "/runs           runs.store"), ("POST  ", "/webhooks/gh    webhooks.github")]
                .iter()
                .map(|(m, r)| format!("<div class=\"tl\" data-s><span class=\"g\">{m}</span> {r}</div>"))
                .collect();
            frame(
                &(bar("~/myapp", "") + &term(&format!("<div class=\"tl\"><span class=\"g\">$</span> ./myapp route:list</div>{lines}"))),
                "seq",
                650,
                "A terminal listing the app's routes",
            )
        }
        "db" => frame(
            &(bar("cargo check", "")
                + &term(concat!(
                    "<div class=\"tl bad\"><b>error[E0308]</b>: mismatched types</div>",
                    "<div class=\"tl\" data-s><span class=\"g\">  --&gt;</span> src/reports.rs:14:12</div>",
                    "<div class=\"tl\" data-s><span class=\"g\">14 |</span>     report(analytics)</div>",
                    "<div class=\"tl bad\" data-s><span class=\"g\">   |</span>            ^^^^^^^^^ expected Db&lt;Primary&gt;, found Db&lt;Analytics&gt;</div>",
                ))),
            "seq",
            1000,
            "A compile error caught when two database handles are mixed up",
        ),
        "configured" => frame(
            &(bar("configured::load(\"config\")", "APP_ENV=production")
                + &term(concat!(
                    "<div class=\"tl\"><span class=\"g\">read</span> config/default.toml, config/production.toml</div>",
                    "<div class=\"tl\" data-s><span class=\"g\">app_name</span> env(APP_NAME, my-app) → my-app</div>",
                    "<div class=\"tl\" data-s><span class=\"g\">app_key</span> required_secret_env(APP_KEY) → ********</div>",
                    "<div class=\"tl bad\" data-s><span class=\"g\">database_url</span> DATABASE_URL is not set</div>",
                ))),
            "seq",
            1100,
            "Config files resolved against env vars: a default, a masked secret, and a missing required var",
        ),
        "auth" => frame(
            &(bar("POST /login", "") + &stepper(&[("Credentials checked", "password::verify"), ("Session regenerated", "new id, old one invalid"), ("Authed&lt;User&gt; available", "Elizabeth Lane")])),
            "steps",
            1100,
            "Login steps: credentials checked, session regenerated, user available",
        ),
        "password" => frame(
            &(bar("password::hash", "")
                + &term(concat!(
                    "<div class=\"tl\"><span class=\"g\">input</span> ••••••••••••</div>",
                    "<div class=\"tl\" data-s><span class=\"g\">hash </span> $argon2id$v=19$m=19456,t=2,p=1$</div>",
                    "<div class=\"tl\" data-s><span class=\"g\">     </span> Ym9vdHN0cmFw$Kx3…9QeA</div>",
                    "<div class=\"tl ok\" data-s><span class=\"g\">verify</span> match</div>",
                ))),
            "seq",
            1000,
            "A password hashed with Argon2id and verified",
        ),
        "socialite" => frame(
            &(bar("Google login", "")
                + &stepper(&[("Redirect", "state and PKCE stored in session"), ("Authorize", "accounts.google.com"), ("Callback", "state matches, code exchanged"), ("SocialUser", "google · elizabeth · email verified")])),
            "steps",
            1100,
            "The OAuth redirect and callback flow ending in a typed user",
        ),
        "passkeys" => frame(
            &(bar("Sign in with a passkey", "")
                + &stepper(&[("Options", "challenge kept in the session"), ("Touch ID", "the device signs for acme.test"), ("Verified", "MacBook · counter and flags saved"), ("auth.login", "Elizabeth Lane · no code asked")])),
            "steps",
            1100,
            "A passwordless sign-in: challenge, the device signs, the server verifies and logs in",
        ),
        "teams" => frame(
            &(bar("Acme · Platform", "")
                + &stepper(&[("Organization", "Acme · you are an admin"), ("Team", "Platform · owner through Acme"), ("Invite", "ada@acme.dev as member"), ("Can<InviteMembers>", "admin · granted")])),
            "steps",
            1100,
            "An organization admin inviting a member to one of its teams",
        ),
        _ => unreachable!("no mock for {id}"),
    }
}
