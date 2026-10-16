# ADR: Local development workbench authentication

> **Last updated**: 2026-10-15
> **Audience**: Lab contributors and reviewers
> **Type**: Architecture decision record
> **Status**: Implemented boundary in DW-02A and local shell/API facade in DW-02B
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
a missing or different instance is refused at the browser gate. Every future API route must stay inside this gate. A JSON mutation handler must also compare its typed body `instance` with the bound label, even when the URL query matches. Same-origin page JS obtains
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

DW-02B serves the static shell, stylesheet and script inside the same
cookie gate. `GET /workbench/api/v1/session?instance=<label>` returns a
versioned `{schema_version,context,data,gaps}` response with the bound
instance, CSRF token, client status and lease status under `data`. Missing
client observations appear in `gaps`. The shell reads status from this
route and keeps view selection in per-instance browser session storage.

`POST /workbench/api/v1/input/key?instance=<label>` is the first typed
mutation facade. The browser gate checks the query, Origin and CSRF;
the handler independently checks the body instance and the named lab
lease before calling the existing supervisor input method. API JSON
replies use the same versioned envelope. A retained PID alone does not
mean a client is running; the shell reads `running` and `uptime_state`.
