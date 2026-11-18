# adoreble_wifi — HANDOFF_NEW_SESSION

## Сталий стан (2026-11-18 — ADO-0 закрито)

| Що | Значення |
|---|---|
| Repo | `S:/rust/adoreble_wifi` → `github.com/platinoff/adoreble_wifi` (**MIT, public**, owner 2026-11-18) |
| Crate | **0.1.0** — workspace: `adoreble-core` · `adoreble-windows` · `adoreble-linux` · `adoreble-api` · `adoreble-cli` · `adoreble-ui` (wasm-bindgen cdylib) · `adoreble-xtask` |
| Rust | **1.100.0** (host default = `stable-x86_64-pc-windows-gnu`; CI: ubuntu=`stable`, windows=`stable-x86_64-pc-windows-msvc`) |
| API | **loopback `127.0.0.1:9770`** — `GET /health` · `GET /api/{status,adapters,settings,settings/stage,settings/apply}` · токен POSTs (`x-adoreble-token`; файл `data/adoreble.token`, 0600 unix) |
| Безпека | loopback-only; settings **staged-in-memory → apply** (POST `/api/settings/apply`); паролі тільки в `data/adoreble.json` (gitignored, user-only); redact у всьому виводі |
| Статус | **ADO-0 scaffold DONE** — `cargo fmt --check` ✅ · clippy `-D warnings` ✅ · tests **8 pass** ✅ · CI `.github/workflows/ci.yml` (ubuntu+windows) у дорозі |
| Власник-рішення | канал **(C)** WinRT Tethering (не Pi 4); `auto_start=on` + **Save/Apply** замість live-мутації; **MIT**; ADO-1 probe |
| План | [`GSV docs/superpowers/plans/2026-11-18-adoreble-wifi-plan.md`](../../GSV/docs/superpowers/plans/2026-11-18-adoreble-wifi-plan.md) |

## Наступні кроки

| Банд | Опис |
|---|---|
| **ADO-1** | capability matrix probe (`netsh wlan show drivers`, WinRT `TetheringManager.getOperationalState`, concurrent bands) → `docs/WIFI_MATRIX.md` |
| **ADO-2** | `LanClient` + probe-hub + `ReleaseFallback` (`netsh wlan connect`, `gsv-hub` адресавтодетект) |
| **ADO-3** | `Hotspot` (WinRT TetheringManager, SSID/password gen, WPA2-AES default; WPA3 якщо піри ок) |
| **ADO-4** | Web UI (WASM/HTML на `127.0.0.1:9770/ui/`, темна тема, QR-набір, SSE логи) |
| **ADO-5** | Pi 4 relay (`hostapd` 4addr br0) — делегується у пізніший банд (owner: Pi не чіпати зараз) |
| **ADO-6** | auto-start (schtask ONLOGON / systemd user unit), SLA watchdog auto-release, GSV PRODUCTS.md row |

## Важливо

- Жодного live-networking коду поки нема — ADO-0 це scaffold; ADO-1 перший, що читає реальну картку.
- **Паролі ніколи не комітимо** — `data/` у `.gitignore`; `data/adoreble.token` створюється на першому старті.
- GSV kit: global-first [`GOBALSkit`](../../GSV/AGENTS.md); MSYS2 bash only; ніколи не kill `S:/rust/GSV/target/live/`.
