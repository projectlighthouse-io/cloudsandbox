//! Builds the toolkit docs site: landing page plus one reference page per component.
//! Reads `content/`, writes `dist/`. Run with `cargo run --manifest-path docs/Cargo.toml`.

mod hl;
mod landing;
mod md;
mod mocks;

use std::fmt::Write as _;
use std::{fs, path::Path};

const CTA: &str = "Read the docs";

const CATS: &[(&str, &[&str])] = &[
    ("Requests", &["route", "validate", "action"]),
    ("Sessions and security", &["securecookie", "session", "csrf"]),
    ("Frontend", &["inertia", "vite", "sse"]),
    ("Background work", &["queue", "event", "command"]),
    ("Data", &["db"]),
    ("Auth", &["auth", "password", "socialite"]),
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

/// A page's URL: `/docs/` for the guide, `/docs/<id>/` for a component.
fn href(id: &str) -> String {
    if id == "docs" { "/docs/".into() } else { format!("/docs/{id}/") }
}

/// Python's `html.escape(t, quote=True)`.
fn esc(t: &str) -> String {
    hl::esc(t).replace('"', "&quot;").replace('\'', "&#x27;")
}

fn strip_tags(t: &str) -> String {
    md::unescape(&md::strip_tags(t)).trim().to_string()
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

fn head(title: &str, desc: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{}</title>
<meta name="description" content="{}">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Geist:wght@400;500;600&family=Geist+Mono:wght@400;500&display=swap">
<link rel="stylesheet" href="/site.css">
<script>document.documentElement.classList.add("js")</script>
</head>"#,
        esc(title),
        esc(desc)
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
            let links: String = ids.iter().map(|i| format!(r#"<a class="mono" href="/docs/{i}/">{i}</a>"#)).collect();
            format!("<div><h3>{cat}</h3>{links}</div>")
        })
        .collect();
    format!(
        r#"<footer class="foot"><div class="rail-box">
<div class="foot-top"><div class="foot-brand"><a class="brand inv" href="/">{LOGO}<span>toolkit</span></a>
<p>Small Rust crates for server-driven web apps. A design draft: the APIs on these pages are sketches and have not been compiled.</p>
<a class="btn btn-inv" href="/docs/">{CTA}</a></div>
<div class="foot-cols">{cols}</div></div>
<div class="foot-legal mono"><span>Design draft · 26 November 2026</span><span>Built as static HTML, CSS and a little JavaScript</span></div>
<div class="wordmark" aria-hidden="true">toolkit</div></div></footer>"#
    )
}

fn tail(index_json: &str) -> String {
    format!(r#"{}<script>window.DOCS={index_json};</script><script src="/site.js" defer></script></body></html>"#, palette())
}

/// Splits `---\nkey: value\n---\n` front matter off a page, returning (builds on, phase) and the Markdown.
fn front_matter(raw: &str) -> ((String, String), String) {
    let Some(rest) = raw.strip_prefix("---\n") else { return (Default::default(), raw.to_string()) };
    let (head, body) = rest.split_once("\n---\n").expect("unclosed front matter");
    let get = |key: &str| {
        head.lines().find_map(|l| l.strip_prefix(key)?.strip_prefix(':')).map_or(String::new(), |v| v.trim().to_string())
    };
    ((get("builds_on"), get("phase")), body.trim_start_matches('\n').to_string())
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
            let module = if MODULES.contains(i) { r#"<span class="mod">module</span>"# } else { "" };
            write!(out, r#"<a class="mono" href="/docs/{i}/"{cur}>{i}{module}</a>"#).unwrap();
        }
    }
    out
}

fn doc_page(all: &[Page], i: usize, raw: &str, meta: &(String, String), r: &md::Rendered, index_json: &str) -> String {
    let p = &all[i];
    let is_comp = p.id != "docs";
    let body = &r.body;
    let toc = &r.toc;
    let mut meta_cells = String::new();
    if is_comp {
        let (deps, phase) = meta;
        let kind = if MODULES.contains(&p.id) { "Module" } else { "Crate" };
        meta_cells = format!(
            r#"<div class="metas"><div><span class="mono">Kind</span><b>{kind}</b></div><div><span class="mono">Area</span><b>{}</b></div><div><span class="mono">Phase</span><b>{phase}</b></div><div class="wide"><span class="mono">Builds on</span><b>{}</b></div></div>"#,
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
<div class="dhead"><h1 class="{}">{}</h1><button type="button" class="btn btn-line btn-sm copy-page">{}<span>Copy page</span></button></div>
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
        raw.replace("</", "<\\/")
    );
    head(&format!("{} · toolkit docs", p.title), first_chars(&strip_tags(&r.lead), 150))
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

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = root.join("dist");
    fs::create_dir_all(&out).unwrap();
    let all = pages();
    let links: Vec<(&str, String)> = all.iter().map(|p| (p.src.as_str(), href(p.id))).collect();
    let links: Vec<(&str, &str)> = links.iter().map(|(s, o)| (*s, o.as_str())).collect();
    let (metas, raws): (Vec<_>, Vec<_>) = all.iter().map(|p| front_matter(&fs::read_to_string(root.join("content").join(&p.src)).unwrap())).unzip();
    let rendered: Vec<md::Rendered> = all.iter().zip(&raws).map(|(p, raw)| md::render(raw, &p.src, &links)).collect();

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
        let dir = if p.id == "docs" { out.join("docs") } else { out.join("docs").join(p.id) };
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("index.html"), doc_page(&all, i, &raws[i], &metas[i], &rendered[i], &index_json)).unwrap();
    }
    fs::write(out.join("site.css"), include_str!("../assets/site.css")).unwrap();
    fs::write(out.join("site.js"), include_str!("../assets/site.js")).unwrap();
    println!("built {} pages", all.len() + 1);
}
