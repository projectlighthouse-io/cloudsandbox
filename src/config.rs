use crate::{error::AppError, state::App};

/// Typed config, read once from the environment (and `.env` if present).
pub struct Config {
    pub app_name: String,
    pub app_key: String,
    pub database_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        let _ = dotenvy::dotenv();
        Ok(Self {
            app_name: std::env::var("APP_NAME").unwrap_or_else(|_| "cloudsandbox".into()),
            app_key: required("APP_KEY")?,
            database_url: required("DATABASE_URL")?,
        })
    }
}

fn required(key: &str) -> Result<String, AppError> {
    std::env::var(key).map_err(|_| AppError::Config(format!("{key} is not set")))
}

#[derive(clap::Args)]
pub struct ShowArgs {}

/// `config:show`: the resolved config, secrets masked.
pub async fn show(app: &App, _: ShowArgs) -> Result<(), AppError> {
    let c = &app.cfg;
    let secret = |s: &str| if s.is_empty() { "(empty)".to_owned() } else { "********".to_owned() };
    for (key, value) in [
        ("APP_NAME", c.app_name.clone()),
        ("APP_KEY", secret(&c.app_key)),
        ("DATABASE_URL", mask_password(&c.database_url)),
    ] {
        println!("{key:14}{value}");
    }
    Ok(())
}

/// `scheme://user:password@host/db` -> `scheme://user:********@host/db`.
fn mask_password(url: &str) -> String {
    let (Some(scheme), Some(at)) = (url.find("://"), url.rfind('@')) else { return url.to_owned() };
    match url[scheme + 3..at].split_once(':') {
        Some((user, _)) => format!("{}{user}:********{}", &url[..scheme + 3], &url[at..]),
        None => url.to_owned(),
    }
}
