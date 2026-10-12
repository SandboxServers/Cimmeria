# Any-install adoption: campaign close notes

> **Type:** Reference
> **Audience:** Maintainers and the next coordinator
> **Last updated:** 2026-10-12

The campaign shipped AI-01 to AI-13, AI-15 and AI-16 on one branch,
`any-install-adoption/ledger`, as one PR. AI-14 (publish the file index) and
AI-17 (Windows UAT) are owner steps; AI-18 finishes after AI-17.

## How it was built

Each packet ran in its own worktree, branched from the campaign tip and merged
back with `git merge` (no rebases). Packets were implemented by Haiku-tier
`packet-coder` agents, split into briefs of roughly one commit each; a brief
that ran out of context left a numbered list, and a fresh coder finished it.

## Review and fixes

Three read-only reviews ran on the whole campaign diff: one on the local
archive and file index, one on the adoption engine, and a trust-boundary
review. None found a way for a hostile folder, archive, webview or network
response to escape the game folder or get unverified bytes trusted. They did
find real defects, all fixed in review batches A to D4b with a test each:

- In-place admission failed with `Corrupt` on a stock folder (a patch-added
  file, or a folder spelled in another case such as `binaries`).
- A custom `LoginInternal.lua` was rewritten by setup although the review
  promised a backup (D-AI13).
- Adoption ignored the player's chosen archive until a file index is
  published; a running Repair or Update could be switched to an archive for
  another seed; `clear()` could orphan a running download.
- The seed archive panel stopped following a check that had just started, and
  showed a stale archive as chosen.
- Replay copied local inputs into a tree that already held the seed, never
  placed the bundled cooked data, and copy mode downloaded the seed for a
  case-only difference. Fixture previews skipped the replay entirely; the
  replay now runs on local artifacts too, so the tests exercise it.
- The download estimate on the built-index path offered a download that had
  already happened.
- Preview could delete a leftover work folder whatever it held; the write
  probe ran through junctions; a stuck work folder blocked re-adoption;
  replacements were written in place (now `.adopt-partial`, then rename).
- Windows-only defects found by enabling the macOS-only copy tests: directory
  sync failed with access denied, and the exclusive rename had no Windows
  implementation.
- `in_place.rs` was over the 700-line cap and is split
  (`in_place_replace.rs`).

## Known gaps

- AI-14 is not done: no file index is published, so production adoption still
  builds the index from the seed (download, or the player's archive) until the
  owner publishes one (D-AI8).
- AI-17 (Windows UAT on a real client) has not run. Untested on a real client:
  a 2009-installer folder, a hand-extracted RAR, a Program Files folder, and
  the Downloads offer with the real archive.
- On macOS, a replay that needs the seed returns `unsupported_mode` (it would
  need the Wine extractor).
- `adoption-native-uat.mjs` (macOS copy UAT) was updated but only syntax
  checked; it needs a macOS run.
- `play_tests` stay macOS-gated: they depend on macOS-only prerequisite
  fixtures.
- The replay test fixtures use overlay patches that do not read their inputs,
  so dropping the local copy of an intact input would not fail a test; the
  seed-collision case it fixed does.
- Revert spot-checks (2026-10-12): removing the admission fix (a hard failure
  on a file with no scan entry) or `keep_login` makes its test fail. Looking
  the reviewed hash up by the folder's own spelling rather than the release
  spelling is not separately guarded: with the release spelling, a file spelled
  in another case is skipped from the reviewed-hash record, which only crash
  recovery reads.
- Index-driven incremental Update and Repair (D-AI14) is out of scope; an
  adopted folder's first game Update still needs the seed.
