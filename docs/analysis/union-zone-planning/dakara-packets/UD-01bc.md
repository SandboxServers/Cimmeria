# UD-01b and UD-01c: Free Jaffa bridge build (skeletons)

> **Type:** How-to (work packet skeletons) · **Wave:** U2 · **Status:** BlockedDependency
> **Index:** [dakara-work-packets.md](../dakara-work-packets.md)

**These are skeletons, not dispatchable packets.** The architect fills them from the sealed `bridge-seal.md` (UD-01a) before dispatch. Three gates stay open until then:

1. D-UD1 answered and UD-01a merged.
2. An authored mission with a dialog override has merged to `main` and passed client journal UAT. The `MISSION_ADDITIONS` mechanism is on main (PR #1734) but empty apart from the test fixture 61000, and today the only dialog-override precedent, Cellblock 62050 with dialog override 60150, is on `content/cellblock-campaign`; PR #1795 closed unmerged (F9). The fill copies that precedent's file layout, so it must exist on main first.
3. H-U07 (rewards) and H-LBL (copy labels) answered, or the fill keeps `[REWARD HELD]` and `[COPY HELD]` and ships behind a disabled chain. That choice is the owner's.

Everything in both packets is **CIMMERIA_AUTHORING**. They can be dropped whole without touching any other packet: chain 8001 and the DK chains do not depend on them.

## Fixed parts (true whatever the seal says)

- IDs only from 63500-63799. Mission 63550; steps 63560+; objectives 63590+; tasks 63620+; dialogs 63650+; sets 63670+; screens 63680+; buttons 63730+; set maps 63770+; chains 63510-63549. Chain 63500 belongs to UD-04a.
- Every chain is gated `archetype eq 7`.
- The seed file is `db/resources/Content/Seed/union_dakara_e1_bridge_chains.sql`, kept separate from UD-04a's file so the bridge can be dropped. It gets its own `\ir` line after `union_dakara_e1_chains.sql` in `db/database.sql`.
- The test module is `chain_replay_tests/union_dakara_e1_bridge.rs`.
- Rules: the same ten as [UD-04a](UD-04a.md#rules-binding-packet-coder-skips-claudemd), with `ud-01b` / `ud-01c` as the worktree.

## UD-01b: catalog and mission rows (packet-coder, about 35k)

| Field | Value |
|---|---|
| Branch / worktree | `union-dakara/ud-01b-bridge-mission` / `ud-01b` |
| Commit subject | `feat(resources): authored Free Jaffa bridge mission 63550 (UD-01b)` |

Allowlist, to confirm at fill time against the precedent:

- `crates/resources/src/base/mission_additions.rs`: one `MISSION_ADDITIONS` entry for 63550 with its steps, objectives and tasks.
- The mission seed rows in `db/resources/` that the precedent touches. Name the exact files at fill time.
- `crates/resources/src/base/resources/tests/mission_additions.rs`: one test that 63550 loads, sits outside the stock pak range and has the sealed step order.

Lane: `fmt`, `clippy --all-targets -- -D warnings`, `nextest run -p cimmeria-resources`, each through `pwsh tools/build-lane/lane.ps1 cargo ... -p cimmeria-resources`.

Docs row: "Mission PAK overrides / authored mission catalog", in `docs/architecture/mission-pak-overrides.md`.

## UD-01c: chains, dialogs and replay tests (packet-coder, about 40k; split if the seal has more than three steps)

| Field | Value |
|---|---|
| Branch / worktree | `union-dakara/ud-01c-bridge-chains` / `ud-01c` (stacks on `ud-01b`) |
| Commit subject | `feat(content): Free Jaffa bridge chains and dialogs (UD-01c)` |

Allowlist: the bridge seed file, the `\ir` line in `db/database.sql`, the dialog override file the precedent uses, the test module, and `mod.rs`.

Tests to write (types per TESTING.md: live-DB chain replay plus one executed `fire_player_loaded` test):

- the offer fires for archetype 7 only, once;
- each step advances only from its own step;
- wrong archetype (1 to 6, 8, `None`) resolves nothing;
- a relog at each step restores it without re-offering;
- completing the mission makes `mission_63550_status = completed`, the condition DK-11 reads;
- chain 8001's Omega grant is unchanged (rerun `chain_replay_tests::dakara_e1_space`).

Telemetry: `Content: accepting mission` (offer chain), `Content: advancing step` per step, and `Content: final required objective completed the mission — firing mission_completed`, all from `executor/mission.rs`.

Docs rows: "Game systems, content chains" (`docs/content/mission-chains.md`, plus a line in the Union block of `.github/instructions/content-chains.instructions.md`). The UAT rows go to UD-05.
