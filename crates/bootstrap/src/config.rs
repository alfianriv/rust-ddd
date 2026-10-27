use figment::Figment;
use figment::providers::{Env, Format, Toml};
use serde::Deserialize;

/// App config. Each adapter section is an enum tagged by `driver`, so adding a
/// new adapter = new variant here + new arm in `factory`.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub password_hasher: PasswordHasherConfig,
    pub token: TokenConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "driver", rename_all = "lowercase")]
pub enum DatabaseConfig {
    Postgres { url: String },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "driver", rename_all = "lowercase")]
pub enum PasswordHasherConfig {
    Argon2,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "driver", rename_all = "lowercase")]
pub enum TokenConfig {
    Jwt { secret: String, ttl_secs: u64 },
}

impl Config {
    /// `config/default.toml` -> `config/{APP_ENV}.toml` -> `APP_*` env vars (later wins).
    /// APP_ENV defaults to `development`. A local `.env` file is loaded first if present.
    pub fn load() -> anyhow::Result<Self> {
        let _ = dotenvy::dotenv();
        let env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".into());
        Ok(Self::figment(&env).extract()?)
    }

    fn figment(env: &str) -> Figment {
        Figment::new()
            .merge(Toml::file("config/default.toml"))
            .merge(Toml::file(format!("config/{env}.toml")))
            .merge(Env::prefixed("APP_").split("__"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::result_large_err)] // Jail's closure signature returns figment::Error.
    fn layers_toml_then_env() {
        figment::Jail::expect_with(|jail| {
            jail.create_dir("config")?;
            jail.create_file(
                "config/default.toml",
                r#"
                [database]
                driver = "postgres"
                url = "postgres://default"
                [password_hasher]
                driver = "argon2"
                [token]
                driver = "jwt"
                secret = ""
                ttl_secs = 3600
                "#,
            )?;
            jail.create_file("config/staging.toml", "[token]\nttl_secs = 60")?;
            jail.set_env("APP_DATABASE__URL", "postgres://from-env");
            jail.set_env("APP_TOKEN__SECRET", "s3cret");

            let cfg: Config = Config::figment("staging").extract()?;
            let DatabaseConfig::Postgres { url } = cfg.database;
            assert_eq!(url, "postgres://from-env");
            let TokenConfig::Jwt { secret, ttl_secs } = cfg.token;
            assert_eq!((secret.as_str(), ttl_secs), ("s3cret", 60));

            jail.set_env("APP_DATABASE__DRIVER", "mongo");
            assert!(Config::figment("staging").extract::<Config>().is_err());
            Ok(())
        });
    }
}
