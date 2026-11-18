# AGENTS.md — adoreble_wifi

GSV-ecosystem **plugin** (own repo, own target). Global kit rules live in
[`S:/rust/GSV/AGENTS.md`](file:///s/rust/GSV/AGENTS.md) — read it first; this file covers
**product specifics only**.

## Invariants (do not loosen)

- **Loopback only.** The API binds `127.0.0.1:9770`. Nothing ever listens on a
  routable address without an explicit owner gate.
- **No live mutation without Save.** Settings are staged in memory; `apply`
  persists + applies. Never mutate the adapter at the stage step.
- **Fail-open.** Any SLA failure against the GSV hub ⇒ revert to the
  last-known-good profile (`LanClient`) and log it. Never leave the host offline.
- **No secrets in the repo.** Passwords are read from the OS/API at runtime,
  never written to committed files; logs always redact.
- **No roaming to Windows/system temp.** Scratch lives in the repo or under `target/`.

## Stack

- Rust **1.100.0** (stable, host default), edition **2024**.
- Windows: WinRT `Windows.Networking.NetworkOperators` (`NetworkOperatorTetheringManager`)
  for the Hotspot path; `netsh wlan` for client profile switching.
- Linux: `nl80211` / `hostapd` control sockets (`adoreble-linux`).
- **Never GitHub MCP** tools against this repo without owner instruction;
  GSV kit canonical `S:/rust/GSV/AGENTS.md` rules still apply for `cargo xtask`,
  MSYS2 bash, and one-commit-per-drain.
- **Do not** commit `data/`, `.env*`, keys, or probe output containing passwords.
