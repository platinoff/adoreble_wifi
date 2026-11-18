# Adoreble Wi-Fi

> GSV-ecosystem Wi-Fi control plugin: loopback-only control plane for scanning,
> switching and (eventually) running a managed Wi-Fi topology
> (client / hotspot / bridge) for the GSV mesh peers.

**Status:** scaffold (ADO-0). Not yet a network-mutating tool — see
[`docs/superpowers/plans/2026-11-18-adoreble-wifi-plan.md`](https://github.com/platinoff/GSV/blob/main/docs/superpowers/plans/2026-11-18-adoreble-wifi-plan.md).

## Design

- **Loopback only** (`127.0.0.1:9770`); token-authenticated POSTs; read-only on the wire by default.
- **Rust 1.100 workspace**, Windows-first (`ADO-0..ADO-3` on `Intel AC-8265` host), Linux/Pi4 backend for `Bridge` mode.
- **Fail-open**: any SLA drop ⇒ revert to the last-known-good profile.
- **Settings staged in memory, applied on Save** — never live-mutated.
- **Secrets never persisted** in a committed file; runtime reads via the host
  (`netsh wlan show profile ... key=clear` on Windows) and are redacted in every
  log line.
- MIT licensed.

## Workspace

| Crate | Purpose |
|---|---|
| `adoreble-core` | domain model: `WifiMode`, `AdapterCapabilities`, `Settings` (stage/apply) |
| `adoreble-windows` | Windows backend (WinRT `NetworkOperatorTetheringManager` + `netsh` + WlanApi) |
| `adoreble-linux` | Linux/Pi4 backend (`nl80211` / `hostapd` / `wpa_supplicant`) |
| `adoreble-api` | loopback HTTP API (axum, `127.0.0.1:9770`) |
| `adoreble-cli` | `adoreble-wifi` binary (`scan | status | up | down | setup-token`) |
| `adoreble-ui` | WebAssembly/thin UI glue (browser, served from the API) |
| `adoreble-xtask` | `cargo xtask verify` (fmt + clippy + test) |

## License

MIT — see [LICENSE](./LICENSE).
