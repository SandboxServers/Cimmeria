# Ammo: Session Resume

> Type: how-to. Audience: the next coordinator session.
> Updated: 2026-09-28. Companions: [launch prompt and decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md).

## State: plan merged, AM-01 already done, AM-F not started

| Packet | Status | PR | Notes |
|---|---|---|---|
| Plan (AM-00) | this PR | — | The ledger: README, audit, work packets, this resume note |
| AM-01 RE | **Done** | [#1040](https://github.com/SandboxServers/Cimmeria/pull/1040) | Ran ahead of the plan; findings folded into every section above. Read [docs/reverse-engineering/findings/ammo-system.md](../../../reverse-engineering/findings/ammo-system.md) before touching AM-04, AM-06 or AM-07 |
| AM-F | Ready | — | The only serial gate. Start here |
| AM-02..AM-07 | BlockedDependency (AM-F) | — | Wave 1, six packets in parallel once AM-F merges |
| AM-08..AM-11c | BlockedDependency (AM-04) | — | Wave 2, six packets in parallel |
| AM-12 | BlockedDependency (everything) | — | Close-out |

## Resuming

1. **First, get the owner's answers to the open questions** in [README.md § Open questions](../README.md#open-questions), especially open question 1 (the toggle-ability design decision — a recommendation is already written, it just needs sign-off) and open question 4 (the pistol/SMG family choice), since AM-F's and AM-04's contracts cite specific ids and a specific design that depend on those answers.
2. **Launch AM-F** as a single worker. It is small by design (see [work-packets.md § AM-F](../work-packets.md#am-f-foundation)) — item ids, the `ammo_item_types` and `ammo_modifiers` tables, the `AmmoReserve` Rust module (real, tested, not a stub), the `ammo.finite_special` flag, and the four new `\ir` lines in `db/database.sql`.
3. **Once AM-F merges, launch Wave 1 as six parallel workers** (AM-02 through AM-07), each in its own worktree per the dispatch rules. Message the coordinator before touching `registry.rs` or `db/database.sql` — both are shared, one-line-per-packet contention points, not exclusive-owned files.
4. **AM-07 runs its spike before batch-authoring.** Do not let it merge its full 15-item scope before the spike (push one item, UAT it live) has a recorded result in `worknotes/am-07.md`. If the spike fails, coordinate the `ammo_items.sql`/`ammo_item_types.sql` repoint with whoever holds AM-F's branch state at that point (it may already be merged, in which case it's a normal small follow-up PR against `main`).
5. **AM-04 needs the owner's sign-off on the toggle-ability design decision before it ships**, not just before it starts — the recommendation (README open question 1, option (a): apply the modifier directly, never cast the toggle ability) is strong enough to code against provisionally, but get the sign-off before merging.
6. **Launch Wave 2 as six parallel workers** (AM-08 through AM-11c) once AM-04 merges. Each is a small, symmetric packet — a new effect file, one `registry.rs` line, one seed file, one `\ir` line.
7. **AM-12 last.** Flip `ammo.finite_special`, update the gameplay docs, extend `docs/guides/unified-uat.md`, and do the one-time `docs/gap-analysis.md`/`docs/project-status.md` update per CLAUDE.md's campaign-close-out cadence.
8. Retire each worktree the day its PR merges (`bash tools/build-lane/rm-worktree.sh <name>`).

## Cross-campaign notes

- **Bank campaign.** Ammo items are ordinary inventory items, so they already work with the personal vault, mail (bags 1/15 are mailable sources), and trade the same way any stackable item does — no coordination needed unless a future packet wants ammo to behave differently in one of those flows.
- **Loot campaign work (#1031).** AM-05 builds directly on the `open_loot` content action and the debug-hub crate's per-player roll; read `docs/gameplay/loot-system.md` § Live Containers before touching loot tables 3, 8 or 9.
- **#602/#603 (open PRs, pre-date this campaign).** AM-03 absorbs #602's fix (same file, same hazard class) rather than letting it merge independently. #603 is unrelated (doc comments only) and can merge on its own schedule — no coordination needed.

## UAT checklist

To be filled in by AM-12 with SigNoz queries, following the outline in [README.md § UAT checklist outline](../README.md#uat-checklist-outline) and the acceptance criteria in issue #1026. Steps should be numbered to continue from wherever the [unified UAT guide](../../../guides/unified-uat.md) leaves off, per that guide's convention.

## Known gaps (carried forward)

- Everything in [README.md § Known issues](../README.md#known-issues) and the open questions not yet answered by an owner.
- `getAmmoTypes`/`getCurrentAmmoType`'s exact lookup key and `Event_NetOut_GiveAmmo`'s exact wire byte layout are unrecovered (AM-01's own open items); neither blocks any packet in this ledger, but a future RE session could close them.
