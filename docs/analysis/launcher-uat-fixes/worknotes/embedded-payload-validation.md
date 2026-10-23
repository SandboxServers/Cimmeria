# Embedded payload validation

> Reference · 2026-10-23 · Design: ../design-embedded-payload.md

Windows packages remain ZIPs with the existing public name, now containing one
root EXE. The shell build embeds three independent SHA-pinned staged Play pieces.
Startup prepares the native app-local content-versioned cache and holds a shared
lease. Cache repair uses atomic files; cleanup is non-recursive, recognizes only
owned versions/fixed names, and defers leased or OS-in-use payloads. The existing
native Play resolver and artifact verification consume the resulting paths.
Mac resource packaging and all game/adoption roots remain unchanged.

Local restored checks passed: engine library 988 passed/3 ignored; Windows shell
156 passed/8 ignored; standalone workspace/all-target clippy with warnings denied;
frontend typecheck/build; 7 release workflow/package Python tests. The shell tests
also check missing/mismatched build pins, and the updater tests accept an old
sidecar installation updating to a signed single-root-EXE ZIP with one relaunch
handoff and acknowledgment, preserving old sidecars and player files.

Nine deliberate regressions failed their intended assertions: cached hash
verification, busy repair lease, active-version cleanup lease, unknown-file
preservation, junction rejection, build pin verification, all required pins, and
ZIP sidecar exclusion, and tagged-build enforcement without the embed flag. Inputs were restored after each. An initial repair mutant
failed cold setup and was not counted; the corrected mutant ignored only failed
lock acquisition and exposed replacement of leased bytes. An initial junction
mutation left the native no-follow open effective and still passed; removing
both parts then failed the redirect assertion. Evidence is in the worktree's
ignored mutation report and lane failure logs; no failed mutant is shipped.

The actual built Windows EXE, copied alone into an empty temporary directory,
passed the release verifier's headless production-cache/native-resource probe
three times: cold extraction, reuse and corrupt-cache repair. The local build
embedded the previously published 2026.10.2303 helper/DLL bytes, downloaded from
its immutable release, checked against the published ZIP checksum and staged by
the existing architecture/digest/player-build verifier. None of those helpers
or DLLs was executed. CI still builds the current-source helper/DLL inputs.
The actual package Python block also zipped the built native EXE alone and proved
one root entry with byte-for-byte executable preservation. No app window, game,
login, injection or Wine process was started by these probes.

A preliminary probe was invoked before the native build completed and found no
EXE; it exercised nothing. The completed build and all three later probes passed.

This is native bootstrap/packaging behavior with no JS/frontend state change; no
new JS REPL pass applies. Not tested locally: native failure-message interaction,
packaged app focus, a real updater process replacement/restart, live game/login,
Wine, macOS execution or SigNoz reception. Cross-platform native/package CI and
fresh read-only review remain the next gates. EP-U1..U3 stay unattested for real
player acceptance.

Doc-map walk: desktop launcher workflows/packaging/release, tester-facing UAT,
and campaign scope/decisions. Desktop references, player/Mac guides, ledger,
RESUME, mirrored UAT and committed project memory were updated. No Plan fields,
consent defaults, signing secrets or generated blocks changed.

Owner GO: proceed with PR/merge, then release from main, retaining the Windows ZIP.
Independent review N1: tagged Windows builds now require complete pins even without CIMMERIA_EMBED_PAYLOAD, and the tag participates in build-script invalidation. Tests cover zero/partial/full pins, development builds and unchanged macOS behavior. Removing the tag requirement fails the new assertion; restored shell156 and all-target clippy pass.
