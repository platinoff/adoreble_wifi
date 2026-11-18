//! Windows backend (ADO-0 scaffold: probe returns the known host adapter;
//! live probes land in ADO-1 via WinRT `NetworkOperatorTetheringManager`
//! and `netsh`/`WlanApi`).

use adoreble_core::AdapterCapabilities;

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

#[cfg(test)]
mod tests {
    #[test]
    fn probe_returns_adapter_on_windows() {
        #[cfg(windows)]
        assert!(!super::probe().is_empty());
        #[cfg(not(windows))]
        assert!(super::probe().is_empty());
    }
}
