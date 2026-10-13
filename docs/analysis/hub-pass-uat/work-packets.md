# Hub pass UAT: work packets

> Type: runbook. Audience: the coordinator who dispatches the drivers and verifiers.
> Companions: [README.md](README.md) (scope, preflight, pipeline, decisions, results), [briefs/](briefs/README.md), [verify/](verify/HP-M1.md).

## Dispatch rules

1. **Preflight first** ([README, Preflight](README.md#preflight-the-coordinator-before-the-first-packet)). Do not start a lab client while a build is running.
2. **Three lanes, one lab instance each, packets in order within a lane.**

   | Lane | Instance (account) | Character | Order |
   |---|---|---|---|
   | A | `default` (`lab`) | Hubmail | HP-M1, HP-M2, HP-M3, HP-M4, HP-C1, then HP-C2 alone (rule 7) |
   | B | `p2` (`lab2`) | Hubpets | HP-P1, then HP-P2, HP-P3, HP-P4, HP-P5 in any order, HP-P6 last |
   | C | `p3` (`lab3`) | Hubdeploy | HP-D1, then HP-D2 and HP-D3 (both need the recipe HP-D1 proved) |

   Start lane A, wait for its driver to be in the world and two minutes more, start lane B, wait two minutes, start lane C. The lanes run in parallel after that. Rough clock: about 12 minutes per packet including its verification, so lane A about 80 minutes, lane B about 75, lane C about 40 plus any recipe work.
3. **Dispatching a driver.** Agent tool, `subagent_type: lab-driver` (run it as defined: no model override, no lab lease lent), prompt = the whole brief file `briefs/<packet>.md` pasted verbatim. The only edit is the `ATTEMPT:` field (1, 2 or 3). The brief creates its own lease (`hubpass-<packet>`, 900 s) and releases it.
4. **One driver per instance at a time.** A second driver on the same instance would be refused at the lease.
5. **Verifying.** When the driver's report arrives, start the Sonnet verifier: Agent tool, `subagent_type: general-purpose`, `model: sonnet`, prompt = `verify/<packet>.md` pasted verbatim with the driver's final report pasted under `## DRIVER REPORT`. The verifier starts its own lookers. The next driver on the same lane may start while the verifier runs; do not start a packet whose verifier called for a re-run of the same state (the verifier says so in `RESUME HINT`).
6. **Copy the recipe forward.** HP-D1's report says which ground-cast attempt worked. HP-D2 and HP-D3 contain the default recipe (attempt A) inside their `GROUND CAST` block; if HP-D1 found a different one, replace that block in both briefs before dispatching them. If HP-D1 ended BLOCKED on the ground cast, do not dispatch HP-D2 or HP-D3: send a Sonnet agent (see Sonnet takeover) to find the recipe on `p3`, then update the three briefs.
7. **HP-C2 is exclusive.** Run it when no other lane is driving and `server_sessions` shows no real tester. Its global `.announce` and `/gmsendgmshout` open a modal prompt on every online client. The other lanes' clients may stay logged in but idle; their next driver starts by dismissing a prompt if one is up (every brief's gotcha 16 covers an unexpected prompt).
8. **Update the status table** in the README after each step (Driving, Driven, Verifying, Verified, Done).
9. **Attempts.** A packet attempt counts when a driver was started on it. Three Haiku attempts, then a Sonnet agent (rules in the [README](README.md#pipeline-and-retry-rules)). Before a retry, run `lab_lease_status {instance}` to see whether the dead driver left a lease; the successor's brief takes it over with `force` only from an owner named exactly `hubpass-<packet>`.
10. **Between packets,** look at `lab_client_status` for the instance (`crash_count_10min`, `login_state`). A crashed client is restarted by the next brief's `lab_ensure_in_world`; two crashes in ten minutes: stop the lane and diagnose before another attempt (a screensaver, a build on the lane, disk full).

### Sonnet takeover

Agent tool, `subagent_type: general-purpose`, `model: sonnet`. Prompt = the packet's brief file (with `ATTEMPT: 4`) preceded by this paragraph:

> You are a Sonnet agent taking over a packet three Haiku drivers did not finish. Everything below applies, except: you may use any `mcp__cimmeria-lab__*` tool, including `client_use_ability` (press an ability by id or name: hotbar key, then click, then the Ability window), `client_hotbar`, `client_target`, `client_combat_log`, `client_events_read`, `lab_screenshot_region`, and `lab-server` read tools (`server_log_tail`, `server_entity_get`, `server_ability_state`). Use them to finish the rows, and to find a working recipe for anything the brief says does not work (write the recipe, with the exact tool calls and arguments, in your report). Do not edit files and do not file issues: the verifier does that. If a row passes at N1 only with a tool the lab-driver lacks, say so, because that is evidence for decision D2.

## Per-packet sections

Every packet: lease owner `hubpass-<packet>`, TTL 900 s, hard stop at 28 tool calls, report format in the brief. "Resume marker" says how a successor sees what is done.

### HP-M1: mail rows 1, 2, 3 (GM `.mail`, then the mailbox window)

- **Instance and character:** `default`, Hubmail (created by this packet). **Rows:** Mail 1, Mail 2, Mail 3. **Estimated calls:** 23. **Depends on:** nothing. Run it first in lane A.
- **Resume marker:** the mailbox headers. A header with subject `HPUM1r1b` = row 1 done; `HPUM1r2` with cash 0 = row 2 done; `HPUM1r3` with item 0 = row 3 done. The probe is step S2 (it opens the mailbox and prints every header).
- **Screenshots:** `M1-mailbox-open`, `M1-cash-mail-before-take`, `M1-cash-mail-after-take`.
- **Risk:** the Access bar mail button may not respond (the brief falls back to `GateMailMod.onToggleMailbox()` and records N3); a header field name may differ (`client_window_read kind mail` is the fallback).
- **Verify:** [verify/HP-M1.md](verify/HP-M1.md).

### HP-M2: mail row 4 (COD)

- **Instance and character:** `default`, Hubmail. **Rows:** Mail 4. **Calls:** 24. **Depends on:** HP-M1 for the character (it creates it if missing).
- **Resume marker:** the header `HPUM2r4` and its state: present with a COD value = unpaid (resume at 4.4); COD 0 with an item = paid (resume at 4.9); item 0 = item taken (resume at the payment mail).
- **Screenshots:** `M2-cod-read-window` (and `M2-pay-failed` if the pay click fails).
- **Risk:** the stock window blocks the unpaid take client-side, so the server guard is probed with an N3 `mailTakeItem`; that is deliberate and the verifier expects it.
- **Verify:** [verify/HP-M2.md](verify/HP-M2.md).

### HP-M3: mail rows 5 and 6 (return a COD; `.mailbox`, `.mail_expire`)

- **Instance and character:** `default`, Hubmail. **Rows:** Mail 5, Mail 6. **Calls:** 21. **Depends on:** HP-M1.
- **Resume marker:** header `HPUM3r5` present and its COD value (0 after the return = row 5 done); header `HPUM3r6a` present = row 6 started (do not create `HPUM3r6a`/`HPUM3r6b` again).
- **Screenshots:** `M3-cod-return-read`, `M3-return-prompt`.
- **Risk:** the return confirmation prompt's button names (`Prompt<N>_Button1`) come from a scan; if the scan finds nothing the driver takes a screenshot and stops that step.
- **Verify:** [verify/HP-M3.md](verify/HP-M3.md).

### HP-M4: the compose window (bad name, postage, Gopher Head)

- **Instance and character:** `default`, Hubmail. **Rows:** Mail 1 (bad name), Mail 2 (the real send with postage), Mail 3a. **Calls:** 25 (the COD variant of 3a is skippable). **Depends on:** HP-M1.
- **Resume marker:** header `HPUM4r2` present = row 2 done; a Gopher Head in the bags (inventory scan) = 3a's give step done; no header `HPUM4r3a`/`HPUM4r3b` is expected (they must be refused).
- **Screenshots:** `M4-compose-filled`, `M4-mission-item-send`.
- **Risk:** `mailSetItemAttachment(container, slot, qty)` is the only way the brief attaches an item (a drag onto the attachment slot is N3 anyway); a self-send may be refused (decision D10, the brief retries once with `Hubpets`).
- **Verify:** [verify/HP-M4.md](verify/HP-M4.md).

### HP-C1: chat rows 9, 9a, 7 and the flood probe

- **Instance and character:** `default`, Hubmail. **Rows:** Chat 9, 9a, 7, plus the ten-lines probe. **Calls:** 19. **Depends on:** HP-M1.
- **Resume marker:** it relogs at the start (to get a fresh welcome line), so the chat ring starts empty; the `HPU`-tagged Say lines (`HPUhi`, `HPUwaves`, `HPUhey`) and Feedback lines show what was done after the relog.
- **Screenshots:** `C1-chat-after-login`, `C1-say-emote-yell`.
- **Risk:** `lab_logout` fails the first time sometimes (a known timeout); the brief retries once and otherwise runs on the current session, which makes the welcome line unreadable and Chat 9 PARTIAL.
- **Verify:** [verify/HP-C1.md](verify/HP-C1.md).

### HP-C2: chat rows 9c and 10 (exclusive)

- **Instance and character:** `default`, Hubmail. **Rows:** Chat 9c, Chat 10. **Calls:** 23. **Depends on:** HP-M1 and dispatch rule 7 (no other lane driving, no real tester online).
- **Resume marker:** the client's channel list (Lua `getChannels()`): `hproom` present = 9c started; the highest `hpaN` present says where to resume; the Server channel lines with `HPUannounce1`/`HPUspace1`/`HPUshout1` say which broadcasts were sent (do not send one twice).
- **Screenshots:** `C2-chat-channels`, `C2-server-message-prompt`.
- **Risk:** a modal "Server Message" prompt stays open until its OK button is clicked; the brief dismisses each one and re-checks.
- **Verify:** [verify/HP-C2.md](verify/HP-C2.md).

### HP-P1: pets U1, U2b, U3

- **Instance and character:** `p2`, Hubpets (created by this packet; WAIT17 applies). **Rows:** U1, U2b, U3. **Calls:** 22. **Depends on:** nothing. First in lane B.
- **Resume marker:** `/say HPUdone HP-P1 u1|u2b|u3` lines (Say channel) and the known-abilities list (2826 present = U2b done).
- **Screenshots:** `P1-pet-bar`, `P1-pet-in-world`.
- **Risk:** the pet spawns behind the avatar; the brief turns the avatar round with the E key to see it. The U3 "moving cancels the warmup" clause uses a GM teleport, because a driver cannot hold a key during a Lua cast within one call.
- **Verify:** [verify/HP-P1.md](verify/HP-P1.md).

### HP-P2: pets U4, U6, U12

- **Instance and character:** `p2`, Hubpets. **Rows:** U4, U6, U12. **Calls:** 22. **Depends on:** HP-P1.
- **Resume marker:** `/say HPUdone HP-P2 u4|u6|u12` lines.
- **Screenshots:** `P2-stance-after-button3`, `P2-stance-after-button1`, `P2-stance-console-aggressive`.
- **Risk:** the run-far test depends on open floor in front of the avatar; a wall makes the leash clause NOT-RUN, not FAIL. The stance button names (`PetContainer1_StanceButton1..3`) come from the client Lua; if a click errors, the brief reads the window list once.
- **Verify:** [verify/HP-P2.md](verify/HP-P2.md).

### HP-P3: pets U5, U7, U8

- **Instance and character:** `p2`, Hubpets. **Rows:** U5, U7, U8 (XP and credit). **Calls:** 24. **Depends on:** HP-P1.
- **Resume marker:** `/say HPUdone HP-P3 u5|u7|u8` lines.
- **Screenshots:** `P3-fight`, `P3-pet-ordered` (and `P3-select-failed`).
- **Risk:** the Rat has to notice the player (aggro on) and the player has no weapon equipped by the brief: only the "what hits you" half of U5 is exercised. Selecting the Rat needs a real left click; the brief allows one retry.
- **Verify:** [verify/HP-P3.md](verify/HP-P3.md).

### HP-P4: pets U14, U15, U16

- **Instance and character:** `p2`, Hubpets. **Rows:** U14, U15 (record only), U16. **Calls:** 23. **Depends on:** HP-P1.
- **Resume marker:** `/say HPUdone HP-P4 u14|u15|u16` lines; the known list (1643, 1644, 1645).
- **Screenshots:** `P4-jaffa`, `P4-lotaur`, `P4-prime`.
- **Risk:** the three looks are the visual point of U14 (the Lo'taur has never been rendered); the verifier leaves them `NEEDS_HUMAN` for the owner.
- **Verify:** [verify/HP-P4.md](verify/HP-P4.md).

### HP-P5: pets U17, U18, U19, U20

- **Instance and character:** `p2`, Hubpets. **Rows:** U17, U18, U19, U20. **Calls:** 20 (U18 waits 70 s). **Depends on:** HP-P1.
- **Resume marker:** `/say HPUdone HP-P5 u17|u18|u19|u20` lines; the known list (2824, 2839, 2852; 2852 is granted only for U19, so its presence means U19 started).
- **Screenshots:** none needed (all server and chat evidence).
- **Risk:** 2852 changes every later summon on this character to instant; run HP-P5 after the packets that need the 6 s warmup (HP-P1, HP-P3).
- **Verify:** [verify/HP-P5.md](verify/HP-P5.md).

### HP-P6: pets U9, U10 and the U2b relog check (last on p2)

- **Instance and character:** `p2`, Hubpets. **Rows:** U9, U10, U2b relog. **Calls:** 18. **Depends on:** HP-P1; run it last on this lane (it kills the character once and relogs).
- **Resume marker:** `/say HPUdone HP-P6 u9` (before the relog; the relog clears the chat ring) or, after it, the world (58 = the zone change is done) and the pet state.
- **Screenshots:** `P6-defeat-failed` only if the death sequence errors.
- **Risk:** the Operatives may kill the pet before they kill the owner; the verifier detects that from the pet's corpse timer.
- **Verify:** [verify/HP-P6.md](verify/HP-P6.md).

### HP-D1: deployables DP-U1, DP-U2, DP-U4 (the ground-cast recipe)

- **Instance and character:** `p3`, Hubdeploy (created by this packet; WAIT17 applies). **Rows:** DP-U1, DP-U2, DP-U4. **Calls:** 22. **Depends on:** nothing. First in lane C.
- **Resume marker:** `/say HPUdone HP-D1 dp1`; the known list (1012); an object in the entity list.
- **Screenshots:** `D1-before-cast`, `D1-warmup`, `D1-object-front`, `D1-object-side`, `D1-after-30s`, and `D-cast-failed` on failure.
- **Risk:** the biggest in the plan. Nobody has placed a deployable in a real client. The brief's attempt A presses the ability with Lua (`useAbility(1012, Unit.Player)`) and clicks a world point with `client_world_click`. If the client does not enter its ground reticle that way, two failed tries end in BLOCKED. Then send a Sonnet agent to find the recipe (the likely alternatives: the Ability window button, the ability on the hotbar with a key press, then a ground click). Update the three D briefs with what works.
- **Verify:** [verify/HP-D1.md](verify/HP-D1.md).

### HP-D2: deployables DP-U3, DP-U5, DP-U8

- **Instance and character:** `p3`, Hubdeploy. **Rows:** DP-U3, DP-U5 (cooldown and range), DP-U8. **Calls:** 24. **Depends on:** HP-D1 proved the ground cast.
- **Resume marker:** `/say HPUdone HP-D2 dp5|dp8`; Rats alive near the avatar (`client_entity_find` "Rat") mean DP-U3 is in progress; god mode on (`/gmsetgodmode 1`) must be off at the end (the brief switches it off).
- **Screenshots:** `D2-rats-in-pulse`, `D2-rats-later`, `D2-attack-object`.
- **Risk:** the 30 s cooldown shapes the order (cooldown refusal inside 25 s, range refusal after 40 s). Without a weapon the client may refuse the attack on the object before the server sees it (the verifier grades PARTIAL then). The behind-a-wall clause is NOT-RUN (decision D7).
- **Verify:** [verify/HP-D2.md](verify/HP-D2.md).

### HP-D3: deployables DP-U6 (death, logout, zone change)

- **Instance and character:** `p3`, Hubdeploy. **Rows:** DP-U6 (variants A, B, C). **Calls:** 24. **Depends on:** HP-D1.
- **Resume marker:** `/say HPUdone HP-D3 dp6a|dp6b|dp6c`; the world (58 = variant C is under way).
- **Screenshots:** none.
- **Risk:** each variant needs 31 s since the last placement, and the event must happen before the object's 30 s run out; the verifier rejects a variant that was too late.
- **Verify:** [verify/HP-D3.md](verify/HP-D3.md).

## After the last packet

1. Every row has a result in the README table, with an issue number for each FAIL.
2. List the lab or driver recipe gaps in one comment on [#1660](https://github.com/SandboxServers/Cimmeria/issues/1660) or in a new lab issue.
3. Retire nothing: this branch has docs only. Commit the filled README table in the same PR as any recipe corrections to the briefs.
4. The campaign close-out (not per packet) updates the Mail, Pets and Deployables sections of [unified-uat.md](../../guides/unified-uat.md) with the results, as round 1 did for bank, consumables and ability trees.
