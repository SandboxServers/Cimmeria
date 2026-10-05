---
name: project_mac_client_cooked_version_zero
description: "Under Wine the client sent cooked version 0 at every login: Wine's msvcp80 strstreambuf::underflow returns EOF for a read that follows a write. Repaired in-process by cimmeria-client-patches (strstream_underflow), verified live 2026-10-05; the same repair made character creation work"
metadata:
  type: project
---

Found 2026-10-04 on a Mac mini (M4, macOS 27.0.1, pinned Wine runtime r17, stock Rosetta) with a fresh desktop-launcher install. Server side confirmed by the colo operator in SigNoz. Detail and evidence: `docs/analysis/playtests/2026-10-03-macos-wine/worknotes/game-telemetry.md`.

**Symptom.** At every login the client reports version 0 for all 21 categories, so the server resyncs everything (~55,000 entries). Windows clients on the same server report real versions.

**The "freeze" is the digest, not a hang.** The server finishes pushing in about two minutes; the client then writes entries for about eight more and neither draws nor sends game messages (the server sees heartbeat-only). `TextStrings.pak` (29,126 entries) is rewritten as it grows. Two busy threads: one in NtWriteFile / NtFlushBuffersFileEx / NtSetInformationFile, one spinning on Sleep(0) + QueryPerformanceCounter. Killing the client then leaves the PAK in progress unreadable ("Error opening static cache archive ...TextStrings.pak" at the next start).

**Ruled out, with evidence:**

- The cache is written. After a resync all 21 PAKs are valid zips with the server's real version in `MetaData`.
- A clean quit changes nothing: all 22 files byte-identical before and after.
- Startup changes nothing: byte-identical at +5, +15 and +30 s.
- The client opens the right files: all 22 from the writable `Cache.en-US`, read-write, full size, at +14.7 s. Nothing from `SourceCache`.
- One cache directory only; both volumes case-insensitive APFS; `SourceCache.en-US` resolves to the launcher's `SourceCache.en-us`.
- No "Non-existent source archive directory" and no cache-open error in the client log on a clean start.
- Not line endings: the check compares one number per category, stored as 4 raw bytes.

**Cause (2026-10-04, from disassembly and Wine's source; RE detail in `docs/reverse-engineering/findings/cooked-data-pipeline.md` Finding 9).** The version read (`0x00478f00`) writes the extracted `MetaData` bytes into a `std::strstream` over an empty dynamic `strstreambuf` and reads four back with no seek between. `strstreambuf::underflow` has to notice the put pointer moved. Wine's (`dlls/msvcp90/ios.c`, shared by `msvcp80.dll`) reads the get pointer where it means the put pointer, so it returns EOF; `istream::read` sets eof and fail, the client does not check, and the version stays 0. The pinned runtime's `msvcp80.dll` has it (`0x10074250`), and so did Wine `master` that day. The client loads Wine's builtin although the prerequisites install Microsoft's 8.0.50727.762, because Wine ships its own assembly as 8.0.50727.9672 and takes the highest.

**Repair (2026-10-05).** `cimmeria-client-patches` probes the runtime at start under Wine and, when the round trip fails, points the `underflow` entry of `strstreambuf`'s vtable at a shim that raises `_Seekhigh` to the put pointer first (`crates/client-patches/src/strstream_underflow/`). First live run, session `411f1000…`: 42 of 42 version reads real, login without a resync, in the world in under three minutes. The process still runs Wine's builtin `msvcp80.dll`. The untried alternative is `WINEDLLOVERRIDES=msvcp80,msvcr80=n` for the game: Wine already resolves the manifest to the 762 `winsxs` directory the prerequisites fill (a crash dump lists the modules there) and maps its builtin only because the builtin is tried first. Its log line starts `strstream:` in `cimmeria-client-patches.log` beside `SGW.exe`.

**It is wider than the version.** The `strstream` constructor (`0x00478970`) has 22 call sites, and every cached element load goes archive → `strstream` → read. A fix that only repairs the version would stop the server pushes and leave the client unable to read its cache. Fix the stream.

**What `client.cooked.version_read` shows for it (seen live 2026-10-05 04:08 UTC, session `29ecefab…`, all 21 archives):** `outcome: stream_read_short`, `extract_version` real, `stream_held: 4`, `stream_read_count: 0`, `stream_state: eof|fail`, `version: 0`.

**Related:** Create New Character did nothing (no error, nothing sent) in every session before the repair and worked in the first session with it. Which `strstream` site it goes through is not traced. Bundled `SourceCache` already matches the server for 16 of 21 categories including `TextStrings`, so seeding the writable cache at install would cut a first login's push to about 27%, but only once the version read works.

**How to apply:** for any client-under-Wine symptom that involves reading data back (cache, saved settings, anything serialised through a `strstream`), suspect this first. Start from the telemetry session of a Mac login (tags `desktop-launcher`, `macos`, `wine`); the version read needs no capture switch. The zip library opens archives through `_wsopen_s`, not `CreateFile`. Do not kill a client during the digest.
