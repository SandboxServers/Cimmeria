---
name: Cellblock continuation close-out
description: Durable facts from the world-12 Cellblock continuation campaign: ID claims, bark and choice-bind patterns, CIMMERIA_AUTHORING labelling, owner-run lab rule, ledger location
type: project
---

2026-10-17: The Cellblock continuation (branch `content/cellblock-campaign`) was integrated through W5-INT `3a6b3cc64` and closed out in docs. The owner has run no lab row; every client row is UNTESTED. The ledger is `docs/analysis/cellblock-continuation/` (README packet table, `worknotes/cb-n08.md`, `worknotes/cb-n08-acceptance.md`, `owner-lab-checklist.md`).

**ID claims (do not reuse).** CB-N05: mission 62050 (nested 62051-62056), dialog 60150, screen 96110, chains 11000-11005. CB-N04: spawns 520-521 only (522 released), holder dialog 60160 (never displayed), screen 200160 (200161-200162 released), chain 11010 only (11011-11012 released). CB-N07a: chain 11020, screen 200170. CB-N06: missions 62060-62063, dialog 60151, screen 96111, chains 11030-11041, item 2893 granted twice. Individual claims live in `docs/analysis/four-zone-planning/id-allocation.md`.

**Bark pattern.** A gated, fire-once `npc_bark` chain: trigger `enter_region`, scope `player`, `once` true, one `step_status` condition (`mission_<m>_step_<s>_status` = `active`), text in `dialog_screens` under holder dialog 60160. It adds no spawn, aggression or time, so it needs no difficulty decision.

**Per-player choice-bind pattern (CB-N06).** One exclusive choice is bound per player on the prisoners and the `329_CellDoorButton` panel through existing dialog-set-maps; either branch grants once and closes the other. The bind chains need exemptions in the `interact_tag` linter.

**Labelling.** New Cimmeria content carries `CIMMERIA_AUTHORING` in every seed and doc and is never presented as 2009 content (D-CC18). Omitted by owner ruling: CB-N07b (D-CC19). Held outside the PR: CB-N05b, on Harset HA-1360.

**Owner-run lab rule.** Agents never drive the lab for this campaign (owner directive 2026-10-17). The owner runs CB-U03, U05, U08-U12 from the checklist and records results in the README rows. Run exactly one `cimmeria-server` first: a second server on auth port 13001 invalidated every lab result on 2026-10-17.

**Why:** keeps a later session from re-claiming IDs, re-driving the lab, or reading server tests as client passes.
**How to apply:** check this before touching world-12 chains or IDs 11000-11041, 60150-60160, 62050-62063.
