---
name: Explicit verified archive selection starts eligible installation
description: Archive-to-install handoff, cancellation marker and no replay.
type: reference
---

2026-10-23: [Design](../../../docs/analysis/launcher-uat-fixes/design-archive-start.md)
and [archive contract](../../../crates/launcher/desktop/docs/seed-archive.md#explicit-selection-starts-eligible-installation)
own this reference. Explicit main-page or Settings choose/Downloads acceptance
hands off once to the existing Install action after successful verification.
That action refreshes native status/revision and admission. Existing, owned or
recovery-blocked installations do not trigger a new install. No handoff on
Inspect, reopen, failure, lost reply, disposal or cancellation.

Cancelled native pickers return optional picker_cancelled=true only on that
response, omitted normally and never persisted. Preserve it through workflow
settlement so an old chosen archive does not look like a fresh successful
selection. Keep the source RAR unchanged; no Plan fields or synthetic clicks.
Prior catalog-key checks passed 1,185 native/275 frontend tests before this
extension; new handoff checks, final review and CI remain pending.
