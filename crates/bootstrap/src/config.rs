use anyhow::{Context, ensure};

/// Read from the environment (12-factor). A local `.env` file is loaded if present.
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_ttl_secs: u64,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let _ = dotenvy::dotenv();
        let jwt_secret = required("JWT_SECRET")?;
        ensure!(
            jwt_secret.len() >= 32,
            "JWT_SECRET must be at least 32 bytes"
        );
        Ok(Self {
            database_url: required("DATABASE_URL")?,
            jwt_secret,
            jwt_ttl_secs: match std::env::var("JWT_TTL_SECS") {
                Ok(v) => v.parse().context("JWT_TTL_SECS must be a number")?,
                Err(_) => 3600,
            },
        })
    }
}

fn required(key: &str) -> anyhow::Result<String> {
    std::env::var(key).with_context(|| format!("{key} is not set"))
}
