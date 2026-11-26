//! Slipway's mock data: seeded deterministically, held in memory per app.

use std::{
    sync::{Arc, Mutex, MutexGuard},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

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
    pub status: String,
    pub cpu: u32,
    pub memory: u32,
    pub disk: u32,
    pub sites_count: u32,
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
    pub status: String,
    pub last_deployed_at: Option<u64>,
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
    next_id: u64,
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

/// A small deterministic generator: the same mock data every start.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len() as u64) as usize]
    }
}

pub const PROVIDERS: [&str; 5] = ["Hetzner", "DigitalOcean", "AWS", "Vultr", "Linode"];
pub const SERVER_STATUSES: [&str; 4] = ["active", "provisioning", "stopped", "failed"];
pub const RUNTIMES: [&str; 5] = ["Node 22", "Rust", "Go 1.23", "Python 3.12", "Static"];

fn regions(provider: &str) -> &'static [&'static str] {
    match provider {
        "Hetzner" => &["fsn1", "nbg1", "hel1", "ash"],
        "DigitalOcean" => &["nyc3", "ams3", "sgp1", "fra1"],
        "AWS" => &["us-east-1", "eu-west-1", "ap-south-1"],
        "Vultr" => &["ewr", "lhr", "nrt"],
        _ => &["us-central", "eu-west", "ap-southeast"],
    }
}

const NAMES: [&str; 12] = ["atlas", "beacon", "cinder", "delta", "ember", "fjord", "granite", "harbor", "iris", "juniper", "kestrel", "lumen"];
const ROLES: [&str; 3] = ["web", "worker", "db"];
const APPS: [&str; 10] = ["billing", "dashboard", "api", "docs", "status", "auth", "search", "media", "mailer", "shop"];
const AUTHORS: [&str; 5] = ["Ada Lovelace", "Grace Hopper", "Alan Turing", "Katherine Johnson", "Linus Torvalds"];
const MESSAGES: [&str; 10] = [
    "Fix pagination on invoices",
    "Bump dependencies",
    "Add retry to webhook sender",
    "Cache feature flags",
    "Speed up search index build",
    "Tidy logging around checkout",
    "Handle empty carts",
    "Rotate signing keys",
    "Add health endpoint",
    "Trim image sizes",
];
const ACTIONS: [&str; 8] = [
    "deployed",
    "restarted",
    "updated environment of",
    "added SSL certificate to",
    "rebooted",
    "created",
    "changed firewall on",
    "rotated keys on",
];

pub fn hash(r: &mut Rng7) -> String {
    format!("{:07x}", r.0.next() & 0xfff_ffff)
}

/// A wrapper so other modules can make commit hashes from the same generator.
pub struct Rng7(Rng);

impl Rng7 {
    pub fn new(seed: u64) -> Self {
        Rng7(Rng(seed))
    }
}

fn seed(now: u64) -> Data {
    let mut r = Rng(42);
    let hour = 3_600_000;
    let user = User { id: 1, name: "Ada Lovelace".into(), email: "ada@slipway.test".into(), avatar: None };
    let teams = vec![Team { id: 1, name: "Acme Platform".into() }, Team { id: 2, name: "Side Projects".into() }];

    let mut servers = vec![];
    for i in 0..24u64 {
        let provider = r.pick(&PROVIDERS).to_owned();
        let status = if i % 9 == 4 { "failed" } else if i % 7 == 3 { "provisioning" } else if i % 11 == 8 { "stopped" } else { "active" };
        let active = status == "active";
        servers.push(Server {
            id: i + 1,
            team_id: if i < 18 { 1 } else { 2 },
            name: format!("{}-{}-{}", NAMES[i as usize % NAMES.len()], r.pick(&ROLES), i / NAMES.len() as u64 + 1),
            region: r.pick(regions(&provider)).to_owned(),
            provider,
            ip: format!("{}.{}.{}.{}", 10 + r.below(180), r.below(255), r.below(255), 2 + r.below(250)),
            status: status.into(),
            cpu: if active { 5 + r.below(85) as u32 } else { 0 },
            memory: if active { 20 + r.below(75) as u32 } else { 0 },
            disk: 10 + r.below(80) as u32,
            sites_count: 0,
        });
    }

    let mut sites: Vec<Site> = vec![];
    for i in 0..40u64 {
        // Site 10 is the suspended one: keep it on a team-1 server, so the default
        // team can see a deploy being refused.
        let server = &mut servers[r.below(if i == 9 { 18 } else { 24 }) as usize];
        server.sites_count += 1;
        let app = APPS[i as usize % APPS.len()];
        let status = if i % 13 == 6 { "failed" } else if i == 9 { "suspended" } else { "deployed" };
        sites.push(Site {
            id: i + 1,
            server_id: server.id,
            domain: format!("{app}{}.slipway.test", if i < 10 { String::new() } else { format!("-{}", i / 10) }),
            repository: format!("acme/{app}"),
            branch: if i % 5 == 0 { "develop".into() } else { "main".into() },
            runtime: r.pick(&RUNTIMES).into(),
            status: status.into(),
            last_deployed_at: Some(now - r.below(72) * hour),
            env: vec![EnvVar { key: "APP_ENV".into(), value: "production".into() }, EnvVar { key: "LOG_LEVEL".into(), value: "info".into() }],
            certificate: None,
        });
    }

    let mut deployments = vec![];
    let mut g = Rng7::new(7);
    for i in 0..120u64 {
        let site = &sites[r.below(40) as usize];
        let failed = r.below(10) == 0;
        deployments.push(Deployment {
            id: i + 1,
            site_id: site.id,
            site: site.domain.clone(),
            commit: hash(&mut g),
            message: r.pick(&MESSAGES).into(),
            author: r.pick(&AUTHORS).into(),
            branch: site.branch.clone(),
            status: if failed { "failed".into() } else { "finished".into() },
            started_at: now - (120 - i) * hour / 2 - r.below(1_800_000),
            duration_ms: Some(20_000 + r.below(140_000)),
        });
    }
    deployments.reverse(); // newest first

    let mut activity = vec![];
    for i in 0..220u64 {
        activity.push(Activity {
            id: 1000 + i,
            at: now - i * (14 * 24 * hour / 220) - r.below(hour),
            actor: r.pick(&AUTHORS).into(),
            action: r.pick(&ACTIONS).into(),
            target: if r.below(2) == 0 { servers[r.below(24) as usize].name.clone() } else { sites[r.below(40) as usize].domain.clone() },
        });
    }

    let members = AUTHORS
        .iter()
        .enumerate()
        .map(|(i, name)| Member {
            id: i as u64 + 1,
            name: (*name).into(),
            email: format!("{}@slipway.test", name.split(' ').next().unwrap().to_lowercase()),
            role: ["owner", "admin", "developer", "developer", "viewer"][i].into(),
        })
        .collect();
    let tokens = vec![
        ApiToken { id: 1, name: "CI pipeline".into(), last_used_at: Some(now - 2 * hour), created_at: now - 900 * hour },
        ApiToken { id: 2, name: "Terraform".into(), last_used_at: None, created_at: now - 300 * hour },
    ];

    Data { user, teams, servers, sites, deployments, activity, members, tokens, next_id: 10_000 }
}
