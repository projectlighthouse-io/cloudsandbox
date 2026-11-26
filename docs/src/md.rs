//! Markdown to HTML in the shape Python-Markdown writes it (block per line,
//! tables one tag per line, ids on every heading), which the design was built from.

use crate::hl;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

pub struct Rendered {
    pub lead: String,
    pub body: String,
    /// (id, inner html) of every h2
    pub toc: Vec<(String, String)>,
    pub words: String,
}

pub fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for c in s.chars() {
        match c {
            '<' => inside = true,
            '>' if inside => inside = false,
            _ if !inside => out.push(c),
            _ => {}
        }
    }
    out
}

/// `strip_tags` with every tag replaced by a space.
fn tags_to_space(s: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for c in s.chars() {
        match c {
            '<' => inside = true,
            '>' if inside => {
                inside = false;
                out.push(' ');
            }
            _ if !inside => out.push(c),
            _ => {}
        }
    }
    out
}

pub fn unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").replace("&amp;", "&")
}

fn slug(text: &str) -> String {
    let lower = strip_tags(text).to_lowercase();
    let mut out = String::new();
    for c in lower.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

/// Rewrites a link target relative to `src` into a site page, `None` to drop the link.
fn link(target: &str, src: &str, pages: &[(&str, &str)]) -> Option<String> {
    if target.starts_with("http://") || target.starts_with("https://") || target.starts_with('#') {
        return Some(target.to_string());
    }
    let (path, frag) = target.split_once('#').map_or((target, ""), |(p, f)| (p, f));
    let mut parts: Vec<&str> = src.split('/').collect();
    parts.pop();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    let norm = parts.join("/");
    if let Some((_, out)) = pages.iter().find(|(s, _)| *s == norm) {
        return Some(if frag.is_empty() { out.to_string() } else { format!("{out}#{frag}") });
    }
    if norm == "crates" || norm == "modules" {
        return Some("/#parts".into());
    }
    None
}

/// `pages` maps a content path (`crates/route.md`) to its output page (`route.html`).
pub fn render(text: &str, src: &str, pages: &[(&str, &str)]) -> Rendered {
    let mut html = String::new();
    let mut ids: Vec<String> = vec![];
    let mut heading: Option<(HeadingLevel, usize)> = None;
    let mut code: Option<(String, String)> = None;
    let mut dropped_links = vec![];
    let mut item_depth = 0usize;
    let mut table_head = false;
    let mut cell = 0usize;
    let mut aligns = vec![];

    let parser = Parser::new_ext(text, Options::ENABLE_TABLES).into_offset_iter();
    let mut events = parser.peekable();
    while let Some((ev, range)) = events.next() {
        match ev {
            Event::Start(Tag::Heading { level, .. }) => heading = Some((level, html.len())),
            Event::End(TagEnd::Heading(_)) => {
                let (level, at) = heading.take().unwrap();
                let inner = html.split_off(at);
                let base = slug(&unescape(&inner));
                let mut id = base.clone();
                let mut n = 1;
                while ids.contains(&id) {
                    id = format!("{base}_{n}");
                    n += 1;
                }
                ids.push(id.clone());
                html.push_str(&format!("<{level} id=\"{id}\">{inner}</{level}>\n"));
            }
            Event::Start(Tag::Paragraph) => {
                if item_depth > 0 && html.ends_with("<li>") {
                    html.push('\n');
                }
                html.push_str("<p>");
            }
            Event::End(TagEnd::Paragraph) => html.push_str("</p>\n"),
            Event::Start(Tag::List(start)) => {
                if html.ends_with("<li>") || (item_depth > 0 && !html.ends_with('\n')) {
                    html.push('\n');
                }
                html.push_str(&match start {
                    None => "<ul>\n".to_string(),
                    Some(1) => "<ol>\n".to_string(),
                    Some(n) => format!("<ol start=\"{n}\">\n"),
                });
            }
            Event::End(TagEnd::List(ordered)) => html.push_str(if ordered { "</ol>\n" } else { "</ul>\n" }),
            Event::Start(Tag::Item) => {
                item_depth += 1;
                html.push_str("<li>");
            }
            Event::End(TagEnd::Item) => {
                item_depth -= 1;
                html.push_str("</li>\n");
            }
            Event::Start(Tag::BlockQuote(_)) => html.push_str("<blockquote>\n"),
            Event::End(TagEnd::BlockQuote(_)) => html.push_str("</blockquote>\n"),
            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(l) => l.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                code = Some((lang, String::new()));
            }
            Event::End(TagEnd::CodeBlock) => {
                let (lang, text) = code.take().unwrap();
                html.push_str("<div class=\"code\"><button type=\"button\" class=\"copy\">Copy</button>");
                html.push_str(&hl::block(&text, &lang));
                html.push_str("</div>\n\n");
            }
            Event::Text(t) if code.is_some() => code.as_mut().unwrap().1.push_str(&t),
            Event::Text(t) => html.push_str(&hl::esc(&t)),
            Event::Code(_) => {
                // from the source: Python-Markdown keeps a wrapped span's newline and indent
                let raw = text[range].trim_matches('`').trim();
                html.push_str(&format!("<code>{}</code>", hl::esc(raw)));
            }
            Event::Html(t) | Event::InlineHtml(t) => html.push_str(&t),
            Event::SoftBreak => {
                html.push('\n');
                // Python-Markdown keeps a list item's continuation indent
                if item_depth > 0 {
                    let rest = &text[range.end..];
                    html.push_str(&rest[..rest.len() - rest.trim_start_matches(' ').len()]);
                }
            }
            Event::HardBreak => html.push_str("<br />\n"),
            Event::Rule => html.push_str("<hr />\n"),
            Event::Start(Tag::Emphasis) => html.push_str("<em>"),
            Event::End(TagEnd::Emphasis) => html.push_str("</em>"),
            Event::Start(Tag::Strong) => html.push_str("<strong>"),
            Event::End(TagEnd::Strong) => html.push_str("</strong>"),
            Event::Start(Tag::Link { dest_url, title, .. }) => match link(&dest_url, src, pages) {
                Some(href) => {
                    dropped_links.push(false);
                    let title = if title.is_empty() { String::new() } else { format!(" title=\"{title}\"") };
                    html.push_str(&format!("<a href=\"{href}\"{title}>"));
                }
                None => dropped_links.push(true),
            },
            Event::End(TagEnd::Link) => {
                if !dropped_links.pop().unwrap() {
                    html.push_str("</a>");
                }
            }
            Event::Start(Tag::Table(a)) => {
                aligns = a;
                html.push_str("<div class=\"scroll tbl\"><table>\n");
            }
            Event::End(TagEnd::Table) => html.push_str("</tbody>\n</table></div>\n"),
            Event::Start(Tag::TableHead) => {
                table_head = true;
                html.push_str("<thead>\n<tr>\n");
            }
            Event::End(TagEnd::TableHead) => {
                table_head = false;
                html.push_str("</tr>\n</thead>\n<tbody>\n");
            }
            Event::Start(Tag::TableRow) => {
                cell = 0;
                html.push_str("<tr>\n");
            }
            Event::End(TagEnd::TableRow) => html.push_str("</tr>\n"),
            Event::Start(Tag::TableCell) => {
                let tag = if table_head { "th" } else { "td" };
                let style = match aligns.get(cell) {
                    Some(pulldown_cmark::Alignment::Left) => " style=\"text-align: left;\"",
                    Some(pulldown_cmark::Alignment::Center) => " style=\"text-align: center;\"",
                    Some(pulldown_cmark::Alignment::Right) => " style=\"text-align: right;\"",
                    _ => "",
                };
                html.push_str(&format!("<{tag}{style}>"));
            }
            Event::End(TagEnd::TableCell) => {
                html.push_str(if table_head { "</th>\n" } else { "</td>\n" });
                cell += 1;
            }
            _ => {}
        }
    }

    // drop the h1 (shown in the page header) and take the first paragraph as the lead
    let mut body = html.trim().to_string();
    if body.starts_with("<h1") {
        let end = body.find("</h1>").unwrap() + 5;
        body = body[end..].trim_start().to_string();
    }
    let mut lead = String::new();
    if body.starts_with("<p>") {
        let end = body.find("</p>").unwrap();
        lead = body[3..end].to_string();
        body = body[end + 4..].trim_start().to_string();
    }
    let mut toc = vec![];
    let mut rest = body.as_str();
    while let Some(i) = rest.find("<h2 id=\"") {
        let r = &rest[i + 8..];
        let q = r.find('"').unwrap();
        let close = r.find("</h2>").unwrap();
        toc.push((r[..q].to_string(), r[q + 2..close].to_string()));
        rest = &r[close..];
    }
    let words = tags_to_space(&body);
    Rendered { lead, body, toc, words }
}
