//! The Inertia lab (`/lab`), one test per experiment.

mod common;

use common::Client;
use route::http::{Method, StatusCode};
use serde_json::json;

#[tokio::test]
async fn shared_props_on_every_page() {
    let mut c = Client::new().await;
    let r = c.visit("/lab", &[]).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.headers["x-inertia"], "true");
    let props = r.props();
    assert_eq!(props["app_name"], "cloudsandbox");
    assert!(props["csrf_token"].as_str().is_some_and(|t| t.len() > 20));
    assert_eq!(props["flash"]["success"], json!(null));
    assert_eq!(r.page()["component"], "lab/index");
}

#[tokio::test]
async fn first_visit_is_html_with_the_page_embedded() {
    let mut c = Client::new().await;
    let r = c.request(Method::GET, "/lab", &[], None).await;
    assert!(r.headers["content-type"].to_str().unwrap().starts_with("text/html"));
    assert!(r.body.contains(r#"<script data-page="app" type="application/json">{"component":"lab/index""#));
}

#[tokio::test]
async fn partial_reload_sends_only_the_asked_prop() {
    let mut c = Client::new().await;
    let r = c.reload("/lab/basics", "lab/basics", "server_time", &[]).await;
    let mut keys: Vec<String> = r.props().as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, ["errors", "server_time"]);
}

#[tokio::test]
async fn optional_and_deferred_props() {
    let mut c = Client::new().await;
    let r = c.visit("/lab/lazy", &[]).await;
    let page = r.page();
    assert!(page["props"].get("stats").is_none() && page["props"].get("report").is_none());
    assert_eq!(page["deferredProps"], json!({ "default": ["report"], "activity": ["activity"] }));

    let r = c.reload("/lab/lazy", "lab/lazy", "stats", &[]).await;
    assert_eq!(r.props()["stats"]["users"], 42);
    let r = c.reload("/lab/lazy", "lab/lazy", "report", &[]).await;
    assert_eq!(r.props()["report"][0]["name"], "builds");
}

#[tokio::test]
async fn merge_props_append_and_reset() {
    let mut c = Client::new().await;
    let r = c.reload("/lab/merge?page=2", "lab/merge", "items,page,has_more", &[]).await;
    let page = r.page();
    assert_eq!(page["props"]["items"][0]["id"], 6);
    assert_eq!(page["mergeProps"], json!(["items"]));
    assert_eq!(page["matchPropsOn"], json!(["items.id"]));

    let r = c.reload("/lab/merge?page=1", "lab/merge", "items,page,has_more", &[("x-inertia-reset", "items")]).await;
    assert!(r.page().get("mergeProps").is_none(), "a reset list is replaced, not merged");
}

#[tokio::test]
async fn once_props_are_skipped_when_remembered() {
    let mut c = Client::new().await;
    let r = c.visit("/lab/once", &[]).await;
    assert!(r.props()["plans"].is_array());
    assert_eq!(r.page()["onceProps"]["plans"], json!({ "prop": "plans", "expiresAt": null }));
    let loads = r.props()["plan_loads"].as_u64().unwrap();

    let r = c.visit("/lab/once/again", &[("x-inertia-except-once-props", "plans")]).await;
    assert!(r.props().get("plans").is_none(), "the client already has it");
    assert!(r.page()["onceProps"].get("plans").is_some(), "still announced");
    assert_eq!(r.props()["plan_loads"].as_u64().unwrap(), loads, "not computed again");
}

#[tokio::test]
async fn validation_errors_redirect_back_with_props_errors() {
    let mut c = Client::new().await;
    c.visit("/lab/form", &[]).await;
    let r = c.send(Method::POST, "/lab/form/contact", json!({ "name": "", "email": "x", "message": "short" }), &[("referer", "/lab/form")]).await;
    assert_eq!((r.status, r.location()), (StatusCode::SEE_OTHER, "/lab/form"));
    let errors = c.visit("/lab/form", &[]).await.props()["errors"].clone();
    assert_eq!(errors["name"], "This field is required.");
    assert_eq!(errors["email"], "Must be a valid email address.");
    assert_eq!(errors["message"], "Must be at least 10 characters.");
}

#[tokio::test]
async fn error_bags_keep_forms_apart() {
    let mut c = Client::new().await;
    c.visit("/lab/form", &[]).await;
    c.send(Method::POST, "/lab/form/newsletter", json!({ "email": "nope" }), &[("referer", "/lab/form"), ("x-inertia-error-bag", "newsletter")]).await;
    let errors = c.visit("/lab/form", &[]).await.props()["errors"].clone();
    assert_eq!(errors, json!({ "newsletter": { "email": "Must be a valid email address." } }));
}

#[tokio::test]
async fn success_flashes_for_one_request() {
    let mut c = Client::new().await;
    c.visit("/lab/form", &[]).await;
    let r = c.send(Method::POST, "/lab/form/contact", json!({ "name": "Ada", "email": "ada@example.com", "message": "Hello from the lab" }), &[]).await;
    assert_eq!(r.status, StatusCode::SEE_OTHER);
    assert_eq!(c.visit("/lab/form", &[]).await.props()["flash"]["success"], "Thanks Ada, your message was received.");
    assert_eq!(c.visit("/lab/form", &[]).await.props()["flash"]["success"], json!(null), "gone after one request");
}

#[tokio::test]
async fn plain_html_form_with_csrf_field() {
    let mut c = Client::new().await;
    let token = c.visit("/lab/form", &[]).await.props()["csrf_token"].as_str().unwrap().to_owned();
    let form = |t: &str| Some(("application/x-www-form-urlencoded", format!("_token={t}&name=Grace")));
    assert_eq!(c.request(Method::POST, "/lab/form/plain", &[], form("wrong")).await.status.as_u16(), 419);
    assert_eq!(c.request(Method::POST, "/lab/form/plain", &[], form(&token)).await.status, StatusCode::SEE_OTHER);
    assert_eq!(c.visit("/lab/form", &[]).await.props()["flash"]["success"], "Plain form posted by Grace (CSRF via _token).");
}

#[tokio::test]
async fn unsafe_methods_redirect_with_303() {
    let mut c = Client::new().await;
    c.visit("/lab/redirects", &[]).await;
    for method in [Method::PUT, Method::PATCH, Method::DELETE] {
        let r = c.send(method.clone(), "/lab/redirects", json!({}), &[]).await;
        assert_eq!((r.status, r.location()), (StatusCode::SEE_OTHER, "/lab/redirects"));
        let flash = c.visit("/lab/redirects", &[]).await.props()["flash"]["success"].as_str().unwrap().to_owned();
        assert!(flash.starts_with(method.as_str()), "{flash}");
    }
}

#[tokio::test]
async fn missing_component_is_an_error_in_dev() {
    let mut c = Client::new().await;
    assert_eq!(c.visit("/lab/broken", &[]).await.status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn stale_asset_version_forces_a_full_reload() {
    let mut c = Client::new().await;
    let r = c.request(Method::GET, "/lab/basics", &[("x-inertia", "true"), ("x-inertia-version", "old-build")], None).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.headers["x-inertia-location"], "/lab/basics");
}
