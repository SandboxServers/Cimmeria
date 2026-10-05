---
name: project_mac_client_cooked_version_zero
description: "Open (2026-10-04): the client under Wine/Rosetta on macOS sends versionInfoRequest version 0 for all 21 cooked categories at every login although its cache PAKs hold real versions; full resync and a ~10-minute digest each time; ruled-out causes"
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

**Leading candidate (from the dumps, unconfirmed):** the version is read into `ServerSource+0x24` at the tail of `ZipStorageBase_OpenArchive` (`0x00479340`) by `FUN_00478f00`, whose body is not decompiled. A failed `MetaData` lookup there under Wine would leave 0 with the file open and intact.

**Related, also open:** Create New Character does nothing (no error, nothing sent) in a session that was just resynced. Bundled `SourceCache` already matches the server for 16 of 21 categories including `TextStrings`, so seeding the writable cache at install would cut a first login's push to about 27%, but only once the version read works.

**How to apply:** start from the telemetry session of a Mac login (tags `desktop-launcher`, `macos`, `wine`) with `CIMMERIA_CLIENT_CAPTURE=unfilter,firehose`; the DLL hooks `CreateFileW/A`. Do not kill a client during the digest.
