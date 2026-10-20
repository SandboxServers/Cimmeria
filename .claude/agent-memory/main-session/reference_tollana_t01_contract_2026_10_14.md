---
name: Tollana T-01 source and placement contract
description: "2026-10-14: bounded OPC 568-570 candidate, build-58674 map hash match, quest placement and mission-568 pilot gates."
type: reference
---

# Tollana T-01 source and placement boundary

O14-20 permits a bounded active arc after review. Game archaeology and the
mission advisor selected OPC 568-570 as the candidate, with 568 first.
Their journal IDs/text are source-backed; the 568 -> 569 -> 570 unlocks,
actor positions and item transfers are `CIMMERIA_RECONSTRUCTION`. The T-00
matrix retains 51 verified holds until implementation contracts seal. Mission
590 is separate: 570 says to obtain Travell's Control Technology, while 590
revisits the bargain and delivery; Sagrine's return needs escort review.

The local 58,674 Tollana root `.umap` matches the handoff map-manifest hash.
The configured lab client's root `.umap` instead matches the archive's
QA4046 row. Neither hash assigns a build to mission rows. Gate 4 and two
world-19 ring-console rows are seed
anchors. `docs/analysis/debug-area/map-selection-study.md` puts gate and west
ring on nav component 89, middle ring on isolated component 885; these are
survey leads, not mission cast coordinates. No quest cast coordinate was
recovered by the bounded named-actor extraction.

A 2026-10-14 lab probe rendered gate and urban survey anchors. A same-world
GM jump to the west-gallery lead (-770, 9, 380) showed void and a fall, then
the client telemetry bridge became unreachable during unverified recovery.
This fails that jump probe, not a native walking/ring route. The lab-server
endpoint reported zero sessions; no server-log corroboration exists. Do not
use that west lead as a placement until the discrepancy is resolved.

T-01's individual point-in-time claims are chain 9001, template 420 and
spawn 19001. Template 420 is for a distinct `Tollana_Copplemann`, since
Harset template 48 has template-scoped dialog bindings. Recheck collisions
before seeding. Mission 568 still needs exact Repair Facility placement,
fragment task/item mapping, acceptance dialog, reward, persistence,
telemetry, client journal and two-player proof. See
`docs/analysis/tollana-continuation/evidence/T-01-source-map-contract.md`.

Objective 2312 is disabled and optional although tasks 2939/5389 are enabled.
Item 2343/2742 names do not prove task ownership. The current asynchronous
content `remove_item` can fail to consume while a chain advances, and the DB
loader cannot author `HasItem`. Native `ConsumeItemForUse` acknowledges an
inventory commit but its ack can be lost and mission persistence is a separate
write. Pilot 568 needs a combined server-owned inventory/mission transaction
with durable retry handling before any coder dispatch, plus missing-item,
wrong-character, duplicate, rollback, relog and two-player guards.
