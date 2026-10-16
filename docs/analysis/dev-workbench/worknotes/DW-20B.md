# DW-20B worknote — dot patrol and entity authoring metadata

The reviewed [DW-20B-E evidence](../evidence/DW-20B-E.md) is the per-command
source contract. The catalog now exposes typed fields, exact forms, target
policy, impact, readback, compensation and availability for its 11 patrol and
17 entity-authoring commands. `registry/commands/{patrol,entity_authoring}.rs`
remain the execution registry. A test compares the metadata names against
both live `SPECS` slices and pins their 28-name cardinality. No raw form is
used to stand in for a missing contract.

Only `.path_show` and `.tag` are marked runnable. The former reports the
shared in-memory session buffer, not a database path. The latter is a
transient tag edit, not a saved spawnlist row. Every other action is disabled
with its own source-backed reason. In particular `.path_add` lacks atomic
ownership across the shared path buffer and per-GM SQL queues, the six
ordinal waypoint commands can retarget at `.seedconfirm`, and `.path_clear`
can partially apply its two deletes. `.lookat` can stop and autosave any
non-player spawnable, including pet or prop; a command-wide runnable flag
cannot enforce the evidence's verified-NPC and autosave-state conditions.
`.name` has no player readback because the player snapshot prefers
`character_name`. `.visible true` is only a feedback acknowledgement, not
an active show operation.

Authoring impact text distinguishes immediate memory edits, per-GM queued
SQL, later positive live-DB results, emitted seed SQL and a developer's
separate committed seed change. The catalog never labels queueing or
`.seedconfirm` feedback a completed seed commit. Existing command handlers
and DB schema were not changed. Tests also dispatch malformed `.path_add`
and missing-target `.tag`/`.path_assign` and assert refusal without mutation
or queued SQL. The palette UI/preview/execution path remains future DW-28–31
work; no lab lease or live client was used.

The packet needed a narrow registry seam (`registry/{mod,workbench}.rs` and
`registry/commands/mod.rs`) to surface this family through the existing
catalog. The dispatch regression file was registered under `console/tests/`.
No SQL query was added, so this packet has no live-DB test.

Local checks through the PowerShell build lane: `cargo fmt -p
cimmeria-cell-console` status=ok; `cargo clippy -p
cimmeria-cell-console --all-targets -- -D warnings` status=ok; `cargo
test -p cimmeria-cell-console --lib` status=ok, 564 passed and 0 failed.

Review follow-up: the original inventory/nonempty test did not guard typed
form drift. `every_world_author_form_has_exact_target_and_ordered_typed_signature`
now pins the target policy and ordered argument name, kind, required flag,
bounds and choices for each of the 28 rows. A temporary mutation changed
`PATH_INDEX_SEQ.seqId` from `integer` to `float`; the targeted lane test
failed at `.path_set_seq` with the differing field tuple. The mutation was
restored before final checks. Baseline handler refusal tests show current
dispatch behavior; the exact-signature test is the catalog regression guard.
