# ADR: Local development workbench authentication

> **Last updated**: 2026-10-15
> **Audience**: Lab contributors and reviewers
> **Type**: Architecture decision record
> **Status**: Implemented boundary in DW-02A; shell and API pending DW-02B
> **Companion docs**: [live-research-lab.md](live-research-lab.md), [../analysis/dev-workbench/README.md](../analysis/dev-workbench/README.md)

## Context

The shared lab daemon already exposes bearer-authenticated `/mcp` and
`/status`. A local browser companion needs a session but must never see
the daemon bearer or bridge secret. An ordinary browser cookie is ambient
authority, so its surface also needs exact local-origin and CSRF checks.

## Decision

The daemon has separate route groups. `/mcp` and `/status` keep the bearer
middleware; `/mcp` keeps rmcp's loopback Host and browser-Origin rejection.
A local CLI caller with the bearer can `POST /workbench/session/open` to
mint a 60-second, one-use random opener path. That endpoint rejects any
Origin header. Visiting the path consumes it and redirects to
`/workbench/` with an HttpOnly, SameSite=Strict cookie scoped to
`/workbench/`. A cookie cannot authenticate MCP, status, or the opener.

The cookie is valid only in the daemon process and expires after eight
hours; restart invalidates it. Every workbench request requires a
loopback Host with a port. An Origin, when present, must exactly match
`http://` plus that Host. Mutations require that exact Origin and a
session-specific `X-Cimmeria-CSRF` header. Same-origin page JS obtains
the CSRF value from the cookie-gated `GET /workbench/session` and never
reads the HttpOnly cookie. The workbench's responses carry
`Referrer-Policy: no-referrer`, `Content-Security-Policy: default-src
'self'`, and `Cache-Control: no-store`. Neither the opener path nor
cookie is logged.

The static shell and its API facade arrive in DW-02B. No workbench
request reaches the game client in this packet. The daemon's existing
lab lease rules will gate the later mutating API facade.
