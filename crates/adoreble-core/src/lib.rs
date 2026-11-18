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
}
