# adoreble_wifi — NEXT_SESSION_PROMPT

**Відкрий сесію на `S:/rust/adoreble_wifi`** (або GSV workspace з цим рядком).

1. `git status -sb` · `git log -1 --oneline` · прочитай [`docs/HANDOFF_NEW_SESSION.md`](./HANDOFF_NEW_SESSION.md).
2. `adoreble-wifi hotspot status` → очікується `operational_state: on`, `client_count >= 1` після онбордингу пірів.
3. Онбординг пірів (ще не зроблено): відкрий `data/peer-join.html` у браузері на цьому ноутбуці → кожен телефон сканує **Wi-Fi QR** (`GSV-Peers-6442`) → потім **mesh QR** (`http://192.168.137.1:9999/wgpu`) — пір сам зареєструється в хабі. Після онбордингу видали `data/peer-join.html` (у ньому пароль).
4. Перевірка пірів у хабі: `curl -s http://192.168.137.1:9999/api/wgpu/mesh` — `client_count` у `hotspot status` і mesh-список оновляться.
5. True zero-touch (`adb shell cmd wifi connect-network`) — лише за повторно увімкненого **Wireless debugging** на телефонах + `adb pair` (зараз `adb devices` порожній).
6. ADO-4 (Web UI на `127.0.0.1:9770/ui/` з тими ж QR + SSE логами) — за потреби owner; ADO-6 (auto-start schtask + SLA auto-release) — теж owner-gate.
7. Відкат: `adoreble-wifi hotspot stop` (STA `BELL409` не чіпається).
8. Кожний банд = один commit + push; CI зелений.
