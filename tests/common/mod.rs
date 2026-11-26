//! A test client for the real HTTP stack (`cloudsandbox::router`): in-memory
//! sessions, a Postgres pool that never connects, a cookie jar, and the
//! Inertia headers a browser would send.

#![allow(dead_code)]

use std::sync::Arc;

use cloudsandbox::{config::Config, state::App};
use db::Db;
use event::Events;
use http_body_util::BodyExt;
use queue::Queue;
use route::{
    Routes, body,
    http::{HeaderMap, Method, Request, StatusCode, header},
};
use securecookie::{Codec, Keys};
use serde_json::Value;
use session::MemoryStore;
use vite::Vite;

pub fn routes() -> Routes {
    let key = Keys::generate();
    let cfg = Config {
        app_name: "cloudsandbox".into(),
        app_key: key.clone(),
        database_url: "postgres://unused@127.0.0.1:1/unused".into(),
        ssr_url: None,
    };
    // Lazy: handlers under test never query the database.
    let pool = sqlx::postgres::PgPoolOptions::new().connect_lazy(&cfg.database_url).unwrap();
    let queue = Queue::new(queue::PostgresStore::new(pool.clone()));
    let events = Arc::new(Events::builder().build());
    let app = App::new(cfg, Db::from_pool(pool), queue, events);
    let vite = Vite::load("public", &["resources/js/app.ts"]).unwrap();
    cloudsandbox::router(&app, MemoryStore::new(), Codec::new(Keys::parse(&key).unwrap()), vite).unwrap()
}

pub struct Response {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: String,
}

impl Response {
    /// The Inertia page object.
    pub fn page(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or_else(|e| panic!("not a page ({e}): {}", self.body))
    }

    pub fn props(&self) -> Value {
        self.page()["props"].clone()
    }

    pub fn location(&self) -> &str {
        self.headers.get(header::LOCATION).map_or("", |v| v.to_str().unwrap())
    }
}

/// A browser: keeps cookies, sends Inertia and XSRF headers.
pub struct Client {
    routes: Routes,
    jar: Vec<(String, String)>,
    version: String,
}

impl Client {
    pub async fn new() -> Self {
        let mut c = Client { routes: routes(), jar: vec![], version: String::new() };
        // A visit with a stale version is answered with the current one.
        let r = c.request(Method::GET, "/", &[("x-inertia", "true"), ("x-inertia-version", "stale")], None).await;
        c.version = r.headers.get("x-inertia-version").map(|v| v.to_str().unwrap().to_owned()).unwrap_or_default();
        c
    }

    pub async fn request(&mut self, method: Method, uri: &str, headers: &[(&str, &str)], content: Option<(&str, String)>) -> Response {
        let mut req = Request::builder().method(method).uri(uri);
        if !self.jar.is_empty() {
            let cookies: Vec<String> = self.jar.iter().map(|(k, v)| format!("{k}={v}")).collect();
            req = req.header(header::COOKIE, cookies.join("; "));
        }
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        let body = match content {
            Some((content_type, b)) => {
                req = req.header(header::CONTENT_TYPE, content_type);
                body(b)
            }
            None => route::empty(),
        };
        let res = self.routes.handle(req.body(body).unwrap()).await;
        for v in res.headers().get_all(header::SET_COOKIE) {
            let (k, v) = v.to_str().unwrap().split(';').next().unwrap().split_once('=').unwrap();
            self.jar.retain(|(name, _)| name != k);
            self.jar.push((k.to_owned(), v.to_owned()));
        }
        let status = res.status();
        let headers = res.headers().clone();
        let body = String::from_utf8(res.into_body().collect().await.unwrap().to_bytes().to_vec()).unwrap();
        Response { status, headers, body }
    }

    fn xsrf(&self) -> String {
        self.jar.iter().find(|(k, _)| k == "XSRF-TOKEN").map(|(_, v)| v.clone()).unwrap_or_default()
    }

    /// An Inertia visit (GET), with optional extra headers such as partial-reload ones.
    pub async fn visit(&mut self, uri: &str, extra: &[(&str, &str)]) -> Response {
        let version = self.version.clone();
        let mut headers = vec![("x-inertia", "true"), ("x-inertia-version", version.as_str())];
        headers.extend_from_slice(extra);
        self.request(Method::GET, uri, &headers, None).await
    }

    /// A partial reload of `component` asking for `only` (comma-separated).
    pub async fn reload(&mut self, uri: &str, component: &str, only: &str, extra: &[(&str, &str)]) -> Response {
        let mut headers = vec![("x-inertia-partial-component", component), ("x-inertia-partial-data", only)];
        headers.extend_from_slice(extra);
        self.visit(uri, &headers).await
    }

    /// An Inertia form submission (what `useForm` and `router.post` send).
    pub async fn send(&mut self, method: Method, uri: &str, json: Value, extra: &[(&str, &str)]) -> Response {
        let (version, xsrf) = (self.version.clone(), self.xsrf());
        let mut headers = vec![("x-inertia", "true"), ("x-inertia-version", version.as_str()), ("x-xsrf-token", xsrf.as_str())];
        headers.extend_from_slice(extra);
        self.request(method, uri, &headers, Some(("application/json", json.to_string()))).await
    }
}
