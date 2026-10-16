# DW-24A stock slash audit

## Boundary

The patch 012 TOML is the maintained source for its 162 words: 161 `/gm*` and
`/debugminigamecomplete`. Its generated `InternalSlashCommands.xml` is checked
by `tools/client-patches/gm_slash_commands.py --check` and is not independent
stock evidence. The Rust catalog keeps those forms disabled with
`native submission unproved`.

This follow-up inspected the two stock XML files named by DW-24A in an
operator-supplied client copy. The identical QA copy was independently hashed.
These sources are outside the public repository; the client files were not
copied into it. Counts below are parsed `<scmd:Command>` elements, excluding
the commented template.

| Relative client source | SHA-256 | Commands | Explicit `Access` |
|---|---|---:|---|
| `Client/game/Common/xml/slash_commands/SlashCommands.xml` | `6e34355a628bd3b5f95e31ef58e7a2e9700dcce6b9798c751bda2431ba0dcd66` | 98 | `/reload`: `qd` |
| `Client/game/Common/xml/slash_commands/FinalSlashCommands.xml` | `7d166f89d3b6829d2fe6c891c90104c5562d1de653864b10d7dd06fbf97263db` | 4 | None |

`Stargate Worlds-QA/Common/xml/slash_commands/` has the same hash for each
respective file. The stock inventory state in
`crates/lab/src/workbench/fixtures/stock_slash_addendum.json` is `complete`
*for these two files*: all 102 command elements and their `Access` attributes
were inspected. There are zero **verified GM-gated extras**. That does not
establish that every stock command has a server binding or that the active
client loads exactly these files.

## Unresolved role-marked candidate and exclusions

`/reload` is the sole row in these files with an explicit `Access` attribute.
It declares `Access="qd"`, `Event_SlashCmd_Reload`, and one required `Integer`
parameter named `reloadType`. Its description says it reloads the active
weapon (`0`) or deployment bar (`1`). The role legend in
`FinalSlashCommands.xml` defines `q` as QA and `d` as Designer. How the
client evaluates the two-character mask is unverified. This is an explicitly
role-marked *client* command candidate. The XML does not prove
that either role maps to Cimmeria GM authorization, that the event reaches a
server method, or that the native submit path works. Consequently `/reload`
is excluded from the **verified GM** addendum and remains unresolved for
DW-27C. `docs/commands.md` calls `/reload` a general data reload; that guide
description conflicts with the inspected stock XML and must not be used as
the palette's effect description.

`/debugperformance` and `/worldreset` occur in `SlashCommands.xml` without
`Access`. The former has `Event_SlashCmd_DebugPerformance`, but no verified
GM gate or server binding. The latter has
`Event_SlashCmd_WorldInstanceReset`; the dispatch table separately identifies
GM-gated inherited method 92 `onWorldInstanceReset`, but the slash-event
binding is unproved. Neither is a verified GM addendum word. No lack of an
`Access` attribute is interpreted as an authorization decision.

DW-27C must resolve the `/reload` authorization/binding question before
claiming complete GM command coverage. DW-24B still owns native submission,
argument formatting, and received-result proof. This audit did not launch the
client or submit a command.

## Existing patch method candidates

The dispatch table's indices and DONE/ADAPT/NEW statuses supply candidates,
not proven event bindings. Its explicit `/gmsetfly` and `/gmsetghost` rows
point to `onPhysics` (221), and `/gmsendgmshout` to `sendGMShout` (222). It names
stock `/GiveNaqahdah` → `gmGiveCash` (134) and `/Spawn` → `gmSpawnByCmd` (185);
patch 012 instead has `/gmgivecash` with `Event_SlashCmd_GiveNaqahdah` and
`/gmspawnbycmd` with `Event_SlashCmd_Spawn`. Those patch-word candidates are
semantic inferences. Every candidate remains `binding_verified=false`.

`docs/commands.md` shows three `/debugminigamecomplete` arguments (`gameId`,
`winnerId`, `loserId`), while patch 012 declares one mandatory `Integer`
`ResultCode`. The inventory follows the patch TOML; DW-24B must reconcile the
native parser before a form can be enabled.
