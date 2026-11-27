# adoreble_wifi — HANDOFF_NEW_SESSION

## Сталий стан (2026-11-27 — HEAD `eb471e3` clean/pushed; ADO-3 Hotspot LIVE; WSH/VBS вотчер замінено на Rust; ADO-5 phase 1 додано)

| **Silent watcher 2026-11-25 (`e738362`)** | `scripts/adoreble-watch.vbs` видалено: після ребуту WSH показував діалог `Illegal assignment: 'log'` (800A01F5, рядок 5 — `Log` зарезервоване в VBScript). `adoreble-xtask watch` тепер сам пробує `127.0.0.1:9770/health`, при потребі тихо респавнить `adoreble-api.exe` (`CREATE_NO_WINDOW`) і пише в `data/adoreble-watchdog.log` (gitignored). HKCU Run `AdorebleWiFi` → `"...\xtask.exe" watch`; жодних системних вікон. `cargo xtask verify` green. |

| **ADO-5 phase 1 (2026-11-27, `eb471e3`)** | `crates/adoreble-linux/src/lib.rs`: read-only sysfs Wi-Fi probe — чистий `probe_from_net_dir(&Path)` знаходить `/sys/class/net/<if>` з маркером `wireless/`/`phy80211`, читає `operstate`; Linux `probe()` викликає його, інші хости лишаються порожніми. Невідомі можливості чесно `false`/`None`/порожньо (guid None, bands [], hosted_network false, wpa3 false). TDD: синтетичне sysfs-дерево в `target/` (wlan0 знайдено, eth0 пропущено, missing dir → порожньо) — 3 тести green. Гейти: fmt/clippy `-D warnings` RC=0, `cargo test` EXIT=0 (локально gnu-toolchain; CI без змін). **AP через `nl80211`/`hostapd` (phase 2) не робився — owner-gated системні зміни.** |
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
