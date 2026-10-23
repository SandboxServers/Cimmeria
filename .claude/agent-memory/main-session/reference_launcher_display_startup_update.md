---
name: Launcher display paths and startup updates
description: Display-only Windows formatting and startup update Play gating.
type: reference
---

2026-10-23: `displayPath` formats absolute Windows verbatim drive/UNC paths only
at render boundaries. Never normalize native snapshots, saved preferences, plan
digests or command arguments just to improve UI copy. Consent-save and reopen
tests must retain the original native spelling.

The application inspects/checks the signed updater at startup; a known offer
blocks native and frontend Play admission. One main-page Update and restart
sequence inspects, prepares, then applies, stopping on uncertainty. Ready packages
skip download; active/reconciliation states are inspected without replay. Keep a
known offer on failed rechecks, or a network error can bypass the mandatory gate.
A failed check with no offer permits Play after the bounded request with retry
feedback. Startup alone never downloads or applies.

Canon: `crates/launcher/desktop/docs/updater.md#startup-and-main-page-integration`.
Design/UAT: `docs/analysis/launcher-uat-fixes/README.md` PDU follow-up. Packaged
replacement/restart and real-client acceptance are separate from fixture evidence.