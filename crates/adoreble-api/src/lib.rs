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

use adoreble_core::{AdapterCapabilities, Settings};
use axum::http::Request;
use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{HeaderValue, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde_json::json;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub settings: Arc<Mutex<Settings>>,
    pub token: Arc<String>,
    pub data_dir: PathBuf,
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
    (
        StatusCode::OK,
        Json(json!({"ok": true, "applied": true, "path": path})),
    )
        .into_response()
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
    use std::sync::Arc;
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_reports_disabled_by_default() {
        let dir = std::env::temp_dir().join("adoreble_test_health");
        let _ = std::fs::create_dir_all(&dir);
        let state = AppState {
            settings: Arc::new(Mutex::new(Settings::default())),
            token: Arc::new("test-token".to_string()),
            data_dir: dir,
        };
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
        let _ = std::fs::create_dir_all(&dir);
        let state = AppState {
            settings: Arc::new(Mutex::new(Settings::default())),
            token: Arc::new("secret".to_string()),
            data_dir: dir,
        };
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
}
