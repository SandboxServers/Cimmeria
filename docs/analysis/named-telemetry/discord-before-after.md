# Discord Notifications: Before and After Named Telemetry

> Type: reference (announcement text). Audience: the restoration team, who read Discord rather than SigNoz. A human may paste the section below into Discord as is.
> Updated: 2026-10-05 (NT-50b). Companions: [campaign ledger](README.md), [discord-notifications.md](../../architecture/discord-notifications.md), [Rule 6](../../architecture/instrumentation-discipline.md#discord). Sources: the before/after examples in PR #1208 (NT-10, typed events) and PR #1199 (NT-11, warnings and errors).

## Message for Discord

**Discord notifications now name things.** Every player, NPC, item, mission, ability and world in a server notification now shows its name with its number in brackets, `Name (#id)`, so you no longer have to look a number up. The examples below use made-up characters; the mission and item are real seed entries.

**Mission completed**

```text
Before:  ✅ Mission completed: #1003      Character: alice
After:   ✅ Mission completed: Question Saarthon (#1003)      Character: Alice (#12)
```

**NPC killed**

```text
Before:  ☠️ NPC killed: Jaffa Guard      Killer: alice      World: Castle_CellBlock
After:   ☠️ NPC killed: Jaffa Guard (#9001)      Template: Demon Jaffa (#87)
         Killer: Alice (#12)      World: Castle_CellBlock (#4)
```

**Item used**

```text
Before:  🧪 Item used: type 2133      Character: alice
After:   🧪 Item used: Med Kit (#2133)      Character: Alice (#12)
```

**A warning from the server** (an ability refused because the target was out of range)

```text
Before (12 fields, including a link most of us can't open):
  player_id: 100 | player_name: Alice | account_id: 6 | account_name: steve
  ability_id: 880 | ability_name: Staff Blast | target: 4123 | target_name: Jaffa Guard
  space_id: 12 | world: Castle_CellBlock | reason: out_of_range
  trace_id: <link to the developers' log server>

After (6 fields):
  Who: Alice (#100) · steve (#6)
  ability: Staff Blast (#880) | target: Jaffa Guard (#4123) | space: Castle_CellBlock (#12)
  reason: out_of_range
```

Two more things changed:

- **No more links you can't open.** Notifications never link to the developers' log server or any other private host. Each message makes sense on its own.
- **A bare `#id` means a missing name.** If you see a number with no name, such as `#880`, the server couldn't find a name for it. That usually means a gap in the game data, so mention it to a developer.
