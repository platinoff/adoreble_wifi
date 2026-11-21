# adoreble_wifi — FUNCTION_MANAGEMENT (симбіоз)

**Companion:** `S:/rust/GSV/docs/gsv/GSV_FUNCTION_MANAGEMENT.md` · [`GSV_ECOSYSTEM_SYMBIOSIS.md`](../../GSV/docs/gsv/GSV_ECOSYSTEM_SYMBIOSIS.md) · [`GSV_AGI_PATH.md`](../../GSV/docs/gsv/GSV_AGI_PATH.md)  
**Шар:** plugin (Wi-Fi/AP control) · **Repo:** `https://github.com/platinoff/adoreble_wifi` · **Branch:** `main`

## 1. Оголошені функції

| Функція | Модулі/шляхи | Статус | Примітка |
|---------|--------------|--------|----------|
| Wi-Fi hotspot/AP control (WinRT tethering lane) | `crates/adoreble-windows`, core/api | active/dev | ADO-0..ADO-6 done per product handoff |
| Linux adapter lane | `crates/adoreble-linux` | scaffold/partial | Platform-specific follow-up |
| Loopback API + CLI + UI | `adoreble-api`, `adoreble-cli`, `adoreble-ui` | active | Token-gated mutating endpoints; localhost bind |
| xtask/ops | `adoreble-xtask`, `scripts/` | active | Rust-first ops; no secrets in git |

## 2. Звірка / gates

- Rust gates: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (CI: ubuntu-latest/windows-latest).
- 2026-11-21: windows-latest green; ubuntu-latest has pre-existing clippy failure (not from FM doc).  
- Network-changing operations remain owner-gated in live ops.

## 3. Дублі / межі (посилання на master §3)

- GSV hub owns mesh/tickets/keep-live; adoreble_wifi owns Wi-Fi/AP product runtime.
- No secrets/tokens committed; runtime config/data stays local.

## 4. Owner-рішення / наступні кроки

- Fix pre-existing ubuntu-latest clippy failure separately (CI evidence needed).
- Update skeleton as ADO-7+ / Linux lane progresses.
