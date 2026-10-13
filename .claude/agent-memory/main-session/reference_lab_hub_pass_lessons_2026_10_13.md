---
name: reference_lab_hub_pass_lessons_2026_10_13
description: Lab driving facts from the 2026-10-13 hub pass (mail, chat, pets, deployables): shared plaza, chat channel id 9, wait_for on world loads, ensure_in_world name read, Prompt1_SingleButton, death recipe, god mode, stale camera schema, covering windows
metadata:
  type: reference
---

Dated 2026-10-13, from the hub pass UAT (`docs/analysis/hub-pass-uat/README.md`, 38 rows, 15 packets, three parallel clients). Lab issues filed: #1673, #1676, #1677, #1679, #1680 (plus a comment on #1660).

- **Lanes must not share a plaza.** Pets and deployables on one world-1300 space saw each other's pets, blocked camera shots, and pets fought the other lane's Rats. Run them one at a time.
- **Server replies are on chat channel id 9**, which `client_chat_log` reports as `None` (no name, colour or tab: #1679). Filter by id `"9"` or `contains`, never `"Feedback"`.
- **Never `client_wait_for` across a world load** (#1677: it aborts on the first bridge failure). Use fixed `client_batch` waits, 2 x 20 s for world 1300.
- **`lab_ensure_in_world` name-read quirk (#1680):** `unitName(Unit.Player)` reads empty on a dead player and the call fails; `lab_logout` then crashed the client. Recovery: `lab_client_restart`, `lab_ensure_in_world`, `.gotolocation`.
- **The Server Message modal's OK button is `Prompt1_SingleButton`** (`Prompt1_Button1` is hidden). `.announce` and `/gmsendgmshout` open it on every online client.
- **Death recipe (Debug Area):** arrive at respawner 131 (`438.0 11.0 -916.0`) and walk east over the rise to the level-31 NID Operatives; a level-1 dies in about 10 s. A direct `.gotolocation` onto them dropped the client through the ground. Deployable lifetime is 30 s, so kill fast.
- **God mode for pet tests** (`/gmsetgodmode 1`): without it the owner dies to the Rats; despawn leftover Rats between runs. Even so a Straegis pet loses to a Rat (#1681).
- **The installed daemon's `client_camera` schema was stale:** numeric `pitch_deg`/`yaw_offset_deg` only; `face_entity_id` on a deployable fails ("no pose"), face a point.
- **A covering window swallows native clicks:** the client dispatches to the topmost visible child. On 2026-10-13 the live Lua preflight returned `BackgroundWin` from `Root:getTargetChildAtPosition` and `AttackDirectionQuartersWin` from `getChildAtPosition` at the same cursor point. Both click tools now use the former, compare its parent chain with the resolved window's `getName()` (path locators are not CEGUI names), and refuse before sending input when another window receives the point. For mail Inbox Take, close Create or Read first.
- **Haiku drivers stayed within about 30 calls** a packet; after three failed attempts or a recipe gap use a Sonnet takeover.
- **GM accounts skip the chat flood limit**, so "too quickly" needs a non-GM account.
