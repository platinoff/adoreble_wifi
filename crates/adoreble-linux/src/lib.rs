//! Linux/Pi4 backend (ADO-0 stub; nl80211/hostapd/wpa_supplicant in ADO-5).

use adoreble_core::AdapterCapabilities;

/// Linux probes require a `/sys/class/net/<if>/wireless` presence; the
/// Pi4 relay path is out of scope for this host (owner decision 2026-11-18).
#[cfg(not(windows))]
pub fn probe() -> Vec<AdapterCapabilities> {
    // Honest stub: no Linux Wi-Fi probing on this host yet (ADO-5).
    Vec::new()
}

#[cfg(windows)]
pub fn probe() -> Vec<AdapterCapabilities> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    #[test]
    fn probe_is_empty_scaffold() {
        assert!(super::probe().is_empty());
    }
}
