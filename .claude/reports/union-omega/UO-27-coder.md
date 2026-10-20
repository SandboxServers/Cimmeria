# UO-27 coder report: Human Armor T1-T4

Status: done (commit in the git log of branch union-omega/uo-27, subject `feat(union-omega): Human Armor vendors, tiers 1-4 (UO-27)`).

## Files changed

- `db/resources/Entities/Seed/entity_templates_union_omega.sql`: 4 rows (63816-63819) and a provenance comment inside the `[UO-27]` pair. Copied from template 1302 with the packet's changes only.
- `db/resources/Items/Seed/item_lists_union_omega.sql`: lists 63804-63807 with their rows, pasted from the UO-21 worknote `### For UO-27` blocks inside the `[UO-27]` pair.
- `db/resources/Worlds/Seed/spawnlist_union_omega.sql`: 6 spawn rows (63847-63852) from the UO-17 and UO-19 worknotes inside the `[UO-27]` pair.
- `crates/cell-catalog/src/cell/spawner/tests/live_db_union_omega_human_armor.rs`: UO-24's stub replaced with the four tests from the packet's Step 5.

Line endings: CRLF kept in all four files, no lone LF. Each seed file's `[UO-27]` pair is the only edited region.

## Rows per list

- 63804 (tier 1, 100 x tc): 638200-638217, 18 rows, designs 4330, 2971, 7374, 2967, 7376, 7375, 2949, 2925, 3401, 3387, 3346, 3372, 3359, 3488, 7089, 6861, 7014, 6938.
- 63805 (tier 2, 2000 at tc 20): 638250-638267, 18 rows.
- 63806 (tier 3, 3300 at tc 33): 638300-638317, 18 rows.
- 63807 (tier 4, 4500 at tc 45): 638350-638367, 18 rows.

Each list has 18 rows, inside the 20-row cap. UO-21 § Gaps reads "None", so no packet-level stop applied.

## Spawns (all stationary, world 18)

- 63847 template 63816 `UnionOmega_HumanArmorT1`: (-169.59, -36.42, -168.97), heading 3.1416.
- 63848 template 63817 `UnionOmega_HumanArmorT2`: (-166.00, -36.42, -169.00), heading 5.7364.
- 63849 template 63818 `UnionOmega_HumanArmorT3`: (-108.00, -47.82, -92.00), heading 3.7390.
- 63850 template 63819 `UnionOmega_HumanArmorT4`: (-108.00, -47.82, -95.00), heading 4.5152.
- 63851 template 63816 `UnionOmega_HumanArmorT1_Common`: (78.00, -44.12, 8.00), heading 4.5305.
- 63852 template 63819 `UnionOmega_HumanArmorT4_Common`: (-64.00, -45.75, -68.00), heading 0.9666.

## Check results

1. `lane.ps1 cargo fmt -p cimmeria-cell-catalog -- --check`: `status=ok exit=0 ran=0.700s`
2. `lane.ps1 cargo clippy -p cimmeria-cell-catalog --all-targets -- -D warnings`: `status=ok exit=0 ran=20.544s`
3. `live-db-test.ps1 live_db_union_omega_human_armor`: `status=ok exit=0 ran=261.258s`, `4 passed, 8630 skipped`
4. `live-db-test.ps1 vendor_arbitrage`: `status=ok exit=0 ran=105.299s`, `1 passed, 8633 skipped`
5. `live-db-test.ps1 spawner_tests::union_omega`: `status=ok exit=0 ran=73.515s`, `11 passed, 8623 skipped`

`cargo fmt` as a formatting pass was not run separately; check 1 is the `--check` form the packet lists, and it passed without changes.

## Revert proof

| Mutation | Result |
|---|---|
| 63817 `interaction_type` to 16384 | Fails `omega_human_armor_templates_sell_one_tier_each`: `template 63817 interaction_type is exactly bit 8192 (D-UO35)` |
| 638200 priced at 200 | Fails `omega_human_armor_lists_follow_d_uo29`: `list 63804 item 638200 price (D-UO29, Armor)`, `left: 200`, `right: 100` |
| `WP-Human.WP_SMG_1A` back on 63818 | Fails `omega_human_armor_vendors_are_unarmed` (panic at the test's assert, line 61) |
| Spawn 63851 tag set to `UnionOmega_HumanArmorT1` | Seed reload fails first: `ERROR: duplicate key value violates unique constraint "spawnlist_tag_key"`, `Key (tag)=(UnionOmega_HumanArmorT1) already exists.`; the lane reports `status=failed exit=1`. The test did not run. Re-proof (fix pass 1, N1): spawn 63852 template_id 63819 -> 63818 (tag unchanged); `live_db_union_omega_human_armor` fails `omega_human_armor_stand_clear_in_the_hub` with `assertion `left == right` failed: spawn 63852 template` (`live_db_union_omega_vendor_common.rs:276:9`), `left: 63818`, `right: 63819`. Seed restored (SHA-256 identical to the pre-mutation backup, `git diff` empty); rerun reports `4 passed, 8630 skipped`. |

Each mutated file was restored from a byte-exact backup; `git diff --numstat` afterwards shows only the insertions (5, 80 and 7 lines).

## Deviations

1. The worktree had no `external/` directory, so the lane could not build (`cc-rs` could not find `external/recast`) and `test-live-db.ps1` could not find `psql.exe`. I copied `external/recast` and `external/postgresql_server/bin` from the main checkout into the worktree. Both copies are untracked and were not staged or committed.
2. The lane writes logs to `C:/Users/Steve/AppData/Local/cimmeria-build/logs/uo-27/`, not to the worktree's `logs/`. The log paths are in the summary lines above.
3. Revert mutation 4 is caught by the seed reload's unique key on `spawnlist.tag`, before `omega_human_armor_stand_clear_in_the_hub` runs. The packet expected that test to fail; the seed constraint is stricter and fires earlier.
4. The stub's `//!` line ("UO-27 fills this ...") was replaced by the header from Step 5 of the packet, which is the packet's literal replacement content.
5. The fourth test uses `sqlx::query_scalar`, so `use sqlx::Row;` is not needed. The file imports `require_db_or_skip` explicitly. The helper's own import is private, so the glob does not conflict.
6. `template_name` is `Union Omega - Human Armor Tier N`. The `name` column stays NULL, as in the source row. The `name` column of template 1302 is NULL, and the source's `Debug Area - Munitions Vendor` value sat in `template_name`, which is the column the packet changes.

## Not run

Telemetry UO-U1 checks belong to the owner's lab UAT and were not run (no lab in this packet). The checks to make are `vendor.open_store` (`crates/base-methods/src/base/world_entry/methods/vendor/store.rs:32`) and an INFO `vendor` `store_opened` line with `buy_count` 18 for each list.
