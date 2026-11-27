//! Rust syntax highlighting that emits Pygments' short class names (`.hl .k`,
//! `.hl .s`, ...), which is what site.css styles. The rules follow Pygments'
//! RustLexer state machine, quirks included, so output matches the design.

const KEYWORDS: &[&str] = &[
    "as", "async", "await", "box", "const", "crate", "dyn", "else", "extern", "for", "if", "impl",
    "in", "loop", "match", "move", "mut", "pub", "ref", "return", "static", "super", "trait",
    "unsafe", "use", "where", "while",
];
const RESERVED: &[&str] = &[
    "abstract", "become", "do", "final", "macro", "override", "priv", "typeof", "try", "unsized",
    "virtual", "yield",
];
const TYPES: &[&str] = &[
    "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i64", "i128", "usize", "isize", "f32",
    "f64", "char", "str", "bool",
];
const BUILTINS: &[&str] = &[
    "Copy", "Send", "Sized", "Sync", "Unpin", "Drop", "Fn", "FnMut", "FnOnce", "drop", "Box",
    "ToOwned", "Clone", "PartialEq", "PartialOrd", "Eq", "Ord", "AsRef", "AsMut", "Into", "From",
    "Default", "Iterator", "Extend", "IntoIterator", "DoubleEndedIterator", "ExactSizeIterator",
    "Option", "Some", "None", "Result", "Ok", "Err", "String", "ToString", "Vec",
];
const MACROS: &[&str] = &[
    "asm", "assert", "assert_eq", "assert_ne", "cfg", "column", "compile_error", "concat",
    "concat_idents", "dbg", "debug_assert", "debug_assert_eq", "debug_assert_ne", "env", "eprint",
    "eprintln", "file", "format", "format_args", "format_args_nl", "global_asm", "include",
    "include_bytes", "include_str", "line", "llvm_asm", "log_syntax", "macro_rules", "matches",
    "module_path", "option_env", "panic", "print", "println", "stringify", "thread_local", "todo",
    "trace_macros", "unimplemented", "unreachable", "vec", "write", "writeln",
];

#[derive(Clone, Copy, PartialEq)]
enum St {
    Base,
    Comment,
    DocComment,
    Str,
    Attr,
    Type,
    Func,
    Mod,
    Life,
    Num,
}

fn word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Length of an identifier `[a-zA-Z_]\w*` at the start of `s`.
fn ident(s: &str) -> usize {
    let mut it = s.char_indices();
    match it.next() {
        Some((_, c)) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return 0,
    }
    it.find(|&(_, c)| !word(c)).map_or(s.len(), |(i, _)| i)
}

/// `(a|b|...)\b`: the longest listed word at the start of `s` that ends on a word boundary.
fn one_of(s: &str, list: &[&str]) -> usize {
    list.iter()
        .filter(|w| s.starts_with(**w) && !s[w.len()..].starts_with(word))
        .map(|w| w.len())
        .max()
        .unwrap_or(0)
}

fn ws(s: &str) -> usize {
    s.find(|c: char| !c.is_whitespace()).unwrap_or(s.len())
}

/// `.*?\n`: up to and including the next newline.
fn to_eol(s: &str) -> Option<usize> {
    s.find('\n').map(|i| i + 1)
}

fn digits(s: &str, ok: impl Fn(char) -> bool) -> usize {
    s.find(|c: char| !ok(c)).unwrap_or(s.len())
}

/// String escapes: `\\['"\\nrt]|\\x[0-7][0-9a-fA-F]|\\0|\\u\{[0-9a-fA-F]{1,6}\}`.
fn escape(s: &str) -> usize {
    let b = s.as_bytes();
    if b.first() != Some(&b'\\') || b.len() < 2 {
        return 0;
    }
    match b[1] {
        b'\'' | b'"' | b'\\' | b'n' | b'r' | b't' | b'0' => 2,
        b'x' if b.len() >= 4 && (b'0'..=b'7').contains(&b[2]) && b[3].is_ascii_hexdigit() => 4,
        b'u' if b.get(2) == Some(&b'{') => {
            let n = digits(&s[3..], |c| c.is_ascii_hexdigit());
            if (1..=6).contains(&n) && s[3 + n..].starts_with('}') { 4 + n } else { 0 }
        }
        _ => 0,
    }
}

/// A char literal starting at `'`.
fn char_lit(s: &str) -> usize {
    let rest = &s[1..];
    let n = match escape(rest) {
        0 => match rest.chars().next() {
            Some(c) if c != '\n' => c.len_utf8(),
            _ => return 0,
        },
        n => n,
    };
    if rest[n..].starts_with('\'') { n + 2 } else { 0 }
}

/// Decimal literal; returns (len, is_float).
fn number(s: &str) -> (usize, bool) {
    let int = 1 + digits(&s[1..], |c| c.is_ascii_digit() || c == '_');
    let r = &s[int..];
    let exp = |r: &str| {
        let b = r.as_bytes();
        if !matches!(b.first(), Some(b'e' | b'E')) {
            return 0;
        }
        let sign = matches!(b.get(1), Some(b'+' | b'-')) as usize;
        let n = digits(&r[1 + sign..], |c| c.is_ascii_digit() || c == '_');
        if n > 0 { 1 + sign + n } else { 0 }
    };
    if let Some(frac) = r.strip_prefix('.') {
        let n = digits(frac, |c| c.is_ascii_digit() || c == '_');
        if n > 0 {
            let e = exp(&frac[n..]);
            if e > 0 {
                return (int + 1 + n + e, true);
            }
        }
        // `\.[0-9_]*(?!\.)`, backtracking over the digits
        for k in (0..=n).rev() {
            if !frac[k..].starts_with('.') {
                return (int + 1 + k, true);
            }
        }
    }
    let e = exp(r);
    if e > 0 { (int + e, true) } else { (int, false) }
}

struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    stack: Vec<St>,
    out: Vec<(&'static str, &'a str)>,
}

impl<'a> Lexer<'a> {
    fn emit(&mut self, cls: &'static str, n: usize) {
        self.out.push((cls, &self.src[self.pos..self.pos + n]));
        self.pos += n;
    }
    fn push(&mut self, cls: &'static str, n: usize, st: St) {
        self.emit(cls, n);
        self.stack.push(st);
    }
    fn pop(&mut self, cls: &'static str, n: usize) {
        self.emit(cls, n);
        self.stack.pop();
    }

    fn run(mut self) -> Vec<(&'static str, &'a str)> {
        while self.pos < self.src.len() {
            let s = &self.src[self.pos..];
            match *self.stack.last().unwrap() {
                St::Base => self.base(s),
                St::Comment | St::DocComment => {
                    let cls = if self.stack.last() == Some(&St::Comment) { "cm" } else { "sd" };
                    let st = *self.stack.last().unwrap();
                    if s.starts_with("/*") {
                        self.push(cls, 2, st);
                    } else if s.starts_with("*/") {
                        self.pop(cls, 2);
                    } else if s.starts_with(['*', '/']) {
                        self.emit(cls, 1);
                    } else {
                        self.emit(cls, s.find(['*', '/']).unwrap_or(s.len()));
                    }
                }
                St::Str => {
                    if s.starts_with('"') {
                        self.pop("s", 1);
                    } else if escape(s) > 0 {
                        self.emit("se", escape(s));
                    } else if s.starts_with('\\') {
                        self.emit("s", 1);
                    } else {
                        self.emit("s", s.find(['\\', '"']).unwrap_or(s.len()));
                    }
                }
                St::Attr => {
                    if s.starts_with('"') {
                        self.push("s", 1, St::Str);
                    } else if s.starts_with('[') {
                        self.push("cp", 1, St::Attr);
                    } else if s.starts_with(']') {
                        self.pop("cp", 1);
                    } else {
                        self.emit("cp", s.find(['"', '[', ']']).unwrap_or(s.len()));
                    }
                }
                St::Type => {
                    let (b, t, id) = (one_of(s, BUILTINS), one_of(s, TYPES), ident(s));
                    if ws(s) > 0 {
                        self.emit("w", ws(s));
                    } else if s.starts_with('&') {
                        self.emit("kp", 1);
                    } else if s.starts_with('\'') {
                        self.push("o", 1, St::Life);
                    } else if b > 0 {
                        self.emit("nb", b);
                    } else if t > 0 {
                        self.emit("kt", t);
                    } else if id > 0 {
                        self.pop("nc", id);
                    } else {
                        self.stack.pop();
                    }
                }
                st @ (St::Func | St::Mod) => {
                    if ws(s) > 0 {
                        self.emit("w", ws(s));
                    } else if ident(s) > 0 {
                        self.pop(if st == St::Func { "nf" } else { "nn" }, ident(s));
                    } else {
                        self.stack.pop();
                    }
                }
                St::Life => {
                    let n = s.find(|c: char| !(c.is_ascii_alphabetic() || c == '_')).unwrap_or(s.len());
                    if s.starts_with("static") || s.starts_with('_') {
                        self.emit("nb", if s.starts_with('_') { 1 } else { 6 });
                    } else if n > 0 {
                        self.emit("na", n + digits(&s[n..], word));
                    } else {
                        self.stack.pop();
                    }
                }
                St::Num => {
                    let suf = ["u8", "u16", "u32", "u64", "usize", "i8", "i16", "i32", "i64", "isize", "f32", "f64"]
                        .iter()
                        .find(|w| s.starts_with(**w));
                    match suf {
                        Some(w) => self.pop("k", w.len()),
                        None => {
                            self.stack.pop();
                        }
                    }
                }
            }
        }
        self.out
    }

    fn base(&mut self, s: &'a str) {
        let b = s.as_bytes();
        if s.starts_with('\n') {
            return self.emit("w", 1);
        }
        if ws(s) > 0 {
            return self.emit("w", ws(s));
        }
        if let Some(n) = s.strip_prefix("//!").and_then(to_eol) {
            return self.emit("sd", 3 + n);
        }
        if let Some(r) = s.strip_prefix("///") {
            if r.starts_with('\n') {
                return self.emit("sd", 4);
            }
            if !r.starts_with('/') {
                if let Some(n) = to_eol(r) {
                    return self.emit("sd", 3 + n);
                }
            }
        }
        if let Some(n) = s.strip_prefix("//").and_then(to_eol) {
            return self.emit("c1", 2 + n);
        }
        if s.starts_with("/**") && b.get(3).is_some_and(|&c| c == b'\n' || (c != b'/' && c != b'*')) {
            return self.push("sd", 4, St::DocComment);
        }
        if s.starts_with("/*!") {
            return self.push("sd", 3, St::DocComment);
        }
        if s.starts_with("/*") {
            return self.push("cm", 2, St::Comment);
        }
        if let Some(r) = s.strip_prefix('$') {
            let n = match ident(r) {
                0 if r.starts_with(['(', ')']) => 1 + r[1..].starts_with(',') as usize,
                0 => r.starts_with(',') as usize,
                n => n,
            };
            return self.emit("cp", 1 + n);
        }
        let n = one_of(s, KEYWORDS);
        if n > 0 {
            return self.emit("k", n);
        }
        let n = one_of(s, RESERVED);
        if n > 0 {
            return self.emit("kr", n);
        }
        let n = one_of(s, &["true", "false"]);
        if n > 0 {
            return self.emit("kc", n);
        }
        if one_of(s, &["self"]) > 0 {
            return self.emit("bp", 4);
        }
        if one_of(s, &["mod"]) > 0 {
            return self.push("k", 3, St::Mod);
        }
        if one_of(s, &["let"]) > 0 {
            return self.emit("kd", 3);
        }
        if one_of(s, &["fn"]) > 0 {
            return self.push("k", 2, St::Func);
        }
        let n = one_of(s, &["struct", "enum", "type", "union"]);
        if n > 0 {
            return self.push("k", n, St::Type);
        }
        if let Some(r) = s.strip_prefix("default") {
            let w = ws(r);
            let k = one_of(&r[w..], &["type", "fn"]);
            if w > 0 && k > 0 {
                self.emit("k", 7);
                self.emit("", w);
                return self.emit("k", k);
            }
        }
        let n = one_of(s, TYPES);
        if n > 0 {
            return self.emit("kt", n);
        }
        if one_of(s, &["self", "Self"]) > 0 {
            return self.emit("bp", 4);
        }
        let n = one_of(s, BUILTINS);
        if n > 0 {
            return self.emit("nb", n);
        }
        if let Some(m) = MACROS.iter().filter(|m| s.starts_with(**m) && s[m.len()..].starts_with('!')).max_by_key(|m| m.len()) {
            return self.emit("fm", m.len() + 1);
        }
        if s.starts_with("::") && s[2..].starts_with(word) {
            return self.emit("p", 2);
        }
        if s.starts_with("->") {
            return self.push("p", 2, St::Type);
        }
        if s.starts_with(':') {
            return self.push("p", 1, St::Type);
        }
        let n = one_of(s, &["break", "continue"]);
        if n > 0 {
            self.emit("k", n);
            let w = ws(&self.src[self.pos..]);
            self.emit("w", w);
            let r = &self.src[self.pos..];
            if r.starts_with('\'') && ident(&r[1..]) > 0 {
                self.emit("nl", 1 + ident(&r[1..]));
            }
            return;
        }
        if s.starts_with('\'') && char_lit(s) > 0 {
            return self.emit("sc", char_lit(s));
        }
        if s.starts_with("b'") && char_lit(&s[1..]) > 0 {
            return self.emit("sc", 1 + char_lit(&s[1..]));
        }
        for (pre, cls, ok) in [
            ("0b", "mb", (|c: char| c == '0' || c == '1' || c == '_') as fn(char) -> bool),
            ("0o", "mo", |c| ('0'..='7').contains(&c) || c == '_'),
            ("0x", "mh", |c| c.is_ascii_hexdigit() || c == '_'),
            ("0X", "mh", |c| c.is_ascii_hexdigit() || c == '_'),
        ] {
            if let Some(r) = s.strip_prefix(pre) {
                let n = digits(r, ok);
                if n > 0 {
                    return self.push(cls, 2 + n, St::Num);
                }
            }
        }
        if b[0].is_ascii_digit() {
            let (n, float) = number(s);
            return self.push(if float { "mf" } else { "mi" }, n, St::Num);
        }
        if s.starts_with("b\"") {
            return self.push("s", 2, St::Str); // ponytail: no bytestring escapes, none in the docs
        }
        if s.starts_with('"') {
            return self.push("s", 1, St::Str);
        }
        let r = s.strip_prefix('b').unwrap_or(s);
        if let Some(r2) = r.strip_prefix('r') {
            let hashes = digits(r2, |c| c == '#');
            if r2[hashes..].starts_with('"') {
                let close = format!("\"{}", &r2[..hashes]);
                let body = &r2[hashes + 1..];
                if let Some(i) = body.find(&close) {
                    return self.emit("s", s.len() - body.len() + i + close.len());
                }
            }
        }
        if s.starts_with('\'') {
            return self.push("o", 1, St::Life);
        }
        if s.starts_with("..") {
            return self.emit("o", 2 + s[2..].starts_with('=') as usize);
        }
        if s.starts_with(['{', '}', '(', ')', '[', ']', ',', '.', ';']) {
            return self.emit("p", 1);
        }
        if s.starts_with(['+', '-', '*', '/', '%', '&', '|', '<', '>', '^', '!', '~', '@', '=', ':', '?']) {
            return self.emit("o", 1);
        }
        let n = ident(s);
        if n > 0 {
            return self.emit("n", n);
        }
        if s.starts_with("#[") || s.starts_with("#![") {
            return self.push("cp", if b[1] == b'!' { 3 } else { 2 }, St::Attr);
        }
        if s.starts_with('#') {
            return self.emit("p", 1);
        }
        self.emit("err", s.chars().next().unwrap().len_utf8());
    }
}

pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Highlighted `<div class="hl"><pre>...</pre></div>`, as Pygments' HtmlFormatter writes it.
pub fn block(code: &str, lang: &str) -> String {
    let code = format!("{}\n", code.trim_matches('\n'));
    let tokens = if lang == "rust" {
        Lexer { src: &code, pos: 0, stack: vec![St::Base], out: vec![] }.run()
    } else {
        vec![("", code.as_str())]
    };
    // split at newlines, merge runs of one class, wrap each run in a span
    let mut out = String::from("<div class=\"hl\"><pre><span></span><code>");
    let mut run: Option<(&str, String)> = None;
    let flush = |out: &mut String, run: &mut Option<(&str, String)>| {
        if let Some((cls, text)) = run.take() {
            if cls.is_empty() {
                out.push_str(&esc(&text));
            } else {
                out.push_str(&format!("<span class=\"{cls}\">{}</span>", esc(&text)));
            }
        }
    };
    for (cls, text) in tokens {
        for (i, part) in text.split('\n').enumerate() {
            if i > 0 {
                flush(&mut out, &mut run);
                out.push('\n');
            }
            if part.is_empty() {
                continue;
            }
            match &mut run {
                Some((c, t)) if *c == cls => t.push_str(part),
                _ => {
                    flush(&mut out, &mut run);
                    run = Some((cls, part.to_string()));
                }
            }
        }
    }
    flush(&mut out, &mut run);
    out.push_str("</code></pre></div>");
    out
}
