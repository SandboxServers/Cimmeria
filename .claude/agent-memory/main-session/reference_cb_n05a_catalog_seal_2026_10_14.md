---
name: Authored mission catalog contract
description: Full-category resync, conservative key guard and test-only CB-N05a fixture provenance.
type: reference
---

# Authored mission catalog contract (2026-10-14)

Source: base `19eab88c9e6ee05ac7f7801a8380b9b84aa61d3e`, `crates/base-session/src/base/cooked_sync/{decision,task}.rs`, `docs/reverse-engineering/findings/{cooked-data-pipeline,cooked-dialog-override-crash}.md`, committed `data/cache/CookedDataMissions.pak`, and scoped independent BigWorld/mission advisor design reviews. Frozen tracked brief: [CB-N05a seal](../../../docs/analysis/four-zone-planning/cb-n05a-seal.md).

Every category mismatch resyncs all entries: first complement version/InvalidateAll, final served version, both with RequiredUpdates=0 and empty InvalidKeys. Override IDs are bookkeeping, not transfer selection. The <=65535 cooked root-key bound is conservative repository admission policy, not a proven client key width; the crash finding refutes the old width theory. Existing masked metadata hash can collide; do not promise “changes iff bytes change.”

CB-N05a reserves test mission 61000/step 61001/objective 61002/task 61003, absent from stock and mission seeds at the reviewed base. Stock `_1624` supplies the schema witness; the exact 1139-byte fixture transform/hash is in the seal. Production registry remains empty, with no seed/offer. Resource injection must use the production loader path; real ClientModel guard explicitly counts the new transfer and must fail when only insertion is reverted. gSOAP/journal/progression/relog live proof remains pending.
