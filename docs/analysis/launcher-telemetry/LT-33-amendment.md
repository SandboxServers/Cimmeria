# LT-33 amendment (2026-10-21)

The first LT-33 coder (worktree `lt-33`, base `9285f623a`, unedited) blocked on six packet gaps. This note gives each ruling. LT-33 is re-sliced into **LT-33a → LT-33b → LT-33c**, in sequence on `launcher-telemetry/adoption`. The packets are in [work-packets-adoption.md § LT-33a..LT-33c](work-packets-adoption.md). C14 and C16 are amended there, and LT-34 has a one-line change.

## Why three packets

One packet would touch the engine (rejection.rs, worker.rs, mod.rs, a new test file), six shell files, the TS schema with its fixtures, and a doc. That is well over the ~40k-token budget for one packet. The cuts follow the crate boundary and the dependency order:

- **LT-33a, engine (~25k tokens).** Adds `refuse`, `ConfirmationWorker::last_rejection()` and the re-exports. The shell can't compile against them until this merges.
- **LT-33b, shell DTOs plus the TS schema (~40k).** R8 requires the TS schema in the same commit as the Rust DTOs, so they stay together.
- **LT-33c, shell refusal logging plus the migration doc (~25k).** Wraps 11 sites in mod.rs (497 lines, call lines only), changes one line in main.rs, and adds one doc paragraph.

## Rulings

| # | Gap | Ruling |
|---|---|---|
| 1 | Engine `rejection` module is private | Keep it private. `storage/adoption/mod.rs` re-exports `pub use rejection::{refuse, Rejection};` (replacing `use rejection::Rejection;` at :85) and `pub use index_source::IndexFallback;`. The shell names `adoption::Rejection` / `adoption::IndexFallback` like `adoption::Error`. (LT-33a) |
| 2 | Shell refusals have no engine `Error` twin | New engine `pub fn refuse(code: &str, r: &Rejection)`. It logs the same `adoption_rejected` WARN through a private `warn` fn shared with `reject`, uses the shell's code, and records nothing. The shell's `refusal(error, check, found) -> AdoptionError` (new `refusal.rs`) calls it with `events::code(error)`. Shell refusals are **logged only**: they come back synchronously as the command's `Err`, which the UI already shows, so they never set `last_rejection`. C14's "WARN only from `reject`" now reads "only in rejection.rs (`reject`, `refuse`)". (LT-33a engine, LT-33c sites) |
| 3 | `ConfirmationWorker` has no `last_rejection()` | It gains a private `watch::Receiver<Option<Rejection>>` and `pub fn last_rejection(&self) -> Option<Rejection>`. `start_confirmation_with` wraps the job in `rejection::scope`, like the preview worker. A panic becomes `reject(PersistenceUncertain, worker_stopped/panic)`. The rejection is sent before the result. (LT-33a) |
| 4 | Engine progress labels need mapping | A closed typed enum `ProgressItem { Seed, Patch, FileIndex, IndexBuild, Reference }` with an exact-label table in C16. `index_build` is new: it is the `Extracting "File index"` stage that builds the index from the seed. `Adoption replay` maps to `patch`. Unknown labels and copy-phase progress map to `None`. LT-34's archive text now shows for `seed`, `patch` or `index_build`. (LT-33b) |
| 5 | `index_fallback` on status vs review | On `Review`, with `skip_serializing_if`, set in `Review::of` from `report.index_fallback`. C16 is corrected. (LT-33b) |
| 6 | `Progress` derives `Copy` | It keeps `Copy`, because `item` is `Option<ProgressItem>` (a Copy enum), not `Option<String>`. `skip_serializing_if`, so the TS uses `optionalKey` and no progress fixture changes. (LT-33b) |
| 7 | (coder's extra) shell mod.rs is 497 lines | LT-33b adds at most 4 declaration and field lines. LT-33c adds wrap-only call lines, capped at 560 after fmt. Both stay under the 700 hard cap. The refusal logic lives in `refusal.rs`. |

The engine and the desktop shell are separate crates. Every edited file stays under 700 lines; the largest is shell `mod.rs` at about 530.

## Accepted gaps (no packet now)

- **publication.rs scan mismatch.** The inventory-scan `SourceChanged` at publication.rs :106/:282 records no rejection. The file is at 697 lines, so `last_rejection` stays `None` for that case and `last_error` still shows `source_changed`. Proposed follow-up: **LT-33d**, which splits publication.rs and names that check `folder_changed_during_check` with `entry` = the first changed relative path. Not scheduled; the orchestrator decides.
- **macOS-only shell refusals** (mod.rs :300, :303) can't be compiled on this host. They are left unlogged.
- **Confirm-side shell checks** (mod.rs :407-436: stale revision, busy, review unavailable) are not wrapped. They are staleness races the UI already reports by code. `consent` at :436 is engine-logged by LT-32.
- **Base-slot leftovers.** An engine `reject` reached outside any `scope` (e.g. `consent` called from the shell) leaves its value in the thread-local base slot. This is harmless: it is only read by `scope`, which installs a fresh slot.
- **`UnsupportedMode`** sites (:210, :232) are programming-contract errors, not player refusals. Not logged.

## Stop rules written into the packets

- LT-33a: the confirmation test must give `source_changed` / `folder_changed_during_check` with entry `launcher-config.json`. If it doesn't, the coder returns blocked with the observed values and doesn't change the engine.
- LT-33b: the shell preview test must give `Network` / `download_http_status` / `seed` / 500, and the confirm test `SourceChanged` / `folder_changed_during_check`. If the fixture reaches a different outcome (for example a local seed is used), the coder returns blocked.
- LT-33c: a listed site that is reached from status polling stays unwrapped and goes in the report. If `f.begin()` with an existing copy folder doesn't return `InvalidDirectory`, the coder returns blocked.

## Tests named in the packets

- LT-33a: `refuse_logs_the_shell_code_and_records_nothing` (rejection.rs), `a_refused_confirmation_carries_its_failed_check` (new confirmation_tests.rs).
- LT-33b:
  - `progress_items_map_the_engine_labels` and `a_rejection_and_a_fallback_convert_field_by_field` (rejection_dto.rs).
  - `a_refused_preview_shows_the_failed_check_beside_its_error`, `a_refused_confirmation_shows_the_failed_check` and `the_status_json_names_the_new_fields` (new rejection_tests.rs).
  - TS: `the status decoder accepts a rejection, a progress item and an index fallback`.
- LT-33c: `refusal_logs_the_check_and_returns_the_error`, `a_shell_refusal_logs_its_check` (refusal.rs).

## Ops requests (for the orchestrator)

- LT-33a reuses the existing `lt-33` worktree (branch `launcher-telemetry/lt-33-shell`, unedited at `9285f623a`). Dispatch it as a fresh packet, not retry 1/3: the block was a packet gap.
- After LT-33a merges into `launcher-telemetry/adoption`: ops retires `lt-33` and creates `lt-33b` (branch `launcher-telemetry/lt-33b-status`) at the new tip.
- After LT-33b merges: ops retires `lt-33b` and creates `lt-33c` (branch `launcher-telemetry/lt-33c-refusals`).
- LT-34 starts after LT-33c merges.
- Each packet gets the usual packet-reviewer loop.

## Ledger edits made

- work-packets-adoption.md: C14 (`refuse`, re-exports, WARN wording), C16 (rewritten), LT-33 replaced by LT-33a..c, and the LT-34 dependency and `item` values.
- RESUME.md: wave 5 row ids.
- README.md: packet table rows and the PR A chain.
- work-packets.md: the wave 5 chain.

No owner decisions (no BlockedDecision).
