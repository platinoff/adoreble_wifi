# adoreble_wifi — NEXT_SESSION_PROMPT

**Відкрий сесію на `S:/rust/adoreble_wifi`** (або GSV workspace з цим рядком).

1. `git status -sb` · `git log -1 --oneline` · прочитай [`docs/HANDOFF_NEW_SESSION.md`](./HANDOFF_NEW_SESSION.md). **Steady state 2026-11-27: HEAD `eb471e3`, clean/pushed; ADO-5 phase 1 (read-only sysfs probe) done — не повторюй. Phase 2 AP (`nl80211`/`hostapd`) owner-gated.**
2. `adoreble-wifi hotspot status` → очікується `operational_state: on`, `client_count >= 1` після онбордингу пірів.
3. Онбординг пірів (ще не зроблено): відкрий `data/peer-join.html` у браузері на цьому ноутбуці → кожен телефон сканує **Wi-Fi QR** (`GSV-Peers-6442`) → потім **mesh QR** (`http://192.168.137.1:9999/wgpu`) — пір сам зареєструється в хабі. Після онбордингу видали `data/peer-join.html` (у ньому пароль).
4. Перевірка пірів у хабі: `curl -s http://192.168.137.1:9999/api/wgpu/mesh` — `client_count` у `hotspot status` і mesh-список оновляться.
5. ADO-6 ✅ встановлено: `cargo run --bin xtask autostart status` (HKCU Run `AdorebleWiFi` → **Rust `xtask.exe watch`**, без VBS/WSH); uninstall/install за потреби. SLA watchdog: 3× fail хаба → хотспот stop + `netsh wlan connect` на `release_ssid` (BELL409). WSH-вікно ребуту 2026-11-25 (`800A01F5`, `log` — зарезервоване слово VBScript) усунено: помилки/респавн пишуться в `data/adoreble-watchdog.log`, діалогів немає.
6. ADO-4 ✅ UI: відкрий http://127.0.0.1:9770/ui (токен у `data/adoreble.token`). Перший запуск API тепер завантажує збережені налаштування.
7. Відкат: `adoreble-wifi hotspot stop` (STA `BELL409` не чіпається); `xtask autostart uninstall` прибирає автозапуск.
8. Кожний банд = один commit + push; CI зелений.
