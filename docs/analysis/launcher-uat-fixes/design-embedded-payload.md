# Embedded Windows launcher payload

> Design before implementation · 2026-10-23 · Owner authorized implementation.

Ship the existing Windows release ZIP with exactly one root launcher EXE. Embed
its three trusted staged Play resources (launch worker, client patches DLL and
player telemetry DLL) in that executable. Keep the ZIP name, checksum, signed
updater feed, launcher-current publication and release signing environment.
WebView2 remains a system prerequisite and game content stays a separate download.
macOS already provides one app bundle and retains its existing resource layout.

The shell build script generates include_bytes entries only for Windows and
checks each embedded file against its independent compile-time SHA-256 pin.
Tagged/packaging builds must explicitly require all three; missing or mismatched
staging fails the build. Unpinned development builds remain possible. No keys or
secrets are embedded. Existing native resource verification still runs at use.

At startup, after logging initializes and before binding launch resources, expand
into the native app-local-data payload cache. The cache version is a digest of
fixed filenames and their trusted digests. Verify cached bytes on every start;
repair ordinary missing/corrupt files atomically from embedded bytes. Reject
links, reparse points, unexpected file types and unsafe paths. No nearby sidecar
fallback is used by an embedded build. Preserve existing host paths by returning
a canonical resource directory with the same windows/ names.

Serialize cache publication/cleanup, hold a shared per-version lease for the
launcher lifetime, and skip cleanup when another process holds that version.
On Windows retain non-delete-sharing directory handles to prevent ancestor
replacement while writing. Cleanup removes only fixed managed files from an
owned cache version; no recursive deletion and no game-folder or old sidecar
cleanup. Windows denies deletion of mapped/running payload files; leave those
versions for a later start. Unknown files or directories are preserved.

Payload setup failure emits a closed-code startup event and a visible native
message with a retry/support instruction. Never run unverified cached helpers.
Successful extraction/reuse/cleanup logs only payload identity and counts.

Extend the existing release build/verify/package stages rather than adding a
parallel tool. Add a headless embedded-payload verification mode to the launcher
so CI can copy only the EXE to a clean folder and prove extraction without
opening a window or launching the game. Keep the macOS staging source independent
of the Windows ZIP. The existing updater accepts a single root EXE already;
prove old multi-file to new single-file ZIP update and restart handoff in tests.

Validation: native cache unit tests for cold/reuse/corrupt repair/version changes,
leases, partial extraction, links/junctions, hostile/unknown cleanup entries and
pinned bytes; build-script pin checks; package/archive tests; updater regression.
Prove selected guards fail with corresponding fixes removed. Run engine/shell
checks through lane-build, frontend build for the shell, and full native/package
CI before queueing through ship-pr. No frontend behavior change, so no new JS
logic UAT is needed. A real game or updater restart and a real release are outside
this implementation; preserve the owner's separate release authorization rule.
Owner scope update (2026-10-23, after design): proceed with the PR and merge, then release the launcher when merged. The release must retain the Windows ZIP. This is explicit GO for the post-merge launcher workflow; it does not authorize game/lab use.
