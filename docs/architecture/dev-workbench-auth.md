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
A local CLI caller with the bearer can `POST /workbench/session/open`
with `{ "instance": "<hosted label>" }` to mint a 60-second, one-use
random opener path. Unknown labels are refused. The nonce retains the
canonical hosted label, and redemption binds it to the browser cookie.
That endpoint rejects any
Origin header. Visiting the path consumes it and redirects to
`/workbench/` with an HttpOnly, SameSite=Strict cookie scoped to
`/workbench/`. A cookie cannot authenticate MCP, status, or the opener.

The cookie is valid only in the daemon process and expires after eight
hours; restart invalidates it. Every workbench request requires a
loopback Host with a port. An Origin, when present, must exactly match
`http://` plus that Host. Mutations require that exact Origin and a
session-specific `X-Cimmeria-CSRF` header. The cookie-gated
`GET /workbench/session?instance=<label>` returns the bound label and
CSRF value. Session and future API requests must name the bound label;
a missing or different instance is refused at the browser gate. Same-origin page JS obtains
the CSRF value there and never
reads the HttpOnly cookie. The workbench's responses carry
`Referrer-Policy: no-referrer`, `Content-Security-Policy: default-src
'self'`, and `Cache-Control: no-store`. Neither the opener path nor
cookie is logged.

Refusals write a static reason code (`host_invalid`, `origin_invalid`,
`cookie_invalid`, `csrf_invalid`, `nonce_invalid`, `instance_mismatch`,
or `instance_unknown`) under `lab.workbench.auth`. Request paths,
cookies, CSRF values, bearer tokens, and submitted instance strings are
not logged.

The static shell and its API facade arrive in DW-02B. No workbench
request reaches the game client in this packet. The daemon's existing
lab lease rules will gate the later mutating API facade.
