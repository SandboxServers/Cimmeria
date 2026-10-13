# Hub pass UAT: driver briefs

> Type: reference. Audience: the coordinator who dispatches the drivers and verifiers.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md).

Each driver brief is the whole prompt for one Haiku `lab-driver` agent (Agent tool, `subagent_type: lab-driver`, no model override). Paste the file verbatim and change only the `ATTEMPT:` field. A brief is self-contained: it holds the lease call, the character creation call, every tool call with its arguments, the chat macros, what success and failure look like, the screenshots to take, the stop rule (28 tool calls) and the report format. Each verifier brief in [../verify/](../verify/HP-M1.md) is the prompt for one Sonnet agent; paste the driver's report under its `## DRIVER REPORT` heading.

| Packet | Driver brief | Verifier brief | Instance | Character | Rows | Est. calls |
|---|---|---|---|---|---|---|
| HP-M1 | [HP-M1.md](HP-M1.md) | [verify](../verify/HP-M1.md) | default | Hubmail | Mail 1, 2, 3 | 23 |
| HP-M2 | [HP-M2.md](HP-M2.md) | [verify](../verify/HP-M2.md) | default | Hubmail | Mail 4 | 24 |
| HP-M3 | [HP-M3.md](HP-M3.md) | [verify](../verify/HP-M3.md) | default | Hubmail | Mail 5, 6 | 21 |
| HP-M4 | [HP-M4.md](HP-M4.md) | [verify](../verify/HP-M4.md) | default | Hubmail | Mail 1, 2 (window), 3a | 25 |
| HP-C1 | [HP-C1.md](HP-C1.md) | [verify](../verify/HP-C1.md) | default | Hubmail | Chat 9, 9a, 7 | 19 |
| HP-C2 | [HP-C2.md](HP-C2.md) | [verify](../verify/HP-C2.md) | default | Hubmail | Chat 9c, 10 | 23 |
| HP-P1 | [HP-P1.md](HP-P1.md) | [verify](../verify/HP-P1.md) | p2 | Hubpets | U1, U2b, U3 | 22 |
| HP-P2 | [HP-P2.md](HP-P2.md) | [verify](../verify/HP-P2.md) | p2 | Hubpets | U4, U6, U12 | 22 |
| HP-P3 | [HP-P3.md](HP-P3.md) | [verify](../verify/HP-P3.md) | p2 | Hubpets | U5, U7, U8 | 24 |
| HP-P4 | [HP-P4.md](HP-P4.md) | [verify](../verify/HP-P4.md) | p2 | Hubpets | U14, U15, U16 | 23 |
| HP-P5 | [HP-P5.md](HP-P5.md) | [verify](../verify/HP-P5.md) | p2 | Hubpets | U17-U20 | 20 |
| HP-P6 | [HP-P6.md](HP-P6.md) | [verify](../verify/HP-P6.md) | p2 | Hubpets | U9, U10 | 18 |
| HP-D1 | [HP-D1.md](HP-D1.md) | [verify](../verify/HP-D1.md) | p3 | Hubdeploy | DP-U1, U2, U4 | 22 |
| HP-D2 | [HP-D2.md](HP-D2.md) | [verify](../verify/HP-D2.md) | p3 | Hubdeploy | DP-U3, U5, U8 | 24 |
| HP-D3 | [HP-D3.md](HP-D3.md) | [verify](../verify/HP-D3.md) | p3 | Hubdeploy | DP-U6 | 24 |

## How the briefs are built

The briefs share one preamble (identity, lease, gotchas, the chat and screenshot macros) and one report format; the pets and deployables briefs add a block of shared definitions (where to stand, the pet and ability reads, the death site). The ground-cast recipe sits inside each deployables brief. A change to a shared block is made in all fifteen briefs; check with `Select-String -Path briefs\*.md -Pattern "<a line of the block>"`.

The calls the briefs rely on, and how each is proven:

| Call | Proven by |
|---|---|
| `lab_lease_acquire`, `lab_ensure_in_world` with `create`, `client_batch`, `client_ui_sequence`, `client_chat_log`, `lab_screenshot` | round 1 ([README](../../haiku-lab-uat/README.md), [BANK-C1](../../haiku-lab-uat/briefs/bank/BANK-C1.md)) |
| `.gotolocation DebugArea x y z`, `client_wait_for` on `getCurrentWorldID()` | round 1 AT-C1 and AT-C3 |
| `client_world_click` on an entity, `client_camera`, a right-click at 3.5-4.5 m | round 1 and the lab-harness close-out |
| `ChatMod.onTextAccepted(Inst1Chat_Input, Inst1Chat_Input)` as a chat submit | read from the client's `ChatWindow.lua` (not run live); the typed path (CHAT1) is the proven one |
| `useAbility`, `usePetAbility`, `PetContainer1_StanceButton1..3`, `Unit.Pet1`, the mail Lua and window names | read from the client's Lua (`Ability.lua`, `PetInfo.lua`, `PetContainer.lua`, `GateMail.lua`); not run live |
| The deployable ground cast (`useAbility(1012, Unit.Player)` then `client_world_click {point}`) | a guess from the client hook anchors (`docs/reverse-engineering/findings/ability-client-hook-anchors.md`); never run |

If a live run shows one of the unproven calls does not work, fix the brief and the matching line above in the same change.
