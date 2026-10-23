---
name: Native launcher phase timing and SGC session review
description: Event transport, nested fields and limits for adoption and Play timing.
type: reference
---

2026-10-23: [Native timing contract](../../../crates/launcher/desktop/docs/launcher-telemetry.md)
and [operator query recipe](../../../docs/operations/telemetry.md#launcher-adoption-and-play-phase-timing)
own this reference. Adoption/Play records use existing `client.native` replay
with `client_target = client.launcher.adoption` or `client.launcher.launch`.
Payload is JSON text in `attributes_string['fields']`, not launcher.summary.
Correlation is process-local numeric data; never combine matching numbers across
launcher runs. Existing opt-in, lossy sender and event-session budgets apply;
missing finish is incomplete evidence, not a confirmed hang.

The [public SGC worknote](../../../docs/analysis/launcher-uat-fixes/worknotes/sgc-session-review-2026-10-23.md)
is anonymized evidence from a partial introductory playthrough. It establishes
three mission completions and unfinished Carter progression, not full SGC UAT.
Historical rows lack granular adoption stages; future measurements require a
launcher containing the new instrumentation. Keep player/machine/session query
values and raw telemetry outside the public repository.
