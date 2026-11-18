//! Windows backend for Adoreble Wi-Fi (ADO-1: capability probe).
//!
//! Reads the **static** adapter/drive info via `netsh` (parsed, no mutations)
//! and the **live** tethering state via WinRT
//! `Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager`.
//! Nothing here changes host networking — read-only probes only.

use serde::Serialize;

use adoreble_core::AdapterCapabilities;

/// One Wi-Fi interface as reported by `netsh wlan show interfaces`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdapterInfo {
    pub name: String,
    pub description: String,
    pub guid: Option<String>,
    pub state: String,
    pub ssid: String,
    pub bssid: Option<String>,
    pub band: String,
    pub channel: Option<u8>,
    pub radio_type: String,
    pub authentication: String,
    pub cipher: String,
    pub rssi: Option<i32>,
    pub signal_pct: Option<u8>,
}

/// Driver view from `netsh wlan show drivers`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DriverInfo {
    pub description: String,
    pub driver_type: String,
    pub radio_types: Vec<String>,
    /// True when the legacy hosted network (SoftAP) is supported
    /// (deprecated on WDI drivers; expect `false` on Intel AC-8265).
    pub hosted_network: bool,
}

/// Split `    Key                   : Value` lines into (key, value).
fn parse_kv_lines(output: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in output.lines() {
        let line = line.trim_end();
        if let Some((k, v)) = line.split_once(':') {
            let k = k.trim().to_string();
            let v = v.trim().to_string();
            if !k.is_empty() && !v.is_empty() {
                out.push((k, v));
            }
        }
    }
    out
}

/// Parse `netsh wlan show interfaces` output.
pub fn parse_interfaces(output: &str) -> Vec<AdapterInfo> {
    // The output is: blank lines, "There are N interfaces on the system:",
    // then per-interface blocks each starting with "Name                   : …".
    let mut blocks: Vec<Vec<(String, String)>> = Vec::new();
    let mut current: Vec<(String, String)> = Vec::new();
    let mut started = false;
    for (k, v) in parse_kv_lines(output) {
        // A new interface block starts at "Name".
        if k == "Name" {
            if started {
                blocks.push(std::mem::take(&mut current));
            }
            started = true;
        }
        if started {
            current.push((k, v));
        }
    }
    if started && !current.is_empty() {
        blocks.push(current);
    }

    let get = |b: &[(String, String)], key: &str| -> Option<String> {
        b.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    };

    blocks
        .into_iter()
        .map(|b| {
            let rssi = get(&b, "Rssi").and_then(|v| v.trim_end_matches(" dBm").parse().ok());
            let signal_pct = get(&b, "Signal").and_then(|v| v.trim_end_matches('%').parse().ok());
            let channel = get(&b, "Channel").and_then(|v| v.parse().ok());
            AdapterInfo {
                name: get(&b, "Name").unwrap_or_default(),
                description: get(&b, "Description").unwrap_or_default(),
                guid: get(&b, "GUID").filter(|s| s != "N/A"),
                state: get(&b, "State").unwrap_or_default(),
                ssid: get(&b, "SSID").unwrap_or_default(),
                bssid: get(&b, "AP BSSID").filter(|s| s != "N/A"),
                band: get(&b, "Band").unwrap_or_default(),
                channel,
                radio_type: get(&b, "Radio type").unwrap_or_default(),
                authentication: get(&b, "Authentication").unwrap_or_default(),
                cipher: get(&b, "Cipher").unwrap_or_default(),
                rssi,
                signal_pct,
            }
        })
        .collect()
}

/// Parse `netsh wlan show drivers` output.
pub fn parse_drivers(output: &str) -> Vec<DriverInfo> {
    // Same single-block format as interfaces; we only need the driver row.
    let kv = parse_kv_lines(output);
    let get =
        |key: &str| -> Option<String> { kv.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone()) };
    if get("Driver").is_none() {
        return Vec::new();
    }
    let hosted = get("Hosted network supported")
        .map(|v| v.eq_ignore_ascii_case("Yes"))
        .unwrap_or(false);
    let radio_types = get("Radio types supported")
        .map(|v| v.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default();
    vec![DriverInfo {
        description: get("Driver").unwrap_or_default(),
        driver_type: get("Type").unwrap_or_default(),
        radio_types,
        hosted_network: hosted,
    }]
}

/// Live tethering capability for one internet connection profile.
#[derive(Debug, Clone, Serialize)]
pub struct TetheringProbe {
    /// Connection profile name (e.g. `BELL409`).
    pub profile: String,
    /// `NetworkOperatorTetheringManager::TetheringOperationalState` debug repr.
    pub operational_state: Option<String>,
    /// `MaxClientCount` from the manager (if creatable).
    pub max_clients: Option<u32>,
    /// `ClientCount` from the manager (if creatable).
    pub client_count: Option<u32>,
    /// `GetTetheringCapabilityFromConnectionProfile(profile)` debug repr.
    pub capability: Option<String>,
    /// Error message when probing partially failed (kept for diagnostics).
    pub error: Option<String>,
}

/// Static placeholder capabilities for this dev box (ADO-0).
/// ADO-1 replaces this with a real probe (tethering state, bands, WPA3).
#[cfg(windows)]
pub fn probe() -> Vec<AdapterCapabilities> {
    vec![AdapterCapabilities {
        name: "Wi-Fi".to_string(),
        // Verified 2026-11-18 via `netsh wlan show interfaces/drivers`:
        // Intel Dual Band Wireless-AC 8265 · hosted network: No · on BELL409 (5 GHz).
        description: "Intel(R) Dual Band Wireless-AC 8265".to_string(),
        guid: Some("1bf194dd-4e7a-404d-9389-f8b32e1a2cfb".to_string()),
        bands: vec!["2.4ghz".to_string(), "5ghz".to_string()],
        hosted_network: false,
        hotspot_via_tethering: None, // unknown until ADO-1 probe
        concurrent: None,            // unknown until ADO-1 probe
        wpa3: false,
    }]
}

#[cfg(not(windows))]
pub fn probe() -> Vec<AdapterCapabilities> {
    Vec::new()
}

/// Live WinRT tethering probe across all internet connection profiles.
/// Never mutates the adapter; safe to call repeatedly (read-only).
#[cfg(windows)]
pub fn probe_tethering() -> Vec<TetheringProbe> {
    use windows::Networking::Connectivity::NetworkInformation;
    use windows::Networking::NetworkOperators::NetworkOperatorTetheringManager;
    use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize};

    // SAFETY: read-only probes; RoInitialize/RoUninitialize pairs required.
    unsafe {
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }

    let mut out = Vec::new();

    let profiles = match NetworkInformation::GetConnectionProfiles() {
        Ok(p) => p,
        Err(e) => {
            unsafe { RoUninitialize() };
            return vec![TetheringProbe {
                profile: "<unknown>".to_string(),
                operational_state: None,
                max_clients: None,
                client_count: None,
                capability: None,
                error: Some(format!("GetConnectionProfiles: {e}")),
            }];
        }
    };

    let size = profiles.Size().unwrap_or(0);
    for i in 0..size {
        let mut probe = TetheringProbe {
            profile: String::new(),
            operational_state: None,
            max_clients: None,
            client_count: None,
            capability: None,
            error: None,
        };
        match profiles.GetAt(i) {
            Ok(profile) => {
                probe.profile = profile
                    .ProfileName()
                    .map(|s| s.to_string_lossy())
                    .unwrap_or_default();
                // Capability
                probe.capability = match NetworkOperatorTetheringManager::GetTetheringCapabilityFromConnectionProfile(
                    &profile,
                ) {
                    Ok(cap) => Some(format!("{cap:?}")),
                    Err(e) => Some(format!("error: {e}")),
                };
                // Manager (operational state / client counts)
                match NetworkOperatorTetheringManager::CreateFromConnectionProfile(&profile) {
                    Ok(mgr) => {
                        probe.operational_state = match mgr.TetheringOperationalState() {
                            Ok(s) => Some(format!("{s:?}")),
                            Err(e) => Some(format!("error: {e}")),
                        };
                        probe.max_clients = mgr.MaxClientCount().ok();
                        probe.client_count = mgr.ClientCount().ok();
                    }
                    Err(e) => {
                        probe.error = Some(format!("CreateFromConnectionProfile: {e}"));
                    }
                }
            }
            Err(e) => {
                probe.error = Some(format!("GetAt({i}): {e}"));
            }
        }
        out.push(probe);
    }

    unsafe { RoUninitialize() };
    out
}

#[cfg(not(windows))]
pub fn probe_tethering() -> Vec<TetheringProbe> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_this_box_captured_interfaces() {
        let out = r"

There is 1 interface on the system:

    Name                   : Wi-Fi
    Description            : Intel(R) Dual Band Wireless-AC 8265
    GUID                   : 1bf194dd-4e7a-404d-9389-f8b32e1a2cfb
    Physical address       : f0:9e:4a:10:06:58
    Interface type         : Primary
    State                  : connected
    SSID                   : BELL409
    AP BSSID               : 0e:ac:8a:5d:be:a1
    Band                   : 5 GHz
    Channel                : 44
    Connected Akm-cipher   : [ akm = 00-0f-ac:02, cipher =  00-0f-ac:04 ]
    Network type           : Infrastructure
    Radio type             : 802.11n
    Authentication         : WPA2-Personal
    Cipher                 : CCMP
    Connection mode        : Profile
    Receive rate (Mbps)    : 240
    Transmit rate (Mbps)   : 240
    Signal                 : 80%
    Rssi                   : -64
    Profile                : BELL409
";
        let v = parse_interfaces(out);
        assert_eq!(v.len(), 1);
        let a = &v[0];
        assert_eq!(a.name, "Wi-Fi");
        assert_eq!(a.description, "Intel(R) Dual Band Wireless-AC 8265");
        assert_eq!(
            a.guid.as_deref(),
            Some("1bf194dd-4e7a-404d-9389-f8b32e1a2cfb")
        );
        assert_eq!(a.state, "connected");
        assert_eq!(a.ssid, "BELL409");
        assert_eq!(a.band, "5 GHz");
        assert_eq!(a.channel, Some(44));
        assert_eq!(a.radio_type, "802.11n");
        assert_eq!(a.authentication, "WPA2-Personal");
        assert_eq!(a.cipher, "CCMP");
        assert_eq!(a.rssi, Some(-64));
        assert_eq!(a.signal_pct, Some(80));
    }

    #[test]
    fn parses_this_box_captured_drivers() {
        let out = r"

Interface name: Wi-Fi

    Driver                    : Intel(R) Dual Band Wireless-AC 8265
    Type                      : Native Wi-Fi Driver
    Radio types supported     : 802.11b 802.11g 802.11n 802.11a
    Hosted network supported  : No
    Infrastructure mode supported : Yes
    Ad hoc mode supported        : No
    Authentication and cipher supported in infrastructure mode:
";
        let v = parse_drivers(out);
        assert_eq!(v.len(), 1);
        let d = &v[0];
        assert_eq!(d.description, "Intel(R) Dual Band Wireless-AC 8265");
        assert_eq!(d.driver_type, "Native Wi-Fi Driver");
        assert_eq!(
            d.radio_types,
            vec!["802.11b", "802.11g", "802.11n", "802.11a"]
        );
        assert!(!d.hosted_network);
    }

    #[test]
    fn empty_input_yields_no_rows() {
        assert!(parse_interfaces("").is_empty());
        assert!(parse_drivers("").is_empty());
    }

    #[test]
    fn probe_returns_adapter_on_windows() {
        #[cfg(windows)]
        assert!(!super::probe().is_empty());
        #[cfg(not(windows))]
        assert!(super::probe().is_empty());
    }
}
