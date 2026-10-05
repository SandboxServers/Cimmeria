# Opt-in game telemetry in the desktop launcher

> **Type:** Reference
> **Audience:** Launcher contributors and reviewers
> **Last updated:** 2026-10-04
> **Status:** Implemented on `launcher/desktop-client-telemetry` (PR #1241); proven once on macOS under Wine
> **Companions:** [launch contract](../../../../../crates/launcher/desktop/docs/launch.md#opt-in-game-telemetry), [telemetry operations](../../../../operations/telemetry.md#player-opt-in), [MacBook checkpoint](macbook-testing-checkpoint.md)

## Why

The desktop launcher never loaded `cimmeria-client-telemetry`
(`game_telemetry_available: false`), so a Mac play session sent no client
telemetry. A fresh-install UAT on a second Mac on 2026-10-04 found a client
problem the server could only see from outside: the client reports version 0
for all 21 cooked-data categories at every login, although its cache PAKs hold
the server's real versions. The server then resyncs everything, and the client
stops drawing for about ten minutes while it writes the entries.

## What was built

The contract is in the [launch reference](../../../../../crates/launcher/desktop/docs/launch.md#opt-in-game-telemetry).
In short: a separate default-off choice (`game-telemetry.json`), the DLL as an
optional pinned bundle resource, one server session per Play, the session
marker beside the game, and the DLL added to the launch request after the
client patches. The Windows launch helper already accepted two DLLs and is
unchanged.

## Evidence

Mac mini (Mac16,10, M4, macOS 27.0.1), pinned Wine runtime r17, stock Rosetta,
branch commit `7294a49c9`. Times are UTC.

| Check | Result |
|---|---|
| Rust tests, lane, macOS | 529 passed, 0 failed, 37 ignored: shell 71, engine 442, runtime probe 16 |
| Engine tests, Linux (WSL) | 358 passed, 0 failed, 4 ignored |
| Strict clippy, workspace, all targets, macOS | clean |
| `cargo fmt --check` | clean |
| Frontend `npm run check`, `npm test` | clean; 74 passed |
| `npm run uat:game-telemetry` | passed (production host, temporary fixture) |
| `npm run uat:launch` | passed |
| Staging tool tests | 4 passed |
| Telemetry DLL | Windows CI run `37254851751` at `a721f092b`; SHA-256 `7d2ceacfee48c2ec235c0093e1850bcb44e4100e2eabc57452ac65f3d21ccd77` matches the build log; no lab-bridge marker |

Native run, one Play with the choice turned on in the real Settings window:

- 02:40:01: the launcher minted session `bd67e0ac-1da8-4a36-95e8-8c464273e940`
  and wrote `sessions/current-session.json`. The launch is recorded `attached`.
- 02:40:05: the game started. The DLL's log beside `SGW.exe` shows `attached`,
  `build flavour: player`, the session loaded, the install lock taken after the
  client-patches DLL, two already-hooked functions chained, and `hooks installed`.
- The game process held an established TCP connection to the server's login
  port and had sent 34 KB before the player logged in.
- The server operator confirmed the session in SigNoz: first event 02:40:07,
  1,315 events of 32 types by 02:40:22, the DLL reporting itself as `player`
  with its hooks installed, from the bundle path of this build.

That run had `unfilter` and `firehose` on because the launcher was started
with `CIMMERIA_CLIENT_CAPTURE=unfilter,firehose` in its environment. They are
off for a player who opens the app normally.

## Limits

- One run, one machine, one server build. Not run on Windows through the
  desktop launcher: the native path is covered by tests only.
- An adopted copy still launches without game telemetry.
- No token refresh, log tailing or bundle upload.
- The accelerator (`rosettax87`) was not bound in this build. With it, the
  launcher loses track of the game seconds after start (the Wine loader hands
  off to a detached process), records the launch `unknown`, and has no recovery
  for that state.

## Open client problems this telemetry is for

Recorded on the same machine on 2026-10-04:

- **Version 0 at every login (cause found, not fixed).** After a full resync the 21 cache PAKs are valid
  and stamped with real versions. A clean quit leaves all 22 files
  byte-identical. At the next start the game opens all 22 from the writable
  `Cache.en-US`, read-write, at full size, 14.7 s in, and they stay
  byte-identical for at least 30 s. The client still sends version 0 for all
  21 categories at login. Both volumes are case-insensitive, there is one cache
  directory, and the client logs no "Error opening static cache archive" or
  "Non-existent source archive directory" line. Windows clients on the same
  server report real versions.

  The first cooked-cache events (session of 2026-10-05 03:19 UTC, DLL at
  `60731aa3f`) narrowed it: `client.cooked.versions_held` at login had 22
  storages, all 0; `client.cooked.version_read` fired 43 times at start, with
  `metadata_entry_not_found` for `covernodes_local.pak` (it has no `MetaData`)
  and `outcome: read`, `version: 0` for the other 21, each read once from the
  writable cache and once from the bundled `SourceCache` (Items at entry 6074
  and 6059). `client.io.pak_open` never fired: the zip library opens through
  the C runtime, not `CreateFile`. So the entry is found and extracted, and the
  four bytes are lost after that.

  What is after that is a `std::strstream`: the client writes the extracted
  bytes into one and reads them straight back, and Wine's `msvcp80.dll` returns
  end of file for a read that follows a write. The mechanism, the line in
  Wine's source and the disassembly of the runtime's DLL are Finding 9 of
  [cooked-data-pipeline.md](../../../../reverse-engineering/findings/cooked-data-pipeline.md).
  The event now carries each hop (`zip_*`, `extract_*`, `stream_*`, `crt_*`)
  and names this outcome `stream_read_short`.

  A start with that DLL (session `29ecefab-24be-4883-a614-dcd705bb9662`,
  2026-10-05 04:08 UTC, no login) showed it for all 21 archives: the right
  four bytes extracted, four bytes held by the stream, none read back,
  `eof|fail`. The file calls underneath were clean, and `client.io.pak_open`
  fired 65 times through the C runtime's open.
- **The resync looks like a freeze.** The server finishes pushing in about two
  minutes. The client then spends about eight more writing entries, rewriting
  `TextStrings.pak` (29,126 entries) as it grows, and neither draws nor sends
  game messages. Killing it then corrupts the PAK in progress.
- **Create New Character does nothing** in a session that has just been
  resynced, with no error and nothing sent to the server. Unresolved. The
  client builds the same kind of stream at 22 places, so the Wine fault above
  is the first thing to rule out.
