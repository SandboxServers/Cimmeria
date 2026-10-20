# HA-17a1: pilot graph design card, Jaffa story 1 (watch rotation)

> Type: design (PROVISIONAL, unsigned). Audience: the coordinator, Lomiada, the owner, the HA-00c signer. Drafted 2026-10-18 against campaign tip `29f9d3fdb` (branch `content/harset-campaign`). Docs only: no ID is allocated, no seed or code was changed, no cargo, lab or client was used.
> Decisions: O18-11 (pilot first, no IDs before graph and collision checks), O18-05 (AI-derived provisional premises and rewards), O14-04, O14-10, O14-11, O14-12, O18-13. Companions: [packet contracts](../work-packets.md), [sign-off cards](HA-00c-signoff-cards.md), [HA-00c](HA-00c.md), [owner decisions 2026-10-18](owner-decisions-2026-10-18.md), [ID proposals](../id-allocation.md), [placements](../placements.md), [Lomiada brief](../lomiada-brief.md).

## 1. What this is, and the labels

This is the design card the pilot needs before any ID is asked for. It is a proposal that a human can reject. Every value marked PROVISIONAL is a placeholder the owner may change.

- **No ID appears here.** Every ID need is a kind, written as "ID needed: kind". Numbers that do appear are existing seed, spawn or template facts.
- **The signature cells stay blank.** Nothing here is a signature, reservation or allocation.
- **Provenance:** new Cimmeria authoring, `CIMMERIA_AUTHORING` in seed and ledger (O14-10), no player-facing provenance label (O14-11).

| Label | Meaning |
|---|---|
| FACT | A file, seed row, code line or recorded result says it. The source is named. |
| INFERENCE | Read from text, adjacency or engine habit. Confidence is high, medium or low. |
| DECISION | Recorded owner decision. It authorizes work; it is not UAT. |
| PROVISIONAL | A value this design proposes and nobody has approved. |

## 2. Premise (PROVISIONAL)

**"Changing of the Watch."** Lo'rak, the Jaffa Requisitions Officer in the Harset bazaar, has noticed the Jaffa watch post in the Jaffa Zone no longer reports on schedule. He asks the player to find the post's sentry, learn how the rotation is being kept, and bring the answer back so the requisition roster can be corrected. The sentry gives a short, dry account. The player returns, Lo'rak adjusts the roster, and pays a small stipend.

- Lore safety (O14-10): a quartermaster fixing a duty roster is generic Jaffa garrison business. It names no canon character, planet or event beyond the actors already in the Harset seeds, and it is not retold from 2009 material. INFERENCE, medium confidence (nobody has compared it to the lost scripts, which cannot be compared).
- Premise choice: the contract offers "watch rotation" or "supply routing between proved district actors". This card takes watch rotation first, matching the cards' low-confidence guess. Story 2 (supply routing) is a separate card and shares nothing with this graph. INFERENCE, low confidence for the ordering; the owner can swap.
- MMO suitability: a short fetch-and-report loop, two steps, no kill requirement, no timer, no shared-world mutation. Any number of players can run it at once because it only changes per-player state.

## 3. Actors (existing spawns only)

All actors are world 57 and already spawned. Nothing is added, moved or despawned.

| Role | Actor | Spawn / template | Evidence | Why this one |
|---|---|---|---|---|
| Giver and closer | `Harset_Lorak`, Jaffa Requisitions Officer | spawn 302 / template 201, stationary at (22, -68.9, -30) | FACT: [placements](../placements.md), `entity_templates.sql` | A stationary vendor-side Jaffa in the bazaar. A quest-giver role suits a quartermaster. |
| Step actor | `Harset_SuspiciousJaffa`, the watch-post sentry | spawn 305 / template 205, stationary at (-159, -41.28, -26) | FACT: [placements](../placements.md) | Stationary, inside the inferred Jaffa Zone region 2100. A fixed sentry is a natural "watch post". |

### Actors rejected

| Actor | Why not |
|---|---|
| Template 204 (spawns 303, 304) | Template 204 is effectively reserved for mission 1326. Dialog-set map 120002 (set 1393, NULL dialog, flag 268435456) lights every template-204 NPC in AoI, and the Jaffa seed note says template 204 must carry only the two mission Jaffa. FACT: `harset_jaffa_chains.sql` header. |
| Moh'katan (template 54, world 68) | World 68 is a different zone, and the offer ladder there (1324, 1325, 1326, 1343, 1347, 1352) is the most crowded in the zone. Forking it is a hazard. FACT: mission review CSV. |
| Template 206 (Angry Jaffa) | No spawn row exists. FACT: `entity_templates.sql`. |

### Shared-template hazards the guards must answer

Both chosen templates are used by other missions. The pilot's binds must be state-disjoint from theirs.

| Template | Other missions that use it | Hazard |
|---|---|---|
| 201 (Lo'rak) | 1244 and 1245 (Goa'uld), 1347 (Loyalist) | Their binds and interact chains also fire on Lo'rak. A pilot bind must be keyed so it can never co-display with those. FACT: template rows, review CSV. |
| 205 (Suspicious Jaffa) | 1322 (Goa'uld), 1371 (OPC), plus an arrest dialog choice | The sentry's interaction is already used. The pilot needs a different tag or a disjoint step guard. FACT: template rows. |

Whether the 1244, 1245, 1347, 1322 and 1371 chains are restored is not settled by this card. The collision-check procedure in section 8 includes them as live neighbours whichever way that goes.

## 4. The graph

A visible accept, two meaningful steps using existing district activity, a closing dialog and a reward, as the contract requires. FACT for the shape; the rows are not authored.

```text
not_active
  | Lo'rak offer dialog closes                  (dialog_choice, zero-button)
  v
STEP A  "Find the watch post"            objective A1: TalkToNpc (sentry, spawn 305)
  | player interacts with the sentry           (interact_tag, advance_step)
  v
STEP B  "Report to Lo'rak"               objective B1: turn-in, closed by complete_mission
  | player interacts with Lo'rak               (interact_tag, remove bind, complete_mission, reward)
  v
completed
```

### 4.1 Mission, step, objective, task shape

| Level | Content | Notes |
|---|---|---|
| Mission | One authored mission. Not repeatable: `num_repeats` 1. Hidden = false so the journal shows it. | Reward fields on the mission row stay 0 (the seed rows are 0 everywhere; rewards ride chain actions). FACT: `complete_mission` grants nothing. |
| Step A | "Find the watch post". One required objective A1. | A1 is a TalkToNpc. The primitive is server-side only; the client sees the wire shape. FACT. |
| Step B | "Report to Lo'rak". One required objective B1. | B1 has no `complete_objective` anywhere; only `complete_mission` closes it. This is the standard turn-in shape (622 ArmYourself, 688). FACT: mission advisor memory note on turn-in objectives. |
| Tasks | One task per objective, count 1. | Wire mismatch: `MissionTaskStatus.count` is INT32 on the wire and INT8 in alias.xml (#266). Counts of 1 are safe either way. FACT. |

### 4.2 Chains by role

Each chain is named by what it does. "ID needed: chain" everywhere. All chains carry `CIMMERIA_AUTHORING`.

| Role | Trigger | Guards (persisted) | Actions, in order | Notes |
|---|---|---|---|---|
| C1 offer bind | `player_loaded Harset` (and world-change path, same trigger) | `mission_status` of the pilot eq `not_active`; faction/archetype gate if the owner wants one | `add_dialog_set` to Lo'rak's slot (offer set) | One bind per template slot. Disjoint from C2 by the `not_active` guard. |
| C2 accept | `dialog_choice` on the offer dialog | `open_dialog_id` eq the offer dialog; `mission_status` eq `not_active` | `remove_dialog_set` (offer), `accept_mission` (Lo'rak has no bind during step A: see 4.3), `display_dialog` (monologue acknowledgement) | Cleanup first, mission mutation after. The accept is on close; see 4.4. |
| C3 sentry flag | `player_loaded Harset` | `mission_status` active, `step_status` A active | `add_dialog_set` carrying the interaction flag on the sentry (NULL-dialog flag carrier) | Restore chain: repaints after relog and world hop. Gated on the ACTIVE step, never on `completed` (finished steps persist empty). |
| C4 sentry advance | `interact_tag` on the sentry | `step_status` A eq `active` | `remove_dialog_set` (sentry flag), `display_dialog` (sentry account, Route A), `advance_step` to step B, `add_dialog_set` (Lo'rak turn-in) | Uses `advance_step`, never `complete_objective` on the last required objective, because that would close the mission early. |
| C5 turn-in restore | `player_loaded Harset` | `mission_status` active, `step_status` B active | `add_dialog_set` (Lo'rak turn-in) | Same repaint pattern for the second step. |
| C6 close and reward | `interact_tag` on Lo'rak | `step_status` B eq `active` | `remove_dialog_set` (turn-in), `display_dialog` (closing monologue), reward actions (4.5), `complete_mission` | Item and bind cleanup before the mission mutation. |
| C7 abandon repaint | `mission_abandoned` keyed on the pilot | none beyond the trigger | `remove_dialog_set` for the sentry and turn-in binds, `add_dialog_set` (offer) | Without it, an abandoned player is stranded with no offer. Copies Harset chain 6308 (`mission_abandoned` repaint) in shape. FACT: `harset_jaffa_chains.sql`. |

### 4.3 Bind slot hygiene

- Bind slots are keyed by template id. A slot holds one live bind: always remove the old bind before adding the new one. FACT: Harset chains 6303 to 6308 in `harset_jaffa_chains.sql` and the Tollana 568 chain spec pattern.
- Lo'rak has exactly one live pilot bind per state: offer (not_active), nothing (step A), turn-in (step B), nothing (completed). Slot 201 is shared with Goa'uld and Loyalist missions; the pilot's binds are keyed to its own mission state, so they cannot be live together with those unless the player is also on those missions.
- Binds do not survive a cross-world hop. C1, C3 and C5 are the restore chains.
- The shared-hub rule: bind per player. `set_interaction_type` mutates shared template state for everyone and is not used. FACT: standing rule for worlds 57 and 68.
- Route rule: the interaction chains (C4, C6) match on `interact_tag`, so they are Route A. A Route A chain does not pin `last_interaction_target`, so a `dialog_choice` follow-up on the display those chains open would not resolve the target. The pilot therefore needs no `dialog_choice` after C4 or C6 and keeps both dialogs button-less. FACT: interact routing memory note.

### 4.4 The accept-on-close wart

The offer dialog has no buttons. A zero-button dialog fires `dialog_choice` with button -1 on close, so Decline and the X also accept. Adding a button strands the accept chain because the chain does not check buttons. This is the cost of a `dialog_choice`-keyed accept. FACT: dialog authoring rules; Tollana 568 chain spec. The owner may accept the wart or ask for a different accept path. That is a MAINTAINER DECISION because the alternative (a real accept button) needs engine branching that does not exist.

### 4.5 Reward (PROVISIONAL)

| Part | Proposal | Engine fact |
|---|---|---|
| XP | A modest amount, to be set by the owner. This card proposes the same band as the Harset restored kill-and-fetch missions and no higher. | `grant_xp` exists (loader, executor arm since PR #618). Zero seed rows use it. The XP formula is gated on GC3 and D-H10, so the amount is open. FACT. |
| Currency | A small stipend is the premise's payoff. | No cash chain action exists in the loader. The base-side cash grant exists, reached by other paths. A currency reward needs a new executor arm, which is an implementation item and an owner call. FACT (loader has no cash name); INFERENCE, medium (only a new arm would fix it). If the owner declines the arm, the pilot pays XP only. |
| Item | None. The pilot needs no supply item, so the HA-15 item gaps do not block it. | `add_item` exists (`GrantItem`). Not used. FACT. |

- One-shot guard: C6 is guarded by `step_status` B active, which is a persisted gate. After `complete_mission` the step is no longer active, so a second click grants nothing. `once` is not used as a guard because it resets on relog and world change since #802.
- `repeats` is not persisted by the world-entry UPSERT. The pilot is not repeatable, so it does not depend on that fix. FACT.
- Reward amounts stay open until GC3 / D-H10 is answered and the owner approves (O14-12: modest, no new currency, no high-tier gear).

## 5. Engine traps checked against this graph

| Trap | Answer for the pilot |
|---|---|
| Every matching chain fires | C1, C3 and C5 share a trigger key but have pairwise disjoint persisted guards (`not_active`, step A active, step B active). C4 and C6 share `interact_tag` but different tags (sentry vs Lo'rak) and different step guards. Neighbour missions on template 201 and 205 are in the collision procedure. |
| `once` resets on relog | Not used as a guard anywhere. |
| Item and bind cleanup before mission mutation | C2, C4 and C6 list cleanup first. |
| `remove_effect` has no executor arm | Not used. |
| Quest speakers carry no vendor or trainer fields | Template 201 has `speaker_id` NULL ("no Lo'rak row in speakers.sql"). The dialogs use the speaker label per dialog, not per screen, and the pilot adds no vendor behaviour. A Lo'rak dialog shown as a quest speaker does not open a shop. |
| Vendors ignore `max_stack_size` and have no faction gate | No vendor behaviour is added. |
| `repeats` not persisted | Not repeatable. |
| Event 5 plus `remove_item` consumes twice | No item event is used. |
| Non-interact dialogs are monologue only | C2 shows a monologue (every screen speaker id 0). C4 and C6 are Route A interaction chains. |
| Shared worlds 57 and 68 | No shared actor is mutated. No hostility, visibility or despawn change. |

## 6. Client catalog delivery path (CB-N05a)

What the client needs to show the journal entry and the dialogs. Source: [CB-N05a seal](../../four-zone-planning/cb-n05a-seal.md) and [mission pak overrides](../../../architecture/mission-pak-overrides.md).

1. **Mission catalog entry.** `MISSION_ADDITIONS` in `crates/resources/src/base/mission_additions.rs` is an empty const today. The pilot needs one `MissionAddition { mission_id, xml }` entry. FACT: CB-N05a.
2. **Admission limits.** The batch is validated as a whole. Root IDs 1 to 65535, nested IDs positive INT32, XML at most 355840 bytes. A bad entry rejects the batch. FACT.
3. **Delivery.** The catalog goes to the client by full category-3 resync on a metadata-version mismatch, as 0x36 resource fragments. FACT.
4. **Dialogs.** The client renders dialogs from its own cooked entries; the server sends only the dialog id. An authored dialog needs a Rust `DialogOverride` (full regeneration through `crates/resources/src/base/dialog_overrides/mod.rs`) in addition to the seed rows. Editing seed `dialog_screens` alone changes nothing the player sees. FACT.
5. **Dialog ID space.** The ID bound is 65535. The convention is 60100 to 60199, which is partly occupied (seed 60100 to 60104 and 60110; Cellblock claims 60150, 60151, 60160). The pilot's dialogs must be proved free by the collision procedure. FACT.
6. **Client behaviour is unproved.** gSOAP parsing, journal and dialog presentation, progression and relog are all UATPending until the first authored quest supplies live evidence. No authored quest is known to display, accept or advance in the client. FACT: [HA-00c section 9](HA-00c.md).
7. **Telemetry.** `cooked_data.mission_catalog_inserted` and `cooked_data.mission_catalog_rejected`. FACT.

## 7. ID needs by kind (no numbers)

| Kind | Count for the pilot | Notes |
|---|---|---|
| Mission | 1 | Root ID within 1 to 65535. Not in the 73 census IDs, the carry-ins 567, 1360, 1377, 1410, mission 1664, or any seed-only occupant. |
| Step | 2 | A and B. |
| Objective | 2 | A1 and B1. |
| Task | 2 | One per objective. |
| Dialog | 4 | Offer (zero-button), accept acknowledgement (monologue), sentry account, closing line. Each a `dialogs` row, `dialog_screens` rows and a `DialogOverride`. The collision procedure may fold the acknowledgement into the offer's follow-up if the owner prefers three. |
| Dialog-set map | 3 | Lo'rak offer, Lo'rak turn-in, sentry flag carrier (NULL dialog); each points at one of the three dialog sets below. The sentry carrier is interaction-only since #661. |
| Dialog set | 3 | Offer, turn-in, sentry. |
| Chain | 7 | C1 to C7. Plus the triggers, conditions and actions rows each chain owns. No candidate range in the ID proposals is addressed to HA-17a, so every chain ID is unassigned. FACT. |
| Item | 0 | |
| Entity template or spawn | 0 | Existing spawns only. |

## 8. Collision-check procedure (run before any allocation)

Owned by the coordinator under O14-04. The pilot's IDs are asked for only after every step passes.

1. **Stock catalog.** Check each candidate against the 1040 numeric entries of `CookedDataMissions.pak` (IDs 38 to 1826) and the ten seed-only occupants (689, 68900, 80622, 80623, 80641, 80688, 90622, 90623, 90641, 90688). A gap scan of the pak alone is not enough.
2. **Seed families.** Check `missions`, `mission_steps`, `mission_objectives`, `mission_tasks`, `dialogs`, `dialog_screens`, `dialog_set_maps`, `dialog_sets`, `content_chains` and the trigger, condition and action tables against the seed maxima recorded in HA-00c. The seed maxima are a ceiling to stay above, not a free range.
3. **Reserved and claimed.** Exclude the 73 census IDs, carry-ins 567, 1360, 1377, 1410, mission 1664, the Cellblock claims (missions 62050 and 62060, dialogs 60150, 60151, 60160, and their steps, objectives and tasks), the test reservation 61000 to 61003, and every Harset chain ID already taken.
4. **Active branches.** Grep every active worktree and branch snapshot (including uncommitted sibling worknotes) for each candidate. HA-00c named this as the part it could not cover. The validator at `.claude/reports/four-zone/validate-harset.ps1` is the starting tool.
5. **Live DB.** The static audit cannot see a live database. Compare against the current seed-loaded database before committing.
6. **Neighbour chain sweep.** List every chain in the seeds that fires on the pilot's trigger keys (`player_loaded Harset`, `interact_tag` on templates 201 and 205, `mission_abandoned`) and prove pairwise disjointness of persisted guards against the pilot chains. Include the Goa'uld chains for 1244, 1245, 1322 and the Loyalist chain for 1347.
7. **Dialog route check.** Confirm every dialog the pilot opens through a non-interact chain is a monologue, and every dialog opened through `interact_tag` has no follow-up `dialog_choice` that needs the target pin.
8. **Record.** Write the allocated IDs into the ID ledger with `CIMMERIA_AUTHORING`, then update the cards. The signature stays with the mission advisor and is not given by this file.

## 9. Owner-run client check script (journal, dialog, advance, relog, reward)

The owner or Lomiada runs this after the implementation exists. No agent runs it, and no lab is used here. A step passes only if the observed result matches. Visible first-press feedback is required at each click (project rule).

| # | Do | Pass if |
|---|---|---|
| 1 | Log in a Harset-eligible character on a fresh slot. Walk to Lo'rak. | Lo'rak shows an available-quest marker. |
| 2 | Click Lo'rak. | The offer dialog opens. Zero buttons: close it with the X. |
| 3 | Open the journal. | The pilot entry is present with step A text. (First authored catalog proof.) |
| 4 | Click Lo'rak again. | No second offer, no second accept. |
| 5 | Walk to the sentry. | The sentry shows the active marker. |
| 6 | Click the sentry. | The account dialog opens at once; the journal advances to step B. |
| 7 | Log out and log in. | Step B persists; Lo'rak shows the turn-in marker; the sentry marker is gone. |
| 8 | Hop to another world and back. | The same state repaints. |
| 9 | Abandon from the journal, then re-enter. | Lo'rak offers again. (Optional; covers C7.) |
| 10 | Re-accept, advance, return to Lo'rak, click. | The closing dialog shows, the mission completes, XP arrives (and currency if the cash arm exists). |
| 11 | Click Lo'rak again, log out and in. | No second reward. The mission stays completed. |
| 12 | Check the server log and telemetry. | `mission_catalog_inserted` present, no `mission_catalog_rejected`. |

If step 3 fails, the failure is the CB-N05a client proof, not this graph. Record it as BlockedEvidence and do not edit the chains.

## 10. Filled copy of card 1 (minus signatures and IDs)

| Field | Content |
|---|---|
| Graph | Authored side story 1 of 2, Jaffa: "Changing of the Watch" (PROVISIONAL). Branch and commit sealed as `harset/ha-17a-jaffa-stories`, `Harset HA-17a: author Jaffa side stories`. |
| Premise | Watch rotation. Lo'rak asks the player to learn from the Jaffa Zone sentry how the watch is kept, and report back. Lore-safe garrison business. INFERENCE, medium (lore), low (ordering). CIMMERIA_AUTHORING. |
| Mission, steps, objectives, tasks | One mission, not repeatable, visible. Step A (find the watch post, TalkToNpc on the sentry), step B (report to Lo'rak, turn-in objective closed by `complete_mission`). One task per objective. Rows are not authored. |
| Grant and recipient | Giver and closer: `Harset_Lorak`, spawn 302 / template 201 (world 57). Step actor: `Harset_SuspiciousJaffa`, spawn 305 / template 205 (world 57). Both are existing proved-seed spawns. FACT. The accepting player is any Harset-eligible character; any faction or archetype gate is open. |
| Reward | Modest XP through `grant_xp` (amount open, formula gated on GC3 / D-H10). Currency only if a cash chain arm is added (owner call). No item. PROVISIONAL. |
| Items needed | None. The HA-15 census lists 23 items, none for a Jaffa mission, and the pilot uses none. |
| Chains and IDs preserved | No existing chain, ID or template row is reused. 6301, 6303 to 6308, 6331 and 6333 to 6342 stay with 1324 and 1326. Seven new chains by role (section 4.2); authored IDs unassigned. |
| Upstream gate | Restored Jaffa arc client UAT (which includes unauthored 1325), and Cellblock's first authored quest client proof. Whether the Harset pilot may itself be the first authored client proof is an open owner question (O18 "what stays open"). |
| Evidence class | Shape, actors and engine behaviour: FACT. Premise ordering and lore fit: INFERENCE. Reward values: open. |
| Open decisions (Lomiada brief wording) | "Premise, reward and reset policy: Concrete values for six authored stories and three repeatables." "Sign-off of the nine mission graphs: Sign or reject each graph." Plus this card's own: accept-on-close wart; cash arm; shared-template 201 and 205 neighbours. |
| What the signer attests to | (a) The exact IDs are listed. (b) Each is absent from the stock pak, seeds and active snapshots, and none reuses a census ID, carry-in or 1664. (c) The objective primitives and reward actions exist as used (`grant_xp` yes; cash no). (d) The actors are real and no shared actor is mutated. (e) A persisted guard stops a second reward (step B active). (f) Premise, reward and provenance are recorded as `CIMMERIA_AUTHORING`. (g) Cellblock's first authored quest has a client proof. |
| Sign-readiness | NOT READY: no IDs, no approved reward, no upstream UAT, no client catalog proof. This design removes the "no graph" reason only. |
| Mission-advisor signature | (blank) |
| ID allocation | (blank) |

## 11. Open owner decisions

| # | Question | Default if silent |
|---|---|---|
| 1 | May the Harset pilot be the first authored client proof? | No; wait for Cellblock. |
| 2 | Accept the zero-button accept-on-close wart? | Yes, as Harset and Tollana already do. |
| 3 | Approve a cash chain action (new executor arm), or pay XP only? | XP only. |
| 4 | XP amount, once GC3 / D-H10 is answered. | Unset. |
| 5 | Swap watch rotation and supply routing between stories 1 and 2? | Keep as drawn. |
| 6 | Any faction or archetype gate on the offer? | None beyond the Harset eligibility the zone already applies. |

## 12. Tests the implementation owes

- A content-engine test that C2 accepts only from `not_active` and a second dialog close does not re-accept.
- A test that C4 advances from step A to step B once, and that `advance_step` is used, not `complete_objective`.
- A test that C6 grants XP once, that a second click grants nothing, and that reverting the `step_status` guard makes the test fail.
- A test that C1, C3 and C5 are pairwise disjoint on persisted guards, and that no chain on the pilot's trigger keys co-fires with the neighbour missions on templates 201 and 205.
- A restore test for relog and world hop in each state (offer, step A, step B, completed).
- A catalog admission test: the pilot's `MissionAddition` is admitted, and an insertion-only revert fails a runtime assertion (CB-N05a gate).
- A dialog test that every non-interact dialog is a monologue and the offer dialog has zero buttons.
- A seed lint pass: no ID collision, `CIMMERIA_AUTHORING` marker present, no player-facing provenance text.

## 13. Telemetry the implementation owes

- `cooked_data.mission_catalog_inserted` and `cooked_data.mission_catalog_rejected` for the pilot entry.
- Mission lifecycle spans or events for accept, advance, complete and abandon, carrying mission id and step id, so a stuck player is debuggable from SigNoz alone.
- A reward event per grant carrying mission id and amount, so a double reward shows as two events.
- A friction signal if the offer bind is repainted but the player's mission state makes it unreachable.
