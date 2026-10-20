# Castle mission catalog: authoring API, ID reservation and collision audit

> **Type:** Reference (evidence). **Audience:** Castle packet authors (CS-00B code half, CS-04a/b/c, CS-05, CS-07) and reviewers.
> **Last updated:** 2026-10-17. **Packet:** CS-00Be. **Base:** `da2b82c709bb28a8d374f535b032f838ec0a743f` (`FEATURE_SHA`).
> **Companions:** [overlay ledger](../README.md), [end UAT file](../uat/end-uat.md), [four-zone ID table](../../four-zone-planning/id-allocation.md), [cb-n05a seal](../../four-zone-planning/cb-n05a-seal.md).

Provenance labels follow the contract: `SHIPPED_DATA`, `CIMMERIA_AUTHORING`, `UNRESOLVED (<gate>)`. Everything below is read from `da2b82c70` and the three sibling branches; nothing was run against a client (D-CA11).

## 1. How an authored mission enters the client catalog (#1734)

The client reads missions from the `CookedDataMissions` category (category id 3). Two server-side mechanisms feed it. Both run at `ResourceCache::load_all`, before any client connects, and both bump the category metadata so a client holding the old version resyncs the whole category.

| Mechanism | Defined at | What it does |
|---|---|---|
| `MissionAddition` / `MISSION_ADDITIONS` | `crates/resources/src/base/mission_additions.rs:6-14` | Inserts a **complete new** `<COOKED_MISSION>` entry. `mission_id: u32` plus `xml: &'static str`. |
| `MissionOverride` / `MISSION_OVERRIDES` | `crates/resources/src/base/mission_overrides.rs:43-50`, table at `:76` | Injects one `<Steps>` block into an **existing** stock mission, after `insert_after_step_id`. Used by missions 622/641-class "Equip the ..." steps. |

Load path:

1. `ResourceCache::load_all(data_dir)` passes the production registry: `crates/resources/src/base/resources/mod.rs:216-218`.
2. `load_all_impl` loads every `.pak` (`:242-263`), then calls `apply_mission_overrides(&mut categories, additions)` first (`:271`).
3. `apply_mission_overrides` (`apply_overrides.rs:217-347`):
   - `apply_mission_additions::prepare` validates the whole batch before any mutation (`:225-226`); a bad entry returns `Err("<reason>: mission <id>")` and the server load fails, with nothing half-applied;
   - `insert` writes the exact admitted bytes under key `mission_id` (`apply_mission_additions.rs:69-76`);
   - the `MISSION_OVERRIDES` and `STEP_TEXT_OVERRIDES` loops run after (`:244-317`), so an override may patch an addition;
   - `compute_metadata_bump(MISSION_OVERRIDES, STEP_TEXT_OVERRIDES, additions)` is added to the category metadata (`:319-332`);
   - each insert logs `event="cooked_data.mission_catalog_inserted"` (`:334-345`); a rejection logs `event="cooked_data.mission_catalog_rejected"` with a `reason` field.
4. Test-only entry point: `ResourceCache::load_all_with_mission_additions` (`mod.rs:222-228`, cfg `test`/`test-support`), fed by `TEST_MISSION_ADDITIONS` (id 61000, `mission_additions.rs:20-31`).

**The production `MISSION_ADDITIONS` is empty** (`mission_additions.rs:14`: `&[]`), guarded by `tests/mission_additions.rs:125` (`assert!(MISSION_ADDITIONS.is_empty())`). The module doc says production authoring belongs there "only after a mission packet seals its XML" (`:1-2`). **CS-00B (code half) must change that assert** when it adds the first entry.

### Admission rules (`apply_mission_additions.rs`)

A `MissionAddition` is rejected, with the quoted `reason`, unless all of these hold:

- `mission_id` is 1..=65535 (`MAX_COOKED_ELEMENT_ID`, `dialog_overrides/mod.rs:74`): `root_id_out_of_range` (`:31`).
- No duplicate id in the registry (`duplicate_registry_id`), XML at most `MAX_MISSION_XML_BYTES` = 355,840 (`xml_too_large`; `mission_additions.rs:18`), and the id is not already a stock key (`stock_id_collision`, `:41`).
- Root is exactly `COOKED_MISSION`; children appear only in the order `HistoryText`, then `Steps` (at least 2 children in a mission and in a step, so a step needs its `StepDisplayLogText` plus at least one `Objectives`), then `Objectives` (`DisplayLogText` first, then `Tasks`): `invalid_nesting_or_order`, `missing_required_child` (`:195-206`, `:158-168`).
- Every attribute for the node is present, with the stock names. Mission: 12 booleans, `NumRepeats`, `Level`, `Difficulty`, `MissionID`, `MissionLabel`, `MissionDefn`. Step: `StepEnabled`, `AwardXP`, `StepID`, `Difficulty`. Objective: `IsOptional`, `AwardXP`, `IsHidden`, `IsEnabled`, `ObjectiveID`, `Difficulty`. Task: `AwardXP`, `IsEnabled`, `TaskType`, `TaskID`, `Difficulty` (`:310-341`). Missing is `missing_required_attribute`; an unknown one is `unsupported_attribute`.
- `MissionID` equals `mission_id` (`mission_id_mismatch`). **Every `MissionID`, `StepID`, `ObjectiveID` and `TaskID` in the whole batch is positive and unique among each other** (`nonpositive_id`, `duplicate_nested_id`; the set is created once per batch at `:26`, so mission, step, objective and task ids must be pairwise disjoint, and disjoint across all additions). Stock ids are **not** checked for nested ids (only the root key is checked against stock).
- No DOCTYPE (`dtd_forbidden`), UTF-8 XML 1.0 declaration only, namespaces fixed to the five SOAP/SGW URIs.

The client keys steps by XML declaration order, not by id (`mission_overrides.rs:33-42`). An authored mission's step order in the XML must match the `advance_step` order its chains use.

### What the server also needs

`MissionAddition` is only the client half. The chain engine and persistence read the DB seeds, which must agree on ids and on the `IsHidden`/`IsOptional` flags (`mission_overrides.rs:52-68`): `db/resources/Missions/Seed/{missions,mission_steps,mission_objectives,mission_tasks}.sql`, plus `Content/Seed/*.sql` chains and `Dialogs/Seed/*`. A missing client entry shows no journal line; a missing seed makes `advance_step` a no-op.

`StepEnabled`/`IsEnabled` are not read at runtime (contract excerpt; `mission_overrides.rs` ships `StepEnabled="false"` steps that work). `UNRESOLVED (end UAT)`: whether the client journal, dialog, advance and relog of an authored mission behave; see [end-uat.md](../uat/end-uat.md).

## 2. The Castle authored ID reservation (10000-10999)

The four-zone table gives Castle the whole `10000-10999` block "for authored stories and repeatables" and names only chains (`id-allocation.md:71`). The audit in section 3 shows that the block is **not empty across namespaces**, so one block-wide partition is unsafe. Per-namespace occupancy at `da2b82c70`:

| Namespace (table or catalog) | Existing ids inside 10000-10999 | Verdict |
|---|---|---|
| `content_chains.chain_id` | none | free |
| `missions`, `mission_steps`, `mission_objectives`, `mission_tasks` (seed) and `CookedDataMissions` pak (root and nested ids) | none | free |
| `dialogs.dialog_id`, `dialog_sets.dialog_set_id` | none | free |
| `dialog_screens.screen_id` | **10000-10247** (11 ids: 10000-10005, 10087, 10092, 10228, 10241, 10247), mirrored in the dialogs pak | use only 10248+ |
| `dialog_screen_buttons.screen_button_id` | none (but its `screen_id` column reuses the 11 screens) | free |
| `dialog_set_maps.dialog_set_map_id` | none; seed max 1000000, column default is a sequence from 1 | free, but the seed convention is 1000000+; not allocated from this block (bullet below) |
| `entity_templates`, `spawnlist` | none (max 1595 and 14095) | free (block not needed) |
| `texts.moniker_id` (and `TextStrings.pak`) | **10000-10999 (995 of 1000 taken)** | never use for text ids |
| `error_texts` (and `ErrorStrings.pak`) | 10000-10009 | never use |
| Kismet sequences, `event_sets`, `event_sets_sequences`, `stargates.event_set_id`, `ring_transport_regions.event_set_id`, `play_sequence` targets, `CookedDataKismetSeqEvent/SetEvent` | 10000-~10014, plus pak names (187 + 14 entries) | never use for `play_sequence` or event-set ids |

### Proposed sub-blocks (`CIMMERIA_AUTHORING`, for CS-07 to write; this packet does not edit `id-allocation.md`)

The ranges are numerically disjoint on purpose, so a bare number in a log or chain row is unambiguous, and so the batch-wide uniqueness rule for mission, step, objective and task ids is met by construction.

| Sub-block | Range | Size | Notes |
|---|---|---:|---|
| Chains (`content_chains.chain_id`) | 10000-10299 | 300 | about 5 chains per mission (offer, advance, victory, completion, relog restore) for up to about 50 missions, plus repeatables |
| Missions (`MissionAddition.mission_id`, `missions.mission_id`) | 10300-10339 | 40 | well under the 65535 cap |
| Steps (`StepID`, `mission_steps.step_id`) | 10340-10459 | 120 | |
| Objectives (`ObjectiveID`) | 10460-10579 | 120 | |
| Tasks (`TaskID`, `mission_tasks.task_id`) | 10580-10699 | 120 | |
| Dialogs (`dialogs.dialog_id`) | 10700-10759 | 60 | |
| Dialog sets (`dialog_sets.dialog_set_id`) | 10760-10799 | 40 | |
| Dialog screens (`screen_id`) | 10800-10919 | 120 | clear of the occupied 10000-10247 |
| Dialog screen buttons (`screen_button_id`) | 10920-10999 | 80 | a screen with Accept and More Info takes 2, so 120 screens cannot all carry two; a packet that runs out claims from the reserve below |

Reserve and exclusions:

- Template candidates 412/413 are free as `entity_templates` ids on all four refs. Spawn ids 412/413 are taken (`spawnlist.sql:1171` and `:1173` on `da2b82c70`; `:1178` and `:1180` on cellblock), so a bare `grep -w 412` hit is neither a template collision nor a free spawn id. Spawn candidates 13801/13802 stay outside this block (section 3.3).
- Dialog set map rows (`dialog_set_maps.dialog_set_map_id`) are not in a sub-block: the seed max is 1000000 and the column default is `nextval` of a sequence that starts at 1. A CS-04 packet passes an explicit id above the seed max and checks it against all four refs before it writes a seed.
- Text ids and `play_sequence`/event-set ids are never allocated from this block (table above). A Castle story that needs a new cinematic claims an event-set id from `11500-12999` (cross-zone reserve) by amending `id-allocation.md`.
- Restoration chains for 1520/1521 keep the `1366-1400` table in the R2 contract excerpt (`r2-packets.md`, an overlay file absent from this worktree until CS-INT2a); they are not drawn from this block.
- Cellblock owns `11000-11499` and Tollana `9000-9999`; this reservation never crosses either.

## 3. Collision audit

Method: the PowerShell script below parses every `db/resources/**/*.sql` file at a ref (`git ls-tree -r --name-only <ref> -- db/resources`, then `git show <ref>:<path>`). It takes the first column of each single-line `INSERT INTO <table> (...) VALUES (<id>,` row, plus the `screen_id` column of `dialog_screens` (the one named-column check). With `-IncludePaks` it also reads every `data/cache/*.pak` zip in the worktree (the tracked paks at `da2b82c70`; the worktree is clean) for entry names and `MissionID`/`StepID`/`ObjectiveID`/`TaskID`/`DialogID`/`ScreenID` attributes in 10000-10999. A table with no hit is absent (count 0) and is not printed. Save the script as `audit-cs00be.ps1` in a directory outside the repository (`<script-dir>` below).

```powershell
# Collision audit for the CS-00Be ID reservation (10000-10999).
# Usage: pwsh -NoProfile -File audit.ps1 -Wt <worktree> -Ref <ref> [-IncludePaks]
param([Parameter(Mandatory)][string]$Wt, [Parameter(Mandatory)][string]$Ref, [switch]$IncludePaks)
$lo = 10000; $hi = 10999
$inBlock = { param($v) $v -ge $lo -and $v -le $hi }
$files = git -C $Wt ls-tree -r --name-only $Ref -- db/resources | Where-Object { $_ -like '*.sql' }
$firstCol = @{}; $maxFirst = @{}; $screenIds = @{}
foreach ($f in $files) {
  foreach ($line in (git -C $Wt show "${Ref}:$f")) {
    if ($line -match '^INSERT INTO (\w+) \(.*?\) VALUES \((-?\d+),') {
      $t = $Matches[1]; $id = [int64]$Matches[2]
      if ($null -eq $maxFirst[$t] -or $id -gt $maxFirst[$t]) { $maxFirst[$t] = $id }
      if (& $inBlock $id) { $firstCol[$t]++ }
    }
    if ($line -match '^INSERT INTO dialog_screens \(dialog_id, screen_id,.*?VALUES \(\d+, (\d+),') {
      $sid = [int64]$Matches[1]
      if (& $inBlock $sid) { $screenIds["$sid"]++ }
    }
  }
}
'== first-column ids in block, per table (ref ' + $Ref + ')'
$firstCol.GetEnumerator() | Sort-Object Value -Descending | ForEach-Object { '{0} {1}' -f $_.Key, $_.Value }
'total first-column rows: ' + (($firstCol.Values | Measure-Object -Sum).Sum)
'dialog_set_maps max seed id: ' + $maxFirst['dialog_set_maps']
'dialog_screens.screen_id in block: ' + (($screenIds.Keys | Sort-Object) -join ' ')
if ($IncludePaks) {
  Add-Type -AssemblyName System.IO.Compression.FileSystem
  foreach ($p in (Get-ChildItem -Path (Join-Path $Wt 'data\cache') -Filter '*.pak' | Sort-Object Name | ForEach-Object { $_.BaseName })) {
    $z = [IO.Compression.ZipFile]::OpenRead((Join-Path $Wt "data\cache\$p.pak"))
    $nameHits = @($z.Entries | Where-Object { $_.Name -match '^_(\d+)$' -and (& $inBlock ([int64]$Matches[1])) }).Count
    $attr = @{}
    foreach ($e in $z.Entries) {
      $xml = [IO.StreamReader]::new($e.Open()).ReadToEnd()
      foreach ($m in [regex]::Matches($xml, '(MissionID|StepID|ObjectiveID|TaskID|DialogID|ScreenID)="(\d+)"')) {
        if (& $inBlock ([int64]$m.Groups[2].Value)) { $attr[$m.Groups[1].Value]++ }
      }
    }
    $z.Dispose()
    $p + ': entry names in block ' + $nameHits + '; attributes in block: ' + $(if ($attr.Count) { ($attr.GetEnumerator() | ForEach-Object { '{0}={1}' -f $_.Key, $_.Value }) -join ' ' } else { 'none' })
  }
}
```

Refs (tips re-stamped 2026-10-17; the earlier stamps were `634b76558` and `e7dd74336`): `da2b82c70` (`FEATURE_SHA`, 386 SQL files), `content/cellblock-campaign` (`82ed4cebf`, 385), `content/harset-campaign` (`d2a6913cd`, 386), `content/tollana-campaign` (`a1e99c0e5`, 385). Commands (`-Wt` is the worktree root):

```powershell
pwsh -NoProfile -File <script-dir>\audit-cs00be.ps1 -Wt (Get-Location).Path -Ref da2b82c70 -IncludePaks
pwsh -NoProfile -File <script-dir>\audit-cs00be.ps1 -Wt (Get-Location).Path -Ref content/cellblock-campaign
pwsh -NoProfile -File <script-dir>\audit-cs00be.ps1 -Wt (Get-Location).Path -Ref content/harset-campaign
pwsh -NoProfile -File <script-dir>\audit-cs00be.ps1 -Wt (Get-Location).Path -Ref content/tollana-campaign
```

Output (run 2026-10-17):

```text
== first-column ids in block, per table (ref da2b82c70)
texts 995
sequences 207
event_sets_sequences 172
event_sets 14
error_texts 10
total first-column rows: 1398
dialog_set_maps max seed id: 1000000
dialog_screens.screen_id in block: 10000 10001 10002 10003 10004 10005 10087 10092 10228 10241 10247
CookedBehaviorEvents: entry names in block 0; attributes in block: none
CookedBlueprints: entry names in block 0; attributes in block: none
CookedCharCreation: entry names in block 0; attributes in block: none
CookedDataAbilities: entry names in block 0; attributes in block: none
CookedDataContainers: entry names in block 0; attributes in block: none
CookedDataDialogs: entry names in block 0; attributes in block: ScreenID=11
CookedDataEffects: entry names in block 0; attributes in block: none
CookedDataItems: entry names in block 0; attributes in block: none
CookedDataKismetSeqEvent: entry names in block 187; attributes in block: none
CookedDataKismetSetEvent: entry names in block 14; attributes in block: none
CookedDataMissions: entry names in block 0; attributes in block: none
CookedDataStargates: entry names in block 0; attributes in block: none
CookedDisciplines: entry names in block 0; attributes in block: none
CookedInteractions: entry names in block 0; attributes in block: none
CookedInteractionSet: entry names in block 0; attributes in block: none
CookedParadigm: entry names in block 0; attributes in block: none
CookedSciences: entry names in block 0; attributes in block: none
CookedWorldInfo: entry names in block 0; attributes in block: none
ErrorStrings: entry names in block 10; attributes in block: none
SpecialWords: entry names in block 0; attributes in block: none
TextStrings: entry names in block 995; attributes in block: none

== first-column ids in block, per table (ref content/cellblock-campaign)
texts 995
sequences 207
event_sets_sequences 172
event_sets 14
error_texts 10
total first-column rows: 1398
dialog_set_maps max seed id: 1000000
dialog_screens.screen_id in block: 10000 10001 10002 10003 10004 10005 10087 10092 10228 10241 10247

== first-column ids in block, per table (ref content/harset-campaign)
texts 995
sequences 207
event_sets_sequences 172
event_sets 14
error_texts 10
total first-column rows: 1398
dialog_set_maps max seed id: 1000000
dialog_screens.screen_id in block: 10000 10001 10002 10003 10004 10005 10087 10092 10228 10241 10247

== first-column ids in block, per table (ref content/tollana-campaign)
texts 995
sequences 207
event_sets_sequences 172
event_sets 14
error_texts 10
total first-column rows: 1398
dialog_set_maps max seed id: 1000000
dialog_screens.screen_id in block: 10000 10001 10002 10003 10004 10005 10087 10092 10228 10241 10247
```

### 3.1 Seed rows with a first-column id in 10000-10999 (same on all four refs)

| Table | Rows | Relevant to Castle? |
|---|---:|---|
| `texts` | 995 | no (text ids) |
| `sequences` | 207 | no (Kismet) |
| `event_sets_sequences` | 172 | no (Kismet) |
| `event_sets` | 14 | no (Kismet) |
| `error_texts` | 10 | no |
| `content_chains`, `missions`, `mission_steps`, `mission_objectives`, `mission_tasks`, `dialogs`, `dialog_sets`, `dialog_screen_buttons`, `entity_templates`, `spawnlist` | **0** | yes: free |

Totals: 1398 first-column rows on each of the four refs, identical across refs, so no campaign branch adds a row in the block. The named-column check gives the 11 `dialog_screens.screen_id` values in 10000-10247 listed in section 2; `dialog_set_maps` seed max is 1000000. The script does not parse multi-line `INSERT ... VALUES` rows (288 at `da2b82c70`); those were checked separately and none has a first id in 10000-10999.

### 3.2 Pak exports (`data/cache/*.pak`, tracked at `FEATURE_SHA`)

- `CookedDataMissions.pak`: 1041 zip entries, **0** entry names and **0** `MissionID`/`StepID`/`ObjectiveID`/`TaskID` values in the block.
- `CookedDataDialogs.pak`: 5406 zip entries, 0 entry names; 11 `ScreenID` attributes in 10000-10247 (the same 11 as the seed), 0 `DialogID`.
- `TextStrings.pak` 995 and `ErrorStrings.pak` 10 entry names; `CookedDataKismetSeqEvent.pak` 187 and `CookedDataKismetSetEvent.pak` 14 entry names. All other paks: 0 (script output above; 21 paks read).

### 3.3 Other candidate ids (contract excerpt)

| Candidate | Check on all four refs | Result |
|---|---|---|
| Chains 1367-1400 | `content_chains` first column | only 1366 present (CS-08a, on `da2b82c70` and `content/harset-campaign`); 1367-1400 absent on all four |
| Entity templates 412, 413 | `entity_templates` first column | absent (449 templates, max 1595) |
| Spawns 13801, 13802 | `spawnlist` first column | absent (632 spawns, 634 on cellblock; max 14095) |

### 3.4 Verdict

- Chains, missions, steps, objectives, tasks, dialogs, dialog sets, screen buttons: **absent** from `db/resources` on all four refs and from the pak exports. The sub-blocks above are clear.
- Screens: occupied 10000-10247; the proposal uses 10800-10919, **absent** (the only in-block screen ids are the 11 listed).
- The block as a whole is **not** absent from `db/resources`: text, Kismet and event-set ids occupy it. A reviewer who runs a bare `git grep -w 10005` will get hits; they belong to those namespaces.
- Point-in-time check, not a lock (same caveat as `id-allocation.md`). Recheck the sub-block a packet draws from on `FEATURE_SHA` and the three branches immediately before it writes a seed. `UNRESOLVED (G-ID)` stays open for any id outside these sub-blocks.
