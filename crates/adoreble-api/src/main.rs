//! `adoreble-api` — loopback HTTP control plane (binds `127.0.0.1:9770`).
//!
//! Security: loopback only; POSTs `x-adoreble-token`; settings staged in
//! memory, applied on `/api/settings/apply`.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use adoreble_core::Settings;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let data_dir = std::env::var("ADOREBLE_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data"));
    let token = adoreble_api::load_or_create_token(&data_dir)?;
    let state = adoreble_api::AppState {
        settings: Arc::new(Mutex::new(Settings::default())),
        token: Arc::new(token),
        data_dir,
    };
    let addr = SocketAddr::from(([127, 0, 0, 1], 9770));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(
        "adoreble-api listening on http://{addr}/ (loopback, token: {})",
        &state.token[..8]
    );
    axum::serve(listener, adoreble_api::router(state)).await?;
    Ok(())
}
