# UO-16: ruling on the wave 6a review

> **Type:** Reference (ruling note) · **Packet:** [UO-16](../UO-16.md) · **Worknote ruled on:** [UO-16-asgard-seal.md](UO-16-asgard-seal.md) at bc3621b16
> **Review:** wave 6a review, § UO-16 (verdict `decision`, notes N1-N6) · **Ruling:** re-spec the worknote, accept with follow-up edits · **Date:** 2026-10-19

The owner asked for a decision, not a block. Every call below is a default the owner may overrule; the cost is in each item.

## Applying this ruling

The implementer edits **only** `docs/analysis/union-zone-planning/omega-packets/worknotes/UO-16-asgard-seal.md`. Rules:

1. PowerShell only, absolute paths. Start every command with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\uo-16;`.
2. Never run `cargo` directly. `nav_inspect` runs only through `pwsh tools/build-lane/lane.ps1 cargo run -p cimmeria-navmesh-extractor --bin nav_inspect -- ...`, in the foreground.
3. `docs/**/*.md` is CRLF. After editing, convert with `[IO.File]::WriteAllText(p, ([IO.File]::ReadAllText(p)).Replace("`r`n","`n").Replace("`n","`r`n"))` on the absolute path.
4. Files: soft cap 500 lines, hard cap 700.
5. Probe files go to `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\uo-16\.claude\reports\union-omega\probes\` and are not committed.
6. No seed, code, ledger or other packet edits. No lab, no `sgw` database, no push, no `git stash`, no `git worktree prune`.
7. One local commit with the attribution lines from your brief. Report to `...\uo-16\.claude\reports\union-omega\UO-16-ruling-1.md`.

## R1. The 481 offerer: a dedicated hub Thor (decision)

**Ruling: option (b), venue-independent.** A persistent world-18 Thor offers 481. The hearing Thor never does.

| Item | Value |
|---|---|
| Spawn | **63873** (`spawnlist`), world 18, key `asg.thor`, tag `UnionOmega_AsgardThor` |
| Template | **63862** (`entity_templates`), moniker 28128 `Thor (Hologram)`, the same row and body as 63858 (D-UO37 Asgard body rule); only its dialog binding differs |
| Place | the Asgard area (west pod) beside trainer 63809, facing the walkway; UO-37 seals the point |
| Role | offers 481 once P5 is complete, with § Departure's predicate. The same predicate offers it again after an abandon (the D-UO10 rule), because the spawn is permanent |
| Never | template 63858 (the hearing Thor, spawn 63836) or 63803 (D-UO9) offers 481 |

Why:

- **No soft-lock.** Option (a) puts the offer on a one-shot hearing screen. A player who abandons 481 has no NPC to take it from again; D-UO10 exists to stop exactly that.
- **Venue-independent.** It holds whether R3's probe finds a chamber or keeps the world-80 fallback, so the probe cannot reopen this call.
- **Its own template.** The Council cast shares template 63858. A separate template keeps the offer off the hearing set (world 80 under the fallback) whether the engine binds a dialog set by template or by spawn. One template row is the whole cost.
- **Not 63838/63839.** Those stay `council.extra.*` for a Council speaker the owner may add under D-UO34.

Collision check (2026-10-19, `git grep` over `db/resources`, `crates` and every `union-zone-planning` copy in all worktrees): spawn 63873 is in the ledger's spare 63873-63879 and used nowhere else. Template 63862 is in the spare 63862-63899 and used nowhere else. (Spawn ids 63862/63863 in UO-18/UO-19/UO-28 are `spawnlist` rows; dialog screen 63874 is a different table.) Spawn 63873 is outside the ambient-only band 63880-63959 (F44).

Cost if overruled: drop one template and one spawn row from UO-42, and move the offer onto the hearing's Accept.

Worknote edits:

- § Departure, "Who offers": replace the bullet with: "**Who offers:** the hub Thor, template 63862 on moniker 28128, spawn 63873 (`asg.thor`, tag `UnionOmega_AsgardThor`), world 18, in the Asgard area beside trainer 63809. He offers 481 once P5 is complete, and again after an abandon (same predicate, D-UO10). The hearing Thor 63858 never offers 481 (UO-16 ruling R1)."
- § Hearing, Speakers: append "The hearing Thor never offers 481 (§ Departure)."
- § Asgard area, Placement keys: append "hub Thor `asg.thor` (spawn 63873, template 63862)".
- § Slice, UO-42 Scope: after "Otin 63859 (only if D-UO34 says yes);" insert "hub Thor template 63862 and spawn 63873;".
- § IDs: in the `entity_templates` 63858/63859 row, add "63862 hub Thor (UO-16 ruling R1)" to Reserved and Used. In the `spawnlist` 63836-63839 row, add "63873 hub Thor".

## R2. Prologue ids and the UO-48 ordering (N3, amended)

**Ruling: accept N3's dependencies, and reserve the row ids now.** No coder asks `database-persistence` mid-wave.

| Table | Reserved | Use |
|---|---|---|
| `missions` | 63800-63805 | P1 63800, P2 63801, P3 63802, P3-S 63803, P4 63804, P5 63805 |
| `mission_steps` | 63810-63829 | 11 steps (P1 2, P2 2, P3 2, P3-S 1, P4 2, P5 2), in mission then step order from 63810 |
| `mission_objectives` | 63830-63849 | one per step, same order from 63830 |
| `mission_tasks` | 63850-63869 | one per objective, same order from 63850 |

The ids are free: no `638xx` or `639xx` id appears under `db/resources/Missions` or `db/resources/Texts` on this branch or on `origin/main`, and no sibling ledger names one for these tables. D-UO1 gives Omega 63800-63999 in every table. Each table has its own key, but distinct numbers across the four tables avoid the S6-style confusion. Dialog, screen, button, text and reward ids are not reserved here: the architect allocates them in the full UO-41..UO-49 packets with a per-table `git grep` (stock screens sit at 63847-63887; dialogs from 63901 and screens from 63908 were free on 2026-10-19).

Worknote edits:

- § IDs, the "missions, steps, objectives, dialogs, texts" row: Reserved becomes the four ranges above; Still needed becomes "dialogs, screens, buttons, texts and reward rows: the architect allocates them in the full packets".
- § Slice: UO-48 Depends on becomes "UO-16". Add "UO-48" to the Depends on cells of UO-43 and UO-47.
- § Slice, UO-48 Scope: append "plus each mission's client copy, a `MISSION_ADDITIONS` entry pinned by a test (precedent: Dakara mission 63100, `crates/resources/src/base/mission_additions_dakara.rs`); the architect splits it if it exceeds 40k".

## R3. Council Chamber probe (N1, re-specified)

N1's grid (x 34..40) is too narrow. The art has no scale: north-south reads about 4.4 m per pixel and east-west about 1.6, so the chamber label (580, 138) maps anywhere from z ≈ 470 to past the mesh edge (z 520). Probe a wide grid instead.

1. **Pass A (discovery).** Write `council.txt` in the probes folder: first `modi_seed78 39.54 -51.91 -129.92`, then 99 points named `cc_x<X>_z<Z>` at y `-45`, for X in -120, -90, -60, -30, 0, 30, 60, 90, 120, 150, 180 and Z in 100, 150, ..., 500. Run:

   ```powershell
   Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\uo-16; pwsh tools/build-lane/lane.ps1 cargo run -p cimmeria-navmesh-extractor --bin nav_inspect -- data/spaces/omega_site.nav --probes C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\uo-16\.claude\reports\union-omega\probes\council.txt --h-tol 2.0 --v-tol 40 --quiet
   ```

   Exit 2 or 3 is expected; read the per-probe lines (`component=`, `h=`, `dy=`, `ok`). Save the output beside the probe file.
2. **Hits.** A *chamber hit* is a line that reads `ok`, has Z ≥ 200 and is on Modi's component (60). Lines at Z 100-150 are the hub's north edge: count them, but they are not the chamber.
3. **Pass B (confirm).** For each hit, probe `modi_seed78`, `pad14 34.56 -36.48 55.68` and the hit at y = -45 − dy (`dy` is probe minus surface), with default tolerances. The hit counts only if that run exits 0.
4. **Verdict.** **Form 2** if the confirmed hits include a 2×2 block of grid-adjacent points (a floor of at least 30 m × 50 m). Otherwise **Form 1**. Hits at Z ≥ 200 on another component are "geometry, not reachable": still Form 1. Name those components.
5. **Record** in § Venue, replacing "and I did not probe the navmesh in this packet": "Probe 2026-10-19: 99 points, x -120..180 step 30, z 100..500 step 50, y -45, v-tol 40. <n> confirmed hits on component 60 at z ≥ 200; <m> lines at z ≥ 200 on components <ids>; <k> at the hub's north edge." Then:
   - **Form 1:** "The Council Chamber has no reachable geometry, so the world-80 fallback stands (D-UO20)." Keep P5, § Hearing and the fallback. Rewrite Open question 3 as: "**Venue:** the probe found no reachable chamber on world 18; the hearing uses the world-80 Command Center (D-UO20). Owner: accept, or commission new geometry (out of scope)."
   - **Form 2:** name the patch (centroid x/z, extent, component 60) as the chamber. P5's Where becomes "World 18 Council Chamber". The § Hearing start event becomes "the player's dialog with the hearing Thor (spawn 63836) while P5 S1 is active (`dialog_choice`, D-UO4; no `interact_tag`)". Delete the world-80 fallback from § Venue and Open question 3, and renumber.

R1 does not change under either form.

## R4. Notes accepted as written

N2, N4, N5 and N6 apply verbatim from the review. N2 deletes Open question 7 (the vendor split); renumber. After R3, check the list numbering once.

## Follow-up edits owed (not part of this fix)

The orchestrator makes these before dispatching UO-37, or in the W10 packet-writing pass. They are docs-only and need no packet.

| File | Edit | Must cite |
|---|---|---|
| `omega-work-packets.md` § ID map | `entity_templates`: add 63862, "Thor (Hologram) 28128, hub 481 offer (UO-42, UO-16 ruling R1)"; spare becomes 63863-63899. `spawnlist`: add 63873, "Asgard hub Thor (UO-42)"; spare becomes 63874-63879. Add a missions/steps/objectives/tasks block with R2's ranges (UO-48). | 63862, 63873, R2 |
| `UO-37.md` | Contract row: key `asg.thor`, spawn 63873, template 63862, tag `UnionOmega_AsgardThor`, consumer UO-42; Asgard area, world 18 under both venue forms. § SQL `### For UO-42` adds it. Acceptance: "13 service rows, the hub Thor and 2-4 cast rows". | **yes** |
| `UO-39.md`, `UO-40.md` | One rule line: "Spawn 63873 and template 63862 belong to UO-42 (the hub Thor); never take them. UO-37 sealed your points' spacing against it." | **yes** |
| `UO-38.md` | none (item lists only) | no |
| `UO-42.md` stub | Templates gain 63862; Spawns gain 63873 (copied from UO-37 § SQL), in the `[UO-42]` pair, whose marker text becomes "63836-63839, 63873". The offer binds to 63862 only. | yes |
| `UO-41.md` stub | "The hearing never offers 481 (UO-16 ruling R1)." | yes |

UO-24 needs no change: the `[UO-42]` marker pair is per packet, and UO-42 widens its own marker text.

## Owner calls left

D-UO34 (questions 1-2) stays BlockedDecision. Every other open question has a recommended default. R1 and R2 are architect defaults the owner may overrule.
