# ADR-002 — settings staged in memory, applied on Save

**Status:** accepted (owner, 2026-11-18).

## Decision

Settings live in memory in `AppState.settings`; `/api/settings/stage`
replaces the in-memory copy and marks it `staged=true`; nothing touches
the disk or the adapter until `/api/settings/apply` is called:

1. validate (password length, channel, band);
2. persist to `data/adoreble.json` (gitignored, user-only file);
3. only then apply to the host.

Live mutation (`stage == apply`) is explicitly rejected by the UI contract
and by the HTTP API.

## Consequences

- `/health` and `/api/status` always reflect the *applied* config plus
  `staged=true` flag; the UI shows an "unsaved changes" pill.
- A crash between stage and apply loses nothing — the disk file is the
  last authority.
