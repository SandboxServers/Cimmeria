---
name: dialog-choice-cannot-branch-on-button
description: OnDialogChoice keys on dialog_id only; button_id is a context param with no authorable condition, so Accept/Decline and Accept/More-Info branches cannot be authored today.
metadata:
  type: project
---

# `dialog_choice` cannot distinguish buttons (confirmed 2026-09-18)

`Trigger::OnDialogChoice { dialog_id }`
(`crates/content-engine/src/triggers/mod.rs:75`) matches on the **dialog id
only**. Every button on that dialog fires the same chain.

`fire_dialog_choice` (`crates/services/src/cell/content/event_dispatch/dialog.rs:90`)
*does* stamp `ctx.params["button_id"]` — but the only authorable condition
types are the six keys in `crates/content-engine/src/loader/condition.rs`:
`mission_status`, `step_status`, `archetype`, `objective_status`, `counter`,
`stat_below_max`. There is **no DB key for `PropertyEquals`**, so
`button_id` is unreachable from a seeded chain.

Consequences for porting any offer dialog (`INT_*MissionAvailable` flags with an
Accept button + a More Info / Decline button):

- You cannot author "Accept → accept_mission, More Info → display_dialog".
  Both buttons run the same action list.
- Workarounds: (a) auto-accept the mission on a `player_loaded` / region chain
  and skip the offer dialog entirely, losing fidelity; (b) put the branch on
  separate *dialogs* rather than separate buttons, if the content allows.
- The real fix is a `button_id` condition key (trivial loader + a
  `Condition::PropertyEquals`-style evaluator — the param is already populated).

Related: `fire_dialog_choice` also does **not** populate `archetype`
(contrast `fire_interact_tag` / `fire_player_loaded`, which do), so an
`archetype`-gated `dialog_choice` chain reads the `-1` default and never
matches `eq`.

`fire_dialog_open` is called after native interaction dispatch and from
`interactions/dispatch/initial_response.rs`. It is a usable trigger for
"dialog with no actionable button", but re-fires on every re-display, so gate it.
