# Cellblock continuation ledger

> Opened 2026-10-14. Static baseline: `d3efb7d41aad0b6008397f112d224ce7430d7acf`.
> **Type**: Reference. **Audience**: Engineers and campaign coordinators.
> **Last updated**: 2026-10-15. **Companions**: [Wave resume](RESUME.md), [CB-N00 worknote](worknotes/cb-n00.md), [CB-N01 handoff](presentation-handoff.md), [CB-N05 proposed seal](worknotes/cb-n05-seal.md).
> CB-N00 is **Integrated** for the evidence/doc gate only. Fresh route acceptance is **UATPending**.

This is the live ledger for the one-time escape in world 12. The
[packet contracts](../four-zone-planning/cellblock-work-packets.md) and
[execution contract](../four-zone-planning/execution-contract.md) govern dispatch.
A completed source audit does not make a runtime packet Ready.

## Evidence and decisions

- [Source and route matrix](evidence-matrix.md): mission dispositions, historical hashes,
  current chain anchors and evidence limits.
- [Route baseline](route-baseline.md): prior observations, current faults, ownership,
  development population and unverified safe pockets.
- [Presentation handoff](presentation-handoff.md): asset, ring, class-start,
  autoplay and lab-reliability ownership with evidence and client row boundaries.
- [Decision log](decision-log.md): settled policy and outstanding design gates.
- [Wave resume](RESUME.md): reviewed commits, checks, next action and blockers.
- [CB-N00 worknote](worknotes/cb-n00.md): checks, scope and measurement handoff.
- [CB-N01 worknote](worknotes/cb-n01.md): presentation handoff checks and limits.
- [CB-N05 proposed seal](worknotes/cb-n05-seal.md): exact first authored story, copy, IDs, action order and review/UAT gates.

## Packet status

| Packet | Status | Dependencies and next seal |
|---|---|---|
| CB-N00 | Integrated | Evidence/doc gate passed two review rounds through `b59c4a124`; fresh route and lineage rows remain UNSEALED and client UATPending. No downstream runtime seal is granted. |
| CB-N01 | Integrated | Presentation and sibling ownership handoff passed independent review and focused status correction review; runtime and client rows remain UATPending. |
| CB-N02 | BlockedDependency | The 2026-10-15 fresh Human p3 run verified native pistol grant, equip, reload and guard damage at zero Focus. CB-U02 remains UATPending for exact server/client/patch revision, local-server packet tap and visual recoil; the prior no-damage symptom did not reproduce. Livewire then stalled, so this was not a complete normal route. See [route baseline](route-baseline.md#2026-10-15-fresh-human-lab-run). |
| CB-N03 | BlockedDependency | Current guard-cover reachability observation and combat/NPC/mission sign-off; no new mandatory kills. |
| CB-N04 | BlockedDependency | O14-12 delegates cast and placement approval; measure safe pockets, then seal one room cluster per child. |
| CB-N05a | UATPending | Catalog infrastructure merged to `main` in #1734 and integrated here; the first real authored mission must pass journal, dialog, advance and relog before this gate closes. |
| CB-N05 | BlockedDependency | Proposed itemless, zero-reward *The Remaining Pods* design is in [the seal](worknotes/cb-n05-seal.md), design review complete. Before coder dispatch, CB-U09 must measure a fresh normal first-run core at 30–45 minutes and show room for this beat within the 10-minute optional budget (O14-14). Native CB-U10 then tests journal/dialog/advance/relog. |
| CB-N05b | BlockedDependency | Separate Castle evidence hand-in is held for Harset HA-1360's reviewed atomic item receipt primitive, Castle recipient ownership and a new item/reward/ID seal. It does not gate CB-N05. |
| CB-N06 | BlockedDependency | Conditional under O14-15: include only after N05 live pacing and safety show benefit within the 10-minute optional budget. |
| CB-N07 | BlockedDependency | Conditional under O14-15: include only after live pacing and safety show benefit; N03 reachability and one-room design remain gates. |
| CB-N08 | BlockedDependency | Selected packets, sibling integrations, full local CI/fresh review; live rows UATPending until approved run. |

The fresh p3 client reached mission 638's Livewire door, but the native
minigame helper and screenshot call stalled. The character then appeared
farther down world 12 without mission advance. This invalidates CB-U09 timing;
no CB-N05 coder dispatch follows from the partial run. The Codex `lab-server`
connection also pointed at a different server, while direct read-only access
to the matching local server confirmed the character session. The client and
lease were stopped and released. The full evidence boundary is in the
[route baseline](route-baseline.md#2026-10-15-fresh-human-lab-run).

The eleven packet families are scopes; an implementation child needs the complete
seal required by the packet contract. CB-N05 now has proposed exact copy and
individual ID claims under O14-04/O14-10, with review pending. They do not
authorize coder dispatch yet. No new placement, item or authored reward is
sealed. Chain block 11000–11499 remains a planning reservation outside the six
individually claimed CB-N05 chains. CB-N05a is the merged early standalone PR
exception. FZ-99 owns shared status docs.

## Integration boundary

CB-N00 delivered its five allowlisted documents. Review fixes add the ledger link
in `docs/readme.md` and a sourced project-memory pointer; the four-zone index also
names this ledger directory. FZ-99 owns shared status documents.
No runtime, seed, shared planning, client or lab changes occurred.

Wave 0 integrates the static evidence only. `north_review` findings were fixed in
`b59c4a124`; fresh reviewer `north_rereview` found no actionable issues in the
full baseline-to-fixer diff. Downstream statuses stay blocked until their named
seals are complete; no runtime packet becomes Ready from this gate alone.
