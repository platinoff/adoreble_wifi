# adoreble_wifi — NEXT_SESSION_PROMPT

**Відкрий сесію на `S:/rust/adoreble_wifi`** (або GSV workspace з цим рядком).

1. `git status -sb` · `git log -1 --oneline` · прочитай [`docs/HANDOFF_NEW_SESSION.md`](./HANDOFF_NEW_SESSION.md).
2. S0 disk gate: `cargo xtask disk` якщо є `target/xtask` — або `df -h /s` (S: вільно ~70 GB).
3. ADO-1: **capability probe** — перший банд з реальними даними:
   - `netsh wlan show interfaces` → Adapter, GUID, bands, channel, RSSI
   - `netsh wlan show drivers` → hosted network support (очікуємо `No` на AC-8265)
   - WinRT `NetworkOperatorTetheringManager.getOperationalState()` через маленький probe bin
   - вихід: `docs/WIFI_MATRIX.md` (адаптер·band·AP-capable·concurrent·WPA3) + тест на моках
4. Після ADO-1 — ADO-2 (`LanClient` + probe hub) або чекати owner'а по ADO-3 (Hotspot через WinRT Tethering).
5. Кожний банд = один commit + push; CI зелений; GSV-тікет з evidence в `docs/gsv/tickets.jsonl`.
