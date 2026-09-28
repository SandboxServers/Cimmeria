# Placing NPCs and objects

This guide is for content authors who place NPCs and objects on our server
from inside the game: spawn them, move them, turn them, remove them, and save
the result so it ends up in the game for good. It ends with a section for the
developer who merges your work.

You need a game account with **GM access** on the server. Ask the server owner
if you don't have it. Everything below is typed into the normal chat box. Lines
that start with a `.` are GM commands: the server reads them, and nobody else
sees them in chat.

## How saving works

Read this once before you start.

1. **Everything you do shows up at once**, for you and for every player nearby.
2. **Nothing is saved until you type `.seedconfirm`.** Until then your changes
   wait in a queue. `.seedpending` shows how many are waiting, and `.seedcancel`
   throws the queue away.
3. **`.seedconfirm` saves to the running server** and sends your changes to
   our log system as a **batch**. It answers with a batch id, such as
   `Sent for merge as batch 12-1790553600123`. Your changes now survive logouts
   and server restarts.
4. **A developer has to merge the batch** into the game's data files, or the
   next server update wipes it. Send them the batch id.

In short: place things, save each one, confirm when you're happy, and send
the batch id to a developer.

## Selecting what you work on

Most commands act on your **selected target**: left-click the NPC or object so
its name shows in your target window. Commands that need a target tell you if
you have none selected.

Objects (consoles, crates, doors and so on) are handled exactly like NPCs:
everything in this guide works on them too.

## Recipes

### Move an existing NPC

1. Walk to where the NPC should stand.
2. Turn your character to face the way the NPC should face.
3. Select the NPC and type `.movehere`.
4. Type `.savespawn`.

The NPC jumps to your spot and turns to match you. The server also makes this
its new home, so it won't walk back to where it used to stand.

### Turn an NPC to face you

1. Stand where the NPC should look, for example the doorway players come in by.
2. Select the NPC and type `.lookat`.
3. Type `.savespawn`.

A heading of exactly 0 usually means someone forgot this step, and the NPC
faces a wall.

### Add a new NPC or object

1. Find its template id: `.searchtemplate <part of the name>`, for example
   `.searchtemplate jaffa`.
2. Stand where it should go, facing the way it should face.
3. Type `.spawn <templateId>`. It appears on your spot, facing your way, and
   you get `spawned npc <id>`.
4. Select it and type `.savespawn`.

### Remove an NPC or object

Select it and type `.delspawn`. It disappears at once and its removal is
queued for `.seedconfirm`.

`.despawn` is different: it removes the NPC until the next server restart and
saves nothing.

### Finish a session

1. Type `.seedpending` to check what's waiting.
2. Type `.seedconfirm`.
3. Send the developer the batch id from the reply, and a line about what you
   changed ("Harset plaza: moved the four gate guards, added a merchant").

## Save automatically

Type `.autosavespawn 1` and every `.movehere`, `.lookat`, `.location` and
`.rotation` on an NPC saves it for you, so you can skip the `.savespawn` step.
You still need `.seedconfirm` at the end. `.autosavespawn 0` turns it off. It
lasts until you log out.

## Command reference

| Command | What it does |
|---|---|
| `.movehere` | Moves the selected NPC or object to where you stand, facing the way you face. Doesn't work on players (use `.summon`). |
| `.lookat` | Turns the selected NPC to face you. |
| `.location` | Shows the selected target's position. `.location x y z` moves it there. |
| `.rotation` | Shows the selected target's facing, in degrees too. `.rotation 0 <yaw> 0` sets it; the yaw is in radians. |
| `.spawn <templateId>` | Creates a new NPC or object on your spot, facing your way. |
| `.savespawn` | Queues the selected NPC's position, facing and tag for saving. |
| `.delspawn` | Removes the selected NPC now and queues deleting it. |
| `.despawn` | Removes the selected NPC until the next restart. Saves nothing. |
| `.autosavespawn 1` / `0` | Turns automatic saving after moves and turns on or off. |
| `.seedpending` | Shows how many changes are waiting. |
| `.seedconfirm` | Saves everything waiting and gives you the batch id. |
| `.seedcancel` | Throws away everything waiting. |
| `.tag <name>` | Gives the selected NPC a content tag that missions refer to. `.savespawn` saves it. |
| `.searchtemplate <text>` | Finds template ids by name. |
| `.help <word>` | Lists commands matching the word. |

## Things to know

- **Save as often as you like; confirm once.** Saving the same NPC twice
  before you confirm keeps only the latest save.
- **After you confirm a new NPC, you can't save it again** until the server
  restarts, because the server doesn't know its row number yet. The server
  tells you if you try. If it's in the wrong place, `.despawn` it, spawn a new
  one, place and save that, and tell the developer to drop the first row for
  that entity.
- **`.seedcancel` doesn't undo what you see.** Moved NPCs stay moved and removed
  ones stay gone until the server restarts. Only the saving is cancelled.
- **Only position, facing and tag are saved.** Name, appearance and dialog
  changes from other `.` commands are for testing and are not saved.
- **Stand on the ground.** `.movehere` uses your exact position, so an NPC
  placed while you're jumping or standing on a crate floats there.
- **Patrol paths** (`.path_add`, `.path_assign`, …) use the same queue and the
  same `.seedconfirm`; see [the command list](../commands.md#dev-console--commands).

## For developers: merging a batch

`.seedconfirm` writes the changes to the live database and emits them to
SigNoz on the `authoring` tracing target. Every event of one confirm carries
the same `batch` attribute, along with the author's `account_id` and
`player_id`.

1. In SigNoz Logs, filter `batch = '<id>'`.
2. Each `seed spawn confirmed` event is one spawnlist change, with the columns
   as attributes: `op` (`insert`, `update` or `delete`), `spawn_id` (absent
   for an insert), `world`, `world_id`, `template_id`, `x`, `y`, `z`,
   `heading` (radians), `heading_deg`, `tag`, `npc_entity_id`, and `sql`, the
   statement that was run live. The float attributes are widened to f64, so
   copy numbers from `sql`, which prints them exactly as stored.
3. Each `seed authoring confirmed` event is the whole batch for one seed file,
   in order.
4. Apply them to `db/resources/Worlds/Seed/spawnlist.sql`:
   - **update**: edit `x`, `y`, `z`, `heading` and `tag` on the existing
     `INSERT` row for that `spawn_id`, and leave its other columns alone.
   - **insert**: add an `INSERT INTO spawnlist (spawn_id, x, y, z, heading,
     world_id, template_id, tag, set_name) VALUES (…)` row with a `spawn_id`
     from a reserved block, next to the world's other rows. If you reserve a
     new block above the current floor, raise the floor in the file's
     `setval` footer too. The event's own `sql` (an `INSERT … SELECT` with no
     `spawn_id`) also works if appended after that footer, but then the id is
     whatever the sequence gives it, so use an explicit id for any spawn a
     content chain refers to.
   - **delete**: remove that `spawn_id`'s row, and check nothing else refers
     to it.
5. If an author confirmed the same new NPC twice (see "Things to know"), keep
   only the row the author tells you to.

If SigNoz is unavailable, the same statements are in
`logs/seed-authoring-<session>.sql` on the server host, with each confirmed
batch marked `CONFIRMED batch <id>`.

The code is in `crates/cell-console/src/cell/console/seed.rs` (the queue and
the events) and `spawn/authoring.rs` (`.savespawn` and `.delspawn`).
