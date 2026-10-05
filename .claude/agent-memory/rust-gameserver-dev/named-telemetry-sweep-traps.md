---
name: named-telemetry-sweep-traps
description: Rule 6 (ID + name) sweep lessons from NT-21: name sources that look absent but exist, fast rescans, logged keys that hold the wrong ID
metadata:
  type: project
---

Lessons from the NT-21 sweep (2026-10-04, PR #1217). They apply to any Rule 6 pairing work.

- **Check the seed before exempting an ID as unnamed.** `content_chains.description` is a real chain name: every seeded chain has one. `loot_tables.description`, `trainer_ability_lists.description`, `event_sets.name`, `sequences.kismet_script_name` (PlaySequence's `sequence_id`) and `dialog_set_maps.topic_text` are names too. All of them are NameBook tables now (`book().chain/loot_table/trainer_ability_list/event_set/sequence/dialog_set_map`).
- **A logged key can hold a different ID than its name says.** The executor's `dialog_set_id` held `dialog_set_maps.dialog_set_map_id` (2794 is "Free Prisoner 329"), and several `player_id` keys held entity ids. Check the value's source before you pair it, or you resolve the wrong table.
- **Fast rescans.** Copy the built `cimmeria_server-*.exe` test binary out of `B:/targets/<wt>/debug/deps/` and run `--ignored --exact logging::unpaired_id_tests::unpaired_id_report` from `crates/server`. The scan reads source at runtime, so it rescans without a rebuild. A temporary env hook in `unpaired_id_report` can dump `file:line key` per site; revert it before committing.
- **A NameBook guard must not cross an `.await`.** Scope `let names = cimmeria_names::book();` in a `{}` block. An `Option<String>` field is a valid tracing value when a guard can't be held.
- **Python edits on CRLF worktree files.** Normalize to `\n`, replace, then restore `\r\n`. Mixed endings make the edit's `old_string` miss silently. See [[python-write-mangles-utf8-and-crlf]].
- **Revert-proof mutation scripts.** Anchor on the LAST occurrence of a log message (`rindex`). The first occurrence is often a comment that mentions the message.
