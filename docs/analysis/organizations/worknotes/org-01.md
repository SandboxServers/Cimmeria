# ORG-01 Worknotes

> Type: reference. Audience: the organizations campaign coordinator.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md).

## Contract

- **Packet:** ORG-01, contract foundation. No gameplay behaviour change.
- **Decisions in force:** D-ORG21 (withdrawing is opt-in; amends D-ORG08's vault bits, relayed by the coordinator after the first push), D-ORG05 (one org-id space, squad range from `0x4000_0000`), D-ORG06 (pending invites, `BASE_INVITE_REQUEST_FLAG = 1 << 29`), D-ORG07 (ranks per type), D-ORG08 (default rank permissions), D-ORG10 (text rules), D-ORG13 (no privilege bit in a cell-to-base payload).
- **Audit rows read:** A-01 to A-10, A-12, A-16, A-19, A-28, A-30, A-41.
- **Base:** `origin/main` @ `95366c59`, branch `org/01-foundation`, worktree `.claude/worktrees/org-01`. Ledger read-only from the `org-plan` worktree (plan PR #855, not on `main`).
- **Owned paths:**
  - `crates/entity/src/organization/` (new: `mod.rs`, `types.rs`, `permissions.rs`, `limits.rs`, `org_text.rs`, `tests.rs`)
  - `crates/wire/src/cell/client_methods/organization/` (was `organization.rs`; adds `builders.rs`, `tests.rs`), `crates/wire/src/cell/client_methods/player.rs` (builders for 121, 134, 135)
  - `crates/wire/src/cell/cell_methods/organization/` (was `organization.rs`; adds `decode.rs`, `reader.rs`, `tests.rs`)
  - `crates/wire/src/base/organization.rs` (new), `crates/wire/src/lib.rs`
  - `crates/wire/src/cell/messages/org_cell_to_base.rs`, `org_base_to_cell.rs` (new), `cell_to_base.rs`, `base_to_cell.rs`, `mod.rs` (outer variants)
  - `crates/wire/src/mercury/mod.rs` (`method_idx` entries and their pin test)
  - `crates/cell-methods/src/cell/cell_methods/organization.rs`, `player/social.rs` (CM 94)
  - `crates/base/src/base/dispatch/mod.rs` (one arm), `dispatch/organization.rs`, `dispatch/tests/organization.rs` (new)
  - `crates/base-world-entry/src/base/world_entry/cell_dispatch/mod.rs` (one arm), `org_dispatch.rs`, `tests_dispatch_arms/org_arms.rs` (new)
  - `crates/cell/src/cell/service/base_messages/mod.rs` (one arm), `org.rs`, `tests/org.rs` (new)
  - `crates/wireclient/src/session.rs`
  - `crates/server/src/logging/filters.rs`, `target_scan_tests.rs`
  - Docs: `docs/gameplay/organization-system.md`, `docs/gap-analysis.md` §23, `docs/project-status.md`, `docs/protocol/client-method-dispatch-table.md`, `docs/protocol/message-catalog.md`, `docs/architecture/wireclient.md`, `docs/architecture/observability.md`, `crates/README.md`, this worknote.
- **Read set:** the ledger sections named in the task; `entities/defs/interfaces/OrganizationMember.def`, `SGWPlayer.def:877-880, 1240-1244, 1326-1333`, `alias.xml:27-36`, `enumerations.xml:104-128, 674-680, 1200-1210, 1668-1674, 1892-1959`; client `Team/Team.lua:1-123`, `Command/Command.lua:1-118`; `git show origin/feat/org-system-568:crates/services/src/base/organization/wire.rs`; `crates/base/src/base/connect_loop/cell_arms.rs`; `crates/cell-methods/src/cell/cell_methods/player/vendor/train_feedback.rs`; `crates/cell-world/src/cell/dispatch/gm_gate.rs`; `crates/base-session/src/base/gm_feedback.rs`; `crates/server/src/logging/filters.rs`, `target_scan_tests.rs`.

## Evidence

- **Editable permission bits.** `Team.lua:3-15` lists 12 (`Invite, Promote, Demote, Eject, OfficerNotes, RankNames, MOTD, DepositBank, WithdrawBank, DepositCash, WithdrawCash, AlterPerms`); `Command.lua:3-17` lists 14 (Team's plus `OfficerChat`, `EmailLists`). Neither has `TransferLeader` or `RosterNotes`. The Team editor edits ranks 2 and 3 (`for rank = 2, 3`), and names 2, 3 and 8; the Command editor edits 1-7 and names 1-8. This matches audit A-12.
- **PR #584 salvage.** `wire.rs` on `origin/feat/org-system-568` had the right `.def` field order for every builder it had (34-40, 45, 51, 134), including `onOrganizationLeft` after its fix. Its defects were in the values, not the layouts: `EReasons` guessed (disbanded 0, left 1, kicked 2; A-10 says requested 0, kicked 1, disbanded 2, logout 3) and a raw `i32` loot type. Every ported builder now takes the typed enum, and 41-44 and 46-50 are new.
- **Client cell-method encoding** (`crates/base/src/base/connect_loop/cell_arms.rs:91-120`): index below 61 is `0x80 | index`; 61 and above is `0xBD`, then the entity id, then `index - 61`. The server hardcodes 61, so `GameSession::cell_method` does too.
- **`onErrorCode` visibility.** AT-E1 found no Lua consumer of `onErrorCode` (`train_feedback.rs:5-9`, citing `docs/reverse-engineering/findings/ability-trainer-ui.md` §2). ORG-E1 Q4 is still open. So the error code alone may show the player nothing; see the base arm decision below.
- **`unicode-normalization`** is in `Cargo.lock` only as a transitive dependency; no workspace crate depends on it directly.

## Design decisions

1. **NFC skipped; the name whitelist is ASCII.** D-ORG10 says "Latin letters". `org_text` accepts ASCII letters, digits, space and `'` `-` `.`, the same class character names use (`crates/base/src/base/character_create.rs:552`). Every accepted string is then already NFC, and NFC-first would be *weaker*: U+212A KELVIN SIGN normalises to ASCII `K`, so it would pass as a homoglyph. No dependency was added, so no hakari or crate-graph run was needed. Widening past ASCII would need NFC first; the module doc says so.
2. **Minimum lengths.** Name 1 (A-13) and rank name 1 (policy: a blank rank label is not a name); MOTD and both notes may be empty (clearing them). Newline (`\n` only) is allowed in MOTD and notes; `\r` is a control and is rejected.
3. **Validation order.** Controls, bidi and zero-width are checked first on every field, so they report their own reason; then the name charset; then normalise (trim, collapse runs of spaces) and check the length on the normalised name. A 61-unit input that normalises to 60 is accepted.
4. **Lone surrogates** cannot exist in a `&str`, so the decoder reports them: `OrgDecodeError::LoneSurrogate`, and `text_reject()` maps it to `TextReject::LoneSurrogate` for the feedback path. The later packets should route that error through the same feedback as a `validate` rejection.
5. **Decoders are strict.** Every `WSTRING` count is bounded by the bytes left before anything is allocated (`saturating_mul(2)` against `remaining()`); short payloads and trailing bytes are rejected. The old decoders used `args.len() >= N` and tolerated trailing bytes. Values are raw (rank as `i32`/`u8`, loot mode as `i32`); range checks belong to the handlers.
6. **`OrgRank` is a newtype with `Ord`**, so D-ORG09 (2) is a plain comparison. `entry_for(type)` returns the first rank of `for_type` (Member for Squad and Team, Initiate for Command).
7. **`default_rank_permissions` returns `Vec<(OrgRank, OrgPermission)>`**, one row per `for_type` rank, lowest first; empty for Squad (never persisted, no editor). Team `SeniorMember` gets `Officer`'s bits literally, including `OfficerChat`, as D-ORG08 says. **D-ORG21** replaces the vault bits: every rank below `Leader` (Command 1-7, Team 2-3, `Initiate` included) gets `DepositBank | DepositCash | ViewBankLogs` (1,376,256) and none gets `WithdrawBank` or `WithdrawCash`. Literal masks: Initiate 1,376,256; Member to SeniorVeteran 1,376,288; Officer and Team SeniorMember 1,377,650; SeniorOfficer 9,766,398; Leader 0x3FF_FFFF. `default_rank_permissions_follow_d_org08_and_d_org21` pins them and asserts no withdraw bit below Leader; it FAILED (exit 101) with `BANK_DEFAULT` temporarily set to `DepositBank | WithdrawBank`, and passes restored.
8. **`method_idx` re-exports** the `client_methods::organization` and `player` constants rather than copying them (the two tables drifted before; `.claude/agent-memory/rust-gameserver-dev/method-idx-duplicate-table-drift.md`). A literal pin test covers the 20 entries.
9. **Messages.** `OrgCellToBase { Create, TransferCash, ForwardCellCall }` and `OrgBaseToCell { SquadInvite, SquadKick }`. `SquadKick` is not in the task list but ORG-03 names it ("0xD1 with a squad-range id, which ORG-01's base arm forwards to the cell"). `Create` carries `org_type: OrgType` (from the pending creation, never the wire). `ForwardCellCall` carries the raw index and args; the base decodes with `decode_org_cell_method`. Each variant has `player_id` and `entity_id`, plus `actor()` and `kind()` for logs.
10. **Base arm feedback (0xCF-0xD2).** Every well-formed call gets `onErrorCode(SystemID 0 = ERRORCODE_SYSTEM_Ability, InstanceID = org id (or the type for invite-by-type), ErrorCodeID 0 = CONDITION_FEEDBACK_InvalidEntity)`, the generic refusal the GM gate already sends (`gm_gate.rs:46-51`), **and** a feedback chat line, "Organizations are not available yet.", through `send_gm_feedback_to_client` (speaker `SYSTEM`, tell channel 9). The line is there because the error code alone may render nothing (Evidence). A malformed payload is logged at WARN and not answered: a real client cannot produce it, and answering would give a forger a free packet per call (the `train_feedback.rs` rule). A session with no player entity is logged and not answered.
11. **The forward of 0xD1 with a squad-range id is not wired** in ORG-01. The base arm answers every 0xD1 with the feedback. ORG-03 adds the forward in `dispatch/organization.rs`.
12. **Cell arms keep INFO** for the existing "UNIMPLEMENTED" rows, now on target `org` with `method`, `org_id` and `text_units`; the text itself is never logged. The no-op message arms log at DEBUG (`org` on the base, `squad` on the cell).
13. **`cell-methods/.../organization.rs` stays one file** (under 200 lines). ORG-03 splits it into `organization/` per the contended-file list.
14. **Docs not touched because #855 already fixes them:** `docs/protocol/sgwplayer-base-method-dispatch-table.md` (argument order) and `docs/protocol/cell-method-dispatch-table.md`. Editing them here would conflict.

## Round 2: authority-review fixes (coordinator, 2026-09-27)

Decision clarifications for the ledger (the coordinator records (a) as D-ORG22 and (b) as D-ORG23):

- **(a) Permission-edit semantics, `OrgPermission::apply_edit(old, wire, org_type, actor)`.** `stored = (old & !editable) | (wire & editable)`: a bit the type's editor does not show (`RosterNotes`, `ViewBankLogs`, `TransferLeader`, ...) keeps its stored value whatever the client sends, so the client's mask can neither strip nor set it. The edit is rejected (`PermEditReject::ChangesUnheldBits(bits)`) when `(stored ^ old) & !actor` is non-zero: only the bits the edit actually changes, granted or revoked, must be held by the actor. An unheld bit left as it was does not block the edit. The function knows no rank, so the caller refuses an edit of the `Leader` row first; `apply_edit_does_not_protect_the_leader_row_itself` pins why.
- **(b) Text classes.** Every field rejects general category `Cf` wholesale through a const range table (`org_text::FORMAT_RANGES`, from Unicode 15.1 with the tag block widened to U+E0000-E007F), and `Zl`/`Zp` (U+2028, U+2029). Reject reasons: `Bidi` (U+061C, U+200E-200F, U+202A-202E, U+2066-2069), `ZeroWidth` (U+200B-200D, U+2060, U+FEFF), `Format` (the rest of `Cf`: U+00AD, U+0600-0605, U+180E, U+2061-2064, U+206A-206F, U+FFF9-FFFB, tags, ...), `LineSeparator` (U+2028-2029). No Unicode-data dependency.
- **(c) Rank names** are free text (any script) but are trimmed and have every internal run of whitespace (`char::is_whitespace`, so NBSP and U+3000 too) collapsed to one ASCII space; empty after that is `TooShort`, and the 32-unit cap applies after collapsing. `validate(RankName, ..)` returns the normalised text.
- **(d)** `limits::route_org_id` (`<= 0` is `None`, `>= 0x4000_0000` is `Squad`, else `Base`) and `route_invite_request` (`<= 0` is `None`; the bit-29 flag is tested on positive ids only, so `-1`, whose bit 29 is set, routes nowhere).
- **(e)** `CashDir { Deposit(u32), Withdraw(u32) }` from CM 19's signed amount via `unsigned_abs`; zero is `OrgDecodeError::InvalidValue { field: "aCash" }`. The sign convention is the client's: `Command.lua:180-194` (`onWithdrawClicked` sends `commandTransferCash(-cashAmt)`, `onDepositClicked` sends `cashAmt`), and `Team.lua:184-199` likewise. `i32::MIN` is `Withdraw(2_147_483_648)`. `OrgCellCall::TransferCash` and `OrgCellToBase::TransferCash` carry the `CashDir`.
- **(f)** CM 10 rejects NaN and the infinities at decode (`InvalidValue { field: "aLocation.x|y|z" }`).
- **(g)** The base `ForwardCellCall` arm refuses a `method_index` outside `8..=17` before decoding (WARN `org.forward_rejected`, `reason = method_out_of_range`), then decodes and logs; a decode failure is WARN `org.forward_rejected` with the decoder's reason.
- **(h)** Cell methods 8-19 and 94 now answer every well-formed call like the base arm: `onErrorCode(0, org id or 0, 0)` then the feedback line, sent as two `EntityMethodCall`s (`organization::send_unavailable_feedback`), and log at DEBUG instead of INFO. Malformed calls still get no answer. The feedback text and the two error-code constants moved into `cimmeria-wire` (`client_methods::organization::ORG_NOT_AVAILABLE_TEXT`, `player::ERRORCODE_SYSTEM_ABILITY`, `CONDITION_FEEDBACK_INVALID_ENTITY`) so both services send the same bytes. The `("org", INFO)` pin left `scan_finds_known_targets`, since no org INFO row remains.
- The `organization-system.md` client-method heading now says 18 methods.

Round 2 regression proof: with the forward range gate disabled, the cell feedback call removed and the `Cf` table check disabled, `lane.sh cargo test -p cimmeria-entity -p cimmeria-cell-methods -p cimmeria-base-world-entry --lib org --no-fail-fast` exited 101 (`forward_outside_8_to_17_or_malformed_is_rejected`, `every_org_method_is_answered`, `motd_is_decoded_and_answered`, `every_field_rejects_each_format_class` FAILED); restored, all pass.

Round 2 commands (all exit 0): the seven-crate `--lib` test run (entity 341, wire 194, cell-methods 224, cell 448, base 72, base-world-entry 120, wireclient 37); `cargo test -p cimmeria-server --bin cimmeria-server logging` (52); `cargo fmt --all -- --check`; `cargo clippy --all-targets -- -D warnings` over the 8 touched crates plus the 14 that name the message enums.

## Commands run

All from the worktree root, through the lane (`target=B:\targets/org-01`). No live-DB run: ORG-01 has no database code.

| Command | Exit | Result |
|---|---|---|
| `lane.sh cargo test -p cimmeria-entity --lib organization` | 0 | 19 passed |
| `lane.sh cargo test -p cimmeria-wire --lib` | 0 | 190 passed |
| `lane.sh cargo check -p cimmeria-wire -p cimmeria-cell-methods -p cimmeria-cell -p cimmeria-base -p cimmeria-base-world-entry -p cimmeria-wireclient --all-targets` | 0 | no warnings |
| `lane.sh cargo test -p cimmeria-wire -p cimmeria-cell-methods -p cimmeria-cell -p cimmeria-base -p cimmeria-base-world-entry -p cimmeria-wireclient --lib org` | 0 | base 3, base-world-entry 1, cell 1, cell-methods 6, wire 48 |
| `lane.sh cargo test -p cimmeria-wireclient --lib session::tests` | 0 | 4 passed |
| `lane.sh cargo test -p cimmeria-server --bin cimmeria-server logging` | 0 | 52 passed (target scan and parity) |
| `lane.sh cargo fmt --all`, then `cargo fmt --all -- --check` | 0, 0 | |
| `lane.sh cargo clippy -p cimmeria-entity -p cimmeria-wire -p cimmeria-cell-methods -p cimmeria-cell -p cimmeria-base -p cimmeria-base-world-entry -p cimmeria-wireclient -p cimmeria-server --all-targets -- -D warnings` | 101, then 0 | `chunks_exact_to_as_chunks` in `reader.rs`, fixed |
| `lane.sh cargo clippy` on every other crate that names `CellToBaseMsg`/`BaseToCellMsg` (admin-api, base-methods, base-session, cell-catalog, cell-combat, cell-console, cell-content, cell-interactions, cell-world, content-engine, lab-mcp, minigame, services, wire-log) `--all-targets -- -D warnings` | 0 | the new outer variants break no exhaustive match |
| `lane.sh cargo test -p cimmeria-entity -p cimmeria-wire -p cimmeria-cell-methods -p cimmeria-cell -p cimmeria-base -p cimmeria-base-world-entry -p cimmeria-wireclient --lib --no-fail-fast` | 0 | 332 / 190 / 222 / 443 / 69 / 119 / 37 passed (live-DB tests self-skipped; none are ORG-01's) |

## Regression proof

Three reverts applied together on a clean tree, run, then restored with `git checkout -- <file>`:

- `org=debug,squad=debug` removed from `OTEL_FILTER`.
- The CM 13 decoder changed to `motd: String::new()` (does not read the `WSTRING`).
- The `ORGANIZATION_INVITE..=ORGANIZATION_RANK_CHANGE` arm removed from `dispatch_sgw_player_base_method`.

`lane.sh cargo test -p cimmeria-wire -p cimmeria-cell-methods -p cimmeria-base --lib org --no-fail-fast` exited 101:

- base: `kick_is_answered_with_error_code_then_feedback`, `all_four_ids_reach_the_org_arm`, `malformed_payload_is_logged_and_not_answered` FAILED;
- cell-methods: `motd_is_decoded_with_its_text` FAILED;
- wire: `cm13_motd_reads_its_wstring`, `wstring_count_is_bounded_by_the_payload`, `lone_surrogate_is_a_text_reject` FAILED.

`lane.sh cargo test -p cimmeria-server --bin cimmeria-server logging` exited 101: `every_source_target_reaches_signoz_at_its_level` FAILED with "org at DEBUG (base\src\base\dispatch\organization.rs:98) reaches 0 OTLP log indexes" and the same for `squad` (`cell\src\cell\service\base_messages\org.rs:15`).

Restored; `git status` clean.

## Known gaps

- `send_gm_feedback_to_client` is named for GM feedback but is the only base-side single-line feedback sender; the org arm reuses it. A neutral name would help when ORG-03/07 add more feedback.
- `RosterInfo` is a wire struct (`cimmeria_wire::cell::client_methods::organization::RosterInfo`), not an entity model; ORG-06 fills it from the roster query.
- Nothing is sent to a client except the base arm's feedback; every builder is exercised only by its byte test.
- The message-catalog Organizations table still lists a non-existent `createOrganization` and attributes cell methods to `OrganizationMember.setMOTD` and similar names; I added a note rather than rewriting the table.

## Integration edits for the coordinator

- None to the ledger are required by the code. The contract names hold, with two refinements to record: `default_rank_permissions` returns a `Vec` (empty for Squad), and `OrgBaseToCell` has a `SquadKick` variant.
- The new docs link to `docs/analysis/organizations/README.md`, which resolves once #855 merges. Merge #855 first, or accept a dangling link until it does.
- `crates/base/src/base/dispatch/mod.rs` and `crates/cell-methods/src/cell/cell_methods/organization.rs` are in their ORG-01 state for ORG-03 to take over (arm and constants landed; file split is ORG-03's).
