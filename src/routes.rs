use route::Router;

use crate::{handlers, state::App};

/// Every route in one place: Slipway at the root, the Inertia lab under /lab.
pub fn register(mut r: Router<App>) -> Router<App> {
    r.get("/health", handlers::health).name("health");
    crate::slipway::register(&mut r);

    r.group("/lab", |r| {
        use handlers::lab;
        r.name_prefix("lab.");
        r.get("/", lab::index).name("index");
        r.get("/basics", lab::basics).name("basics");
        r.get("/lazy", lab::lazy).name("lazy");
        r.get("/merge", lab::merge).name("merge");
        r.get("/once", lab::once).name("once");
        r.get("/once/again", lab::once_again).name("once.again");
        r.get("/form", lab::form).name("form");
        r.post("/form/contact", lab::contact).name("form.contact");
        r.post("/form/newsletter", lab::newsletter).name("form.newsletter");
        r.post("/form/plain", lab::plain).name("form.plain");
        r.get("/redirects", lab::redirects).name("redirects");
        r.put("/redirects", lab::mutate);
        r.patch("/redirects", lab::mutate);
        r.delete("/redirects", lab::mutate);
        r.get("/broken", lab::broken).name("broken");
    });
    r
}
