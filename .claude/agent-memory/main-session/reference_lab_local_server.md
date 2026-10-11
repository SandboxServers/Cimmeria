---
name: reference-lab-local-server
description: "Lab client on a local server: the setup steps graduated to docs/guides/lab-contributor-setup.md (2026-10-11); what stays here is the in-world gotchas from the DA-09 run (2026-10-05)."
metadata:
  type: reference
---

**Graduated 2026-10-11.** Pointing the lab at a local server (one `LoginInternal.lua` row, the seeded `lab` / `test` account, shard `Test`, the HMAC secret and the lab endpoint env) is now [docs/guides/lab-contributor-setup.md](../../../docs/guides/lab-contributor-setup.md). Its local endpoint port is 8451, not 8444 (the dev supervisor's default). On the owner's machine, back up `LoginInternal.lua` before adding a `Local` row and restore it afterwards: a stale row once sent the lab to the local server while `server_db_query` read the colo.

Still only here, from the DA-09 run (2026-10-05), not re-verified since:

- A DB reload deletes the lab character: recreate it with `lab_create_character`.
- Click `Inst1Chat_Input` before `client_type_text`. Enter does not reliably focus chat, and the typed letters then hit keybinds (C/I windows, WASD movement).
- A new character's intro dialog swallows input until `lab_finish_dialog`.
- A same-space `.gotolocation` sometimes snaps the avatar back client-side after `client_move_to`, though the server logs the teleport.