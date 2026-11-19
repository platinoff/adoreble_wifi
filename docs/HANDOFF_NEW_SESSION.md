# adoreble_wifi — HANDOFF_NEW_SESSION

## Сталий стан (2026-11-18 — ADO-3 Hotspot LIVE)

| Що | Значення |
|---|---|
| Repo | `S:/rust/adoreble_wifi` → `github.com/platinoff/adoreble_wifi` (**MIT, public**, owner 2026-11-18) |
| Crate | **0.1.0** — workspace: `adoreble-core` · `adoreble-windows` · `adoreble-linux` · `adoreble-api` · `adoreble-cli` · `adoreble-ui` (wasm-bindgen cdylib) · `adoreble-xtask` |
| Rust | **1.100.0** (host default = `stable-x86_64-pc-windows-gnu`; CI: ubuntu=`stable`, windows=`stable-x86_64-pc-windows-msvc`) |
| API | **loopback `127.0.0.1:9770`** — `GET /health` · `GET /api/{status,adapters,settings,settings/stage,settings/apply}` · токен POSTs (`x-adoreble-token`; файл `data/adoreble.token`, 0600 unix) |
| Безпека | loopback-only; settings **staged-in-memory → apply** (POST `/api/settings/apply`); паролі тільки в `data/adoreble.json` (gitignored, user-only); redact у всьому виводі |
| Статус | **ADO-3 Hotspot LIVE** — SSID `GSV-Peers-6442` · 2.4 GHz · WPA2 · шлюз `192.168.137.1` · хаб `:9999` reachable з AP-підмережі · CLI `hotspot {start,stop,status,qr}` + API `/api/hotspot/*` · пірів очікуємо (QR-онбординг) |
| **ADO-6 Auto-start DONE 2026-11-19** | Бут API підіймає хотспот (`auto_start` + enabled + mode=hotspot) · SLA watchdog (30с probe → auto-release на STA при 3× fail × threshold) · **autostart task**: `cargo run --bin xtask autostart install|uninstall|status` (HKCU Run або schtasks) · **25 тестів green** · встановлено на цьому боксі (HKCU Run AdorebleWiFi) |
| Owner-рішення | канал **(C)** WinRT Tethering (не Pi 4); `auto_start=on` + **Save/Apply** замість live-мутації; **MIT**; ADO-3 go — хотспот + авто-підключення пірів |
| ADO-2 | dry-run LAN-client scaffold — `probe-hub` / `POST /api/probe-hub`, `ReleasePlan`, `evaluate_release`; live `netsh wlan connect` не виконується |
| План | [`GSV docs/superpowers/plans/2026-11-18-adoreble-wifi-plan.md`](../../GSV/docs/superpowers/plans/2026-11-18-adoreble-wifi-plan.md) |

## Наступні кроки

| Банд | Опис |
|---|---|
| **ADO-1** | ✅ done (`60cdf60`) — capability matrix у `docs/WIFI_MATRIX.md`; AC-8265: hosted-network No, WinRT Tethering **Enabled**, MaxClientCount 8 |
| **ADO-2** | ✅ done (dry-run only) — `adoreble-wifi probe-hub` / `POST /api/probe-hub`, `ReleasePlan`, `evaluate_release`; **live switch (`netsh wlan connect`) не виконується — owner-gate** |
| **ADO-3** | ✅ **LIVE** — хотспот `GSV-Peers-6442` (2.4 GHz/WPA2, BELL409 shared); хаб reachable з AP-підмережі (`192.168.137.1:9999`); `client_count=0` до онбордингу пірів (QR у `data/peer-join.html`); відкат — `hotspot stop` |
| **ADO-4** | Web UI (WASM/HTML на `127.0.0.1:9770/ui/`, темна тема, QR-набір, SSE логи) |
| **ADO-5** | Pi 4 relay (`hostapd` 4addr br0) — делегується у пізніший банд (owner: Pi не чіпати зараз) |
| **ADO-6** | auto-start (schtask ONLOGON / systemd user unit), SLA watchdog auto-release, GSV PRODUCTS.md row |

## Важливо

- ADO-3 працює у **Live** — хотспот працює; `peer-join.html` містить Wi-Fi пароль — видалити після онбордингу пірів.
- ADO-2 (`netsh wlan connect`) навмисно не реалізований — ADO-3 обмежився хотспотом за рішенням owner.
- **Паролі ніколи не комітимо** — `data/` у `.gitignore`; `data/adoreble.token` створюється на першому старті.
- GSV kit: global-first [`GOBALSkit`](../../GSV/AGENTS.md); MSYS2 bash only; ніколи не kill `S:/rust/GSV/target/live/`.
