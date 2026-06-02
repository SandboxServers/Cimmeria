---
name: project-mission-dialog-audit-2026-05-31
description: CAT-J audit findings as of 2026-05-31 — mission/dialog/interaction handlers
metadata:
  type: project
---

CAT-J audit of mission/dialog/interaction handlers (cell methods 52-54
Missionary + 74-76, 87 SGWPlayer + chain-engine event_dispatch path) found
9 reportable findings; one Critical (DIALOG_BUTTON_CHOICE has no
open-dialog tracking — allows forging any OnDialogChoice chain
side-effect), one High (mission accept has no prereq validation), one
High-latent (ChosenRewards unimplemented — reward-pick design constraint
to lock in before implementation), one Medium-latent (the Mission*/Debug
names that fall through unhandled today).

**Why:** Pre-implementation findings are filed as advisory to lock the
design constraints into reviewer-greppable evidence BEFORE the
implementation PRs land — same pattern as the mail/trade handler
findings, where filing the latent gap early let the implementer use the
finding as a checklist rather than re-deriving it.

**How to apply:** When `chosenRewards`, `MissionAdvance`,
`MissionComplete`, `MissionAssign`, `MissionReset`, `MissionClear*`,
`MissionSetAvailable`, `DebugInteract`, `ShareMission`,
`ShareMissionResponse`, or `AbandonMission` (slash-cmd) handlers come
off the UNIMPLEMENTED log path, ensure the implementing PR cites
**CAT-J-04** (prereq validation for accepts), **CAT-J-05** (server-side
offered-rewards-set pin for ChosenRewards), **CAT-J-06** (group +
recipient-consent for ShareMission), or **CAT-J-08** (GM gating for the
Debug + admin Mission* variants). Implementer should also clear
**CAT-J-01** (open-dialog tracking) before any new chain-firing handler
trusts a client-supplied dialog/state-key id.

Links: [[reference-dialog-choice-exploit-shape]]
[[project-trade-handlers-unimplemented]]
[[project-mail-handlers-unimplemented]]
