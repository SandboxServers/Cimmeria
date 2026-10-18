---
name: tollana-568-spec
description: Sealed T-03a chain design for Tollana mission 568 (chains 9001-9018) - bind-slot hygiene pattern, zero-button accept-on-close wart, one-shot via chain gate, item-less pilot verdicts.
metadata:
  type: project
---

Spec: `docs/analysis/tollana-continuation/evidence/T-03a-568-chain-spec.md`
(zone-tollana worktree, 2026-10-17). All chains are reconstruction; mission
568 has `script_name` NULL.

Reusable patterns it settled:
- One dialog-set slot per template (slot = template id, 420), exactly one live
  bind per mission state, always remove-before-add. Restore via
  `player_loaded '<World>'` per state; abandon repaint copies Harset 6120.
- Zero-button offer dialog (2153) means Decline and the X also accept; that is
  the cost of a `dialog_choice`-keyed accept. A button strands the accept.
- `num_repeats` 1 plus `repeats` 1 after completion still passes the offer
  guard; one-shot-ness comes only from `mission_status eq not_active` on the
  offer chains (completed reads `completed`).
- Props that must be clickable in every state: template default
  `interaction_type 1073741824` (Dakara 445-448 precedent), tag chains decide
  the effect, linter allowlist entry with reason; no `set_interaction_type`.
- Never `complete_objective` the last required objective of a step to move on;
  use `advance_step`. Optional objectives get force-credited by it.
- Reward 0/0 is an owner decision; `complete_mission` grants nothing.
Open at seal time: T-02 claims for template 421 / spawns 19002-19004 / mesh /
positions.
