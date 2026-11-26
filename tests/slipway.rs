//! Slipway: each page's props and the Inertia feature it relies on.

mod common;

use std::time::Duration;

use common::Client;
use route::http::{Method, StatusCode};
use serde_json::{Value, json};

fn len(v: &Value) -> usize {
    v.as_array().map_or(0, Vec::len)
}

fn names(v: &Value, key: &str) -> Vec<String> {
    v.as_array().unwrap().iter().map(|s| s[key].as_str().unwrap().to_owned()).collect()
}

#[tokio::test]
async fn shared_props_and_history_encryption() {
    let mut c = Client::new().await;
    let r = c.visit("/", &[]).await;
    assert_eq!(r.status, StatusCode::OK);
    let page = r.page();
    assert_eq!(page["component"], "Overview");
    assert_eq!(page["encryptHistory"], true, "Slipway pages are encrypted in history");
    let props = &page["props"];
    assert_eq!(props["auth"]["user"], json!({ "id": 1, "name": "Elizabeth Lane", "email": "elizabeth@acme.test", "avatar": null }));
    assert_eq!(props["team"], json!({ "id": 1, "name": "Acme Labs" }));
    assert_eq!(len(&props["teams"]), 2);
    assert_eq!(props["flash"], json!({ "success": null, "error": null }));
}

#[tokio::test(start_paused = true)]
async fn overview_tiles_and_deferred_usage() {
    let mut c = Client::new().await;
    let page = c.visit("/", &[]).await.page();
    let props = &page["props"];
    assert_eq!(props["summary"], "6 servers across 3 providers");

    let tiles = &props["tiles"];
    assert_eq!(names(tiles, "label"), ["Servers", "Sites", "Deploys today", "Failed today"]);
    assert_eq!((tiles[0]["value"].as_u64(), tiles[0]["delta"]["text"].as_str()), (Some(6), Some("+1 this week")));
    assert_eq!(tiles[0]["note"], json!({ "text": "5 healthy, 1 degraded", "tone": "warn", "at": null }));
    assert_eq!((tiles[1]["value"].as_u64(), tiles[1]["delta"]["text"].as_str()), (Some(14), Some("+2 this week")));
    assert_eq!((tiles[2]["value"].as_u64(), tiles[2]["delta"]["text"].as_str()), (Some(23), Some("+38% vs avg")));
    assert!(tiles[2]["note"]["text"].as_str().unwrap().starts_with("Median "));
    assert_eq!((tiles[3]["value"].as_u64(), tiles[3]["delta"]["text"].as_str()), (Some(1), Some("-2 vs yesterday")));
    assert_eq!((tiles[3]["note"]["text"].as_str(), tiles[3]["note"]["tone"].as_str()), (Some("api.acme.test"), Some("bad")));
    assert!(tiles.as_array().unwrap().iter().all(|t| len(&t["series"]) == 7));

    assert_eq!(names(&props["recent"], "commit"), ["a41c9e2", "7be02d1", "c09f3aa", "1f6d8b4", "e2a7715"]);
    assert_eq!(props["recent"][0]["status"], "running");

    // Two deferred groups: four servers arrive first, two from a slower source.
    assert!(props.get("usage").is_none() && props.get("more_usage").is_none());
    assert_eq!(page["deferredProps"], json!({ "default": ["usage"], "slow": ["more_usage"] }));
    let usage = c.reload("/", "Overview", "usage", &[]).await.props()["usage"].clone();
    assert_eq!(names(&usage, "name"), ["web-prod-01", "web-prod-02", "worker-01", "db-primary"]);
    assert_eq!(len(&usage[0]["trend"]), 9);
    let more = c.reload("/", "Overview", "more_usage", &[]).await.props()["more_usage"].clone();
    assert_eq!(names(&more, "name"), ["staging-01", "edge-cache-01"]);
    assert_eq!((more[1]["cpu"].clone(), len(&more[1]["trend"])), (json!(null), 0), "a provisioning server has no metrics yet");
}

#[tokio::test]
async fn servers_filter_sort_and_paginate() {
    let mut c = Client::new().await;
    let props = c.visit("/servers", &[]).await.props();
    assert_eq!(props["servers"]["meta"], json!({ "page": 1, "perPage": 10, "total": 6 }));
    assert_eq!(names(&props["servers"]["data"], "name"), ["db-primary", "edge-cache-01", "staging-01", "web-prod-01", "web-prod-02", "worker-01"]);
    assert_eq!(props["providers"], json!(["AWS", "DigitalOcean", "Hetzner"]));

    // The search box: a partial reload of the table and filters only.
    let r = c.reload("/servers?q=prod", "servers/Index", "servers,filters", &[]).await;
    let mut keys: Vec<String> = r.props().as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, ["errors", "filters", "servers"]);
    assert_eq!(names(&r.props()["servers"]["data"], "name"), ["web-prod-01", "web-prod-02"]);
    assert_eq!(r.props()["filters"]["q"], "prod");

    let props = c.visit("/servers?status=degraded", &[]).await.props();
    assert_eq!(names(&props["servers"]["data"], "name"), ["worker-01"]);
    let props = c.visit("/servers?provider=Hetzner", &[]).await.props();
    assert_eq!(names(&props["servers"]["data"], "name"), ["db-primary", "worker-01"]);

    let props = c.visit("/servers?sort=cpu&dir=desc", &[]).await.props();
    assert_eq!(names(&props["servers"]["data"], "name"), ["worker-01", "web-prod-01", "web-prod-02", "db-primary", "staging-01", "edge-cache-01"], "no metrics sorts last");
}

#[tokio::test(start_paused = true)]
async fn server_tabs_are_pages_with_lazy_props() {
    let mut c = Client::new().await;
    let r = c.visit("/servers/1", &[]).await;
    assert_eq!((r.status, r.location()), (StatusCode::SEE_OTHER, "/servers/1/metrics"));

    let page = c.visit("/servers/1/metrics", &[]).await.page();
    assert_eq!((page["component"].as_str(), page["props"]["server"]["name"].as_str()), (Some("servers/Metrics"), Some("web-prod-01")));
    assert_eq!((page["props"]["server"]["vcpus"].as_u64(), page["props"]["server"]["memoryGb"].as_u64()), (Some(4), Some(8)));
    assert_eq!(page["deferredProps"], json!({ "default": ["history"] }));
    let history = c.reload("/servers/1/metrics", "servers/Metrics", "history", &[]).await.props()["history"].clone();
    for series in ["cpu", "memory", "disk", "load", "bandwidth"] {
        assert_eq!(len(&history[series]), 60, "{series}");
    }

    let sites = c.visit("/servers/1/sites", &[]).await.props()["sites"].clone();
    assert_eq!(names(&sites, "domain"), ["shop.acme.test", "api.acme.test", "docs.acme.test", "status.acme.test"]);
    assert_eq!(len(&c.visit("/servers/1/scheduler", &[]).await.props()["tasks"]), 3);
    assert_eq!(len(&c.visit("/servers/1/workers", &[]).await.props()["workers"]), 3);
    assert_eq!(c.visit("/servers/1/logs", &[]).await.page()["deferredProps"], json!({ "default": ["lines"] }));
    assert_eq!(len(&c.reload("/servers/1/logs", "servers/Logs", "lines", &[]).await.props()["lines"]), 40);
}

#[tokio::test]
async fn other_teams_servers_are_not_found() {
    let mut c = Client::new().await;
    let r = c.visit("/servers/7/metrics", &[]).await; // server 7 belongs to Lane Studio
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!((r.page()["component"].as_str(), r.props()["status"].as_u64()), (Some("errors/NotFound"), Some(404)));
}

#[tokio::test]
async fn restart_flashes_and_logs_activity() {
    let mut c = Client::new().await;
    c.visit("/servers/1/metrics", &[]).await;
    let r = c.send(Method::POST, "/servers/1/reboot", json!({}), &[]).await;
    assert_eq!((r.status, r.location()), (StatusCode::SEE_OTHER, "/servers/1/metrics"));
    let latest = c.visit("/activity", &[]).await.props()["days"][0]["entries"][0].clone();
    assert_eq!((latest["actor"].as_str(), latest["action"].as_str(), latest["target"].as_str()), (Some("Elizabeth Lane"), Some("restarted"), Some("web-prod-01")));
}

#[tokio::test]
async fn sites_list_with_their_server() {
    let mut c = Client::new().await;
    let r = c.visit("/sites", &[]).await;
    assert_eq!(r.page()["component"], "sites/Index");
    let sites = r.props()["sites"].clone();
    assert_eq!(len(&sites), 14);
    assert_eq!((sites[0]["domain"].as_str(), sites[0]["server"].as_str()), (Some("shop.acme.test"), Some("web-prod-01")));
}

const CERT: &str = "-----BEGIN CERTIFICATE-----\nMIIBfake\n-----END CERTIFICATE-----";

#[tokio::test]
async fn site_form_validates_every_field() {
    let mut c = Client::new().await;
    let r = c.visit("/sites/create", &[]).await;
    assert_eq!(r.page()["component"], "sites/Create");
    let props = r.props();
    assert_eq!(props["site"], json!(null));
    assert_eq!(len(&props["servers"]), 6);
    assert_eq!(props["runtimes"], json!(["Rust binary", "Node.js 22", "Python 3.12", "Static site"]));

    let bad = [
        ("serverId", "999"),
        ("domain", "not a domain"),
        ("repository", "nope"),
        ("branch", "has space"),
        ("runtime", "COBOL"),
        ("env[0][key]", "lower"),
        ("env[1][key]", "SAME"),
        ("env[2][key]", "SAME"),
    ];
    let r = c.send_multipart(Method::POST, "/sites", &bad, Some(("certificate", "cert.txt", "not a cert"))).await;
    assert_eq!(r.status, StatusCode::SEE_OTHER, "an Inertia visit is redirected back");
    let errors = c.visit("/sites/create", &[]).await.props()["errors"].clone();
    assert_eq!(errors["serverId"], "Pick a server.");
    assert_eq!(errors["domain"], "Must be a domain like app.example.com.");
    assert_eq!(errors["repository"], "Must look like owner/name.");
    assert_eq!(errors["branch"], "No spaces.");
    assert_eq!(errors["runtime"], "Must be one of the allowed values.");
    assert_eq!(errors["env.0.key"], "Use UPPER_SNAKE_CASE.");
    assert_eq!(errors["env.2.key"], "Duplicate key.");
    assert_eq!(errors["certificate"], "Must be a PEM certificate.");

    let taken = [("serverId", "1"), ("domain", "help.acme.test"), ("repository", "acme/help"), ("branch", "main"), ("runtime", "Rust binary")];
    c.send_multipart(Method::POST, "/sites", &taken, None).await;
    assert_eq!(c.visit("/sites/create", &[]).await.props()["errors"]["domain"], "This domain is already used by another site on web-prod-02.");
}

#[tokio::test]
async fn site_create_and_edit_with_certificate_upload() {
    let mut c = Client::new().await;
    c.visit("/sites/create", &[]).await;
    let fields = [
        ("serverId", "2"),
        ("domain", "Ledger.Acme.test"),
        ("repository", "acme/ledger"),
        ("branch", "main"),
        ("runtime", "Rust binary"),
        ("env[0][key]", "DATABASE_URL"),
        ("env[0][value]", "postgres://db/ledger"),
    ];
    let r = c.send_multipart(Method::POST, "/sites", &fields, Some(("certificate", "ledger.pem", CERT))).await;
    assert_eq!((r.status, r.location()), (StatusCode::SEE_OTHER, "/servers/2/sites"));

    let props = c.visit("/servers/2/sites", &[]).await.props();
    assert_eq!(props["flash"]["success"], "ledger.acme.test saved.");
    let id = props["sites"].as_array().unwrap().iter().find(|s| s["domain"] == "ledger.acme.test").expect("created, lowercased")["id"].as_u64().unwrap();

    let r = c.visit(&format!("/sites/{id}/edit"), &[]).await;
    assert_eq!(r.page()["component"], "sites/Edit");
    assert_eq!(r.props()["site"]["env"], json!([{ "key": "DATABASE_URL", "value": "postgres://db/ledger" }]));
    assert_eq!(r.props()["site"]["hasCertificate"], true);

    let edited = [("serverId", "2"), ("domain", "ledger.acme.test"), ("repository", "acme/ledger"), ("branch", "release"), ("runtime", "Node.js 22")];
    assert_eq!(c.send_multipart(Method::PUT, &format!("/sites/{id}"), &edited, None).await.status, StatusCode::SEE_OTHER);
    let site = c.visit(&format!("/sites/{id}/edit"), &[]).await.props()["site"].clone();
    assert_eq!((site["branch"].as_str(), site["runtime"].as_str()), (Some("release"), Some("Node.js 22")));
    assert_eq!(site["hasCertificate"], true, "an edit without a file keeps the certificate");
}

#[tokio::test]
async fn deployments_scroll_page_by_page() {
    let mut c = Client::new().await;
    let page = c.visit("/deployments", &[]).await.page();
    assert_eq!(page["component"], "deployments/Index");
    assert_eq!(len(&page["props"]["deployments"]), 15);
    assert_eq!(page["props"]["deployments"][0]["commit"], "a41c9e2");
    assert_eq!(page["scrollProps"]["deployments"], json!({ "pageName": "page", "previousPage": null, "nextPage": 2, "currentPage": 1, "reset": false }));
    assert!(page["props"].get("log").is_none(), "the log is optional");

    // What <InfiniteScroll> sends for the next page.
    let r = c.reload("/deployments?page=2", "deployments/Index", "deployments", &[("x-inertia-infinite-scroll-merge-intent", "append")]).await;
    let page = r.page();
    assert_eq!((page["mergeProps"].clone(), page["matchPropsOn"].clone()), (json!(["deployments"]), json!(["deployments.id"])));
    assert_eq!(page["scrollProps"]["deployments"]["previousPage"], 1);

    // The failed deploy's log, expanded.
    let failed = c.visit("/deployments", &[]).await.props()["deployments"].as_array().unwrap().iter().find(|d| d["commit"] == "1f6d8b4").unwrap().clone();
    let log = c.reload(&format!("/deployments?log={}", failed["id"]), "deployments/Index", "log", &[]).await.props()["log"].clone();
    assert_eq!(log["lines"].as_array().unwrap().last().unwrap(), "error: relation \"jobs\" already exists");
}

#[tokio::test(start_paused = true)]
async fn deploy_runs_through_its_states_and_can_be_cancelled() {
    let mut c = Client::new().await;
    c.visit("/deployments", &[]).await;

    let r = c.send(Method::POST, "/deployments", json!({ "siteId": 1 }), &[]).await;
    assert_eq!((r.status, r.location()), (StatusCode::SEE_OTHER, "/deployments"));
    let newest = c.visit("/deployments", &[]).await.props()["deployments"][0].clone();
    assert_eq!((newest["site"].as_str(), newest["status"].as_str(), newest["author"].as_str()), (Some("shop.acme.test"), Some("queued"), Some("Elizabeth Lane")));

    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(c.visit("/deployments", &[]).await.props()["deployments"][0]["status"], "running");
    tokio::time::sleep(Duration::from_secs(10)).await;
    let done = c.visit("/deployments", &[]).await.props()["deployments"][0].clone();
    assert!(["finished", "failed"].contains(&done["status"].as_str().unwrap()), "{done}");
    assert!(done["durationMs"].is_u64());

    // A finished deploy cannot be cancelled: the client's optimistic change rolls back.
    let id = done["id"].as_u64().unwrap();
    c.send(Method::POST, &format!("/deployments/{id}/cancel"), json!({}), &[("referer", "/deployments")]).await;
    assert_eq!(c.visit("/deployments", &[]).await.props()["errors"]["deployment"], "Only queued or running deployments can be cancelled.");

    c.send(Method::POST, "/deployments", json!({ "siteId": 2 }), &[]).await;
    let id = c.visit("/deployments", &[]).await.props()["deployments"][0]["id"].as_u64().unwrap();
    assert_eq!(c.send(Method::POST, &format!("/deployments/{id}/cancel"), json!({}), &[]).await.status, StatusCode::SEE_OTHER);
    tokio::time::sleep(Duration::from_secs(10)).await;
    assert_eq!(c.visit("/deployments", &[]).await.props()["deployments"][0]["status"], "cancelled", "the simulation stops");
}

#[tokio::test]
async fn deploying_a_suspended_site_fails_validation() {
    let mut c = Client::new().await;
    c.visit("/deployments", &[]).await;
    let sandbox = c.visit("/deployments", &[]).await.props()["sites"].as_array().unwrap().iter().find(|s| s["status"] == "suspended").unwrap()["id"].clone();
    c.send(Method::POST, "/deployments", json!({ "siteId": sandbox }), &[("referer", "/deployments")]).await;
    assert_eq!(c.visit("/deployments", &[]).await.props()["errors"]["siteId"], "sandbox.acme.test is suspended; deploys are blocked.");
}

#[tokio::test]
async fn activity_loads_older_days_on_request() {
    let mut c = Client::new().await;
    let r = c.visit("/activity", &[]).await;
    assert_eq!(r.page()["component"], "Activity");
    let props = r.props();
    assert_eq!(len(&props["days"]), 3);
    assert_eq!((props["days"][0]["label"].as_str(), props["days"][1]["label"].as_str()), (Some("Today"), Some("Yesterday")));
    assert_eq!(props["days"][1]["entries"][0]["target"], "sam@acme.test");
    assert!(props.get("older").is_none(), "loaded when visible, not up front");

    let older = c.reload("/activity", "Activity", "older", &[]).await.props()["older"].clone();
    assert!(len(&older) > 5);
}

#[tokio::test]
async fn settings_profile_members_and_tokens() {
    let mut c = Client::new().await;
    let r = c.visit("/settings", &[]).await;
    assert_eq!((r.status, r.location()), (StatusCode::SEE_OTHER, "/settings/profile"));
    let r = c.visit("/settings/profile", &[]).await;
    assert_eq!(r.page()["component"], "settings/Profile");
    let props = r.props();
    assert_eq!(names(&props["members"], "name"), ["Elizabeth Lane", "Tomas Vidal", "Imani Okoro"]);
    assert_eq!(names(&props["tokens"], "name"), ["ci-deploy", "grafana-read"]);

    c.send(Method::PUT, "/settings/profile", json!({ "name": "", "email": "nope" }), &[("referer", "/settings/profile")]).await;
    let errors = c.visit("/settings/profile", &[]).await.props()["errors"].clone();
    assert_eq!((errors["name"].as_str(), errors["email"].as_str()), (Some("This field is required."), Some("Must be a valid email address.")));

    c.send(Method::PUT, "/settings/profile", json!({ "name": "Liz Lane", "email": "liz@acme.test" }), &[]).await;
    let props = c.visit("/settings/profile", &[]).await.props();
    assert_eq!(props["auth"]["user"]["name"], "Liz Lane", "shared props reflect the change");
    assert_eq!(props["flash"]["success"], "Profile saved.");

    c.send(Method::PATCH, "/settings/members/3", json!({ "role": "admin" }), &[]).await;
    assert_eq!(c.visit("/settings/profile", &[]).await.props()["members"][2]["role"], "admin");
    c.send(Method::DELETE, "/settings/members/1", json!({}), &[("referer", "/settings/profile")]).await;
    assert_eq!(c.visit("/settings/profile", &[]).await.props()["errors"]["member"], "The owner cannot be removed.");
    c.send(Method::DELETE, "/settings/members/3", json!({}), &[]).await;
    assert_eq!(len(&c.visit("/settings/profile", &[]).await.props()["members"]), 2);

    c.send(Method::POST, "/settings/tokens", json!({ "name": "deploy-bot" }), &[]).await;
    let props = c.visit("/settings/profile", &[]).await.props();
    assert!(props["flash"]["success"].as_str().unwrap().starts_with("Token created: slp_"));
    let id = props["tokens"].as_array().unwrap().iter().find(|t| t["name"] == "deploy-bot").unwrap()["id"].as_u64().unwrap();
    c.send(Method::DELETE, &format!("/settings/tokens/{id}"), json!({}), &[]).await;
    assert_eq!(len(&c.visit("/settings/profile", &[]).await.props()["tokens"]), 2);
}

#[tokio::test]
async fn switching_teams_scopes_everything() {
    let mut c = Client::new().await;
    c.visit("/", &[]).await;
    let r = c.send(Method::POST, "/teams/2", json!({}), &[]).await;
    assert_eq!((r.status, r.location()), (StatusCode::SEE_OTHER, "/"));
    let props = c.visit("/", &[]).await.props();
    assert_eq!(props["team"]["name"], "Lane Studio");
    assert_eq!(props["tiles"][0]["value"], 2);
    assert_eq!(c.visit("/servers", &[]).await.props()["servers"]["meta"]["total"], 2);
}

#[tokio::test]
async fn sign_out_everywhere_clears_history_on_the_next_page() {
    let mut c = Client::new().await;
    c.visit("/settings/profile", &[]).await;
    let r = c.send(Method::POST, "/logout", json!({}), &[]).await;
    assert_eq!((r.status, r.location()), (StatusCode::SEE_OTHER, "/"));
    assert_eq!(c.visit("/", &[]).await.page()["clearHistory"], true);
    assert!(c.visit("/", &[]).await.page().get("clearHistory").is_none(), "only once");
}

#[tokio::test]
async fn palette_lists_the_teams_servers_and_sites() {
    let mut c = Client::new().await;
    let r = c.request(Method::GET, "/palette", &[("accept", "application/json")], None).await;
    let items: Value = serde_json::from_str(&r.body).unwrap();
    let items = items.as_array().unwrap();
    assert_eq!(items.iter().filter(|i| i["kind"] == "server").count(), 6);
    let shop = items.iter().find(|i| i["label"] == "shop.acme.test").unwrap();
    assert_eq!(shop["hint"], "on web-prod-01");
}

#[tokio::test]
async fn error_pages_are_inertia_pages_with_their_status() {
    let mut c = Client::new().await;
    for status in [403, 419, 500, 503] {
        let r = c.visit(&format!("/errors/{status}"), &[]).await;
        assert_eq!(r.status.as_u16(), status);
        assert_eq!((r.page()["component"].as_str(), r.props()["status"].as_u64()), (Some("errors/Error"), Some(status as u64)));
    }
    // Unknown URLs, past the static files, render the 404 page.
    let r = c.visit("/no/such/page", &[]).await;
    assert_eq!((r.status, r.page()["component"].as_str()), (StatusCode::NOT_FOUND, Some("errors/NotFound")));
    assert_eq!(c.visit("/errors/999", &[]).await.status, StatusCode::NOT_FOUND);
}
