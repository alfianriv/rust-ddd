use std::sync::Arc;

use bootstrap::{Config, Container};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    bootstrap::init_tracing();
    let cfg = Config::load()?;
    let container = Arc::new(Container::new(&cfg).await?);
    let app = apis::router(container, &cfg.apis)?;

    let listener = tokio::net::TcpListener::bind(&cfg.apis.addr).await?;
    tracing::info!(addr = %cfg.apis.addr, "apis listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// Ctrl+C locally, SIGTERM from Docker/Kubernetes. In-flight requests finish first.
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(_) => std::future::pending().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutting down");
}
