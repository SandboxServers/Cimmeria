# Cellblock presentation and sibling handoff

> **Type:** Reference. **Audience:** Cellblock, asset, ring, class-start, autoplay and lab UAT owners.
> **Last updated:** 2026-10-14.
> **Companions:** [Cellblock ledger](README.md), [route baseline](route-baseline.md),
> [asset evidence](../cellblock-asset-audit/evidence-ledger.md),
> [asset gaps](../cellblock-asset-audit/knowledge-gaps.md),
> [ring campaign](../ring-transport-cellblock-castle/README.md),
> [class-start ledger](../class-start-v6/README.md),
> [presentation UAT](../cellblock-asset-audit/uat-guide.md).

This is the CB-N01 ownership handoff, not a new gameplay contract. `S` means
current server seed/test source, `P` parsed QA package or static binary, and `T`
an actual client observation. A static export, dispatched sequence, replay or
helper success cannot pass a visual row. The [asset audit](../cellblock-asset-audit/README.md)
owns visual restoration; the [ring campaign](../ring-transport-cellblock-castle/README.md)
owns chain 1109 and the exit pad; [class-start](../class-start-v6/README.md)
owns starting and crate gear. Cellblock owns a *newly reproduced* route-state
defect through a sealed CB-N02 child, not a second visual or client patch.

## Revision and evidence boundary

The source join is the Cellblock integration branch `content/cellblock-campaign`
at `70605d817b` (working tree also has concurrent ledger edits), compared with
local `origin/main` `817d3792c8`. The asset investigation branch
`origin/docs/cellblock-asset-audit` ends at `44f5c378eb`; its audit was
subsequently represented on main by #1317, but that tip is not in main's ancestry.
Its QA `Castle_CellBlock-fffffffe.umap` is identified by SHA-256
`d00b8261b7d882fd7ff6cf8329c21a347db7f5969175d186989610ebbfa0c4e2`.
The [asset corpus and coverage](../cellblock-asset-audit/evidence-ledger.md#corpus-and-coverage)
are static: five actor exports and persistent Kismet export 136 remain parsing
gaps. The ring ledger reports a 2026-09-19 Phase 0 client load/pass of a test
station; its Phase 1 Armory clone is built but has no recorded client pass.
Client patch branch `origin/feat/client-patch-007-castle-armory-ring` ends at
`6a9818c09f` separately from main. No evidence here proves that patch is
installed in today's client. The [class-start CS-04 work](../class-start-v6/worknotes/CS-04.md)
is integrated as #1278; [CS-08](../class-start-v6/worknotes/CS-08.md)
observed the M622 intro, pistol grant/equip/reload and relog, not the full escape.

In the table, **first** is a native route interaction; **repeat** is another
input at the same boundary; **relog** includes reconnect or streamed recovery;
**late** means a second viewer or phase-different entrant. IDs point to
[CA-U scenarios](../cellblock-asset-audit/uat-guide.md#scenarios) and
[CB-U route rows](route-baseline.md#bounded-fresh-acceptance-rows). They are
acceptance targets, not passes.

| Surface and exact known source | Exclusive owner, packet status/branch and current evidence | First | Repeat | Relog | Late entry |
|---|---|---|---|---|---|
| **Pods:** chain 1061 completes 641 after Livewire and highlights rings, with no pod action (S). `fffefffc` has a `StasisBlockDoubleDoors` event and open/close Interps (P); that graph does not identify pod lids. | Asset CA-03B **Ready**, CA-06B **BlockedEvidence** in its [packet ledger](../cellblock-asset-audit/work-packets.md); campaign tip `origin/docs/cellblock-asset-audit` `44f5c378eb`, no packet branch verified. Static P/S; pod target and playback **unproven**. | CA-U07, CB-U04 | CA-U16 | CA-U17 | CA-U18/19 |
| **Doors:** `fffefffd` `Prisoner329CellDoor` event → SetBool/PlaySound/Interp (P). Armory chain 1107 advances 688 step 2356→80688 and highlights the switch without a door action (S). Exact Armory door event/actor is **unproven**. | Asset CA-03C/D **Ready**, CA-06B **BlockedEvidence** on `origin/docs/cellblock-asset-audit`; no packet branch verified. Cellblock N02 is **BlockedDependency** until a server-state fault is reproduced. Static P/S. | CA-U02/14, CB-U03/07 | CA-U16 | CA-U17 | CA-U18/19 |
| **Escape barrier:** Castle fence-material and `ForceFieldCollisionPlane` names occur in parsed map tables (P), without a selected mission actor or verified collision transition. | Asset CA-03C **Ready**, CA-06B **BlockedEvidence** on `origin/docs/cellblock-asset-audit`; no packet branch verified. Identity, phase and live appearance **unproven**. | CA-U08 | CA-U16 | CA-U17 | CA-U18/19 |
| **Straegis scene:** QA `fffffffe` sequence export 953, InterpData 281 (10.0095968246 s), camera/Director groups 286/287, variable 988 → CameraActor 89 (P). Sequence 1751/event 6000 and chain 1161 trigger it (S). Seven historical `StraegisAttack` Matinee roots are presentation leads, **not combat-spawn proof**. | Asset CA-04A/B **Ready**, CA-06C **BlockedEvidence** on `origin/docs/cellblock-asset-audit`; no packet branch verified. Camera binding static P, live acquisition/rift **unproven**. `Str-GroundVoid`/`Hold` exports 35142/35143 prove existence only. | CA-U10/11, CB-U06 | CA-U16 | CA-U17 | CA-U18/19 |
| **Recovery and aftermath:** chain 1161 removes Marsh and delays dialog 2516 by 10,100 ms; current 1172 delays 5859 by 26,100 ms (S). `CA-Props.upk` puddle exports 632–634 and material 38 are not blood proof (P). | Asset CA-04C **Ready**, CA-06D **BlockedEvidence** on `origin/docs/cellblock-asset-audit`, no packet branch verified; Cellblock N02 is **BlockedDependency** until a dialog/route-state fault is reproduced. Render, interruption and late-viewer state **unproven**. | CA-U12/13, CB-U06 | CA-U16 | CA-U17 | CA-U18/19 |
| **Prison Boots:** internal mission 689, item 3438, abilities 1597/1598 and chains 1022–1025 implement lock/Livewire/unlock (S). No historical package claim is needed. | Cellblock N02 **BlockedDependency**, no child branch before reproduction; asset visual owner on `origin/docs/cellblock-asset-audit`. Fresh client lock/unlock **unproven**. | CB-U01, CA-U01 | CA-U16 | CB-U01 | CA-U19 (observer phase only) |
| **Class gear:** CS-04/#1278 grants M622 pistol/tutorials and five-way M687 crate rewards (S); CS-08 saw the opening pistol/reload and relog (limited T). Preparation item 21 is SGHC 6 SMG with `WP-Human.WP_SMG_1A` (S); a P90 label is not its visual identity. | Class-start CS-04 **Integrated**, CS-08 partial live pass on `origin/campaign/class-start-v6` `4e8e165c0`; asset CA-02B **Ready** on `origin/docs/cellblock-asset-audit`. Full-route class grant and locker rendering **unproven**. | CA-U01/06/13, CB-U02/07 | CA-U16 | CB-U02/07 | CA-U19 (phase/observer only) |
| **Armory ring:** `fffeffff` base 1174, console 1178, top ring 226; Castle `00090004` base 433, console 434, ring 77 (P), with no original Kismet at either pad. Spawn 79/region 33 and chain 1109 complete 688 then use `cross_world_teleport` (S). | Ring Phase 0 passed limited client load; Phase 1 **Built/UATPending** on `feat/client-patch-007-castle-armory-ring` `6a9818c09`, with ring system code already on main. Ring owns patch, map binding, FSM and chain 1109. Exit ceremony, transfer/arrival and installed lineage **unproven**. | CA-U15, CB-U07 | CA-U16 | CA-U17 | CA-U18/19 |

## Tooling ownership and server-work verdict

The [autoplay campaign](../cellblock-autoplay/README.md) owns route-driving and
walkthrough specs. Its 2026-09-29 ledger predates AP-02/03/10 work represented
on main; the separately registered AP-20 spec tip `baf1f3f593` and route-tool
tip `013a48cb31` are not ancestors of local `origin/main`. Their presence does
not attest an end-to-end walkthrough or native Livewire solve. The
[lab-reliability ledger](../lab-uat-reliability/README.md) owns helper truthfulness,
dead-player/session observation and ring-click provenance. It lists merged
helper PRs mostly `UATPending`; LR-05a tip `91bdb57cf8` is separate. Issue
#1706 (dialog reported closed while open) and #1708 (point click hit avatar)
must be checked as helper faults before attributing a route failure to content.
AP-20 and LR-05a need lineage reconciliation by their owners before CB-N08
uses their results as proof.

**No unowned Cellblock server change is proven by this audit.** Existing seed
actions and static map exports identify candidate gaps, not present runtime
faults. The first-guard #1667 failure remains a named combat/items issue and
CB-U02 reproduction gate; a normal-resource, known-character shot trace must
precede any server correction. A later observed server fault goes into one
sealed CB-N02 child with its reproducer. The asset, ring and class owners keep
their own file and UAT rows; Cellblock CB-N08 joins them at the route boundary.
