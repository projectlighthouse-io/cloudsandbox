//! Slipway's data: the design's world (Acme Labs, Elizabeth Lane, six servers),
//! seeded fresh per app and held in memory.

use std::{
    sync::{Arc, Mutex, MutexGuard},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

pub const MINUTE: u64 = 60_000;
pub const HOUR: u64 = 60 * MINUTE;
pub const DAY: u64 = 24 * HOUR;

#[derive(Clone, Serialize)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
    pub avatar: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct Team {
    pub id: u64,
    pub name: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    pub id: u64,
    #[serde(skip)]
    pub team_id: u64,
    pub name: String,
    pub provider: String,
    pub region: String,
    pub ip: String,
    /// `healthy`, `degraded` (high load), `provisioning` or `offline`.
    pub status: String,
    /// Percentages; `None` while provisioning or offline.
    pub cpu: Option<u32>,
    pub memory: Option<u32>,
    pub disk: u32,
    pub vcpus: u32,
    pub memory_gb: u32,
    pub sites_count: u32,
    #[serde(skip)]
    pub created_at: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvVar {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    pub id: u64,
    pub server_id: u64,
    pub domain: String,
    pub repository: String,
    pub branch: String,
    pub runtime: String,
    /// `deployed`, `deploying`, `failed` or `suspended`.
    pub status: String,
    pub last_deployed_at: Option<u64>,
    #[serde(skip)]
    pub created_at: u64,
    #[serde(skip)]
    pub env: Vec<EnvVar>,
    #[serde(skip)]
    pub certificate: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Deployment {
    pub id: u64,
    pub site_id: u64,
    pub site: String,
    pub commit: String,
    pub message: String,
    pub author: String,
    pub branch: String,
    /// `queued`, `running`, `finished`, `failed` or `cancelled`.
    pub status: String,
    pub started_at: u64,
    pub duration_ms: Option<u64>,
}

#[derive(Clone, Serialize)]
pub struct Activity {
    pub id: u64,
    pub at: u64,
    pub actor: String,
    pub action: String,
    pub target: String,
}

#[derive(Clone, Serialize)]
pub struct Member {
    pub id: u64,
    pub name: String,
    pub email: String,
    pub role: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiToken {
    pub id: u64,
    pub name: String,
    pub last_used_at: Option<u64>,
    pub created_at: u64,
}

pub struct Data {
    pub user: User,
    pub teams: Vec<Team>,
    pub servers: Vec<Server>,
    pub sites: Vec<Site>,
    pub deployments: Vec<Deployment>,
    pub activity: Vec<Activity>,
    pub members: Vec<Member>,
    pub tokens: Vec<ApiToken>,
    /// Per team, the last seven days (oldest first, today last).
    pub history: Vec<(u64, History)>,
    next_id: u64,
}

/// Seven-day series behind the overview's stat tiles.
#[derive(Clone)]
pub struct History {
    pub servers: [u32; 7],
    pub sites: [u32; 7],
    pub deploys: [u32; 7],
    pub failures: [u32; 7],
}

impl Data {
    pub fn next_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    pub fn log(&mut self, action: &str, target: &str) {
        let id = self.next_id();
        let actor = self.user.name.clone();
        self.activity.insert(0, Activity { id, at: now_ms(), actor, action: action.into(), target: target.into() });
    }
}

/// The store. Cheap to clone; clones share data.
#[derive(Clone)]
pub struct Store(Arc<Mutex<Data>>);

impl Store {
    pub fn lock(&self) -> MutexGuard<'_, Data> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn seeded() -> Self {
        Store(Arc::new(Mutex::new(seed(now_ms()))))
    }
}

pub const RUNTIMES: [&str; 4] = ["Rust binary", "Node.js 22", "Python 3.12", "Static site"];
pub const SERVER_STATUSES: [&str; 4] = ["healthy", "degraded", "provisioning", "offline"];

/// A small deterministic generator, for the older history.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

struct ServerSeed {
    team: u64,
    name: &'static str,
    provider: &'static str,
    region: &'static str,
    ip: &'static str,
    status: &'static str,
    usage: Option<(u32, u32)>,
    disk: u32,
    size: (u32, u32),
    new: bool,
}

const SERVERS: [ServerSeed; 8] = [
    ServerSeed { team: 1, name: "web-prod-01", provider: "DigitalOcean", region: "lon1", ip: "203.0.113.14", status: "healthy", usage: Some((42, 61)), disk: 51, size: (4, 8), new: false },
    ServerSeed { team: 1, name: "web-prod-02", provider: "DigitalOcean", region: "lon1", ip: "203.0.113.15", status: "healthy", usage: Some((38, 58)), disk: 47, size: (4, 8), new: false },
    ServerSeed { team: 1, name: "worker-01", provider: "Hetzner", region: "fsn1", ip: "198.51.100.27", status: "degraded", usage: Some((87, 74)), disk: 38, size: (8, 16), new: false },
    ServerSeed { team: 1, name: "db-primary", provider: "Hetzner", region: "fsn1", ip: "198.51.100.31", status: "healthy", usage: Some((23, 69)), disk: 74, size: (8, 32), new: false },
    ServerSeed { team: 1, name: "staging-01", provider: "AWS", region: "eu-west-2", ip: "192.0.2.80", status: "healthy", usage: Some((12, 33)), disk: 29, size: (2, 4), new: false },
    ServerSeed { team: 1, name: "edge-cache-01", provider: "AWS", region: "eu-west-2", ip: "192.0.2.91", status: "provisioning", usage: None, disk: 3, size: (2, 2), new: true },
    ServerSeed { team: 2, name: "studio-web-01", provider: "Hetzner", region: "hel1", ip: "198.51.100.60", status: "healthy", usage: Some((18, 40)), disk: 22, size: (2, 4), new: false },
    ServerSeed { team: 2, name: "studio-db-01", provider: "Hetzner", region: "hel1", ip: "198.51.100.61", status: "offline", usage: None, disk: 64, size: (2, 4), new: false },
];

/// Domain and server id. Two are new this week ("+2 this week").
const SITES: [(&str, u64); 16] = [
    ("shop.acme.test", 1),
    ("api.acme.test", 1),
    ("docs.acme.test", 1),
    ("status.acme.test", 1),
    ("admin.acme.test", 2),
    ("blog.acme.test", 2),
    ("help.acme.test", 2),
    ("careers.acme.test", 2),
    ("staging.shop.acme.test", 5),
    ("staging.api.acme.test", 5),
    ("staging.docs.acme.test", 5),
    ("preview.acme.test", 5),
    ("sandbox.acme.test", 5),
    ("cdn.acme.test", 6),
    ("lanestudio.test", 7),
    ("portfolio.lanestudio.test", 7),
];

fn seed(now: u64) -> Data {
    let user = User { id: 1, name: "Elizabeth Lane".into(), email: "elizabeth@acme.test".into(), avatar: None };
    let teams = vec![Team { id: 1, name: "Acme Labs".into() }, Team { id: 2, name: "Lane Studio".into() }];
    let old = now - 90 * DAY;
    let today_start = now - now % DAY;

    let mut servers: Vec<Server> = SERVERS
        .iter()
        .enumerate()
        .map(|(i, s)| Server {
            id: i as u64 + 1,
            team_id: s.team,
            name: s.name.into(),
            provider: s.provider.into(),
            region: s.region.into(),
            ip: s.ip.into(),
            status: s.status.into(),
            cpu: s.usage.map(|u| u.0),
            memory: s.usage.map(|u| u.1),
            disk: s.disk,
            vcpus: s.size.0,
            memory_gb: s.size.1,
            sites_count: 0,
            created_at: if s.new { now - 2 * DAY } else { old },
        })
        .collect();

    let mut sites = vec![];
    for (i, (domain, server_id)) in SITES.into_iter().enumerate() {
        let id = i as u64 + 1;
        let app = domain.trim_end_matches(".acme.test").trim_end_matches(".test").replace('.', "-");
        servers.iter_mut().find(|s| s.id == server_id).expect("seeded server").sites_count += 1;
        let static_site = ["docs", "blog", "cdn", "status"].iter().any(|p| domain.starts_with(p));
        sites.push(Site {
            id,
            server_id,
            domain: domain.into(),
            repository: format!("acme/{app}"),
            branch: "main".into(),
            runtime: if static_site { "Static site" } else { "Rust binary" }.into(),
            // sandbox is suspended, to show a deploy being refused.
            status: if domain == "sandbox.acme.test" { "suspended" } else { "deployed" }.into(),
            last_deployed_at: Some(now - (id * 7 % 48) * HOUR),
            created_at: if domain == "cdn.acme.test" || domain == "preview.acme.test" { now - 3 * DAY } else { old },
            env: vec![EnvVar { key: "APP_ENV".into(), value: "production".into() }, EnvVar { key: "RUST_LOG".into(), value: "info".into() }],
            certificate: None,
        });
    }

    let site_id = |domain: &str| sites.iter().find(|s| s.domain == domain).map(|s| s.id).expect("seeded site");
    let mut deployments: Vec<Deployment> = vec![];
    let mut add = |id: u64, domain: &str, commit: &str, message: &str, author: &str, status: &str, at: u64, secs: Option<u64>| {
        deployments.push(Deployment {
            id,
            site_id: site_id(domain),
            site: domain.into(),
            commit: commit.into(),
            message: message.into(),
            author: author.into(),
            branch: "main".into(),
            status: status.into(),
            started_at: at,
            duration_ms: secs.map(|s| s * 1000),
        });
    };
    // The design's recent five. The rest of today keeps "23 today, 1 failed, median 48s".
    let early = |minutes_after_midnight: u64, ago: u64| now.saturating_sub(ago).max(today_start + minutes_after_midnight * MINUTE);
    add(500, "shop.acme.test", "a41c9e2", "Fix cart total rounding", "Elizabeth Lane", "running", now - 38_000, None);
    add(499, "api.acme.test", "7be02d1", "Add rate limit headers", "Tomas Vidal", "finished", early(30, 4 * MINUTE), Some(52));
    add(498, "docs.acme.test", "c09f3aa", "Update install guide", "Imani Okoro", "finished", early(25, 22 * MINUTE), Some(19));
    add(497, "api.acme.test", "1f6d8b4", "Bump queue driver", "Tomas Vidal", "failed", early(20, 70 * MINUTE), Some(51));
    add(496, "shop.acme.test", "e2a7715", "New checkout banner", "Elizabeth Lane", "finished", early(15, 95 * MINUTE), Some(47));

    let mut r = Rng(42);
    let authors = ["Elizabeth Lane", "Tomas Vidal", "Imani Okoro"];
    let messages = ["Tidy logging", "Bump dependencies", "Speed up search", "Cache feature flags", "Add health endpoint", "Trim image sizes", "Handle empty carts", "Rotate signing keys"];
    let acme: Vec<&str> = SITES.iter().filter(|(_, s)| *s <= 6).map(|(d, _)| *d).collect();
    let mut id = 495;
    for i in 0..18u64 {
        let domain = acme[r.below(acme.len() as u64) as usize];
        let hash = format!("{:07x}", r.next() & 0xfff_ffff);
        add(id, domain, &hash, messages[r.below(8) as usize], authors[r.below(3) as usize], "finished", early(14 - i % 14, (100 + i * 23) * MINUTE), Some(40 + (i * 7) % 16));
        id -= 1;
    }
    for i in 0..150u64 {
        let domain = acme[r.below(acme.len() as u64) as usize];
        let status = if r.below(12) == 0 { "failed" } else { "finished" };
        let hash = format!("{:07x}", r.next() & 0xfff_ffff);
        let at = today_start - (i + 1) * 40 * MINUTE - r.below(20 * MINUTE);
        add(id, domain, &hash, messages[r.below(8) as usize], authors[r.below(3) as usize], status, at, Some(20 + r.below(70)));
        id -= 1;
    }
    for i in 0..12u64 {
        let domain = if i % 2 == 0 { "lanestudio.test" } else { "portfolio.lanestudio.test" };
        let hash = format!("{:07x}", r.next() & 0xfff_ffff);
        add(id, domain, &hash, "Update copy", "Elizabeth Lane", "finished", today_start - i * 9 * HOUR - HOUR, Some(30 + i));
        id -= 1;
    }
    deployments.sort_by_key(|d| std::cmp::Reverse(d.started_at));

    let mut activity = vec![];
    let mut entry = |at: u64, actor: &str, action: &str, target: &str| {
        activity.push(Activity { id: 1000 + activity.len() as u64, at, actor: actor.into(), action: action.into(), target: target.into() });
    };
    let today = |h: u64, m: u64| (today_start + h * HOUR + m * MINUTE).min(now);
    entry(today(10, 4), "Elizabeth Lane", "requested a deployment of", "shop.acme.test");
    entry(today(9, 40), "Tomas Vidal", "rotated the API token", "ci-deploy");
    entry(today(9, 15), "Tomas Vidal", "retried the failed deployment", "1f6d8b4");
    entry(today(8, 52), "Imani Okoro", "added the scheduled task", "art queue:prune");
    let yesterday = |h: u64, m: u64| today_start - DAY + h * HOUR + m * MINUTE;
    entry(yesterday(17, 31), "Elizabeth Lane", "invited", "sam@acme.test");
    entry(yesterday(14, 8), "Imani Okoro", "resized the server", "worker-01");
    entry(yesterday(11, 26), "Elizabeth Lane", "renewed the certificate for", "docs.acme.test");
    let older = [
        ("Tomas Vidal", "restarted", "worker-01"),
        ("Imani Okoro", "added the site", "preview.acme.test"),
        ("Elizabeth Lane", "changed the firewall on", "db-primary"),
        ("Tomas Vidal", "created the server", "edge-cache-01"),
        ("Imani Okoro", "deployed", "docs.acme.test"),
        ("Elizabeth Lane", "updated the environment of", "api.acme.test"),
    ];
    for day in 3..16u64 {
        for (j, (actor, action, target)) in older.iter().enumerate().take(3 + (day % 4) as usize) {
            entry(today_start - day * DAY + (17 - j as u64 * 2) * HOUR + (day * 7 % 60) * MINUTE, actor, action, target);
        }
    }

    let members = vec![
        Member { id: 1, name: "Elizabeth Lane".into(), email: "elizabeth@acme.test".into(), role: "owner".into() },
        Member { id: 2, name: "Tomas Vidal".into(), email: "tomas@acme.test".into(), role: "admin".into() },
        Member { id: 3, name: "Imani Okoro".into(), email: "imani@acme.test".into(), role: "member".into() },
    ];
    let tokens = vec![
        ApiToken { id: 1, name: "ci-deploy".into(), last_used_at: Some(now - 26 * MINUTE), created_at: now - 120 * DAY },
        ApiToken { id: 2, name: "grafana-read".into(), last_used_at: Some(now - 3 * DAY), created_at: now - 60 * DAY },
    ];

    let history = vec![
        // Previous six days average 16.7 deploys, so 23 today reads "+38% vs avg".
        (1, History { servers: [4, 4, 5, 5, 5, 6, 6], sites: [10, 11, 11, 12, 12, 14, 14], deploys: [12, 17, 14, 20, 18, 19, 23], failures: [2, 1, 3, 0, 2, 3, 1] }),
        (2, History { servers: [2; 7], sites: [2; 7], deploys: [1, 0, 2, 1, 0, 1, 1], failures: [0; 7] }),
    ];

    Data { user, teams, servers, sites, deployments, activity, members, tokens, history, next_id: 10_000 }
}
