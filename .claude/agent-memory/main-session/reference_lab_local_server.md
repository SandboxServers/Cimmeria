---
name: reference-lab-local-server
description: "How the lab client was pointed at a local worktree server (2026-10-05, DA-09): temporary Local row in the install's LoginInternal.lua, shard name 'Test', seeded lab account, gotchas (typed text leaks to keybinds, same-space teleport snap-back)."
metadata:
  type: reference
---

The lab client normally plays on the colo (`LoginMod.servers["Cimmeria"]` in `<install>/SGWGame/Content/UI/Startup/Login/LoginInternal.lua`). To test a branch before release (DA-09, 2026-10-05):

1. Build `cimmeria-server --release` through the lane, reload the worktree DB (`tools/build-lane/reload-db.sh`), and run the exe from the worktree root with `DB_URL=<sgw_<worktree> url>`, a throwaway `CIMMERIA_TELEMETRY_HMAC_SECRET` (the lab mints its telemetry token from the login server and refuses to launch without one) and `OTEL_EXPORTER_OTLP_ENDPOINT` unset.
2. Back up `LoginInternal.lua` and add `LoginMod.servers["Local"] = "http://127.0.0.1:8081"`. `lab_client_start server=Local`, then `lab_login server=Test`: the second name is the **shard** row the local server lists, not the login row. Restore the file afterwards.
3. The seed's `lab` account (id 10) has the lab password; don't insert another (account names have no unique constraint). A DB reload deletes the lab character: recreate it with `lab_create_character`.

Gotchas: click `Inst1Chat_Input` before `client_type_text`. Pressing Enter does not reliably focus chat, and the typed letters then hit keybinds (C/I windows, WASD movement). A new character's intro dialog swallows input until `lab_finish_dialog`. A same-space `.gotolocation` sometimes snaps the avatar back client-side after `client_move_to`, though the server logs the teleport. `client_move_to` and `client_camera` can't turn (#1243), so teleport to a spot whose default north facing shows the target.
