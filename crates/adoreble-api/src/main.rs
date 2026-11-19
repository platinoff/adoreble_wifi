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
    let settings =
        adoreble_core::load_settings(&data_dir.join("adoreble.json")).unwrap_or_else(|e| {
            tracing::warn!("adoreble.json load failed ({e}); starting with defaults");
            Settings::default()
        });
    let token = adoreble_api::load_or_create_token(&data_dir)?;
    let state = adoreble_api::AppState {
        settings: Arc::new(Mutex::new(settings)),
        token: Arc::new(token),
        data_dir: data_dir.clone(),
        log_ring: Arc::new(Mutex::new(std::collections::VecDeque::new())),
        log_seq: Arc::new(std::sync::atomic::AtomicU64::new(0)),
    };
    // ADO-6: auto-start the hotspot on boot (fail-open; respects staged settings).
    {
        let s = state.settings.lock().unwrap().clone();
        if s.auto_start && s.enabled && s.preferred_mode == adoreble_core::WifiMode::Hotspot {
            let st = state.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                let s = st.settings.lock().unwrap().clone();
                if s.hotspot.password.is_empty() {
                    adoreble_api::push_log(
                        &st,
                        "warn",
                        "auto_start: no hotspot password persisted; skipping",
                    );
                    return;
                }
                #[cfg(windows)]
                match adoreble_windows::hotspot_start(&s.hotspot.ssid, &s.hotspot.password) {
                    Ok(_) => adoreble_api::push_log(&st, "info", "auto_start: hotspot started"),
                    Err(e) => {
                        adoreble_api::push_log(&st, "warn", format!("auto_start failed: {e}"))
                    }
                }
                #[cfg(not(windows))]
                {
                    let _ = (&s.hotspot.ssid, &s.hotspot.password);
                    adoreble_api::push_log(&st, "warn", "auto_start: hotspot is Windows-only");
                }
            });
        }
    }
    // ADO-6: SLA watchdog (auto-release to STA profile on sustained hub loss).
    adoreble_api::spawn_watchdog(state.clone());
    let addr = SocketAddr::from(([127, 0, 0, 1], 9770));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(
        "adoreble-api listening on http://{addr}/ (loopback, token: {})",
        &state.token[..8]
    );
    axum::serve(listener, adoreble_api::router(state)).await?;
    Ok(())
}
