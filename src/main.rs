use std::sync::Arc;

use anyhow::Result;
use time::UtcOffset;
use time::format_description::well_known::Rfc3339;
use tokio::net::TcpListener;
use tokio::signal::{self, unix::SignalKind};
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::time::OffsetTime;

mod config;
mod error;
mod handlers;
mod mam;
mod models;
mod scheduler;
mod state;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("LOG_LEVEL"))
        .with_timer(OffsetTime::new(
            UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC),
            Rfc3339,
        ))
        .init();

    run()
}

#[tokio::main]
async fn run() -> Result<()> {
    let config = config::Config::load()?;
    let port = config.port;
    let app_state = state::AppState::load(config)?;
    let client = Arc::new(mam::MamClient::new(&app_state.config.user_agent));
    let shared = Arc::new(RwLock::new(app_state));

    tokio::spawn(scheduler::run_scheduler(shared.clone(), client.clone()));

    let app = handlers::router(shared, client).layer(TraceLayer::new_for_http());

    let listener = TcpListener::bind(("0.0.0.0", port)).await?;
    info!(port, "mouser listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("shutdown complete");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = signal::ctrl_c();
    let mut term = signal::unix::signal(SignalKind::terminate()).expect("SIGTERM handler");

    tokio::select! {
        _ = ctrl_c => {}
        _ = term.recv() => {}
    }

    info!("shutdown signal received");
}
