# Launcher sign-in: relay, ticket file and scrub

> **Type:** Reference
> **Audience:** Engineers working on the desktop launcher
> **Last updated:** 2026-10-26
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
| Epoch fence, host slots, parked flows | `engine/src/sign_in/` (`driver.rs`, `driver_epoch.rs`, `host_slot.rs`, `flow.rs`) |
| `LoginInternal.lua` ticket rendering | `cimmeria-launcher-core` `client_setup::login_session` |

## The relay

- Binds `127.0.0.1:0`. Accepts only `POST /SGWLogin/UserAuth` and `POST /SGWLogin/ServerSelection`; anything else is 404. `Host` must be `127.0.0.1:<port>` (421), any `Origin` header is refused (403), bodies are capped at 64 KiB (413).
- Forwards body, `Content-Type`, `SOAPAction` and `Cookie` to the server's HTTPS auth base and returns status, body, `Content-Type` and `Set-Cookie`. Upstream failure answers 502 with an empty body. No log line carries a body, cookie or header value.
- It starts only when the server's discovery says `soap_relay: true`. It is never the telemetry endpoint: telemetry keeps the real server list. The panel and Play each keep their own 10-minute discovery cache (`DISCOVERY` in `storage/launch/sign_in_play.rs` and `SignInDriver.discovery`), so Play can probe the server again right after the panel did.

## The ticket file and the scrub

Signed in, Play mints a launch ticket with a `launch_id` derived from the operation id, then writes a one-server `LoginInternal.lua` in ticket mode: the server entry points at the relay, and the account name and ticket are in `CimmeriaLogin`. The file holds a live credential for the length of the launch, so:

1. When the game exits (or the launch fails after the file was written), `finish` writes the plain server list back over `LoginInternal.lua` **before** it tells the server to end the ticket. A slow or dead server therefore cannot leave a ticket in the install. A failed scrub logs `scrub_failed` with the error kind only.
2. It then calls `POST /launch-ticket/end` and stops the relay.
3. While the game runs, a heartbeat goes out every 10 minutes; three failures in a row stop it without ending the game.

If the launcher is closed while a relayed game runs, the shell asks first ("Game still connected": Keep running or Close anyway), because the game cannot change server or log in again without the relay. "Close anyway" is the only answer that closes it.

## The sign-in engine

The state machine behind the panel lives in `engine/src/sign_in/`. Four rules keep it safe when a player presses buttons during a sign-in.

### The epoch fence

`SignInDriver` holds an epoch counter. `begin` and `add_method` read it when they start; `sign_out` and Cancel bump it (`supersede`), which voids every step still in flight. A step checks its `Epoch` (`host_slot::Epoch::is_current`) at the points that write state:

- `adopt_signed_in` checks under the host slot's lock. A completion that lost the race is not stored, and its fresh refresh token is logged out on the server.
- `installable` checks under the state lock before the signed-in state is installed.
- `FlowSlot::run` checks again before it opens the browser, so a Cancel during discovery does not open a tab.

One window stays open on purpose: a Cancel that lands after the slot and keyring are written but before the state installs replies Cancelled, and the next Inspect shows signed in. The state is consistent and the session is the player's own.

### Per-host slots

Each auth host has one `HostSlot`, shared by every `SignIn` the driver makes for it. The slot holds the access token and its expiry, the account, and the store state. A refresh and a sign-out on any of them serialize on the slot's lock, so one refresh token is spent once. Keyring entries are keyed per host (`cimmeria-launcher/<auth host>`), so a session for one server is never offered to another.

The store state records what the credential store refused:

| State | Set when | Effect | `storage_warning` |
|---|---|---|---|
| `session_only` | The store would not save a new sign-in | The sign-in works until the launcher exits | `session_only` |
| Tombstone | The store would not delete on sign-out | The host reads as signed out, so the next launch does not restore the account | `sign_out_incomplete` |
| Unreadable | The store cannot be read | No saved sign-in is restored | `store_unreadable` |

### Resume

A `begin` or `add_method` whose browser will not open parks its flow (listener, verifier, cancel handle, epoch and what it continues) and answers `browser_failed` with the authorize address. `resume` awaits the parked flow and completes it as the original command would. With nothing parked, `resume` fails `no_pending_flow`. Cancel and the flow's 5-minute limit clear it. Two concurrent `resume` calls yield one completion and one `no_pending_flow`. Cancel also clears the address box in the panel.

### The effective list

Sign-in talks to the first server of the **effective** list, the list Play writes: the adopted list of a legacy-adopted installation, else saved preferences, else the defaults. `DesktopState::sign_in_login_server` and `launch_login_servers` both call `effective_login_servers`. An error reading the installation never falls back to another list. A host that is an IP literal gets no sign-in.

### While an install, update or repair runs (CT-90)

When the effective list cannot be read (`Busy` during an install, update or repair of the installed copy, or `PersistenceUncertain`):

- **Inspect** and **Cancel** answer the held sign-in's status in the offline shape: `available: false`, no `server_name`.
- **Sign out** signs out with no server.
- Every other command answers `unavailable`.
- The panel is hidden during an update, except a held sign-in with **Sign out**.
- A browser sign-in that finishes during an update is not kept.
- A launcher started mid-update restores the saved sign-in once the update ends.

### Known races

The window above (Cancel between the keyring write and the state install) is the one residual race. A completion that arrives after the effective list changed is written to the old host's slot and keyring entry but not installed; the session is remembered for a host the player left, never offered to the new one.

## Never a password fallback

Signed in, any failure to get a ticket (discovery fails, `soap_relay` is false, the relay will not start, the mint is refused or offline) ends the launch as not started. `DesktopState` records `ticket_unavailable`, or `sign_in_ended` when the stored sign-in was refused, until the next Play; the panel shows it as `play_refusal`. Relay-password mode is for players who are not signed in. The credential store (the OS keyring on Windows and macOS, memory only on other platforms, so a Linux sign-in ends when the launcher closes) holds the refresh token, account name and display name under `cimmeria-launcher/<auth host>` and never appears in launcher files or logs; details in the [privacy inventory](privacy-inventory.md).
