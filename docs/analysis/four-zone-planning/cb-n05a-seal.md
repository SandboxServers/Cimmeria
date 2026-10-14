# CB-N05a authored mission catalog seal

> **Title:** CB-N05a authored mission catalog seal
> **Type:** Reference
> **Audience:** Sol implementation coder, infrastructure coordinator, BigWorld/mission advisors and reviewers
> **Last updated:** 2026-10-14
> **Companions:** [packet](cellblock-work-packets.md#cb-n05a--full-new-mission-catalog-entries-for-authored-quests), [execution contract](execution-contract.md#client-mission-catalog-prerequisite), [planning index](README.md), [PAK format](../../engine/cooked-data-pak-format.md), [pipeline](../../engine/cooked-data-pipeline.md), [override architecture](../../architecture/mission-pak-overrides.md), [TESTING.md](../../../TESTING.md)

Status: **design/fixture sealed for implementation** at base `19eab88c9e6ee05ac7f7801a8380b9b84aa61d3e`, branch `infra/cb-n05a-mission-catalog`, registered worktree `cb-n05a-catalog`. The packet owns file permissions, execution order and handoff. You use the existing in-memory category-3 cache and cooked-sync transport. This seal adds no runtime behavior and attests no new-ID client presentation. After implementation merges, client proof remains `UATPending` under the execution contract.

## Source and fixture identity

| Evidence | Frozen value |
|---|---|
| Source archive | `data/cache/CookedDataMissions.pak`, committed QA archive; 1040 numeric entries, IDs 38–1826, little-endian u32 `MetaData` 7538 |
| Archive SHA-256 | `9A96ED8278957CF37E337367AE592C8D8155A0901F1061CA0D3873D3AAA570EA` |
| Schema witness | Stock `_1624` (Ambush), one step/objective/task, stock flags, namespace declarations and child order; 1093 raw bytes |
| `_1624` SHA-256 | `9AFFDF20C17B238A8DE76B27001E7EFEA31D0EFE1B5AB5E141D723B859B0DE21` |
| Test reservation | Mission **61000**, step **61001**, objective **61002**, task **61003**; coordinator records in continuation ledger/worknote before dispatch |
| Fixture encoding | UTF-8, one LF after XML declaration, no final newline; **1139 bytes** |
| Fixture SHA-256 | `AC37C6BD2A3D78F3BEE111789D66D6B44B038683BD603197EB6B57206CF7D0FD` |
| Collision checks at base | No fixture IDs in mission seed tokens or stock MissionID/StepID/ObjectiveID/TaskID attributes; 1041 seed mission rows, IDs 38–1826; independent mission review counts 12,906 stock ID attributes |

The fixture is `CIMMERIA_AUTHORING` for tests. Production `MISSION_ADDITIONS` is **empty**. You do not seed, offer, accept or persist 61000; these four IDs are a dated test reservation, not a production quest range.

Exact fixture bytes are the following two lines with **no newline after the closing root** (the Markdown fence is not part of the fixture):

```xml
<?xml version="1.0" encoding="UTF-8"?>
<COOKED_MISSION xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:CookedData1="SGW" AwardXP="true" ShowFactionChangeIcon="false" ShowInstanceIcon="false" ShowPVPIcon="false" IsShareable="false" NumRepeats="1" CanAbandon="false" CanRepeatOnFail="true" CanFail="false" Level="1" IsHidden="false" IsOverrideMission="false" IsEnabled="true" IsAStory="true" Difficulty="1" MissionLabel="General" MissionDefn="CB-N05a catalog fixture" MissionID="61000"><HistoryText>CB-N05a catalog fixture</HistoryText><Steps StepEnabled="false" StepID="61001" AwardXP="false" Difficulty="1"><StepDisplayLogText>Verify the authored mission catalog.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="61002" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText> </DisplayLogText><Tasks TaskType="1" TaskID="61003" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps></COOKED_MISSION>
```

Independent stock-reference test reconstructs it from committed ZIP `_1624`: replace MissionID 1624→61000, StepID 4818→61001, ObjectiveID 6198→61002, TaskID 8213→61003; replace MissionDefn/HistoryText `Ambush` with `CB-N05a catalog fixture`, StepDisplayLogText `Eliminate the Mutated Asgard.` with `Verify the authored mission catalog.`, and empty DisplayLogText with one space. Compare literal bytes to the transformed raw source; retain the source's declaration separator. Keep step declaration order, which controls client step indices. One task is this fixture's supported example; stock missions without tasks also exist.

## Existing source and transport seams

Paths in this table are relative to `crates/`.

| Stage | Existing seam / invariant |
|---|---|
| Load | `resources/src/base/resources/mod.rs::ResourceCache::{load_all,load_pak}` builds `CategoryData`; missions are category 3. |
| Mutation | `resources/src/base/resources/apply_overrides.rs::ResourceCache::apply_mission_overrides`; keep existing ordered step/text overrides, missing-target warning skips and one final metadata bump. |
| Hash | `resources/src/base/resources/metadata_bump.rs::compute_metadata_bump`; additions extend the existing hash, sorted by ID, with ID and exact XML bytes. Empty additions hash nothing extra. |
| Decision | `base-session/src/base/cooked_data.rs::handle_version_info_request` → `cooked_sync/decision.rs::VersionReply::decide`: mismatch selects full category resync; override IDs do not select entries. |
| Task | `base-session/src/base/cooked_sync/task.rs::{start,push_next_entry,finish,send_entry}` streams all sorted keys through existing pacing (24 outstanding packets). Missions can continue streaming after Play. |
| Version replies | First: category 3, `!served`, InvalidateAll=true, RequiredUpdates=0, InvalidKeys=[]. Last: category 3, served, false, 0, []. Matching versions do not resync. |
| Fragment | `wire/src/mercury/protocol/resources.rs::build_resource_fragment`, message 0x36 with WORD-length framing; u16 dataId/u8 chunkId/u8 flags, first fragment msgType=0/u32 category/u32 element ID, uncompressed UTF-8 XML. MAX_CHUNK=1390, maximum 256 fragments. Fixture is one fragment. |
| Addressing | Account requests 0xC0/0xC1 and receives 0x80 at character select; in-world SGWPlayer requests 0xD4/0xD5 and receives method 96 (0xBD, sub-index 35). Preserve existing routing. |

Evidence companions: [cooked-data finding 1 and 4a](../../reverse-engineering/findings/cooked-data-pipeline.md), [message catalog](../../protocol/message-catalog.md), [client dispatch](../../protocol/client-method-dispatch-table.md), [server-message dispatch](../../protocol/message-dispatch-table.md). Client handler 0x00dddd80 receives `resourceFragment`; category-3 miss function is 0x00d20150. Shared-template pushed-entry evidence is 0x0043dad0→0x0043bdb0→0x0043a9d0; the decompiled template instance is category 6. It supports pushes without a preceding request, not a new category-3 decompilation claim.

Preserve the hash policy `& 0xffff | 1`: its effective 15-bit hash can collide. Representative content edits and deterministic loads are testable; collision-free “metadata changes iff content changes” is not promised. Preserve root-key admission `1..=MAX_COOKED_ELEMENT_ID` (65535). This is conservative repository policy: [cooked-dialog crash evidence](../../reverse-engineering/findings/cooked-dialog-override-crash.md) refutes the old 16-bit key theory; keys are 32-bit and the observed mask applies to iteration position. Do not lift the guard or redesign versioning here.

## Frozen API and application invariants

```rust
// base/mission_additions.rs
pub struct MissionAddition {
    pub mission_id: u32,
    pub xml: &'static str,
}
pub const MISSION_ADDITIONS: &[MissionAddition] = &[];

// Existing ResourceCache associated method, apply_overrides.rs
pub(super) fn apply_mission_overrides(
    categories: &mut HashMap<u32, CategoryData>,
    additions: &[MissionAddition],
) -> Result<HashMap<u32, Vec<u32>>, String>;

// Existing metadata helper gains the complete additions argument.
pub(super) fn compute_metadata_bump(
    overrides: &[MissionOverride],
    text_overrides: &[StepTextOverride],
    additions: &[MissionAddition],
) -> u32;

// ResourceCache associated test-support methods, resources/mod.rs
#[cfg(any(test, feature = "test-support"))]
pub fn load_all_with_mission_additions(
    data_dir: &str,
    additions: &[MissionAddition],
) -> Result<Self, String>;

#[cfg(any(test, feature = "test-support"))]
pub fn load_raw_mission_category_for_test(data_dir: &str) -> Result<CategoryData, String>;
```

The production loader and injection loader share a private implementation. `load_all` supplies the empty registry; the raw accessor reuses private `load_pak` for stock category 3. Expose the literal fixture only under the same test-support cfg. `base-session/Cargo.toml` already enables resources' test-support dev feature; no dependency edit or second harness is necessary.

Preflight the whole additions batch before **any mission mutation**, including stock step/text patches. `apply_mission_additions.rs` owns validation/preparation and exact-byte insertion. Use existing `quick-xml`; do not use substring validation or parse/re-emit stock XML. An invalid entry returns `Err(String)` with a stable reason and leaves keys, bytes and metadata unchanged. Preflight malformed/duplicate entries even when category 3 is absent. A valid nonempty batch with absent category warns `category_not_loaded`, returns an empty map and creates no category. Empty registry keeps existing absent-category behavior.

The supported authored subset has one closed `COOKED_MISSION` root, the witness's required scalar fields, HistoryText, at least one Steps with StepDisplayLogText and Objectives with DisplayLogText. Tasks are optional; present Tasks carry the stock fields. Recognized nesting/order and unique positive mission/step/objective/task IDs are required; root MissionID equals registry key, root key obeys 65535 policy and nested IDs fit positive signed INT32. Validate duplicate attributes, entities and scalar forms. Reject DTD/entity declarations and a second/trailing root. This is bounded mission validation, not a framework for every historical gSOAP extension.

| Node | Required data attributes in supported subset |
|---|---|
| COOKED_MISSION | Boolean: AwardXP, ShowFactionChangeIcon, ShowInstanceIcon, ShowPVPIcon, IsShareable, CanAbandon, CanRepeatOnFail, CanFail, IsHidden, IsOverrideMission, IsEnabled, IsAStory. Integer: NumRepeats, Level, Difficulty, MissionID. Text: MissionLabel, MissionDefn. |
| Steps | Boolean: StepEnabled, AwardXP. Integer: StepID, Difficulty. |
| Objectives | Boolean: IsOptional, AwardXP, IsHidden, IsEnabled. Integer: ObjectiveID, Difficulty. |
| Tasks (when present) | Boolean: AwardXP, IsEnabled. Integer: TaskType, TaskID, Difficulty. |

Reject XML above **355840 UTF-8 bytes** (256 × 1390) at admission. The existing task skips larger entries yet can stamp the completed version; additions must not reach that condition. Preserve accepted bytes, old patch order (622 has dependent anchors), and one final bump from original metadata. Sort/deduplicate override bookkeeping. A second application to an already-mutated cache is unsupported and rejects collisions. Existing stock-key additions fail, unlike item's skip policy. Baseline comparisons distinguish raw unpatched stock from existing authorized served overrides.

## Validation and regression gates

| Test owner / type | Required assertions |
|---|---|
| Resources: unit + stock-reference | Literal/source transform and shape; raw 61000 absent and all four IDs collision-free; exact 1139-byte insertion; one bookkeeping occurrence; unchanged unrelated stock and existing 622/641 patches. |
| Resources: compatibility/determinism | Empty registry reproduces old cache/version/keys; production has no fixture; fresh identical loads and permuted complete additions match; representative ID/XML-byte edits change the tested bump; old missing patch targets never create partial entries. |
| Resources: refusal/atomicity | Valid-first/bad-second batches leave every key/byte/metadata unchanged for malformed XML, wrong root/nesting, missing required fields, invalid booleans/integers/entities, duplicate attributes/registry/nested IDs, mismatched MissionID, second root, DTD, stock collision, zero/above-policy root key and oversize XML. Missing category is explicit, with no created category; invalid batch still rejects when absent. |
| Resources: negative-log | LogCapture pins malformed, duplicate, collision, missing-category reasons and successful insertion fields, with no success before final application. |
| Base-session: real transport/ClientModel | Fixture cache via common injection loader assigned to Rig.cache; independently raw-loaded stock client category version 7538/no 61000; `rig.request(3, 7538)` and real task/TestTransport; exact opening/closing replies, exactly one transfer head `(3,61000)`, exact body, final 61000 entry and full 1041-entry table/version equal fixture cache. |
| Base-session: boundary cross-check | Pin resources' admission bound to current MAX_CHUNK × 256 without introducing a reverse resources→base-session dependency. |

Global `server_entries(3)`/`server_version(3)` use the production empty registry, so they are not expected-fixture oracles. Generic “client equals server” can pass if insertion disappears from both sides: the explicit transfer/body/new-ID assertion is mandatory. Revert **only insertion**, retain fixture/API/test and require that assertion to fail at runtime; restore and rerun. Commit safe state before temporary edits, and record actual commands/results. Compilation failure does not prove the guard. Generic interruption/miss/pacing tests already cover their seams; no transport rewrite or SQL/live-DB test is required.

Implementation gate commands, in order (compiling calls always use the lane):

```powershell
cargo fmt --all -- --check
pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-resources -p cimmeria-base-session --all-targets -- -D warnings
pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-resources -p cimmeria-base-session
```

Then record insertion-only revert failure and restored pass, targeted Markdown/link checks and `git diff --check`. Coordinator owns current full pre-PR gates and fresh review/fixer rounds before shipping the standalone PR. The exact documentation allowlist is in the packet; the [doc-update map](../../agents/doc-update-map.md) routes mission PAK and engine changes to their existing companions. No global test/review policy changes are introduced.

## Telemetry, signoffs and limits

Successful application emits INFO `event="cooked_data.mission_catalog_inserted"`, category_id=3 and category_name, mission_id and mission_name from validated MissionDefn, bytes, final metadata_version. Rejection emits `cooked_data.mission_catalog_rejected` with stable reason, category/available ID/name and safe context; missing category warns `category_not_loaded`. Never log full XML. Reuse `cooked_data.version_reply`, `cooked_data.sync_start`, `cooked_data.sync_finish`, `cooked_data.miss_served` for delivery. Progression telemetry belongs to later authored-content packets.

| Advisor / date | Scoped signoff |
|---|---|
| BigWorld engine advisor, 2026-10-14 | Existing ResourceCache/category-3/full-resync transport and stock-schema fixture are compatible with the existing client contract; no unresolved evidence blocker after the corrections frozen here. Read-only seal at the named base; no runtime or client attestation. |
| Mission systems advisor, 2026-10-14 | Independent read-only design/fixture review at the named base finds no blocking issue; independently checks stock/seed counts, ID absence, exact `_1624` transform, 1139 bytes and fixture SHA. Production registry empty; fixture neither seeded nor offered. |

The BigWorld signoff originates in the coordinator's ignored `.claude/reports/four-zone/cb-n05a-seal.md`; the mission signoff is the independent advisor's final response, relayed by the coordinator. This tracked brief freezes both scoped conclusions for fresh readers.

No gSOAP parsing, rendered journal/dialog text, step/objective progression or relog presentation of the new ID is attested. The first real authored quest in any zone supplies that live proof after owner-approved lab access; additional authored-quest implementation stays gated until it passes. CB-N05a remains `UATPending` until the result is recorded. Documentation debt: the canonical `spec.engine.cooked-data-pipeline` chapter is still listed as coming soon in the [Bible index](../../spec/README.md); this packet consumes existing source/protocol/RE evidence and does not author that infrastructure chapter.
