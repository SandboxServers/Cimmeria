# Ground-target deployable press ignored by the client (Focus Regen, Med Station)

Rows: armory-abilities-scientist Scientist-1223 (Deployable: Focus Regen), Scientist-1224 (Deployable: Med Station).
Runs: C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqrovhriypya
Expected: setup client_use_ability sends the press.
Observed: both rows BLOCKED "setup client_use_ability failed: client_use_ability denied: ignored: The game read the input but nothing changed. Something is blocking it". SigNoz abilities scope for 1223 and 1224: zero events, so the press never left the client. Seed: both are target_type_id 3 (ground target), ABILITY_TYPE_DD, effect_ids empty. Other ground deployables in the same run pressed fine (962-966, 1012, 1015, 1236 reached use_ability_on_ground_recv). The two rows ran back to back (23:33 and 23:34Z). Cause not proven. Same family as DR-L2-soldier-0-flushing-fire-press-ignored.md. If they send, expect no_mechanics (no effects) and add them to DR-L4-scientist-no-mechanics.md.
Repro: lab_uat_run {sections: ["armory-abilities-scientist"], rows: ["Scientist-00", "Scientist-1223", "Scientist-1224"], specs_dir: "C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\docs\guides\uat-specs"}
Area: lab runner / client press path; unknown.
Labels: lab, debug-armory
