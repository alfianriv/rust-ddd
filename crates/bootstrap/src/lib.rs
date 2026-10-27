pub mod config;
mod container;
pub mod factory;

pub use config::Config;
pub use container::Container;

/// Logs to stdout. Level via RUST_LOG (default `info`), JSON lines when LOG_FORMAT=json.
pub fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    if std::env::var("LOG_FORMAT").as_deref() == Ok("json") {
        builder.json().init();
    } else {
        builder.init();
    }
}
