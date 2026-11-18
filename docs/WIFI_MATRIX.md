# WIFI capability matrix (ADO-1 probe, 2026-11-18, live)

**Host:** `Intel(R) Dual Band Wireless-AC 8265` · GUID `1bf194dd-4e7a-404d-9389-f8b32e1a2cfb` · Windows 11 (build 26200) · on `BELL409` (5 GHz, ch 44, WPA2-Personal).

| Probe | Result | Source |
|---|---|---|
| `netsh wlan show interfaces` | Wi-Fi connected, SSID BELL409, 5 GHz ch44, RSSI -67 dBm, signal 72 %, 240 Mbps | `adoreble-wifi scan` |
| `netsh wlan show drivers` | Type: Native Wi-Fi Driver · Radio: 802.11b/g/n/a · **Hosted network supported: No** | same |
| WinRT `GetTetheringCapabilityFromConnectionProfile` | **`Enabled` (TetheringCapability(0))** for every connection profile incl. `BELL409` | same (`NetworkInformation::GetConnectionProfiles`) |
| WinRT `TetheringOperationalState` | `Off (2)` — hotspot not running | same |
| WinRT `MaxClientCount` | **8** | same |
| WinRT `ClientCount` | 0 | same |

**Conclusion:** legacy Hosted Network/SoftAP is dead on this card (expected), **but the
supported path — WinRT Mobile Hotspot (`NetworkOperatorTetheringManager`) — is available and
eagerly enabled.** ADO-3 (Hotspot) is viable on this host without new hardware.

**Profiles currently visible to the host:** `BELL409` (current), `Bluetooth Network Connection`,
`BunkeR_net`, `Mesh`, `MeshAlt`, `Annear` (+ others) — tethering capability enabled on all.

## ADO-2 (2026-11-18, пізно)

- `adoreble-wifi probe-hub` → read-only HTTP GET `/api/health` для кандидатів (LAN-first). На момент перевірки **hub не відповідає** (timeout ×2: `192.168.2.238` і `127.0.0.1`) — хаб падає від RAM; P0 власника.
- `POST /api/probe-hub` (token-gated) на loopback API — те саме для UI/автоматизації.
- Saved profiles знайдено через `netsh wlan show profiles` (parser у плагіні).
- **Live-перемикання (`netsh wlan connect`) НЕ виконується** — тільки сухий план (`adoreble-wifi release`).ADO-3 потребує owner-gate.

## Rules

- Nothing on this page was enabled or changed — probes are read-only WinRT/netsh calls.
- ADO-3 must be owner-gated: it changes host networking (Windows ICS NAT, SSID/password,
  `192.168.137.0/24`).
