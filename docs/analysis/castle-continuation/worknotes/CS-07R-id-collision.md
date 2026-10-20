# CS-07R-ID: Castle/Tollana entity template collision (420/421)

> **Type:** Ruling and packet (architect). **Audience:** Castle orchestrator, the CS-07R-ID packet-coder, the CS-07R rerun worker.
> **Date:** 2026-10-18. **Inputs:** [CS-07R coder report](../../../../.claude/reports/castle-continuation/CS-07R-coder.md) (zone-castle worktree), Castle `887f5d48b`, four-praxis campaign `b28b4fa11`, preserved merge `3f9b7c49e` (branch `wip/cs-07r-merge-unresolved-ids`), merge-base `b2dd72c66`, `origin/main` `dcf736016`.
> **Code edits made by this note:** none.

## 1. Decision: Castle renumbers 420 → 424 and 421 → 425

| Old | New | Castle row | Spawn | Dialog slot |
|---:|---:|---|---|---|
| 420 | **424** | `Castle_HubSupplyOfficer`, the repeatable_q1 hub giver | 13809 | `castle_repeatable_chains.sql` `"slot": 420` ×9 |
| 421 | **425** | `Castle_DefenseQ3_Giver`, the defense_q3 giver | 14096 | `castle_side_stories_chains.sql` `"slot": 421` ×4 |

Tollana keeps 420 (`Tollana_Copplemann`, spawn 19001) and 421 (the food-unit prop, spawns 19002-19004). Nothing on the Tollana side changes.

Evidence, all from git (`REPO`):

1. **Tollana claimed first, in the shared ledger.** `docs/analysis/four-zone-planning/id-allocation.md` at `b28b4fa11` has "### W1 Tollana claim — 2026-10-17", which claims "entity templates **420–421** and spawns **19001–19004**". The rows are also in `tollana-id-register.csv`. Castle never registered a template claim in that ledger.
2. **Tollana's scan was clean when it claimed.** The same section records a per-table scan on 2026-10-17 that found neither id on the Castle branch. Castle introduced them afterwards:
   - `57396ebab` (2026-10-17 21:15) seeded Tollana 420/421.
   - `936baba66` (2026-10-18 17:05, CS-04b3s) added Castle 421.
   - `c24fe9b10` (2026-10-18 17:24, CS-05b1s) added Castle 420.
3. **Tollana has more frozen references.** Its references are in `entity_templates_tollana.sql`, `spawnlist_tollana.sql`, `tollana_568_chains.sql`, `chain_replay_tests/mission_568/*`, `cell-world` `tollana_world19_cast.rs`, the `interact_tag_linter.rs` range comment, `w1-frozen.md` and the register. Castle's references are confined to 22 files on its own unmerged branch (section 3).
4. **No wire or client tie.** Both pairs are server-only templates. The id never reaches a client asset, so neither side is cheaper for client reasons.

**Why both sides can't share the ids, even with distinct rows:** the template id is also the per-player `add_dialog_set` slot (`available_interactions.entry(slot)` in `crates/cell-content/src/cell/content/executor/dialog/mod.rs`). Sharing it would (a) fail `reload-db` on `entity_templates_pkey`, which is what CS-07R hit, and (b) at runtime, put Castle quest offers on Tollana NPCs and the reverse.

## 2. Replacement ids are free (scan record, 2026-10-18)

424 and 425 were checked on these refs:

- Castle `887f5d48b`, the four-praxis campaign `b28b4fa11` and the preserved merge `3f9b7c49e`;
- `origin/main` `dcf736016`;
- Harset `1f935687c` (local) and `59d5d6f0c` (origin);
- Cellblock `836e12091` and Tollana `91acbf2f4`.

Every ref is clean for:

- `INSERT INTO entity_templates ... VALUES (424|425,` in any `db/resources` file: 0 hits;
- the `spawnlist` template column `, 424|425, '`: 0 hits;
- the `content_actions` `"slot": 424|425`: 0 hits;
- the ledgers (`id-allocation.md`, and the tollana-, cellblock- and harset-continuation docs): no claim. The one Harset hit, `harset_jaffa_chains.sql:425` in HA-03a, is a file line number.

Other constraints:

- **Reserved ranges:** Dakara owns templates 440-479. The proposed Harset subrange 260-269 is inactive. No other campaign reserves templates in 420-439.
- **Other tables:** 424/425 appear only in other namespaces: `char_creation_choices` choice_id, `dialog_screen_buttons` screen_button_id, and `dialog_screens` dialog_id 424. None of these is the template or slot space.
- **Sequence:** `entity_templates_template_id_seq` is `GREATEST(MAX, last, 1399)`, so 424/425 never reach the sequence.
- **Why not 422-423:** they stay free as Tollana headroom.
- **Why not 430/431** (CS-07R's suggestion): spawn ids 430/431 exist in `spawnlist.sql`, and `entity_templates.sql:127` contains `431, `. That makes the zero-grep checks noisy.

## 3. Other collision categories (beyond CS-07R's INSERT-tuple scan)

A scan of the Castle and praxis diffs against the merge-base found:

| Category | Result |
|---|---|
| Primary keys in every `INSERT` (single-line and multi-row `(id, ...)` continuation rows) | Only `entity_templates` 420/421 |
| Identical rows added on both sides | mission_steps 4037/4038, objectives 4650/4651, chain 1366: byte-identical, benign |
| `UPDATE` / `DELETE` in seeds | Neither side adds any |
| `setval` | Only the praxis side; `dialog_screens` resolved to 200170 in `3f9b7c49e` (correct) |
| Spawn ids, spawn tags, interact tags, dialogs, screens, dialog_set_maps, dialog_sets, missions, tasks | Disjoint |
| `add_dialog_set` slots | Only 420/421 shared (they are the template ids) |
| `\ir` lines in `db/database.sql`, Rust `mod` lines | No duplicates in the union |
| Cross-file uniqueness of template and spawn ids (simulated with the section 4 guard's parser) | Castle `887f5d48b`: 0 duplicates in 457 templates and 631 spawns. Merged `3f9b7c49e`: exactly 420 and 421 (`entity_templates.sql` with `entity_templates_tollana.sql`), 0 spawn duplicates |
| Rust test fn name duplicates across the union | **Not checked.** The rerun's compile settles it |

## 4. Packet CS-07R-ID (packet-coder, one commit)

### Rules (inlined; the coder does not read CLAUDE.md)

- **Shell:** PowerShell only, with absolute paths. No bash. Never `git stash`, `git worktree prune` or `git push`.
- **Paths:** begin every PowerShell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle;`. `[IO.File]` calls take absolute paths only.
- **Cargo:** never run it directly. Use `pwsh tools/build-lane/lane.ps1 cargo ...` in the foreground, and wait for it.
- **Files you may touch:** only the files in the edit tables, plus one new test file. Do not add `castle-pg.log` (untracked; leave it alone).
- **Line endings:** every file in this worktree is CRLF on disk. Keep it that way. The replace script preserves it; write the new `.rs` file with CRLF. Docs under `docs/` must stay CRLF, with no bare LF.
- **Size:** files have a soft cap of 500 lines and a hard cap of 700. No file here grows past 25 lines of change.
- **Seeds:** edit the seed files in place. No `db/scripts` migration.
- **Stopping:** if any expected count differs, stop and return `blocked`. Do not "fix" a count by editing extra lines.

### Pre-flight (all must hold, else return `blocked`)

```powershell
Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle; git rev-parse --short HEAD; git status --short
# expect: 887f5d48b; status shows only "?? castle-pg.log" and "?? docs/analysis/castle-continuation/worknotes/CS-07R-id-collision.md" (this note)
Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle; foreach ($r in 'HEAD','campaign/four-praxis-zone-restoration','origin/main','content/tollana-campaign','content/cellblock-campaign','content/harset-campaign','origin/content/harset-campaign') { $a=@(git grep -E "INSERT INTO entity_templates .*VALUES \((424|425)," $r -- db/resources).Count; $b=@(git grep -E '"slot": ?(424|425)[^0-9]' $r -- db/resources).Count; "$r $a $b" }
# expect every line to end "0 0"
```

### Edit A: blanket replacement in 14 files (scripted, with exact counts)

Every standalone 420 or 421 in these files is the Castle template or slot. That includes the identifiers `on_420`, `on_421`, `spawns_of_420` and `castle_side_repeatable_q1_slot_420_has_one_spawn`; they are renamed with it. Run this script once. It counts every file first and writes only if all the counts match.

```powershell
Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle
$wt = 'C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle'
$plan = @(
  @{ f='crates/cell-catalog/src/cell/spawner/tests/live_db_castle_side_defense_q3.rs'; o='421'; n='425'; c=7 },
  @{ f='crates/cell-catalog/src/cell/spawner/tests/live_db_castle_side_repeatable_q1.rs'; o='420'; n='424'; c=9 },
  @{ f='crates/cell-catalog/src/cell/spawner/tests/mod.rs'; o='420'; n='424'; c=1 },
  @{ f='crates/cell-content/src/cell/content/chain_replay_tests/castle_side_defense_q3.rs'; o='421'; n='425'; c=7 },
  @{ f='crates/cell-content/src/cell/content/chain_replay_tests/castle_side_defense_q3_chains.rs'; o='421'; n='425'; c=2 },
  @{ f='crates/cell-content/src/cell/content/chain_replay_tests/castle_side_defense_q3_lifecycle.rs'; o='421'; n='425'; c=1 },
  @{ f='crates/cell-content/src/cell/content/chain_replay_tests/castle_side_defense_q3_lifecycle/harness.rs'; o='421'; n='425'; c=2 },
  @{ f='crates/cell-content/src/cell/content/chain_replay_tests/castle_side_repeatable_q1.rs'; o='420'; n='424'; c=3 },
  @{ f='crates/cell-content/src/cell/content/chain_replay_tests/castle_side_repeatable_q1_flow.rs'; o='420'; n='424'; c=2 },
  @{ f='crates/resources/src/base/dialog_overrides/override_seed_agreement_castle_repeatable_q1.rs'; o='420'; n='424'; c=5 },
  @{ f='crates/resources/src/base/resources/tests/mission_additions_castle_defense_q3.rs'; o='421'; n='425'; c=3 },
  @{ f='crates/resources/src/base/resources/tests/mission_additions_castle_repeatable_q1.rs'; o='420'; n='424'; c=7 },
  @{ f='db/resources/Content/Seed/castle_repeatable_chains.sql'; o='420'; n='424'; c=9 },
  @{ f='db/resources/Content/Seed/castle_side_stories_chains.sql'; o='421'; n='425'; c=4 }
)
$bad = 0
foreach ($p in $plan) { $t = [IO.File]::ReadAllText((Join-Path $wt $p.f)); $k = ([regex]::Matches($t, "(?<![0-9])$($p.o)(?![0-9])")).Count; if ($k -ne $p.c) { "COUNT MISMATCH $($p.f): expected $($p.c) got $k"; $bad++ } }
if ($bad) { throw "stop: $bad count mismatches, nothing written" }
foreach ($p in $plan) { $path = Join-Path $wt $p.f; $t = [IO.File]::ReadAllText($path); $t = [regex]::Replace($t, "(?<![0-9])$($p.o)(?![0-9])", $p.n); [IO.File]::WriteAllText($path, $t, [Text.UTF8Encoding]::new($false)); "$($p.f): $($p.c) -> $($p.n)" }
```

Key results to eyeball afterwards:

- `const OFFER_SLOT: i32 = 425;` appears in `castle_side_defense_q3.rs` and `castle_side_defense_q3_chains.rs`;
- `pub(super) const OFFER_SLOT: i32 = 425;` appears in `harness.rs`;
- `pub(super) const SLOT: i32 = 424;` appears in `castle_side_repeatable_q1.rs`;
- `row(&ents, "VALUES (425, ")` and `assert_eq!(spawn[6], "425", "spawn template");` appear in `mission_additions_castle_defense_q3.rs`;
- `row(&templates, "VALUES (424, NULL, ")` appears in `mission_additions_castle_repeatable_q1.rs`;
- `"slot": 424` / `"slot": 425` appear in the two chain seeds.

### Edit B: exact-string replacements (these files have unrelated 420/421 hits; never blanket-replace them)

Use the Edit tool, one literal edit each. Each "old" string occurs exactly the number of times shown.

| File | Old (count) | New |
|---|---|---|
| `db/resources/Entities/Seed/entity_templates.sql` | `VALUES (420, ` (1) | `VALUES (424, ` |
| same | `VALUES (421, ` (1) | `VALUES (425, ` |
| `db/resources/Worlds/Seed/spawnlist.sql` | `1.5708, 8, 420, 'Castle_HubSupplyOfficer'` (1) | `1.5708, 8, 424, 'Castle_HubSupplyOfficer'` |
| same | `5.3559, 8, 421, 'Castle_DefenseQ3_Giver'` (1) | `5.3559, 8, 425, 'Castle_DefenseQ3_Giver'` |
| `docs/content/mission-chains.md` | `giver template 421, spawn 14096` (1) | `giver template 425, spawn 14096` |
| same | `set map 1000010, slot 421;` (1) | `set map 1000010, slot 425;` |
| same | `(1000010, slot 421)` (4; replace all) | `(1000010, slot 425)` |
| same | `supply officer template 420, spawn 13809` (1) | `supply officer template 424, spawn 13809` |
| same | `_slot_420_has_one_spawn` (1) | `_slot_424_has_one_spawn` |
| `docs/analysis/castle-continuation/uat/end-uat.md` | `the giver (template 421)` (1) | `the giver (template 425)` |
| same | `the supply officer (template 420)` (1) | `the supply officer (template 424)` |

### Edit C: errata in the two sealed worknotes (history kept, no rewrite)

- **CS-04bD2b.md** (`docs/analysis/castle-continuation/worknotes/`): insert one line directly after line 6 (the `> **Companions:** ...` line), before the blank line 7:
  `> **Erratum (CS-07R-ID, 2026-10-18):** the giver template and its dialog slot 421 in this seal are now **425**; 421 collided with Tollana's registered template ([CS-07R-id-collision](CS-07R-id-collision.md)). Read every "template 421" and "slot 421" below as 425.`
- **CS-05S3.md**: insert one line directly after line 4 (the `> **Status:** SEALED...` line), before the blank line 5:
  `> **Erratum (CS-07R-ID, 2026-10-18):** the supply officer template and its dialog slot 420 in this seal are now **424**; 420 collided with Tollana's registered template ([CS-07R-id-collision](CS-07R-id-collision.md)). Read every "template 420" and "slot 420" below as 424.`

### Edit D: claim the ids in the shared ledger

In `docs/analysis/four-zone-planning/id-allocation.md`, insert this block directly before the line `### Non-chain namespaces` (line 85 at `887f5d48b`), followed by one blank line:

```
### Castle CS-07R-ID template claims — 2026-10-18

Castle claims entity templates **424** (`Castle_HubSupplyOfficer`, the repeatable_q1 hub giver, spawn 13809) and **425** (`Castle_DefenseQ3_Giver`, the defense_q3 giver, spawn 14096). They were first authored as 420 and 421, which the W1 Tollana claim of 2026-10-17 already held, so Castle renumbered ([CS-07R-ID](../castle-continuation/worknotes/CS-07R-id-collision.md)). Each template id is also that NPC's per-player `add_dialog_set` slot. Table-scoped seed scans on 2026-10-18 found no entity template, spawn template or dialog slot 424 or 425 at `origin/main` `dcf736016`, the four-praxis campaign `b28b4fa11`, or the Castle, Cellblock, Harset and Tollana campaign branches. Templates 422 and 423 stay free next to Tollana's 420–421. This is a point-in-time collision check.
```

### Edit E: new guard test (type: unit, seed-agreement; reads files, no DB)

Create `crates/resources/src/base/resources/tests/seed_id_uniqueness.rs` (CRLF) with this content:

```rust
//! Cross-file primary-key uniqueness for the per-zone seed splits.
//!
//! `entity_templates*.sql` and `spawnlist*.sql` are split per zone and campaign
//! (Castle rows in `entity_templates.sql`, Tollana in
//! `entity_templates_tollana.sql`, ...). An id inserted by two files only fails
//! at `reload-db` (`entity_templates_pkey`), and a template id doubles as the
//! per-player `add_dialog_set` slot, so a shared id would also put one NPC's
//! quest offers on another. CS-07R hit exactly this with templates 420/421
//! (docs/analysis/castle-continuation/worknotes/CS-07R-id-collision.md).

use std::collections::BTreeMap;
use std::path::Path;

const DB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../db/resources");

/// The first column of every row: a single-line
/// `INSERT INTO t (...) VALUES (<id>, ...` or a multi-row continuation `(<id>, ...`.
fn first_column_ids(text: &str) -> Vec<i64> {
    let mut ids = Vec::new();
    for line in text.lines() {
        let t = line.trim_start();
        let rest = if t.starts_with("INSERT INTO") {
            match t.find(") VALUES (") {
                Some(i) => &t[i + ") VALUES (".len()..],
                None => continue,
            }
        } else if let Some(r) = t.strip_prefix('(') {
            r
        } else {
            continue;
        };
        if let Some(id) = rest.split(',').next().and_then(|s| s.trim().parse::<i64>().ok()) {
            ids.push(id);
        }
    }
    ids
}

/// Ids inserted more than once across `<DB>/<dir>/Seed/<prefix>*.sql`, with the
/// files that insert them.
fn duplicates(dir: &str, prefix: &str) -> BTreeMap<i64, Vec<String>> {
    let seed = Path::new(DB).join(dir).join("Seed");
    let mut seen: BTreeMap<i64, Vec<String>> = BTreeMap::new();
    let mut files = 0;
    let entries = std::fs::read_dir(&seed).unwrap_or_else(|e| panic!("read {}: {e}", seed.display()));
    for entry in entries {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if !(name.starts_with(prefix) && name.ends_with(".sql")) {
            continue;
        }
        files += 1;
        let text = std::fs::read_to_string(&path).unwrap();
        for id in first_column_ids(&text) {
            seen.entry(id).or_default().push(name.clone());
        }
    }
    assert!(files > 0, "no {prefix}*.sql under {}", seed.display());
    seen.retain(|_, f| f.len() > 1);
    seen
}

#[test]
fn entity_template_ids_unique_across_seed_files() {
    let dups = duplicates("Entities", "entity_templates");
    assert!(dups.is_empty(), "template_id inserted more than once: {dups:?}");
}

#[test]
fn spawn_ids_unique_across_seed_files() {
    let dups = duplicates("Worlds", "spawnlist");
    assert!(dups.is_empty(), "spawn_id inserted more than once: {dups:?}");
}

#[test]
fn first_column_ids_reads_both_row_shapes() {
    let sql = "INSERT INTO t (a, b) VALUES (420, 'x');\n\
               INSERT INTO t (a, b) VALUES\n\
               (421, 'y'),\n\
               (422, 'z');\n\
               SELECT pg_catalog.setval('s', 1, true);\n";
    assert_eq!(first_column_ids(sql), vec![420, 421, 422]);
}
```

Then in `crates/resources/src/base/resources/tests/mod.rs`:

- add `mod seed_id_uniqueness;` between `mod overrides;` and `mod stargate_overrides;`;
- add the doc bullet `//! - [`seed_id_uniqueness`]: template and spawn ids unique across the per-zone seed splits.` after the `stargate_overrides` bullet.

### Checks after the edits (all must return what is stated)

```powershell
# 1. zero standalone 420/421 left in the 14 blanket files (expect no output)
Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle; git grep -nP "(?<![0-9])42[01](?![0-9])" -- crates/cell-catalog/src/cell/spawner/tests/live_db_castle_side_defense_q3.rs crates/cell-catalog/src/cell/spawner/tests/live_db_castle_side_repeatable_q1.rs crates/cell-catalog/src/cell/spawner/tests/mod.rs crates/cell-content/src/cell/content/chain_replay_tests/castle_side_defense_q3.rs crates/cell-content/src/cell/content/chain_replay_tests/castle_side_defense_q3_chains.rs crates/cell-content/src/cell/content/chain_replay_tests/castle_side_defense_q3_lifecycle.rs crates/cell-content/src/cell/content/chain_replay_tests/castle_side_defense_q3_lifecycle/harness.rs crates/cell-content/src/cell/content/chain_replay_tests/castle_side_repeatable_q1.rs crates/cell-content/src/cell/content/chain_replay_tests/castle_side_repeatable_q1_flow.rs crates/resources/src/base/dialog_overrides/override_seed_agreement_castle_repeatable_q1.rs crates/resources/src/base/resources/tests/mission_additions_castle_defense_q3.rs crates/resources/src/base/resources/tests/mission_additions_castle_repeatable_q1.rs db/resources/Content/Seed/castle_repeatable_chains.sql db/resources/Content/Seed/castle_side_stories_chains.sql
# 2. zero old Castle refs anywhere outside the sealed worknotes (expect no output)
#    (pattern is single-quoted on purpose; ''Castle_ is PowerShell's escaped quote. Before the edits it hits 54 lines in 17 files.)
Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle; git grep -nP 'template 42[01]\b|slot 42[01]\b|slot_42[01]_|"slot": ?42[01]\b|8, 42[01], ''Castle_|on_42[01]\b|spawns_of_42[01]\b' -- db crates docs/content docs/analysis/castle-continuation/uat
# 3. new ids present exactly once (expect 1 1 1 1)
Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle; @(git grep -F "VALUES (424, " -- db/resources/Entities/Seed/entity_templates.sql).Count; @(git grep -F "VALUES (425, " -- db/resources/Entities/Seed/entity_templates.sql).Count; @(git grep -F "8, 424, 'Castle_HubSupplyOfficer'" -- db/resources/Worlds/Seed/spawnlist.sql).Count; @(git grep -F "8, 425, 'Castle_DefenseQ3_Giver'" -- db/resources/Worlds/Seed/spawnlist.sql).Count
# 4. changed paths: 22 modified (14 blanket + 2 seeds + mission-chains.md + end-uat.md + 2 worknotes + id-allocation.md + tests/mod.rs)
#    plus 2 new (seed_id_uniqueness.rs and this ruling note, CS-07R-id-collision.md) = 24. Ignore "?? castle-pg.log".
Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle; git status --short; git diff --stat | Select-Object -Last 1
# 5. CRLF: no bare LF in the touched docs (expect 0 for each)
Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle; foreach ($f in 'docs\content\mission-chains.md','docs\analysis\castle-continuation\uat\end-uat.md','docs\analysis\castle-continuation\worknotes\CS-04bD2b.md','docs\analysis\castle-continuation\worknotes\CS-05S3.md','docs\analysis\four-zone-planning\id-allocation.md') { $t=[IO.File]::ReadAllText("C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle\$f"); "$f " + ([regex]::Matches($t,"(?<!`r)`n")).Count }
```

### Lane checks, in order (foreground; on failure, up to 5 fix rounds per check)

```powershell
Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle; pwsh tools/build-lane/lane.ps1 cargo fmt --all
Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-resources -p cimmeria-cell-content -p cimmeria-cell-catalog --all-targets -- -D warnings
Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle; pwsh tools/build-lane/lane.ps1 cargo nextest run --profile=ci -p cimmeria-resources -p cimmeria-cell-content -p cimmeria-cell-catalog
Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle; pwsh tools/build-lane/live-db-test.ps1 castle_side
```

`live-db-test.ps1` reloads this worktree's own DB (`sgw_zone_castle`) from the seeds, then runs the live-DB tests matching `castle_side` (including `live_db_castle_side_defense_q3` and `live_db_castle_side_repeatable_q1`). Record each check's `status=`, `exit=` and its test count in the report. If `cargo fmt` changed any file, include that file in the commit.

### Regression proof (run after the lanes are green, before committing)

On the Castle branch the renumber is a consistent rename, so reverting it fails nothing here. The duplicate only exists once Tollana's seed is present. Prove instead that the guard catches a cross-file duplicate:

1. Create the scratch file `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle\db\resources\Entities\Seed\entity_templates_zz_proof.sql` with one line: `INSERT INTO entity_templates (template_id) VALUES (424, NULL);`
2. `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle; pwsh tools/build-lane/lane.ps1 cargo nextest run --profile=ci -p cimmeria-resources seed_id_uniqueness`. It **must fail**, and the message must name `424` with both `entity_templates.sql` and `entity_templates_zz_proof.sql`.
3. Delete the scratch file. Rerun step 2: it must pass with 3 tests.
4. Record both runs (exit code and the assertion line) in the report.

The merged-tree proof, where reverting the renumber fails on 420/421, belongs to the CS-07R rerun (section 5, step 6).

### Commit (one commit, explicit paths, no `git add -A`)

Stage the 24 paths from check 4 by name. This ruling note (`docs/analysis/castle-continuation/worknotes/CS-07R-id-collision.md`) is one of them, because the errata and the ledger link to it. Commit with `git commit -F <message file in the scratchpad>`.

- Subject: `fix(castle): renumber Castle templates 420/421 to 424/425 (Tollana collision, CS-07R-ID)`
- Body:
  - templates 420 and 421 collide with Tollana W1's registered templates (`entity_templates_pkey` in CS-07R);
  - the dialog slots, spawns 13809 and 14096, tests and docs follow the renumber;
  - a new `seed_id_uniqueness` unit test guards template and spawn id uniqueness across the per-zone seed files;
  - the ledger claim is in `id-allocation.md`.
- End the message with the attribution lines given in the dispatch brief.

### Report and return

- Report: `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle\.claude\reports\castle-continuation\CS-07R-ID-coder.md`. It holds the pre-flight output, the replace script output, checks 1-5, each lane line, both regression-proof runs and the commit sha.
- Return exactly one line: `CS-07R-ID <done|blocked|failed> <commit-sha|-> <report-path>`

## 5. Follow-up: CS-07R rerun (after CS-07R-ID commits as R)

**Agent:** use a defined Sonnet implementer (`rust-gameserver-dev`), not a Haiku packet-coder. This is the first compile of `reliable_fit.rs`'s semantic re-apply (about 657 lines in `cimmeria-base-session`, never built) and the first compile of the 18-file union.

**Basis:** reuse the reviewed resolution in `3f9b7c49e`; don't re-resolve by hand. That resolution is valid only for the praxis tip `b28b4fa11`.

Steps (worktree zone-castle, HEAD = R):

1. **Praxis tip check.** Run `git rev-parse --short campaign/four-praxis-zone-restoration`. If it is no longer `b28b4fa11`, still merge `b28b4fa11` below. Then report the newer praxis commits as a separate later merge; don't fold them in.
2. **Start the merge.** Run `git merge --no-ff --no-commit b28b4fa11`. The same 18 conflicts as the CS-07R report are expected.
3. **Take the reviewed tree.** Run `git checkout 3f9b7c49e -- .`. This resolves and stages every path to the reviewed union. R's two new files (`seed_id_uniqueness.rs` and this note) stay in place because `3f9b7c49e` doesn't contain them.
4. **Re-apply R on top.** Write `git diff --diff-filter=M 887f5d48b R` (modifications only, 22 paths) to a patch file in the scratchpad, then run `git apply --3way --index <patch>`.
   - Expected: it applies cleanly, because R's hunks have identical context in the union.
   - If `tests/mod.rs`, `id-allocation.md` or `mission-chains.md` conflict, keep the union content plus R's change, and nothing else.
5. **Verify the delta.**
   - `git diff --cached 3f9b7c49e --name-only` must equal `git diff 887f5d48b R --name-only`: the same 24 paths.
   - `git diff --cached 3f9b7c49e -- db/resources/Entities/Seed/entity_templates_tollana.sql` must be empty, because Tollana is untouched.
   - Then commit the merge, reusing the CS-07R merge message plus the line `Includes CS-07R-ID renumber (Castle templates 424/425).`
6. **Merged-tree regression proof.** In the working tree only:
   - edit `entity_templates.sql` `VALUES (424, ` → `VALUES (420, ` and `VALUES (425, ` → `VALUES (421, `;
   - run `lane.ps1 cargo nextest run --profile=ci -p cimmeria-resources seed_id_uniqueness`. It must FAIL, listing 420 and 421 with `entity_templates.sql` and `entity_templates_tollana.sql`;
   - restore the file with `git checkout -- db/resources/Entities/Seed/entity_templates.sql` and rerun: it must pass.
7. **Lanes** (the original CS-07R lane list, with these specifics):
   - `pwsh tools/build-lane/reload-db.ps1` must now succeed. It failed on `entity_templates_pkey` (420) before.
   - `lane.ps1 cargo fmt --all`.
   - `lane.ps1 cargo clippy` and `lane.ps1 cargo nextest run --profile=ci`, each with `-p cimmeria-base-session -p cimmeria-resources -p cimmeria-cell-content -p cimmeria-cell-catalog -p cimmeria-cell-world -p cimmeria-services`. `cimmeria-base-session` is listed for `reliable_fit.rs`.
   - `live-db-test.ps1 castle_side`, then `live-db-test.ps1 mission_568` (Tollana's chains still bind slot 420/421 to its own NPCs).
   - A duplicate Rust test name across the union shows up here as a compile error. Fix it by renaming the Castle-side test, and record the rename.

**What differs from the previously resolved merge `3f9b7c49e`:**

- the 24 R paths:
  - 14 blanket-renumbered test and chain files;
  - `entity_templates.sql` rows 424/425 and `spawnlist.sql` rows 13809/14096 (template column);
  - `mission-chains.md` (7 lines) and `end-uat.md` (2 lines);
  - two worknote errata;
  - `id-allocation.md`, with the new "Castle CS-07R-ID template claims" section placed before "### Non-chain namespaces", after the praxis-side CB-N0x and CS-08a sections;
  - `tests/mod.rs`, which gains `mod seed_id_uniqueness;` in the union;
  - the new `seed_id_uniqueness.rs` and this ruling note;
- nothing else. Every other resolution (`dialog_screens` setval 200170, the mission row unions, the `dialog_overrides` and `mission_additions` unions, `reliable_fit.rs`, `na13_chain_armed_aggro.rs` and the doc unions) is byte-identical to `3f9b7c49e`.
- Retire `wip/cs-07r-merge-unresolved-ids` through `ops` once the rerun merge commit exists.

## 6. Owner questions

None. The collision is a ledger-order call, and section 1 settles it from the shared ledger. One optional, non-owner item: the Tollana coordinator may formally reserve templates 422-423 in `id-allocation.md`. They are left free for that reason, and nothing here depends on it.
