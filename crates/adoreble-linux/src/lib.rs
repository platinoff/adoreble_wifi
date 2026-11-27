//! Linux/Pi4 backend — read-only Wi-Fi interface detection via sysfs (ADO-5 phase 1).
//!
//! Scope deliberately narrow: enumerate `/sys/class/net/<if>` entries that
//! carry a `wireless/` or `phy80211` marker and read `operstate`. This makes
//! no system changes and does not claim AP capability. Actual AP setup via
//! nl80211/hostapd stays a separate, owner-gated band.

use adoreble_core::AdapterCapabilities;
#[cfg(any(target_os = "linux", test))]
use std::path::Path;

/// Pure, host-independent reader: `net_dir` is normally `/sys/class/net`.
/// Interfaces without a wireless marker are skipped. Unknown capabilities
/// are reported as false/None/empty rather than guessed.
#[cfg(any(target_os = "linux", test))]
fn probe_from_net_dir(net_dir: &Path) -> Vec<AdapterCapabilities> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(net_dir) else {
        return out;
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort(); // deterministic ordering for callers/tests
    for name in names {
        let dir = net_dir.join(&name);
        let is_wifi = dir.join("wireless").exists() || dir.join("phy80211").exists();
        if !is_wifi {
            continue;
        }
        let operstate = std::fs::read_to_string(dir.join("operstate"))
            .unwrap_or_default()
            .trim()
            .to_owned();
        let description = if operstate.is_empty() {
            format!("Linux Wi-Fi interface {name} (sysfs)")
        } else {
            format!("Linux Wi-Fi interface {name} (sysfs, operstate={operstate})")
        };
        out.push(AdapterCapabilities {
            name,
            description,
            guid: None,                  // non-Windows: no GUID
            bands: Vec::new(),           // sysfs does not expose an honest band list here
            hosted_network: false,       // AP capability needs an nl80211 probe — not done
            hotspot_via_tethering: None, // WinRT-only concept
            concurrent: None,
            wpa3: false, // unknown from sysfs; never claim it
        });
    }
    out
}

/// Read-only detection on Linux. No device configuration, no hostapd control.
#[cfg(target_os = "linux")]
pub fn probe() -> Vec<AdapterCapabilities> {
    probe_from_net_dir(Path::new("/sys/class/net"))
}

/// Other hosts have no Linux sysfs; keep the honest empty result.
#[cfg(not(target_os = "linux"))]
pub fn probe() -> Vec<AdapterCapabilities> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn probe_is_empty_scaffold() {
        assert!(probe().is_empty());
    }

    fn fixture_root() -> std::path::PathBuf {
        let root = std::env::current_dir()
            .expect("cwd")
            .join("target")
            .join(format!("ado5_sysfs_{}", std::process::id()));
        let net = root.join("sys/class/net");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(net.join("wlan0/wireless")).expect("wlan0 wireless marker");
        std::fs::write(net.join("wlan0/operstate"), "up\n").expect("operstate");
        std::fs::create_dir_all(net.join("eth0")).expect("eth0");
        std::fs::write(net.join("eth0/operstate"), "up\n").expect("eth0 operstate");
        root
    }

    /// The pure helper runs on every host: only the wireless-marked
    /// interface is reported, with an honest unknowns profile.
    #[test]
    fn sysfs_probe_detects_only_wireless_interfaces() {
        let root = fixture_root();
        let found = probe_from_net_dir(&root.join("sys/class/net"));
        assert_eq!(found.len(), 1, "{found:?}");
        let wlan = &found[0];
        assert_eq!(wlan.name, "wlan0");
        assert!(
            wlan.description.contains("operstate=up"),
            "{}",
            wlan.description
        );
        assert!(wlan.bands.is_empty() && wlan.guid.is_none());
        assert!(!wlan.hosted_network && !wlan.wpa3);
        assert!(wlan.hotspot_via_tethering.is_none() && wlan.concurrent.is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn sysfs_probe_on_missing_dir_is_empty_not_panic() {
        let root = fixture_root();
        assert!(probe_from_net_dir(&root.join("does-not-exist")).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }
}
