//! Slipway (`/slipway`): each page's props and the Inertia feature it relies on.

mod common;

use std::time::Duration;

use common::Client;
use route::http::{Method, StatusCode};
use serde_json::{Value, json};

fn len(v: &Value) -> usize {
    v.as_array().map_or(0, Vec::len)
}

#[tokio::test]
async fn shared_props_and_history_encryption() {
    let mut c = Client::new().await;
    let r = c.visit("/slipway", &[]).await;
    assert_eq!(r.status, StatusCode::OK);
    let page = r.page();
    assert_eq!(page["component"], "slipway/overview");
    assert_eq!(page["encryptHistory"], true, "Slipway pages are encrypted in history");
    let props = &page["props"];
    assert_eq!(props["auth"]["user"]["name"], "Ada Lovelace");
    assert_eq!(props["team"], json!({ "id": 1, "name": "Acme Platform" }));
    assert_eq!(len(&props["teams"]), 2);
    assert_eq!(props["flash"], json!({ "success": null, "error": null }));
}

#[tokio::test(start_paused = true)]
async fn overview_defers_sparklines_for_polling() {
    let mut c = Client::new().await;
    let page = c.visit("/slipway", &[]).await.page();
    assert_eq!(page["props"]["tiles"]["servers"], 18);
    assert_eq!(page["props"]["tiles"]["sites"].as_u64().unwrap() > 0, true);
    assert_eq!(len(&page["props"]["recent"]), 8);
    assert!(page["props"].get("sparklines").is_none());
    assert_eq!(page["deferredProps"], json!({ "default": ["sparklines"] }));

    // What both the deferred fetch and every poll send.
    let r = c.reload("/slipway", "slipway/overview", "sparklines", &[]).await;
    let sparks = r.props()["sparklines"].clone();
    assert!(len(&sparks) > 0);
    assert_eq!(len(&sparks[0]["cpu"]), 24);
    assert!(sparks[0]["cpu"].as_array().unwrap().iter().all(|v| (1..=99).contains(&v.as_u64().unwrap())));
}

#[tokio::test]
async fn servers_filter_sort_and_paginate() {
    let mut c = Client::new().await;
    let props = c.visit("/slipway/servers", &[]).await.props();
    assert_eq!(props["servers"]["meta"], json!({ "page": 1, "perPage": 10, "total": 18 }));
    assert_eq!(len(&props["servers"]["data"]), 10);
    let names: Vec<&str> = props["servers"]["data"].as_array().unwrap().iter().map(|s| s["name"].as_str().unwrap()).collect();
    assert!(names.windows(2).all(|w| w[0] <= w[1]), "sorted by name: {names:?}");
    assert_eq!(props["filters"]["sort"], "name");

    // A partial reload, as the search box sends: only the table and filters.
    let r = c.reload("/slipway/servers?search=atlas", "slipway/servers/index", "servers,filters", &[]).await;
    let mut keys: Vec<String> = r.props().as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, ["errors", "filters", "servers"]);
    assert!(r.props()["servers"]["data"].as_array().unwrap().iter().all(|s| s["name"].as_str().unwrap().contains("atlas")));
    assert_eq!(r.props()["filters"]["search"], "atlas");

    let props = c.visit("/slipway/servers?status=failed", &[]).await.props();
    assert!(props["servers"]["data"].as_array().unwrap().iter().all(|s| s["status"] == "failed"));

    let props = c.visit("/slipway/servers?sort=cpu&dir=desc&page=2", &[]).await.props();
    assert_eq!(props["servers"]["meta"]["page"], 2);
    assert_eq!(len(&props["servers"]["data"]), 8);
    let cpu: Vec<u64> = props["servers"]["data"].as_array().unwrap().iter().map(|s| s["cpu"].as_u64().unwrap()).collect();
    assert!(cpu.windows(2).all(|w| w[0] >= w[1]), "sorted by cpu, descending: {cpu:?}");
}

#[tokio::test(start_paused = true)]
async fn server_tabs_are_pages_with_lazy_props() {
    let mut c = Client::new().await;
    let r = c.visit("/slipway/servers/1", &[]).await;
    assert_eq!((r.status, r.location()), (StatusCode::SEE_OTHER, "/slipway/servers/1/sites"));

    let page = c.visit("/slipway/servers/1/sites", &[]).await.page();
    assert_eq!(page["component"], "slipway/servers/sites");
    assert_eq!(page["props"]["server"]["id"], 1);
    assert!(page["props"]["sites"].as_array().unwrap().iter().all(|s| s["serverId"] == 1));

    let page = c.visit("/slipway/servers/1/metrics", &[]).await.page();
    assert_eq!(page["deferredProps"], json!({ "default": ["history"] }));
    let history = c.reload("/slipway/servers/1/metrics", "slipway/servers/metrics", "history", &[]).await.props()["history"].clone();
    for series in ["cpu", "memory", "disk", "load", "bandwidth"] {
        assert_eq!(len(&history[series]), 60, "{series}");
    }

    assert_eq!(len(&c.visit("/slipway/servers/1/scheduler", &[]).await.props()["tasks"]), 3);
    assert_eq!(len(&c.visit("/slipway/servers/1/workers", &[]).await.props()["workers"]), 3);
    assert_eq!(c.visit("/slipway/servers/1/logs", &[]).await.page()["deferredProps"], json!({ "default": ["lines"] }));
    let lines = c.reload("/slipway/servers/1/logs", "slipway/servers/logs", "lines", &[]).await.props()["lines"].clone();
    assert_eq!(len(&lines), 40);
}

#[tokio::test]
async fn other_teams_servers_are_not_found() {
    let mut c = Client::new().await;
    let r = c.visit("/slipway/servers/19/sites", &[]).await; // server 19 belongs to team 2
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.page()["component"], "slipway/error");
    assert_eq!(r.props()["status"], 404);
}

#[tokio::test]
async fn reboot_flashes_and_logs_activity() {
    let mut c = Client::new().await;
    c.visit("/slipway/servers/1/sites", &[]).await;
    let r = c.send(Method::POST, "/slipway/servers/1/reboot", json!({}), &[]).await;
    assert_eq!((r.status, r.location()), (StatusCode::SEE_OTHER, "/slipway/servers/1/sites"));
    let props = c.visit("/slipway/activity", &[]).await.props();
    let latest = &props["days"][0]["entries"][0];
    assert_eq!((latest["action"].as_str(), latest["actor"].as_str()), (Some("rebooted"), Some("Ada Lovelace")));
}

const CERT: &str = "-----BEGIN CERTIFICATE-----\nMIIBfake\n-----END CERTIFICATE-----";

#[tokio::test]
async fn site_form_validates_every_field() {
    let mut c = Client::new().await;
    let props = c.visit("/slipway/sites/create", &[]).await.props();
    assert_eq!(props["site"], json!(null));
    assert_eq!(len(&props["servers"]), 18);
    assert_eq!(len(&props["runtimes"]), 5);

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
    let r = c.send_multipart(Method::POST, "/slipway/sites", &bad, Some(("certificate", "cert.txt", "not a cert"))).await;
    assert_eq!(r.status, StatusCode::SEE_OTHER, "an Inertia visit is redirected back");
    let errors = c.visit("/slipway/sites/create", &[]).await.props()["errors"].clone();
    assert_eq!(errors["serverId"], "Pick a server.");
    assert_eq!(errors["domain"], "Must be a domain like app.example.com.");
    assert_eq!(errors["repository"], "Must look like owner/name.");
    assert_eq!(errors["branch"], "No spaces.");
    assert_eq!(errors["runtime"], "Must be one of the allowed values.");
    assert_eq!(errors["env.0.key"], "Use UPPER_SNAKE_CASE.");
    assert_eq!(errors["env.2.key"], "Duplicate key.");
    assert_eq!(errors["certificate"], "Must be a PEM certificate.");

    let taken = [("serverId", "1"), ("domain", "billing.slipway.test"), ("repository", "acme/billing"), ("branch", "main"), ("runtime", "Rust")];
    c.send_multipart(Method::POST, "/slipway/sites", &taken, None).await;
    assert_eq!(c.visit("/slipway/sites/create", &[]).await.props()["errors"]["domain"], "This domain is already taken.");
}

#[tokio::test]
async fn site_create_and_edit_with_certificate_upload() {
    let mut c = Client::new().await;
    c.visit("/slipway/sites/create", &[]).await;
    let fields = [
        ("serverId", "2"),
        ("domain", "Ledger.Slipway.test"),
        ("repository", "acme/ledger"),
        ("branch", "main"),
        ("runtime", "Rust"),
        ("env[0][key]", "DATABASE_URL"),
        ("env[0][value]", "postgres://db/ledger"),
    ];
    let r = c.send_multipart(Method::POST, "/slipway/sites", &fields, Some(("certificate", "ledger.pem", CERT))).await;
    assert_eq!((r.status, r.location()), (StatusCode::SEE_OTHER, "/slipway/servers/2/sites"));

    let props = c.visit("/slipway/servers/2/sites", &[]).await.props();
    assert_eq!(props["flash"]["success"], "ledger.slipway.test saved.");
    let site = props["sites"].as_array().unwrap().iter().find(|s| s["domain"] == "ledger.slipway.test").expect("created, domain lowercased").clone();

    let id = site["id"].as_u64().unwrap();
    let form = c.visit(&format!("/slipway/sites/{id}/edit"), &[]).await.props();
    assert_eq!(form["site"]["env"], json!([{ "key": "DATABASE_URL", "value": "postgres://db/ledger" }]));
    assert_eq!(form["site"]["hasCertificate"], true);

    let edited = [("serverId", "2"), ("domain", "ledger.slipway.test"), ("repository", "acme/ledger"), ("branch", "release"), ("runtime", "Go 1.23")];
    let r = c.send_multipart(Method::PUT, &format!("/slipway/sites/{id}"), &edited, None).await;
    assert_eq!(r.status, StatusCode::SEE_OTHER);
    let form = c.visit(&format!("/slipway/sites/{id}/edit"), &[]).await.props();
    assert_eq!((form["site"]["branch"].as_str(), form["site"]["runtime"].as_str()), (Some("release"), Some("Go 1.23")));
    assert_eq!(form["site"]["hasCertificate"], true, "an edit without a file keeps the certificate");
}

#[tokio::test]
async fn deployments_scroll_page_by_page() {
    let mut c = Client::new().await;
    let page = c.visit("/slipway/deployments", &[]).await.page();
    assert_eq!(len(&page["props"]["deployments"]), 15);
    assert_eq!(page["scrollProps"]["deployments"], json!({ "pageName": "page", "previousPage": null, "nextPage": 2, "currentPage": 1, "reset": false }));
    assert!(page["props"].get("log").is_none(), "the log is optional");

    // What <InfiniteScroll> sends for the next page.
    let r = c.reload("/slipway/deployments?page=2", "slipway/deployments/index", "deployments", &[("x-inertia-infinite-scroll-merge-intent", "append")]).await;
    let page = r.page();
    assert_eq!(page["mergeProps"], json!(["deployments"]));
    assert_eq!(page["matchPropsOn"], json!(["deployments.id"]));
    assert_eq!(page["scrollProps"]["deployments"]["previousPage"], 1);

    let first = c.visit("/slipway/deployments", &[]).await.props()["deployments"][0].clone();
    let id = first["id"].as_u64().unwrap();
    let log = c.reload(&format!("/slipway/deployments?log={id}"), "slipway/deployments/index", "log", &[]).await.props()["log"].clone();
    assert_eq!(log["id"], id);
    assert!(log["lines"][0].as_str().unwrap().contains(first["commit"].as_str().unwrap()));
}

#[tokio::test(start_paused = true)]
async fn deploy_runs_through_its_states_and_can_be_cancelled() {
    let mut c = Client::new().await;
    c.visit("/slipway/deployments", &[]).await;

    let r = c.send(Method::POST, "/slipway/deployments", json!({ "siteId": 1 }), &[]).await;
    assert_eq!((r.status, r.location()), (StatusCode::SEE_OTHER, "/slipway/deployments"));
    let newest = c.visit("/slipway/deployments", &[]).await.props()["deployments"][0].clone();
    assert_eq!((newest["siteId"].as_u64(), newest["status"].as_str()), (Some(1), Some("queued")));

    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(c.visit("/slipway/deployments", &[]).await.props()["deployments"][0]["status"], "running");
    tokio::time::sleep(Duration::from_secs(10)).await;
    let done = c.visit("/slipway/deployments", &[]).await.props()["deployments"][0].clone();
    assert!(["finished", "failed"].contains(&done["status"].as_str().unwrap()), "{done}");
    assert!(done["durationMs"].is_u64());

    // A finished deploy cannot be cancelled: the client's optimistic change rolls back.
    let id = done["id"].as_u64().unwrap();
    c.send(Method::POST, &format!("/slipway/deployments/{id}/cancel"), json!({}), &[("referer", "/slipway/deployments")]).await;
    assert_eq!(c.visit("/slipway/deployments", &[]).await.props()["errors"]["deployment"], "Only queued or running deployments can be cancelled.");

    // A fresh one can.
    c.send(Method::POST, "/slipway/deployments", json!({ "siteId": 2 }), &[]).await;
    let id = c.visit("/slipway/deployments", &[]).await.props()["deployments"][0]["id"].as_u64().unwrap();
    let r = c.send(Method::POST, &format!("/slipway/deployments/{id}/cancel"), json!({}), &[]).await;
    assert_eq!(r.status, StatusCode::SEE_OTHER);
    tokio::time::sleep(Duration::from_secs(10)).await;
    assert_eq!(c.visit("/slipway/deployments", &[]).await.props()["deployments"][0]["status"], "cancelled", "the simulation stops");
}

#[tokio::test]
async fn deploying_a_suspended_site_fails_validation() {
    let mut c = Client::new().await;
    c.visit("/slipway/deployments", &[]).await;
    let r = c.send(Method::POST, "/slipway/deployments", json!({ "siteId": 10 }), &[("referer", "/slipway/deployments")]).await;
    assert_eq!(r.status, StatusCode::SEE_OTHER);
    let errors = c.visit("/slipway/deployments", &[]).await.props()["errors"].clone();
    assert!(errors["siteId"].as_str().unwrap().ends_with("is suspended; deploys are blocked."), "{errors}");
}

#[tokio::test]
async fn activity_loads_older_days_on_request() {
    let mut c = Client::new().await;
    let props = c.visit("/slipway/activity", &[]).await.props();
    assert_eq!(len(&props["days"]), 3);
    assert_eq!(props["days"][0]["label"], "Today");
    assert!(props.get("older").is_none(), "loaded when visible, not up front");

    let older = c.reload("/slipway/activity", "slipway/activity/index", "older", &[]).await.props()["older"].clone();
    assert!(len(&older) > 5);
    assert!(older[0]["entries"][0]["at"].as_u64().unwrap() < props["days"][2]["entries"][0]["at"].as_u64().unwrap());
}

#[tokio::test]
async fn settings_profile_members_and_tokens() {
    let mut c = Client::new().await;
    let props = c.visit("/slipway/settings", &[]).await.props();
    assert_eq!((len(&props["members"]), len(&props["tokens"]), len(&props["roles"])), (5, 2, 4));

    c.send(Method::PUT, "/slipway/settings/profile", json!({ "name": "", "email": "nope" }), &[("referer", "/slipway/settings")]).await;
    let errors = c.visit("/slipway/settings", &[]).await.props()["errors"].clone();
    assert_eq!((errors["name"].as_str(), errors["email"].as_str()), (Some("This field is required."), Some("Must be a valid email address.")));

    c.send(Method::PUT, "/slipway/settings/profile", json!({ "name": "Ada King", "email": "ada@king.test" }), &[]).await;
    let props = c.visit("/slipway/settings", &[]).await.props();
    assert_eq!(props["auth"]["user"]["name"], "Ada King", "shared props reflect the change");
    assert_eq!(props["flash"]["success"], "Profile updated.");

    c.send(Method::PATCH, "/slipway/settings/members/3", json!({ "role": "admin" }), &[]).await;
    assert_eq!(c.visit("/slipway/settings", &[]).await.props()["members"][2]["role"], "admin");

    c.send(Method::DELETE, "/slipway/settings/members/1", json!({}), &[("referer", "/slipway/settings")]).await;
    assert_eq!(c.visit("/slipway/settings", &[]).await.props()["errors"]["member"], "The owner cannot be removed.");
    c.send(Method::DELETE, "/slipway/settings/members/5", json!({}), &[]).await;
    assert_eq!(len(&c.visit("/slipway/settings", &[]).await.props()["members"]), 4);

    c.send(Method::POST, "/slipway/settings/tokens", json!({ "name": "Deploy bot" }), &[]).await;
    let props = c.visit("/slipway/settings", &[]).await.props();
    assert!(props["flash"]["success"].as_str().unwrap().starts_with("Token created: slp_"));
    let id = props["tokens"].as_array().unwrap().iter().find(|t| t["name"] == "Deploy bot").unwrap()["id"].as_u64().unwrap();
    c.send(Method::DELETE, &format!("/slipway/settings/tokens/{id}"), json!({}), &[]).await;
    assert_eq!(len(&c.visit("/slipway/settings", &[]).await.props()["tokens"]), 2);
}

#[tokio::test]
async fn switching_teams_scopes_everything() {
    let mut c = Client::new().await;
    c.visit("/slipway", &[]).await;
    let r = c.send(Method::POST, "/slipway/teams/2", json!({}), &[]).await;
    assert_eq!((r.status, r.location()), (StatusCode::SEE_OTHER, "/slipway"));
    let props = c.visit("/slipway", &[]).await.props();
    assert_eq!(props["team"]["name"], "Side Projects");
    assert_eq!(props["tiles"]["servers"], 6);
    assert_eq!(c.visit("/slipway/servers", &[]).await.props()["servers"]["meta"]["total"], 6);
}

#[tokio::test]
async fn logout_clears_history_on_the_next_page() {
    let mut c = Client::new().await;
    c.visit("/slipway/settings", &[]).await;
    let r = c.send(Method::POST, "/slipway/logout", json!({}), &[]).await;
    assert_eq!((r.status, r.location()), (StatusCode::SEE_OTHER, "/slipway"));
    let page = c.visit("/slipway", &[]).await.page();
    assert_eq!(page["clearHistory"], true);
    assert!(page.get("clearHistory").is_some() && c.visit("/slipway", &[]).await.page().get("clearHistory").is_none(), "only once");
}

#[tokio::test]
async fn palette_lists_the_teams_servers_and_sites() {
    let mut c = Client::new().await;
    let r = c.request(Method::GET, "/slipway/palette", &[("accept", "application/json")], None).await;
    let items: Value = serde_json::from_str(&r.body).unwrap();
    let servers = items.as_array().unwrap().iter().filter(|i| i["kind"] == "server").count();
    assert_eq!(servers, 18);
    assert!(items.as_array().unwrap().iter().any(|i| i["kind"] == "site" && i["href"].as_str().unwrap().ends_with("/edit")));
}

#[tokio::test]
async fn error_pages_are_inertia_pages_with_their_status() {
    let mut c = Client::new().await;
    for status in [403, 404, 419, 500, 503] {
        let r = c.visit(&format!("/slipway/errors/{status}"), &[]).await;
        assert_eq!(r.status.as_u16(), status);
        assert_eq!((r.page()["component"].as_str(), r.props()["status"].as_u64()), (Some("slipway/error"), Some(status as u64)));
    }
    let r = c.visit("/slipway/no/such/page", &[]).await;
    assert_eq!((r.status, r.props()["status"].as_u64()), (StatusCode::NOT_FOUND, Some(404)));
    assert_eq!(c.visit("/slipway/errors/999", &[]).await.status, StatusCode::NOT_FOUND);
}
