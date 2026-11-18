//! Adoreble Wi-Fi — domain model (ADO-0 scaffold).
//!
//! Owner decisions (2026-11-18, chat):
//! - plugin = MIT, registered in GSV PRODUCTS.md immediately;
//! - loopback-only control plane (`127.0.0.1`), token-authenticated POSTs;
//! - settings are staged in memory and applied on explicit Save
//!   (never live-mutated);
//! - secrets are never committed nor logged (redacted).

use serde::{Deserialize, Serialize};

/// Which radio state the adapter should be in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum WifiMode {
    /// Client to a known SSID (default; e.g. current `BELL409`).
    #[default]
    LanClient,
    /// Access point for GSV mesh peers (Windows: WinRT TetheringManager).
    Hotspot,
    /// L2 bridge (eth0 + wlan0) — Linux/Pi4 only.
    Bridge,
    /// Revert to the last-known-good profile (fail-open).
    ReleaseFallback,
}

impl WifiMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            WifiMode::LanClient => "lan-client",
            WifiMode::Hotspot => "hotspot",
            WifiMode::Bridge => "bridge",
            WifiMode::ReleaseFallback => "release-fallback",
        }
    }
}

/// What one adapter can do, detected or configured explicitly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterCapabilities {
    pub name: String,
    pub description: String,
    pub guid: Option<String>,
    /// e.g. `["2.4ghz", "5ghz"]`
    pub bands: Vec<String>,
    /// Intel AC-8265: `false` (WDI + Windows 11 deprecation).
    pub hosted_network: bool,
    /// Whether the adapter can run Mobile Hotspot via WinRT
    /// `NetworkOperatorTetheringManager` (unknown ⇒ `None`).
    pub hotspot_via_tethering: Option<bool>,
    /// True when the adapter can run STA on one band and AP on another.
    pub concurrent: Option<bool>,
    /// True when SAE/WPA3-Personal is available.
    pub wpa3: bool,
}

/// Hotspot (access point) settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotspotSettings {
    pub ssid: String,
    /// Empty until the owner sets/generates one at runtime; never committed.
    pub password: String,
    /// `"wpa2-aes"` (default) or `"wpa3-sae"` when peers support it.
    pub security: String,
    /// `"2.4ghz"` (default) or `"5ghz"`.
    pub band: String,
    /// Preferred channel (`None` = auto; 1/6/11 on 2.4 GHz).
    pub channel: Option<u8>,
    pub max_clients: u8,
}

/// Plugin settings; persisted to `data/adoreble.json` (gitignored).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub enabled: bool,
    pub preferred_mode: WifiMode,
    pub hotspot: HotspotSettings,
    pub auto_start: bool,
    /// Revert to `LanClient` after this many ms without a successful
    /// GSV-hub probe (fail-open).
    pub auto_release_unreachable_ms: u64,
    /// Staging marker: true between `/api/settings/stage` and `/api/settings/apply`.
    pub staged: bool,
    /// GSV hub health-probe candidates, tried in order.
    #[serde(default = "default_candidates")]
    pub gsv_hub_candidates: Vec<String>,
}

fn default_candidates() -> Vec<String> {
    default_hub_candidates()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            preferred_mode: WifiMode::LanClient,
            hotspot: HotspotSettings {
                ssid: "GSV-Peers".to_string(),
                password: String::new(),
                security: "wpa2-aes".to_string(),
                band: "2.4ghz".to_string(),
                channel: None,
                max_clients: 8,
            },
            auto_start: true,
            auto_release_unreachable_ms: 120_000,
            staged: false,
            gsv_hub_candidates: default_hub_candidates(),
        }
    }
}

impl Settings {
    /// Clone with the hotspot password wiped (for logs / API responses).
    pub fn redacted(&self) -> Self {
        let mut out = self.clone();
        out.hotspot.password.clear();
        out.staged = false;
        out
    }
}

/// Redact a potential secret for log output.
pub fn redact_str(_value: &str) -> &'static str {
    "<redacted>"
}

/// One GSV-hub probe outcome (read-only HTTP GET of `/api/health`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HubProbeResult {
    pub url: String,
    pub ok: bool,
    /// Round-trip milliseconds (0 when the probe failed before any response).
    pub latency_ms: u64,
    /// HTTP status as string, or the transport error message.
    pub detail: String,
}

/// Decision returned by the fail-open policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReleaseAction {
    /// Keep current mode.
    Stay,
    /// SLA broken: revert to the last-known-good profile (`LanClient`).
    Release,
}

/// Fail-open policy: release only after **both** a sustained outage window
/// and repeated probe failures. Pure function — never performs I/O.
///
/// `consecutive_failures`: probes in a row that failed.
/// `unreachable_ms`: how long the hub has been continuously unreachable.
/// `threshold_ms`: configured `auto_release_unreachable_ms`.
pub fn evaluate_release(
    consecutive_failures: u32,
    unreachable_ms: u64,
    threshold_ms: u64,
) -> ReleaseAction {
    const MIN_FAILURES: u32 = 3;
    if consecutive_failures >= MIN_FAILURES && unreachable_ms >= threshold_ms {
        ReleaseAction::Release
    } else {
        ReleaseAction::Stay
    }
}

/// What fail-open `release` would do, without doing it. Pure; the caller
/// decides whether to execute (owner-gated in ADO-3+).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleasePlan {
    pub from: Option<String>,
    pub to: String,
    /// The exact command vectors that would run, in order.
    pub commands: Vec<Vec<String>>,
}

/// Build (but never run) the fail-open release plan.
pub fn plan_release(from: Option<&str>, to: &str) -> ReleasePlan {
    ReleasePlan {
        from: from.map(str::to_string),
        to: to.to_string(),
        commands: vec![vec![
            "netsh".to_string(),
            "wlan".to_string(),
            "connect".to_string(),
            format!("name={to}"),
        ]],
    }
}

/// Generate a WPA2-compatible passphrase (letters+digits, unambiguous).
/// Pure; never logged — callers persist it to `data/adoreble.json` only.
pub fn generate_wpa2_passphrase(len: usize) -> String {
    use rand::Rng;
    // Avoid ambiguous glyphs (l/1/I, O/0) for phone-side typing.
    const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz23456789";
    let mut rng = rand::thread_rng();
    (0..len.max(12))
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect()
}

/// Deterministic-ish SSID suffix from RNG (e.g. `GSV-Peers-4821`).
pub fn generate_ssid(prefix: &str) -> String {
    use rand::Rng;
    let n: u32 = rand::thread_rng().gen_range(1000..10000);
    format!("{prefix}-{n}")
}

/// Wi-Fi QR payload (`WIFI:T:WPA;S:<ssid>;P:<pass>;;`).
/// Escapes `\ ; , : "` in ssid/pass per ZXing spec.
pub fn wpa_qr_payload(ssid: &str, passphrase: &str) -> String {
    fn esc(s: &str) -> String {
        let mut out = String::with_capacity(s.len() + 4);
        for c in s.chars() {
            if matches!(c, '\\' | ';' | ',' | ':' | '"') {
                out.push('\\');
            }
            out.push(c);
        }
        out
    }
    format!("WIFI:T:WPA;S:{};P:{};;", esc(ssid), esc(passphrase))
}

/// GSV hub mesh page URL for a given gateway/AP host.
pub fn mesh_url(gateway: &str) -> String {
    format!("http://{gateway}:9999/wgpu")
}

/// Load settings from `path`; missing file ⇒ `Settings::default()`.
pub fn load_settings(path: &std::path::Path) -> Result<Settings, String> {
    if !path.exists() {
        return Ok(Settings::default());
    }
    let raw = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    serde_json::from_str(&raw).map_err(|e| format!("parse {}: {e}", path.display()))
}

/// Persist settings atomically (tmp file + rename). Unix: 0600 perms.
pub fn persist_settings(path: &std::path::Path, settings: &Settings) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    }
    let tmp = path.with_extension("json.tmp");
    let body =
        serde_json::to_vec_pretty(settings).map_err(|e| format!("serialize settings: {e}"))?;
    std::fs::write(&tmp, &body).map_err(|e| format!("write {}: {e}", tmp.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, path).map_err(|e| format!("rename {}: {e}", path.display()))
}

/// Fill missing hotspot secrets in-place (SSID suffix + WPA2 passphrase).
/// Returns `(ssid, passphrase)` — passphrase must never be logged.
pub fn ensure_hotspot_secrets(settings: &mut Settings) -> (String, String) {
    let ssid = if settings.hotspot.ssid.trim().is_empty() || settings.hotspot.ssid == "GSV-Peers" {
        generate_ssid("GSV-Peers")
    } else {
        settings.hotspot.ssid.clone()
    };
    let passphrase = if settings.hotspot.password.is_empty() {
        generate_wpa2_passphrase(12)
    } else {
        settings.hotspot.password.clone()
    };
    settings.hotspot.ssid = ssid.clone();
    settings.hotspot.password = passphrase.clone();
    settings.enabled = true;
    settings.preferred_mode = WifiMode::Hotspot;
    (ssid, passphrase)
}

/// Default probe candidates (LAN-first per GSV band 233; loopback last).
/// Overridable via `Settings::gsv_hub_candidates`.
pub fn default_hub_candidates() -> Vec<String> {
    vec![
        "http://192.168.2.238:9999/api/health".to_string(),
        "http://127.0.0.1:9999/api/health".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_are_fail_open() {
        let s = Settings::default();
        assert!(!s.enabled, "plugin must default to disabled");
        assert_eq!(s.preferred_mode, WifiMode::LanClient);
        assert_eq!(s.auto_release_unreachable_ms, 120_000);
    }

    #[test]
    fn redacted_settings_wipe_password() {
        let mut s = Settings::default();
        s.hotspot.password = "sup3r-secret".to_string();
        let r = s.redacted();
        assert!(r.hotspot.password.is_empty());
        assert!(!r.staged);
    }

    #[test]
    fn mode_strings_are_stable() {
        assert_eq!(WifiMode::LanClient.as_str(), "lan-client");
        assert_eq!(WifiMode::Hotspot.as_str(), "hotspot");
        assert_eq!(WifiMode::Bridge.as_str(), "bridge");
        assert_eq!(WifiMode::ReleaseFallback.as_str(), "release-fallback");
    }

    #[test]
    fn release_policy_needs_both_signals() {
        // Sustained outage + repeated failures => release.
        assert_eq!(
            evaluate_release(3, 120_000, 120_000),
            ReleaseAction::Release
        );
        assert_eq!(evaluate_release(10, 60_000, 120_000), ReleaseAction::Stay);
        assert_eq!(evaluate_release(2, 200_000, 120_000), ReleaseAction::Stay);
        assert_eq!(evaluate_release(0, 0, 120_000), ReleaseAction::Stay);
    }

    #[test]
    fn wpa2_passphrase_is_typed_unambiguous_and_long_enough() {
        let p = generate_wpa2_passphrase(16);
        assert_eq!(p.len(), 16);
        assert!(p.chars().all(|c| c.is_ascii_alphanumeric()));
        assert!(!p.contains('l') && !p.contains('I') && !p.contains('O') && !p.contains('0'));
        assert_ne!(generate_wpa2_passphrase(16), generate_wpa2_passphrase(16));
    }

    #[test]
    fn ssid_suffix_stays_in_range() {
        for _ in 0..50 {
            let s = generate_ssid("GSV-Peers");
            let n: u32 = s.strip_prefix("GSV-Peers-").unwrap().parse().unwrap();
            assert!((1000..10000).contains(&n));
        }
    }

    #[test]
    fn qr_payload_escapes_specials() {
        assert_eq!(
            wpa_qr_payload("plain", "pass"),
            "WIFI:T:WPA;S:plain;P:pass;;"
        );
        assert_eq!(wpa_qr_payload("a;b", "p:q"), "WIFI:T:WPA;S:a\\;b;P:p\\:q;;");
    }

    #[test]
    fn mesh_url_targets_hub_port() {
        assert_eq!(mesh_url("192.168.137.1"), "http://192.168.137.1:9999/wgpu");
    }

    #[test]
    fn ensure_hotspot_secrets_fills_and_is_idempotent() {
        let mut s = Settings::default();
        let (ssid, pass) = ensure_hotspot_secrets(&mut s);
        assert!(ssid.starts_with("GSV-Peers-"));
        assert_eq!(pass.len(), 12);
        assert!(s.enabled);
        assert_eq!(s.preferred_mode, WifiMode::Hotspot);
        let pass2 = s.hotspot.password.clone();
        let (_, pass_again) = ensure_hotspot_secrets(&mut s);
        assert_eq!(
            pass_again, pass2,
            "passphrase must not be regenerated once set"
        );
    }

    #[test]
    fn settings_roundtrip_via_persist() {
        let dir = std::env::temp_dir().join(format!("adoreble-test-{}", std::process::id()));
        let path = dir.join("adoreble.json");
        let mut s = Settings::default();
        s.hotspot.password = "roundtrip-secret".to_string();
        persist_settings(&path, &s).unwrap();
        let loaded = load_settings(&path).unwrap();
        assert_eq!(loaded.hotspot.password, "roundtrip-secret");
        let missing = load_settings(&dir.join("nope.json")).unwrap();
        assert!(!missing.enabled);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn default_candidates_are_lan_first() {
        let c = default_hub_candidates();
        assert!(c.len() >= 2);
        assert!(c[0].contains("192.168."), "LAN candidate must come first");
        assert!(c.iter().all(|u| u.ends_with("/api/health")));
    }

    #[test]
    fn settings_deserialize_without_candidates_field() {
        // Backward compat with ADO-0's data/adoreble.json (no gsv_hub_candidates).
        let legacy = r#"{
            "enabled": false,
            "preferred_mode": "lan-client",
            "hotspot": {
                "ssid": "GSV-Peers",
                "password": "",
                "security": "wpa2-aes",
                "band": "2.4ghz",
                "channel": null,
                "max_clients": 8
            },
            "auto_start": true,
            "auto_release_unreachable_ms": 120000,
            "staged": false
        }"#;
        let s: Settings = serde_json::from_str(legacy).expect("legacy settings must load");
        assert_eq!(s.gsv_hub_candidates, default_hub_candidates());
    }
}
