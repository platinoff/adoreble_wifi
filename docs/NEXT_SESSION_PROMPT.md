# adoreble_wifi — NEXT_SESSION_PROMPT

**Відкрий сесію на `S:/rust/adoreble_wifi`** (або GSV workspace з цим рядком).

1. `git status -sb` · `git log -1 --oneline` · прочитай [`docs/HANDOFF_NEW_SESSION.md`](./HANDOFF_NEW_SESSION.md).
2. S0 disk gate: `cargo xtask disk` якщо є `target/xtask` — або `df -h /s` (S: вільно ~70 GB).
3. ADO-1 ✅ (`60cdf60`, matrix у `docs/WIFI_MATRIX.md`); ADO-2 ✅ (dry-run: `adoreble-wifi probe-hub`, `adoreble-wifi release --to <SSID>` — **мережа не змінюється**; fail-open policy `evaluate_release` у ядрі).
4. ADO-3 (Hotspot, WinRT `StartTetheringAsync`) — **owner-gate**: змінює host networking (Windows ICS NAT, `192.168.137.0/24`). Перед бандом уточни: (a) go на AC-8265; (b) SSID + password політика; (c) `auto_start` / `auto_release` on/off.
5. Кожний банд = один commit + push; CI зелений; GSV-тікет ADO-* з evidence в `docs/gsv/tickets.jsonl`.
