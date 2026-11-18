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

## ADO-3 (2026-11-18, live — owner go)

- **Hotspot LIVE**: SSID `GSV-Peers-6442` (auto-generated), band **2.4 GHz**, auth **WPA2-PSK**, shared profile **BELL409**, gateway **192.168.137.1**, `client_count=0` (waiting for phone onboarding).
- Hub досяжний з AP-підмережі: `http://192.168.137.1:9999/api/health` → **200** (`/api/wgpu/mesh` → 200 з даними про піри).
- SSID/пароль згенеровано й застосовано через plugin (staged→persisted у `data/adoreble.json` — **gitignored**, пароль ніде не логувався). SSID згенеровано як було заплановано (`GSV-Peers-xxxx`).
- **Онбординг пірів**: `data/peer-join.html` (SVG QR: Wi-Fi join + mesh page `http://192.168.137.1:9999/wgpu`) — один раз відсканувати кожним телефоном; далі Android сам перепідключається.
- **ADB zero-touch**: недоступний — `adb devices` порожній (потрібне увімкнене Wireless debugging на телефонах + повторний pairing).
- Відкат: `adoreble-wifi hotspot stop` (STA-профіль BELL409 не чіпається).
- Pi 4 — не чіпали (рішення owner: канал C).

## ADO-2 (2026-11-18, пізно)

- `adoreble-wifi probe-hub` / `POST /api/probe-hub` — read-only HTTP GET `/api/health` для кандидатів (LAN-first). На момент ADO-2 хаб не відповідав (timeout); на момент ADO-3 хаб живий (200 на `:9999` і на `192.168.137.1:9999`).
- Saved profiles знайдено через `netsh wlan show profiles` (parser у плагіні).
- Live LAN-перемикання досі НЕ виконується (`netsh wlan connect` тільки в dry-run); ADO-3 свідомо обмежений хотспотом.


## Rules

- Nothing on this page was enabled or changed — probes are read-only WinRT/netsh calls.
- ADO-3 must be owner-gated: it changes host networking (Windows ICS NAT, SSID/password,
  `192.168.137.0/24`).
