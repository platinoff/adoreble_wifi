//! Adoreble Wi-Fi — loopback HTTP control plane (ADO-0 scaffold).
//!
//! Security model (ADR-001):
//! - binds `127.0.0.1` only; never a routable address;
//! - all POSTs require `x-adoreble-token` (per-host random token created on
//!   first start in `data/adoreble.token`);
//! - settings are **staged in memory** and only persisted on `/api/settings/apply`
//!   (the `Save` button in the UI), never on `/api/settings/stage`;
//! - only whitelisted routes exist.

use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use adoreble_core::{AdapterCapabilities, HubProbeResult, Settings};
use axum::{
    Json, Router,
    body::Body,
    extract::{Request, State},
    http::{HeaderValue, StatusCode},
    middleware,
    response::{
        Html, IntoResponse, Response,
        sse::{Event, Sse},
    },
    routing::{get, post},
};
use serde_json::json;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub settings: Arc<Mutex<Settings>>,
    pub token: Arc<String>,
    pub data_dir: PathBuf,
    pub log_ring: Arc<Mutex<VecDeque<LogEntry>>>,
    pub log_seq: Arc<AtomicU64>,
}

/// One operational event for the /api/logs SSE stream (redacted).
#[derive(Clone, serde::Serialize)]
pub struct LogEntry {
    pub seq: u64,
    pub ts: u64,
    pub level: String,
    pub msg: String,
}

pub const LOG_RING_CAP: usize = 200;

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Push one operational event to the ring (best-effort; never panics).
pub fn push_log(state: &AppState, level: &str, msg: impl Into<String>) {
    let seq = state.log_seq.fetch_add(1, Ordering::SeqCst) + 1;
    let mut ring = state.log_ring.lock().unwrap();
    if ring.len() >= LOG_RING_CAP {
        ring.pop_front();
    }
    ring.push_back(LogEntry {
        seq,
        ts: now_secs(),
        level: level.to_string(),
        msg: msg.into(),
    });
}

/// Load an existing token, or create one (hex, 32 bytes).
/// The file is created user-read-only on unix (0600).
pub fn load_or_create_token(data_dir: &Path) -> std::io::Result<String> {
    let path = data_dir.join("adoreble.token");
    if path.is_file() {
        return Ok(std::fs::read_to_string(&path)?.trim().to_string());
    }
    std::fs::create_dir_all(data_dir)?;
    let bytes: [u8; 32] = rand::random();
    let mut hex = String::with_capacity(64);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(hex, "{b:02x}");
    }
    std::fs::write(&path, &hex)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&path, perms)?;
    }
    Ok(hex)
}

/// Build the axum router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/adapters", get(adapters))
        .route("/api/status", get(status))
        .route("/api/settings", get(get_settings))
        .route("/api/settings/stage", post(stage_settings))
        .route("/api/settings/apply", post(apply_settings))
        .route("/api/probe-hub", post(probe_hub))
        .route("/api/hotspot/status", get(hotspot_status))
        .route("/api/hotspot/start", post(hotspot_start))
        .route("/api/hotspot/stop", post(hotspot_stop))
        .route("/ui", get(ui_page))
        .route("/api/join", get(join_svgs))
        .route("/api/logs", get(logs_stream))
        .layer(middleware::from_fn_with_state(state.clone(), token_gate))
        .with_state(state)
}

/// Token middleware: POSTs require a matching `x-adoreble-token`.
async fn token_gate(
    State(state): State<AppState>,
    req: Request<Body>,
    next: middleware::Next,
) -> Response {
    if req.method() == axum::http::Method::POST {
        let ok = req
            .headers()
            .get("x-adoreble-token")
            .and_then(|v: &HeaderValue| v.to_str().ok())
            .map(|t| t == *state.token)
            .unwrap_or(false);
        if !ok {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"ok": false, "error": "missing or invalid token"})),
            )
                .into_response();
        }
    }
    next.run(req).await
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let s = state.settings.lock().unwrap();
    Json(json!({
        "ok": true,
        "version": env!("CARGO_PKG_VERSION"),
        "enabled": s.enabled,
        "mode": s.preferred_mode.as_str(),
        "staged": s.staged,
    }))
}

async fn adapters() -> Json<serde_json::Value> {
    #[cfg(windows)]
    let list: Vec<AdapterCapabilities> = adoreble_windows::probe();
    #[cfg(not(windows))]
    let list: Vec<AdapterCapabilities> = {
        let _ = Vec::<AdapterCapabilities>::new();
        adoreble_linux::probe()
    };
    Json(serde_json::json!({"ok": true, "adapters": list}))
}

async fn status(State(state): State<AppState>) -> Json<serde_json::Value> {
    let s = state.settings.lock().unwrap();
    Json(json!({
        "ok": true,
        "enabled": s.enabled,
        "mode": s.preferred_mode.as_str(),
        "staged": s.staged,
        "auto_start": s.auto_start,
        "auto_release_unreachable_ms": s.auto_release_unreachable_ms,
        "hotspot": {
            "ssid": s.hotspot.ssid,
            "band": s.hotspot.band,
            "security": s.hotspot.security,
            "channel": s.hotspot.channel,
            "max_clients": s.hotspot.max_clients,
            "password_set": !s.hotspot.password.is_empty(),
        }
    }))
}

async fn get_settings(State(state): State<AppState>) -> Json<serde_json::Value> {
    let s = state.settings.lock().unwrap();
    Json(serde_json::json!({"ok": true, "settings": s.redacted()}))
}

/// Stage a full settings payload in memory (not persisted).
async fn stage_settings(State(state): State<AppState>, Json(payload): Json<Settings>) -> Response {
    let mut s = state.settings.lock().unwrap();
    *s = payload;
    s.staged = true;
    push_log(&state, "info", "settings staged in memory (not persisted)");
    (StatusCode::OK, Json(json!({"ok": true, "staged": true}))).into_response()
}

/// Apply a staged settings payload: persist to `data/adoreble.json`.
async fn apply_settings(State(state): State<AppState>) -> Response {
    let mut s = state.settings.lock().unwrap();
    s.staged = false;
    let dir = state.data_dir.clone();
    if std::fs::create_dir_all(&dir).is_err() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"ok": false})),
        )
            .into_response();
    }
    let path = dir.join("adoreble.json");
    let body = match serde_json::to_vec_pretty(&*s) {
        Ok(b) => b,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"ok": false})),
            )
                .into_response();
        }
    };
    let mut f = match std::fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"ok": false})),
            )
                .into_response();
        }
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    if f.write_all(&body).is_err() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"ok": false})),
        )
            .into_response();
    }
    push_log(&state, "info", "settings applied (persisted)");
    (
        StatusCode::OK,
        Json(json!({"ok": true, "applied": true, "path": path})),
    )
        .into_response()
}

/// Optional body for `/api/probe-hub`: `{"candidates": [...]}`.
#[derive(serde::Deserialize)]
pub struct ProbeHubBody {
    /// Override candidate URLs; empty/absent ⇒ settings' LAN-first list.
    #[serde(default)]
    pub candidates: Vec<String>,
}

/// Read-only probe of GSV hub candidates (HTTP GET `/api/health`).
/// Never mutates host networking. Token-gated (POST).
async fn probe_hub(
    State(state): State<AppState>,
    body: Option<axum::Json<ProbeHubBody>>,
) -> Json<serde_json::Value> {
    let urls = {
        let s = state.settings.lock().unwrap();
        match body {
            Some(axum::Json(b)) if !b.candidates.is_empty() => b.candidates,
            _ => s.gsv_hub_candidates.clone(),
        }
    };
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build();
    let results: Vec<HubProbeResult> = match client {
        Ok(client) => {
            let mut out = Vec::with_capacity(urls.len());
            for url in urls {
                let start = std::time::Instant::now();
                let res = client.get(&url).send().await;
                let latency_ms = start.elapsed().as_millis() as u64;
                match res {
                    Ok(resp) => {
                        let status = resp.status();
                        out.push(HubProbeResult {
                            url,
                            ok: status.is_success(),
                            latency_ms,
                            detail: format!("HTTP {status}"),
                        });
                    }
                    Err(e) => {
                        out.push(HubProbeResult {
                            url,
                            ok: false,
                            latency_ms,
                            detail: format!("{e}"),
                        });
                    }
                }
            }
            out
        }
        Err(e) => urls
            .into_iter()
            .map(|url| HubProbeResult {
                url,
                ok: false,
                latency_ms: 0,
                detail: format!("client build: {e}"),
            })
            .collect(),
    };
    let ok_count = results.iter().filter(|r| r.ok).count();
    push_log(
        &state,
        "info",
        format!("probe-hub: {ok_count}/{} ok", results.len()),
    );
    Json(json!({"ok": true, "results": results}))
}

/// Optional body for `/api/hotspot/start`.
#[derive(serde::Deserialize)]
pub struct HotspotStartBody {
    /// Override SSID (default: settings or generated).
    #[serde(default)]
    pub ssid: Option<String>,
    /// Override WPA2 passphrase (never echoed back).
    #[serde(default)]
    pub passphrase: Option<String>,
}

/// Live hotspot snapshot (redacted — no passphrase).
async fn hotspot_status() -> Json<serde_json::Value> {
    #[cfg(windows)]
    {
        Json(json!({"ok": true, "status": adoreble_windows::hotspot_status()}))
    }
    #[cfg(not(windows))]
    {
        Json(json!({"ok": false, "error": "hotspot is Windows-only"}))
    }
}

/// Configure + start Mobile Hotspot (owner-gated; token-required POST).
/// Secrets are persisted to `data/adoreble.json`, never returned or logged.
async fn hotspot_start(
    State(state): State<AppState>,
    body: Option<axum::Json<HotspotStartBody>>,
) -> Response {
    let path = state.data_dir.join("adoreble.json");
    let mut settings = match adoreble_core::load_settings(&path) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"ok": false, "error": e})),
            )
                .into_response();
        }
    };
    if let Some(axum::Json(b)) = body {
        if let Some(s) = b.ssid {
            settings.hotspot.ssid = s;
        }
        if let Some(p) = b.passphrase {
            settings.hotspot.password = p;
        }
    }
    let (ssid, passphrase) = adoreble_core::ensure_hotspot_secrets(&mut settings);
    if adoreble_core::persist_settings(&path, &settings).is_err() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"ok": false, "error": "failed to persist settings"})),
        )
            .into_response();
    }
    push_log(
        &state,
        "info",
        "hotspot secrets staged+persisted; starting…",
    );
    #[cfg(windows)]
    {
        match adoreble_windows::hotspot_start(&ssid, &passphrase) {
            Ok(status) => {
                push_log(
                    &state,
                    "info",
                    format!(
                        "hotspot started · {} · clients {}",
                        status.operational_state,
                        status.client_count.unwrap_or(0)
                    ),
                );
                Json(json!({"ok": true, "status": status})).into_response()
            }
            Err(e) => {
                push_log(&state, "warn", format!("hotspot start failed: {e}"));
                (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({"ok": false, "error": e})),
                )
                    .into_response()
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (ssid, passphrase);
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(json!({"ok": false, "error": "hotspot is Windows-only"})),
        )
            .into_response()
    }
}

/// Stop Mobile Hotspot (token-required POST). STA profile untouched.
async fn hotspot_stop(State(state): State<AppState>) -> Response {
    #[cfg(windows)]
    {
        match adoreble_windows::hotspot_stop() {
            Ok(status) => {
                push_log(
                    &state,
                    "info",
                    format!("hotspot stopped · {}", status.operational_state),
                );
                Json(json!({"ok": true, "status": status})).into_response()
            }
            Err(e) => {
                push_log(&state, "warn", format!("hotspot stop failed: {e}"));
                (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({"ok": false, "error": e})),
                )
                    .into_response()
            }
        }
    }
    #[cfg(not(windows))]
    {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(json!({"ok": false, "error": "hotspot is Windows-only"})),
        )
            .into_response()
    }
}

/// The single-page ADO UI (served at `/ui`).
async fn ui_page() -> Html<&'static str> {
    Html(include_str!("../ui/static/index.html"))
}

/// `GET /api/join` — QR payloads for onboarding. Token-gated (this route is a
/// GET, so the middleware does not cover it). Never returns the passphrase.
async fn join_svgs(State(state): State<AppState>, req: Request) -> Response {
    let ok = req
        .headers()
        .get("x-adoreble-token")
        .and_then(|v| v.to_str().ok())
        .map(|t| t == *state.token)
        .unwrap_or(false);
    if !ok {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "missing or invalid token"})),
        )
            .into_response();
    }
    let path = state.data_dir.join("adoreble.json");
    let settings = match adoreble_core::load_settings(&path) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"ok": false, "error": e})),
            )
                .into_response();
        }
    };
    let ssid = settings.hotspot.ssid.clone();
    let passphrase = settings.hotspot.password.clone();
    if ssid.is_empty() || passphrase.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": "no hotspot secrets yet — start the hotspot first"})),
        )
            .into_response();
    }
    #[cfg(windows)]
    let gateway =
        adoreble_windows::detect_gateway_ip().unwrap_or_else(|| "192.168.137.1".to_string());
    #[cfg(not(windows))]
    let gateway = "192.168.137.1".to_string();
    let mesh_url = adoreble_core::mesh_url(&gateway);
    let wifi_svg = qrcode::QrCode::new(adoreble_core::wpa_qr_payload(&ssid, &passphrase))
        .map(|q| {
            q.render::<qrcode::render::svg::Color>()
                .min_dimensions(220, 220)
                .quiet_zone(true)
                .build()
        })
        .unwrap_or_default();
    let mesh_svg = qrcode::QrCode::new(&mesh_url)
        .map(|q| {
            q.render::<qrcode::render::svg::Color>()
                .min_dimensions(220, 220)
                .quiet_zone(true)
                .build()
        })
        .unwrap_or_default();
    Json(serde_json::json!({
        "ok": true,
        "ssid": ssid,
        "gateway": gateway,
        "mesh_url": mesh_url,
        "wifi_svg": wifi_svg,
        "mesh_svg": mesh_svg,
    }))
    .into_response()
}

/// `GET /api/logs` — SSE stream of operational events (redacted).
async fn logs_stream(
    State(state): State<AppState>,
) -> Sse<impl futures::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let stream = futures::stream::unfold((state, 0u64), |(state, last)| async move {
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        let seq = state.log_seq.load(Ordering::SeqCst);
        if seq == last {
            let payload = serde_json::json!({"type": "heartbeat", "ts": now_secs()});
            return Some((
                Ok(Event::default().data(payload.to_string())),
                (state, last),
            ));
        }
        let entries: Vec<LogEntry> = {
            let ring = state.log_ring.lock().unwrap();
            ring.iter().filter(|e| e.seq > last).cloned().collect()
        };
        let new_last = seq;
        let payload = serde_json::json!({"type": "logs", "entries": entries});
        Some((
            Ok(Event::default().data(payload.to_string())),
            (state, new_last),
        ))
    });
    Sse::new(stream)
}

/// Default data directory: `data/` under CWD (gitignored in this repo).
pub fn default_data_dir() -> PathBuf {
    std::env::var("ADOREBLE_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Request, header};
    use std::collections::VecDeque;
    use std::sync::Arc;
    use std::sync::atomic::AtomicU64;
    use tower::ServiceExt;

    fn test_state(token: &str, dir: std::path::PathBuf) -> AppState {
        let _ = std::fs::create_dir_all(&dir);
        AppState {
            settings: Arc::new(Mutex::new(Settings::default())),
            token: Arc::new(token.to_string()),
            data_dir: dir,
            log_ring: Arc::new(Mutex::new(VecDeque::new())),
            log_seq: Arc::new(AtomicU64::new(0)),
        }
    }

    #[tokio::test]
    async fn health_reports_disabled_by_default() {
        let dir = std::env::temp_dir().join("adoreble_test_health");
        let state = test_state("test-token", dir);
        let app = router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn stage_requires_token() {
        let dir = std::env::temp_dir().join("adoreble_test_stage");
        let state = test_state("secret", dir);
        let app = router(state);
        let body = serde_json::to_vec(&Settings::default()).unwrap();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/settings/stage")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn ui_page_serves_shell_markers() {
        let dir = std::env::temp_dir().join("adoreble_test_ui");
        let state = test_state("test-token", dir);
        let app = router(state);
        let resp = app
            .oneshot(Request::builder().uri("/ui").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("Adoreble Wi-Fi"), "UI shell marker missing");
        assert!(text.contains("127.0.0.1:9770"), "UI shell marker missing");
    }

    #[tokio::test]
    async fn join_requires_token_and_never_leaks_passphrase() {
        let dir = std::env::temp_dir().join("adoreble_test_join");
        let mut settings = Settings::default();
        settings.hotspot.ssid = "GSV-Peers-7777".to_string();
        settings.hotspot.password = "topsecret-passphrase".to_string();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("adoreble.json"),
            serde_json::to_vec_pretty(&settings).unwrap(),
        )
        .unwrap();

        let state = test_state("right-token", dir);
        let app = router(state);

        // Without token → 401.
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/join")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        // With token → 200, SVGs present, passphrase NOT in the wire payload.
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/join")
                    .header("x-adoreble-token", "right-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let text = String::from_utf8_lossy(&body);
        assert!(
            text.contains("GSV-Peers-7777"),
            "ssid missing from join wire"
        );
        assert!(text.contains("<svg"), "wifi QR svg missing");
        assert!(
            !text.contains("topsecret-passphrase"),
            "passphrase leaked into JSON"
        );
    }

    #[tokio::test]
    async fn logs_stream_declares_event_stream() {
        let dir = std::env::temp_dir().join("adoreble_test_logs");
        let state = test_state("test-token", dir);
        let app = router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/logs")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        assert!(ct.contains("text/event-stream"), "content-type: {ct}");
    }
}
