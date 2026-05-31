# Security Policy

Cimmeria is a fan-made server emulator for the cancelled Stargate Worlds MMO. It is a hobby project — not a production service — but it does process player credentials, run a database, and expose network listeners, so we take vulnerabilities seriously.

## Reporting a vulnerability

**Please do not report security vulnerabilities via public GitHub issues.**

If you believe you've found a security issue in Cimmeria, please report it privately so we can review and fix it before any public disclosure. Two routes:

1. **GitHub private vulnerability reporting** (preferred) — open a draft advisory at <https://github.com/SandboxServers/Cimmeria/security/advisories/new>. This keeps the report visible only to maintainers.
2. **Email** — send a description to **<steven.cady@gmail.com>** with `[Cimmeria Security]` in the subject line.

Please include:

- A description of the issue and the impact you believe it has.
- Steps to reproduce, or a proof-of-concept if you have one.
- The commit SHA or release tag you tested against.
- Any logs, packet captures, or test artifacts that help us reproduce.

We will acknowledge receipt within a few days. There is no formal SLA — this is a volunteer project — but the maintainers will prioritise security fixes over feature work.

## Scope

In scope:

- The Rust server in [`crates/`](crates/) (auth, base, cell, mercury, admin API).
- The bundled PostgreSQL provisioning under [`bootstrap/`](bootstrap/) and [`db/`](db/).
- The Tauri admin app and player-facing `sgw-launcher` under [`crates/launcher/`](crates/launcher/) and [`tools/`](tools/).
- The Mercury wire protocol implementation and AES-256-CBC + HMAC-MD5 crypto.
- The dev-session telemetry pipeline and any HMAC token surfaces.

Out of scope:

- The retired C++ implementation under [`deprecated/cpp/`](deprecated/cpp/). It is not built or run by the active project.
- Vulnerabilities in the Stargate Worlds **client binary** (`sgw.exe`) — we cannot modify the client.
- Vulnerabilities in third-party crates we depend on — please report those upstream. Tell us if a dependency CVE affects our usage.
- Findings that require an attacker who already has shell access to the server host.

## What counts as a security issue

The kinds of issues we want reports on:

- **Authentication bypass** — anything that lets a client play as another account or escape the test-credential gate.
- **Authorization escapes** — getting GM commands, admin API endpoints, or restricted actions without the right access level.
- **Server-trust violations** — the server trusting client-supplied data where it shouldn't (movement validation gaps, ability cooldown skips, inventory dup paths, etc.).
- **SQL injection** in handlers that take client input.
- **Memory-safety issues** in `unsafe` blocks (we have very few; treat each one as a hotspot).
- **Denial of service** that an unauthenticated attacker can trigger remotely — e.g. malformed Mercury packets that crash the server, allocation amplification, fragment-reassembly bugs.
- **Crypto misuse** — incorrect AES mode, weak key derivation, predictable IVs.
- **Secret leakage** — credentials, HMAC keys, or session tokens appearing in logs, error messages, or telemetry payloads.

## What doesn't count

- Issues only reproducible against an unmaintained release. We support `main` and the latest tagged release.
- Brute-force attacks against the `test`/`test` development account. Operators are expected to disable or replace test accounts before exposing the server to untrusted networks.
- Self-XSS or attacks requiring local server-host access.
- Missing security headers on the admin API in development mode. Production deployments are responsible for fronting the admin API with TLS (Cloudflare Tunnel, reverse proxy, etc.).

## Disclosure

Once a fix is shipped and a reasonable upgrade window has passed (typically 30 days for non-critical, faster for critical issues affecting deployed servers), we will publish the advisory with credit to the reporter unless the reporter prefers anonymity.

## Operational guidance

If you operate a Cimmeria server publicly, please:

- Disable or rotate the default `test` account before opening to untrusted players.
- Front the admin API (port 8443) with a TLS proxy. The container-deployment runbooks under [`docs/operations/`](docs/operations/) cover Cloudflare Tunnel as one option.
- Treat the Python console as if it grants shell access — because it does. Disable it (`py_console_password` empty) unless you specifically need it for debugging.
- Keep your `setup.ps1` checkout current; security fixes ship on `main` and roll into the next release container.
