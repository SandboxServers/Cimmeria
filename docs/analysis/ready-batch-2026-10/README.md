# ready-batch-2026-10: campaign ledger

Purpose: ship the eight issues triaged as ready in October 2026 (#171, #246, #447, #448, #733, #1119, #1249, #1675) as small independent packets, so cheap `packet-coder` sessions can do most of the work and one `packet-review-fixer` reviews each wave.

Packets and their contract: [work-packets.md](work-packets.md). Resume card: [RESUME.md](RESUME.md).

## Findings

| # | Issue | Finding | Packet |
|---|---|---|---|
| F1 | #448 | **Already fixed.** PR #1051 (AM-03) made `validate_ammo_change` fail closed with `AmmoChangeRefusal::WeaponDefCacheMiss`. The guard test is `crates/cell-methods/src/cell/cell_methods/inventory/tests/ammo_change/mod.rs:306`. Nothing to build. | none; close #448 citing #1051 (P12) |
| F2 | #447 | `crates/auth/src/auth/soap_response.rs` formats `msg`, the shard name and host, the session key and the ticket into XML attributes with no escaping. Stale PR #604 targets the old `crates/services/src/auth/handlers.rs`, which no longer exists. | P01; close #604 (P12) |
| F3 | #733 | `crates/mercury/src/bundle.rs` `Bundle::encode` clamps the payload length to `u16::MAX` and silently truncates the payload to 65535 bytes, so the message is cut short with no error. `Bundle` has no production caller. | P02 |
| F4 | #246 | Three groups of small review follow-ups: wrapping `as i32` narrowings in `loader/action.rs`; base-side tidy-ups (a duplicated player-id decode, a history comment, a non-exhaustive tag test); and mercury/cell-content tidy-ups (no byte-exact `Packet::encode` test, the misnamed `removed_flags`, and a `require_db_or_skip!` check). | P03, P04, P05 |
| F5 | #1249 | `check_kinematics` measures speed from a single packet. Jittered packets (25 ms then 175 ms) imply up to 4x top speed and raise `movement.speed_warning`. PR #1463 only demoted the dt≈0 case (`speed_bunched`) to DEBUG. The fix is a per-entity sliding window. | P06 (window), P10 (wire it in) |
| F6 | #171 | `mercury::method_idx` (wire `mercury/mod.rs`, lines 205-351) repeats about 75 literals that `cell::client_methods` already declares. cell-console keeps 5 local copies. Conformance test #801 catches drift, but the duplicates remain. | P07 |
| F7 | #1675 | `/gmgiveitem` asks for `INV_MAIN`. `grant_container` keeps a carried-bag request even when the item lists only `{2}` (mission), so mission-only items land in Main. The cell cache knows only the preferred container, so a cell-side fix would misroute `{3,1,17}` weapons. Fix it at the base. | P08 |
| F8 | #1119 | Mostly done already. #1299 added the timeouts, the 256-total/8-per-IP caps, refusal of a second claim and send timeouts. #1348 refuses peers with no session before any byte is read. #1624 added `pending_sessions`. What is left from the owner decisions of 2026-09-30: replace the caps with **one connection per valid ticket**, and match non-public peers as one class. | P09 (admission), P11 (listener) |
| F9 | all | Files already over the 700-line hard cap on these paths: `cell-world/.../client_move.rs` (716). Over the 500 soft cap: `wire/.../mercury/mod.rs` (657), `entity/.../movement_validation/mod.rs` (613), `content-engine/.../loader/action.rs` (565), `minigame/.../session.rs` (589), `listener_tests.rs` (629), `minigame/.../tests.rs` (644). Every packet that touches one of them must shrink it, or at least not push it past 700. | P03, P06, P07, P09, P10, P11 |

## Decisions

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-01 | #171: `cell::client_methods` holds the only literals. `mercury::method_idx` becomes `pub use` re-exports, plus a new `client_methods/mob.rs` for SGWMob 27/28. | Decided (architect) | This **reverses the issue body**, which proposed keeping `method_idx`. The tables are already complete over SGWPlayer's 157 indices and checked by #801 surface B, and re-exports keep every `method_idx::X` caller compiling. |
| D-02 | #733: `Bundle::encode` returns `cimmeria_common::Result<()>`. It refuses any payload of `0xFFFF` bytes or more (the BigWorld escape is unsupported) and leaves the buffer untouched. `decode` refuses the `0xFFFF` sentinel. | Decided (architect) | Failing loudly beats a silent corrupt frame. There is no production caller, so it breaks nobody. Deleting the module was the alternative. |
| D-03 | #1675: in `grant_container`, a carried-bag request for an item whose sets list no carried bag but include `INV_MISSION` (2) goes to 2. | Decided (architect) | This is server-authoritative with no client patch. The cell-side alternative misroutes weapons (`{3,1,17}` would go to the bandolier). Trade also calls `grant_container`; D-08 rules on that. |
| D-08 | #1675, trade side of D-03: a trade of a mission-only item (`{2}`) is **refused** (`TradeAbort::NoDestination`, "cannot be traded" to both sides). `trade_destination` keeps calling `grant_container`, which now gives 2, and 2 is not in `TRADEABLE_CONTAINERS`. Trade gets no separate rule, and the whitelist is unchanged. | Decided (architect, 2026-10-20 ruling) | Mission items are not tradeable by design: the mission bag is off the whitelist, and `IneligibleContainer` names the mission-item-share exploit. 657 of the 749 `{2}` types lack `CAN_BE_SOLD`, which the Python trade gated on. A `{2}` item in bag 1 is a #1675 leftover; the move path refuses to put one there. The old rule copied it into the recipient's backpack. The owner can drag it into the mission bag (2 is player-movable). No seed, wire or client change. Worknote: `rb10-p08/docs/analysis/ready-batch-2026-10/worknotes/P08.md`. |
| D-09 | #1675 docs: the stale sentence at `docs/gameplay/mail-system.md:42` and the unified-uat K38 step ("mission-only item lands in the Main bag") go to P12. P08 is not widened for them. | Decided (orchestrator, 2026-10-21) | P08 has used up its 3 review passes. Both are doc-only, outside P08's file list (R7), and P12 already owns unified-uat. |
| D-04 | #1119: keep the merged timeouts (30 s handshake, 30 min idle) and the reason strings `unexpected_peer` / `unexpected_peer_with_sessions`. Add `no_free_ticket`. | Decided (architect) | #1299 chose them on purpose and tests pin them. The issue's 10 s / 15 min values predate that. |
| D-05 | #1119: remove the global (256) and per-IP (8) connection caps and replace them with per-ticket admission. | Decided (owner, 2026-09-30 in #1119) | The owner wrote "No global or per-IP connection caps" and "one connection per valid ticket". **This undoes the caps #1299 added.** The orchestrator should confirm before P11 ships. |
| D-06 | #1249: window rule. Over a 250 ms window, warn when the summed distance divided by `max(summed dt, 250 ms)` exceeds top speed x 1.5. Skip samples under 0.01 u. Keep at most 32 samples. | Decided (architect) | A 250 ms window absorbs client send jitter, and the dt floor stops a burst of near-zero-dt packets from reading as infinite speed. |
| D-07 | #448 needs no packet; ops closes it citing #1051. Ops closes the stale PR #604; P01 replaces it. | Decided (architect) | F1, F2. |

No packet is BlockedDecision. None adds an opcode, touches wire crypto, needs a client patch or changes a seed. D-05 is the only owner-sourced decision; it is already decided, but flag it before shipping P11.

## Packet status

Closed 2026-10-21 by P12. "UATPending" means the code is shipped (or awaiting merge) and the owner steps added to [unified-uat.md](../../guides/unified-uat.md) have not been run: mission-bag grant (K38, Mail 3a/3a2, Bank 8), per-ticket minigame admission (MG-U11, MG-U12) and the speed window (TG-UAT-G11).

| Packet | Issue | Wave | Implementer | Depends on | Status |
|---|---|---|---|---|---|
| P01 auth XML escaping | #447 | 1 | packet-coder (Haiku) | none | Done. PR #1955 merged. |
| P02 Bundle::encode refuses oversize | #733 | 1 | packet-coder (Haiku) | none | Done. PR #1956 merged. |
| P03 action loader narrowing | #246-A | 1 | packet-coder (Haiku) | none | Done. PR #1957 merged. |
| P04 base follow-ups | #246-B | 1 | packet-coder (Haiku) | none | Done. PR #1958 merged. |
| P05 mercury/cell-content follow-ups | #246-C | 1 | packet-coder (Haiku) | none | Done. PR #1960 merged. |
| P06 speed window (entity) | #1249-A | 1 | packet-coder (Haiku) | none | Done. PR #1959 merged. |
| P07 method-index single source | #171 | 2 | packet-coder (Haiku) | none | Done. PR #1961 merged. |
| P08 mission-only grant placement | #1675 | 2 | packet-coder (Haiku) + items-systems-advisor read-only pass | none | UATPending. PR #1963 is open and awaiting the maintainer; P12 was written as if it had landed. |
| P09 per-ticket admission (registry) | #1119-A | 2 | rust-gameserver-dev (Sonnet) | none | Done. PR #1962 merged. |
| P10 speed window in client_move | #1249-B | 2 | rust-gameserver-dev (Sonnet) + movement-teleport-advisor pass | P06 merged | UATPending. PR #1966 is open and awaiting the maintainer; P12 was written as if it had landed. |
| P11 per-ticket admission (listener) | #1119-B | 3 | rust-gameserver-dev (Sonnet) + minigame-systems-advisor + server-authority-enforcer | P09 merged | UATPending. PR #1967 is open and awaiting the maintainer; P12 was written as if it had landed. |
| P12 close-out | all | 3 | documentation-writer | P01-P11 merged | Done. This PR; it merges after #1963, #1966 and #1967. |

## File overlap

- P06 then P10: `crates/entity/src/movement_validation/mod.rs`. These run one after the other, in different waves.
- P09 then P11: `crates/minigame/src/minigame/session.rs`. These run one after the other, in different waves.
- P02 and P05 share the crate `cimmeria-mercury`, but no file (`bundle.rs` against `packet/tests.rs`).
- P07 and P08 share the crate `cimmeria-cell-console`, but no file (`net.rs` and `tests/*` against `gm/give.rs`, where P08 changes only a comment).
- P04 and P08 do not overlap. P04 touches `base-session` and `base` and `base-world-entry`; P08 touches `cell-catalog` and `base-methods` (with D-08, also `trade/execute/{placement,abort,tests}.rs`, `docs/gameplay/trade-system.md` and one line of `docs/architecture/observability-target-catalog.md`).

## Out of scope (follow-ups to file at close-out)

- #1675: vendor purchases still put a mission-only item in the main bag (`vendor/purchase/placement.rs::purchase_container` uses `first_player_container(..).unwrap_or(INV_MAIN)` and does not call `grant_container`).
- #1675: `{2}` rows already sitting in bag 1 stay there; there is no data migration. Trades of them are refused (D-08), and the owner can drag them into the mission bag. Mention this in the #1675 close-out comment.
- #1249: `docs/architecture/discord-notifications.md:291` still says "infinite implied-speed ratios". That is stale after P10 and is a one-line P12 docs fix.
- #171: other test-local index copies in `wireclient`, `base-session`, the cell dispatch and `respec_burst` tests, and the production literals 80/81/82 in `crates/cell-content/src/cell/missions/mod.rs:134-138`.
