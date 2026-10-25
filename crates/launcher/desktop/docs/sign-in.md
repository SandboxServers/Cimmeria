# Launcher sign-in: relay, ticket file and scrub

> **Type:** Reference
> **Audience:** Engineers working on the desktop launcher
> **Last updated:** 2026-10-25
> **Companions:** [Launch](launch.md), [Privacy inventory](privacy-inventory.md), [Launcher sign-in and linked accounts](../../../../docs/technical/launcher-sign-in.md) (the end-to-end flow), [player guide](../../../../docs/client/launcher-sign-in-guide.md)

This page covers what Play does when the player is signed in. The flow, linking rules and server side are in the [flow page](../../../../docs/technical/launcher-sign-in.md); the shared names are in the [campaign contract](../../../../docs/analysis/client-tls-oidc/contract.md) (C8, C9, C13).

Nothing here has run against the live game. The client overlay in the live game, the relay under Wine and the packaged Windows and macOS keyring are unverified.

## Where the code lives

| Piece | Path |
|---|---|
| Loopback relay and browser callback | `crates/launcher/desktop/relay/` (`cimmeria-launcher-relay`) |
| Discovery, PKCE, credential store, session | `engine/src/sign_in/` (`discovery.rs`, `oauth.rs`, `store.rs`, `session.rs`) |
| The single frontend command | `engine/src/sign_in/commands.rs` (`sign_in_command`), forwarded by `shell/src/host/sign_in.rs` |
| Play's ticket path | `engine/src/storage/launch/sign_in_play.rs` |
| `LoginInternal.lua` ticket rendering | `cimmeria-launcher-core` `client_setup::login_session` |

## The relay

- Binds `127.0.0.1:0`. Accepts only `POST /SGWLogin/UserAuth` and `POST /SGWLogin/ServerSelection`; anything else is 404. `Host` must be `127.0.0.1:<port>` (421), any `Origin` header is refused (403), bodies are capped at 64 KiB (413).
- Forwards body, `Content-Type`, `SOAPAction` and `Cookie` to the server's HTTPS auth base and returns status, body, `Content-Type` and `Set-Cookie`. Upstream failure answers 502 with an empty body. No log line carries a body, cookie or header value.
- It starts only when the server's discovery says `soap_relay: true`. It is never the telemetry endpoint: telemetry keeps the real server list.

## The ticket file and the scrub

Signed in, Play mints a launch ticket with a `launch_id` derived from the operation id, then writes a one-server `LoginInternal.lua` in ticket mode: the server entry points at the relay, and the account name and ticket are in `CimmeriaLogin`. The file holds a live credential for the length of the launch, so:

1. When the game exits (or the launch fails after the file was written), `finish` writes the plain server list back over `LoginInternal.lua` **before** it tells the server to end the ticket. A slow or dead server therefore cannot leave a ticket in the install. A failed scrub logs `scrub_failed` with the error kind only.
2. It then calls `POST /launch-ticket/end` and stops the relay.
3. While the game runs, a heartbeat goes out every 10 minutes; three failures in a row stop it without ending the game.

If the launcher is closed while a relayed game runs, the shell asks first ("Game still connected": Keep running or Close anyway), because the game cannot change server or log in again without the relay. "Close anyway" is the only answer that closes it.

## Never a password fallback

Signed in, any failure to get a ticket (discovery fails, `soap_relay` is false, the relay will not start, the mint is refused or offline) ends the launch as not started. `DesktopState` records `ticket_unavailable`, or `sign_in_ended` when the stored sign-in was refused, until the next Play; the panel shows it as `play_refusal`. Relay-password mode is for players who are not signed in. The credential store (the OS keyring on Windows and macOS, memory only on other platforms, so a Linux sign-in ends when the launcher closes) holds the refresh token, account name and display name under `cimmeria-launcher/<auth host>` and never appears in launcher files or logs; details in the [privacy inventory](privacy-inventory.md).
