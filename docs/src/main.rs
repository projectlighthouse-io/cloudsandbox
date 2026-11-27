//! Builds the toolkit docs site: landing page plus one reference page per component.
//! Reads `content/`, writes `dist/`. Run with `cargo run --manifest-path docs/Cargo.toml`.

mod highlight;
mod landing;
mod markdown;
mod mocks;

use std::fmt::Write as _;
use std::{fs, path::Path};

const CTA: &str = "Read the docs";

const CATS: &[(&str, &[&str])] = &[
    ("Requests", &["route", "validate", "action"]),
    ("Sessions and security", &["securecookie", "session", "csrf", "throttle"]),
    ("Frontend", &["inertia", "vite", "sse"]),
    ("Background work", &["queue", "event", "command"]),
    ("Data", &["db"]),
    ("Modules", &["auth", "password", "socialite"]),
];
const MODULES: &[&str] = &["auth", "password", "socialite"];

struct Page {
    id: &'static str,
    title: &'static str,
    group: &'static str,
    src: String,
}

fn pages() -> Vec<Page> {
    let mut out = vec![Page { id: "docs", title: "Using them together", group: "Getting started", src: "app-layout.md".into() }];
    for (cat, ids) in CATS {
        for id in *ids {
            let dir = if MODULES.contains(id) { "modules" } else { "crates" };
            out.push(Page { id, title: id, group: cat, src: format!("{dir}/{id}.md") });
        }
    }
    out
}

/// A page's URL: `/docs/` for the guide, `/docs/crates/<id>/` or `/docs/modules/<id>/` for a component.
pub fn href(id: &str) -> String {
    match id {
        "docs" => "/docs/".into(),
        id if MODULES.contains(&id) => format!("/docs/modules/{id}/"),
        id => format!("/docs/crates/{id}/"),
    }
}

/// Python's `html.escape(t, quote=True)`.
fn esc(t: &str) -> String {
    highlight::esc(t).replace('"', "&quot;").replace('\'', "&#x27;")
}

fn strip_tags(t: &str) -> String {
    markdown::unescape(&markdown::strip_tags(t)).trim().to_string()
}

fn first_chars(s: &str, n: usize) -> &str {
    s.char_indices().nth(n).map_or(s, |(i, _)| &s[..i])
}

fn icon(name: &str, cls: &str) -> String {
    let path = match name {
        "search" => r#"<path d="M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM20 20l-3.5-3.5"/>"#,
        "arrow" => r#"<path d="M5 12h14M13 6l6 6-6 6"/>"#,
        "copy" => r#"<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/>"#,
        "plus" => r#"<path d="M12 5v14M5 12h14"/>"#,
        "layers" => r#"<path d="M12 3l9 5-9 5-9-5 9-5zM3 13l9 5 9-5"/>"#,
        "shield" => r#"<path d="M12 3l8 3v6c0 4.5-3.4 8.3-8 9-4.6-.7-8-4.5-8-9V6l8-3z"/><path d="M8.5 12l2.5 2.5 4.5-5"/>"#,
        "snow" => r#"<path d="M12 2v20M4.9 7l14.2 10M4.9 17L19.1 7"/>"#,
        "menu" => r#"<path d="M4 7h16M4 12h16M4 17h16"/>"#,
        "check" => r#"<path d="M5 12.5l4.5 4.5L19 7.5"/>"#,
        _ => unreachable!(),
    };
    format!(r#"<svg class="{cls}" viewBox="0 0 24 24" aria-hidden="true">{path}</svg>"#)
}

fn ic(name: &str) -> String {
    icon(name, "ic")
}

const LOGO: &str = concat!(
    r#"<svg class="logo" viewBox="0 0 24 24" aria-hidden="true"><rect width="24" height="24" rx="6"/>"#,
    r##"<path d="M8 6v12M8 9.5h5M8 15.5h5" stroke="#fff" stroke-width="1.8" fill="none" stroke-linecap="round"/>"##,
    r##"<circle cx="15.5" cy="9.5" r="1.9" fill="#4db6a0"/><circle cx="15.5" cy="15.5" r="1.9" fill="#fff"/></svg>"##,
);

/// The published origin. Canonical links, the sitemap and social previews need
/// it absolute. `SITE_URL` at build time overrides it, e.g. for a staging host.
fn site() -> &'static str {
    static SITE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SITE.get_or_init(|| match std::env::var("SITE_URL") {
        Ok(url) if !url.trim().is_empty() => url.trim().trim_end_matches('/').to_owned(),
        _ => "https://toolkit.projectlighthouse.io".to_owned(),
    })
}

const SITE_NAME: &str = "toolkit";

/// `<head>` with SEO: canonical URL, Open Graph and Twitter cards, icons, and
/// `ld` as JSON-LD structured data. `path` is the page's root-relative URL.
fn head(title: &str, desc: &str, path: &str, kind: &str, ld: &str) -> String {
    let url = format!("{}{path}", site());
    let image = format!("{}/og.png", site());
    let ld = ld.replace("</", "<\\/");
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{t}</title>
<meta name="description" content="{d}">
<link rel="canonical" href="{url}">
<meta name="theme-color" content="#151715">
<link rel="icon" href="/favicon.svg" type="image/svg+xml">
<link rel="apple-touch-icon" href="/apple-touch-icon.png">
<meta property="og:type" content="{kind}">
<meta property="og:site_name" content="{SITE_NAME}">
<meta property="og:title" content="{t}">
<meta property="og:description" content="{d}">
<meta property="og:url" content="{url}">
<meta property="og:image" content="{image}">
<meta property="og:image:width" content="1200">
<meta property="og:image:height" content="630">
<meta property="og:image:alt" content="toolkit: opinionated Rust batteries">
<meta name="twitter:card" content="summary_large_image">
<meta name="twitter:title" content="{t}">
<meta name="twitter:description" content="{d}">
<meta name="twitter:image" content="{image}">
<script type="application/ld+json">{ld}</script>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Geist:wght@400;500;600&family=Geist+Mono:wght@400;500&display=swap">
<link rel="stylesheet" href="/site.css">
<script>document.documentElement.classList.add("js")</script>
</head>"##,
        t = esc(title),
        d = esc(desc),
    )
}

fn nav(active: &str) -> String {
    let items: String = [("/docs/", "Docs"), ("/#parts", "Components"), ("/#faq", "FAQ")]
        .iter()
        .map(|(h, t)| format!(r#"<a href="{h}"{}>{t}</a>"#, if *t == active { r#" aria-current="page""# } else { "" }))
        .collect();
    format!(
        r#"<header class="nav"><div class="rail-box nav-in">
<a class="brand" href="/" aria-label="toolkit home">{LOGO}<span>toolkit</span></a>
<nav class="links" aria-label="Main">{items}</nav>
<div class="nav-r">
<button type="button" class="btn-search" data-open-palette aria-label="Search the docs">{}<span>Search</span><kbd class="kbd mono" data-modkey>Ctrl K</kbd></button>
<a class="btn btn-ink" href="/docs/">{CTA}</a>
</div></div></header>"#,
        ic("search")
    )
}

fn palette() -> String {
    format!(
        r#"<div class="pal" id="palette" hidden>
<div class="pal-box" role="dialog" aria-modal="true" aria-label="Search the docs">
<div class="pal-in">{}<input id="palQ" type="text" autocomplete="off" placeholder="Search components, types and sections" aria-label="Search" role="combobox" aria-expanded="true" aria-controls="palList"><kbd class="kbd mono">Esc</kbd></div>
<ul id="palList" role="listbox" aria-label="Results"></ul>
<div class="pal-foot mono"><span><kbd class="kbd">↑</kbd><kbd class="kbd">↓</kbd> move</span><span><kbd class="kbd">Enter</kbd> open</span><span><kbd class="kbd">?</kbd> shortcuts</span></div>
</div></div>
<div class="pal" id="keys" hidden><div class="pal-box keys" role="dialog" aria-modal="true" aria-label="Keyboard shortcuts">
<h2>Keyboard shortcuts</h2>
<dl><div><dt><kbd class="kbd mono" data-modkey>Ctrl K</kbd></dt><dd>Search the docs</dd></div>
<div><dt><kbd class="kbd mono">/</kbd></dt><dd>Search the docs</dd></div>
<div><dt><kbd class="kbd mono">?</kbd></dt><dd>Show this list</dd></div>
<div><dt><kbd class="kbd mono">Esc</kbd></dt><dd>Close a dialog</dd></div></dl>
<button type="button" class="btn btn-line" data-close>Close</button></div></div>"#,
        ic("search")
    )
}

fn foot() -> String {
    let cols: String = CATS
        .iter()
        .map(|(cat, ids)| {
            let links: String = ids.iter().map(|i| format!(r#"<a class="mono" href="{}">{i}</a>"#, href(i))).collect();
            format!("<div><h3>{cat}</h3>{links}</div>")
        })
        .collect();
    format!(
        r#"<footer class="foot"><div class="rail-box">
<div class="foot-top"><div class="foot-brand"><a class="brand inv" href="/">{LOGO}<span>toolkit</span></a>
<p>Small Rust crates for server-driven web apps. Take one crate, or all of them.</p>
<a class="btn btn-inv" href="/docs/">{CTA}</a></div>
<div class="foot-cols">{cols}</div></div>
<div class="foot-legal mono"><a href="https://github.com/projectlighthouse-io/toolkit">Source on GitHub</a><span>Built as static HTML, CSS and a little JavaScript</span></div>
<div class="wordmark" aria-hidden="true">toolkit</div></div></footer>"#
    )
}

fn tail(index_json: &str) -> String {
    format!(r#"{}<script>window.DOCS={index_json};</script><script src="/site.js" defer></script></body></html>"#, palette())
}

/// Splits `---\nkey: value\n---\n` front matter off a page, returning (builds on, phase) and the Markdown.
/// Splits front matter off a page: (builds on, version) and the Markdown.
fn front_matter(raw: &str) -> ((String, String), String) {
    let Some(rest) = raw.strip_prefix("---\n") else { return (Default::default(), raw.to_string()) };
    let (head, body) = rest.split_once("\n---\n").expect("unclosed front matter");
    let get = |key: &str| {
        head.lines().find_map(|l| l.strip_prefix(key)?.strip_prefix(':')).map_or(String::new(), |v| v.trim().to_string())
    };
    ((get("builds_on"), get("version")), body.trim_start_matches('\n').to_string())
}

fn code_spans(t: &str) -> String {
    let mut out = String::new();
    for (i, part) in t.split('`').enumerate() {
        // ponytail: assumes balanced backticks, true for every "Builds on" line
        if i % 2 == 1 {
            write!(out, "<code>{part}</code>").unwrap();
        } else {
            out.push_str(part);
        }
    }
    out
}

fn side(active: &str) -> String {
    let mut out = format!(
        r#"<p class="sg mono">Getting started</p><a href="/docs/"{}>Using them together</a>"#,
        if active == "docs" { r#" aria-current="page""# } else { "" }
    );
    for (cat, ids) in CATS {
        write!(out, r#"<p class="sg mono">{cat}</p>"#).unwrap();
        for i in *ids {
            let cur = if *i == active { r#" aria-current="page""# } else { "" };
            write!(out, r#"<a class="mono" href="{}"{cur}>{i}</a>"#, href(i)).unwrap();
        }
    }
    out
}

fn doc_page(all: &[Page], i: usize, raw: &str, meta: &(String, String), r: &markdown::Rendered, index_json: &str) -> String {
    let p = &all[i];
    let is_comp = p.id != "docs";
    let body = &r.body;
    let toc = &r.toc;
    let mut meta_cells = String::new();
    if is_comp {
        let (deps, version) = meta;
        // The crate's `version` in its Cargo.toml. Everything is beta while it is 0.x.
        assert!(!version.is_empty(), "{}: front matter needs `version:` (the crate's Cargo.toml version)", p.src);
        let version = if version.starts_with("0.") { format!(r#"<em class="ver">{version}</em><em class="beta">beta</em>"#) } else { format!(r#"<em class="ver">{version}</em>"#) };
        let kind = if MODULES.contains(&p.id) { "Module" } else { "Crate" };
        meta_cells = format!(
            r#"<div class="metas"><div><span class="mono">Kind</span><b>{kind}</b></div><div><span class="mono">Area</span><b>{}</b></div><div><span class="mono">Version</span><b>{version}</b></div><div class="wide"><span class="mono">Builds on</span><b>{}</b></div></div>"#,
            p.group,
            code_spans(&esc(&deps))
        );
    }
    let mock_html = if is_comp { format!(r#"<div class="docmock">{}</div>"#, mocks::mock(p.id)) } else { String::new() };
    let pager_link = |q: &Page, cls: &str, label: &str| {
        format!(
            r#"<a{cls} href="{}"><span class="mono">{label}</span><b{}>{}</b></a>"#,
            href(q.id),
            if q.id != "docs" { r#" class="mono""# } else { "" },
            q.title
        )
    };
    let mut pager = String::from(r#"<nav class="pager" aria-label="Previous and next">"#);
    pager += &i.checked_sub(1).map_or("<span></span>".into(), |j| pager_link(&all[j], "", "Previous"));
    pager += &all.get(i + 1).map_or("<span></span>".into(), |n| pager_link(n, r#" class="nx""#, "Next"));
    pager += "</nav>";
    let tocs: String = toc.iter().map(|(a, b)| format!(r##"<a href="#{a}">{}</a>"##, strip_tags(b))).collect();
    let lead = if r.lead.is_empty() { String::new() } else { format!(r#"<p class="dlead">{}</p>"#, r.lead) };
    let toc_html = if toc.is_empty() { String::new() } else { format!(r#"<p class="sg mono">On this page</p>{tocs}"#) };
    let main = format!(
        r#"<div class="rail-box docs">
<aside class="dside" id="dside"><nav aria-label="Documentation">{}</nav></aside>
<article class="dmain">
<p class="crumb mono"><a href="/docs/">Docs</a> / <span>{}</span></p>
<div class="dhead"><h1 class="{}">{}</h1><div class="page-actions"><button type="button" class="btn btn-line btn-sm copy-page">{}<span>Copy page</span></button><a class="btn btn-line btn-sm" href="{md}" title="This page as Markdown, for LLMs">Markdown</a><a class="btn btn-line btn-sm" href="{claude}" target="_blank" rel="noopener">Open in Claude</a></div></div>
{lead}{meta_cells}{mock_html}
<div class="prose">{body}</div>
{pager}
</article>
<aside class="dtoc" aria-label="On this page">{toc_html}</aside>
</div>
<script type="text/markdown" id="pageMd">{}</script>"#,
        side(p.id),
        p.group,
        if is_comp { "mono" } else { "" },
        esc(p.title),
        ic("copy"),
        raw.replace("</", "<\\/"),
        md = md_href(p.id),
        claude = format!(
            "https://claude.ai/new?q={}",
            percent(&format!("Read {}{} and help me use it in my Rust app.", site(), md_href(p.id)))
        ),
    );
    let lead_text = strip_tags(&r.lead);
    let desc = description(&lead_text, &paragraphs(&r.body));
    let desc = desc.as_str();
    // Component pages lead with a one-line summary; it makes the title say what the crate does.
    let summary = format!("{}: {} · {SITE_NAME}", p.title, lead_text.trim().trim_end_matches('.'));
    let title = if is_comp && !lead_text.is_empty() && summary.chars().count() <= 62 { summary } else { format!("{} · Rust {SITE_NAME} docs", p.title) };
    let path = href(p.id);
    let mut crumbs = vec![("Docs".to_owned(), "/docs/".to_owned())];
    if is_comp {
        crumbs.push((p.title.to_owned(), path.clone()));
    }
    let ld = format!(
        r#"[{{"@context":"https://schema.org","@type":"TechArticle","headline":{},"description":{},"url":{},"image":{},"isPartOf":{{"@type":"WebSite","name":{},"url":{}}}}},{{"@context":"https://schema.org","@type":"BreadcrumbList","itemListElement":[{}]}}]"#,
        json_str(&title),
        json_str(desc),
        json_str(&format!("{}{path}", site())),
        json_str(&format!("{}/og.png", site())),
        json_str(SITE_NAME),
        json_str(&format!("{}/", site())),
        crumbs
            .iter()
            .enumerate()
            .map(|(n, (name, u))| format!(r#"{{"@type":"ListItem","position":{},"name":{},"item":{}}}"#, n + 1, json_str(name), json_str(&format!("{}{u}", site()))))
            .collect::<Vec<_>>()
            .join(",")
    );
    head(&title, desc, &path, "article", &ld)
        .replace("</head>", &format!("<link rel=\"alternate\" type=\"text/markdown\" href=\"{}\">\n</head>", md_href(p.id)))
        + r##"<body class="doc"><a class="skip" href="#main">Skip to content</a>"##
        + &nav(if is_comp { "Components" } else { "Docs" })
        + &format!(r#"<button type="button" class="dmenu" aria-expanded="false" aria-controls="dside">{}<span>Menu</span></button>"#, ic("menu"))
        + r#"<main id="main" class="page">"#
        + &main
        + "</main>"
        + &foot()
        + &tail(index_json)
}

/// A JSON string the way Python's `json.dumps(ensure_ascii=False)` writes it.
fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => write!(out, "\\u{:04x}", c as u32).unwrap(),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The text of the `<p>` elements in rendered HTML: prose only, no headings or code.
fn paragraphs(html: &str) -> String {
    html.split("<p>").skip(1).filter_map(|chunk| chunk.split_once("</p>")).map(|(p, _)| strip_tags(p)).collect::<Vec<_>>().join(" ")
}

/// A search snippet: the page's summary, then whole sentences of its body
/// while they fit in 155 characters.
fn description(lead: &str, body: &str) -> String {
    const MAX: usize = 155;
    let squash = |t: &str| t.split_whitespace().collect::<Vec<_>>().join(" ");
    let (mut out, text) = (squash(lead), squash(body));
    let mut rest = text.as_str();
    for sentence in text.split_inclusive(". ") {
        let next = if out.is_empty() { sentence.trim().to_owned() } else { format!("{out} {}", sentence.trim()) };
        if next.chars().count() > MAX {
            break;
        }
        out = next;
        rest = &rest[sentence.len().min(rest.len())..];
    }
    // Too short to be a useful snippet: fill with whole words, then an ellipsis.
    if out.chars().count() < 100 {
        for word in rest.split_whitespace() {
            if out.chars().count() + word.chars().count() + 2 > MAX {
                out.push('…');
                break;
            }
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(word);
        }
    }
    out
}

/// The page's Markdown twin, for agents: `/docs/crates/route/` -> `/docs/crates/route.md`.
fn md_href(id: &str) -> String {
    format!("{}.md", href(id).trim_end_matches('/'))
}

const REPO: &str = "https://github.com/projectlighthouse-io/toolkit";

fn percent(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

/// A page as Markdown for LLM agents: title and summary, then a metadata block
/// (kind, version, install line, HTML URL), then the page, with links between
/// pages pointing at their Markdown twins.
fn page_markdown(all: &[Page], p: &Page, raw: &str, meta: &(String, String)) -> String {
    let mut blocks = raw.trim().splitn(3, "\n\n");
    let (title, lead, rest) = (blocks.next().unwrap_or(""), blocks.next().unwrap_or(""), blocks.next().unwrap_or(""));
    let mut info = vec![format!("- Page: {}{}", site(), href(p.id))];
    if p.id != "docs" {
        let kind = if MODULES.contains(&p.id) { "module" } else { "crate" };
        let (deps, version) = meta;
        info.insert(0, format!("- Kind: {kind} (area: {})", p.group));
        info.insert(1, format!("- Version: {version}{}", if version.starts_with("0.") { " (beta: minor versions may break the API)" } else { "" }));
        info.insert(2, format!("- Install: `{} = {{ git = \"{REPO}\" }}` (git dependency; not on crates.io)", p.id));
        if !deps.is_empty() {
            info.insert(3, format!("- Builds on: {deps}"));
        }
    }
    let mut body = format!("{title}\n\n{lead}\n\n{}\n\n{rest}\n", info.join("\n"));
    for q in all {
        let file = q.src.rsplit('/').next().unwrap_or(&q.src);
        body = body.replace(&format!("]({file})"), &format!("]({}{})", site(), md_href(q.id)));
    }
    body
}

/// `/llms.txt` (llmstxt.org): what the toolkit is and a link to every page's Markdown.
fn llms_txt(all: &[Page], leads: &[String]) -> String {
    let mut out = format!(
        "# {SITE_NAME}\n\n> Opinionated Rust batteries: small, independent crates for server-driven web apps (routing, validation, sessions, CSRF, auth, social login, rate limiting, Inertia + Vite, queues, events, server-sent events), plus `art`, a CLI that scaffolds and runs apps. Laravel is the reference for developer experience; each crate stands alone and the app owns its `main.rs`.\n\n\
Key facts for agents:\n\n\
- Install crates from git, not crates.io: `route = {{ git = \"{REPO}\" }}`. Optional parts are Cargo features listed on each page.\n\
- Everything is beta (`0.x`): a minor version bump may break the API. Each page states its crate's version.\n\
- Start an app with `cargo install --git {REPO} art` and `art new myapp` (Postgres by default; `--driver sqlite` or `--driver mysql`). `art publish <crate>` copies a module's handlers, migrations and pages into the app, to edit freely.\n\
- No framework: there is no glue crate, ORM, controller or template layer. Handlers are plain async functions; layers are `tower` layers added in `main.rs`.\n\
- Every page below is plain Markdown. All of them in one file: {}/llms-full.txt\n",
        site()
    );
    let mut group = "";
    for (p, lead) in all.iter().zip(leads) {
        if p.group != group {
            group = p.group;
            out += &format!("\n## {group}\n\n");
        }
        let lead = lead.split_whitespace().collect::<Vec<_>>().join(" ");
        out += &format!("- [{}]({}{}): {}\n", p.title, site(), md_href(p.id), lead.trim_end_matches('.'));
    }
    out
}

fn sitemap(paths: &[String]) -> String {
    let urls: String = paths.iter().map(|p| format!("  <url><loc>{}{}</loc></url>\n", site(), esc(p))).collect();
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n{urls}</urlset>\n")
}

/// The page for unknown URLs. Not indexed; links back into the docs.
fn not_found(index_json: &str) -> String {
    let ld = format!(r#"{{"@context":"https://schema.org","@type":"WebPage","name":"Page not found","url":{}}}"#, json_str(&format!("{}/404.html", site())));
    head(&format!("Page not found · {SITE_NAME}"), "This page does not exist. Start from the docs or the list of components.", "/404.html", "website", &ld)
        .replace("<link rel=\"canonical\"", "<meta name=\"robots\" content=\"noindex\">\n<link rel=\"canonical\"")
        + r##"<body><a class="skip" href="#main">Skip to content</a>"##
        + &nav("")
        + r#"<main id="main" class="page"><section class="hero"><div class="hero-tex" aria-hidden="true"></div><div class="rail-box hero-in" style="padding-bottom:120px">
<p class="crumb mono">404</p>
<h1>This page does not exist.</h1>
<p class="sub">It may have moved. Start from the docs, or search with Ctrl K.</p>
<div class="ctas"><a class="btn btn-ink btn-lg" href="/docs/">Read the docs</a><a class="btn btn-line btn-lg" href="/#parts">Browse components</a></div>
</div></section></main>"#
        + &foot()
        + &tail(index_json)
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = root.join("dist");
    fs::create_dir_all(&out).unwrap();
    let all = pages();
    let links: Vec<(&str, String)> = all.iter().map(|p| (p.src.as_str(), href(p.id))).collect();
    let links: Vec<(&str, &str)> = links.iter().map(|(s, o)| (*s, o.as_str())).collect();
    let (metas, raws): (Vec<_>, Vec<_>) = all.iter().map(|p| front_matter(&fs::read_to_string(root.join("content").join(&p.src)).unwrap())).unzip();
    let rendered: Vec<markdown::Rendered> = all.iter().zip(&raws).map(|(p, raw)| markdown::render(raw, &p.src, &links)).collect();

    let items: Vec<String> = all
        .iter()
        .zip(&rendered)
        .map(|(p, r)| {
            let heads: Vec<String> = r
                .toc
                .iter()
                .map(|(a, b)| format!("[{}, {}]", json_str(a), json_str(&strip_tags(b))))
                .collect();
            let text = format!("{} {}", strip_tags(&r.lead), strip_tags(&r.words));
            let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
            format!(
                r#"{{"u": {}, "t": {}, "g": {}, "m": {}, "h": [{}], "x": {}}}"#,
                json_str(&href(p.id)),
                json_str(p.title),
                json_str(p.group),
                p.id != "docs",
                heads.join(", "),
                json_str(first_chars(&text, 1500))
            )
        })
        .collect();
    let index_json = format!("[{}]", items.join(", ")).replace("</", "<\\/");

    fs::write(out.join("index.html"), landing::page(&index_json)).unwrap();
    let _ = fs::remove_dir_all(out.join("docs"));
    for (i, p) in all.iter().enumerate() {
        let dir = if p.id == "docs" { out.join("docs") } else { out.join(href(p.id).trim_matches('/')) };
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("index.html"), doc_page(&all, i, &raws[i], &metas[i], &rendered[i], &index_json)).unwrap();
    }
    // Markdown twins of every page, llms.txt and llms-full.txt, for LLM agents.
    let markdown: Vec<String> = all.iter().zip(&raws).zip(&metas).map(|((p, raw), meta)| page_markdown(&all, p, raw, meta)).collect();
    for (p, md) in all.iter().zip(&markdown) {
        let file = out.join(md_href(p.id).trim_start_matches('/'));
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, md).unwrap();
    }
    let leads: Vec<String> = rendered.iter().map(|r| strip_tags(&r.lead)).collect();
    let llms = llms_txt(&all, &leads);
    fs::write(out.join("llms.txt"), &llms).unwrap();
    fs::write(out.join("llms-full.txt"), format!("{llms}\n---\n\n{}", markdown.join("\n---\n\n"))).unwrap();

    let urls: Vec<String> = std::iter::once("/".to_owned()).chain(all.iter().map(|p| href(p.id))).collect();
    fs::write(out.join("sitemap.xml"), sitemap(&urls)).unwrap();
    fs::write(out.join("robots.txt"), format!("User-agent: *\nAllow: /\n\nSitemap: {}/sitemap.xml\n", site())).unwrap();
    fs::write(out.join("404.html"), not_found(&index_json)).unwrap();
    fs::write(out.join("favicon.svg"), include_str!("../assets/favicon.svg")).unwrap();
    fs::write(out.join("apple-touch-icon.png"), include_bytes!("../assets/apple-touch-icon.png")).unwrap();
    fs::write(out.join("og.png"), include_bytes!("../assets/og.png")).unwrap();
    // Cloudflare static assets read `_headers`: Markdown and llms files as text.
    fs::write(out.join("_headers"), include_str!("../assets/_headers")).unwrap();
    fs::write(out.join("site.css"), include_str!("../assets/site.css")).unwrap();
    fs::write(out.join("site.js"), include_str!("../assets/site.js")).unwrap();
    println!("built {} pages", all.len() + 1);
}
