# ADR-001 — loopback-only API and token auth (security)

**Status:** accepted (owner, 2026-11-18).

## Decision

`adoreble-api` binds `127.0.0.1:9770` only. All mutating routes
(`POST /api/settings/stage|apply`) require a per-host random token
created on first start in `data/adoreble.token` (user-read-only).
Read-only `GET` routes may be consumed from any loopback client.
No routable bind is ever allowed without a new owner-gated ADR.

## Consequences

- The web UI lives behind the same loopback origin; the token is supplied
  by the operator (never embedded in HTML, never committed).
- Phones/mesh peers never reach `adoreble-api`; they reach the GSV hub.
- Any incident without a GSV owner present cannot be triggered remotely.
