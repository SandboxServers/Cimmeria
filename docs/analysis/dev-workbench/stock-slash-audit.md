# DW-24A stock slash audit

The patch 012 TOML is the maintained source for its 162 words: 161 `/gm*` and
`/debugminigamecomplete`. The generated `InternalSlashCommands.xml` is checked
against that TOML by `tools/client-patches/gm_slash_commands.py --check`; it is
not independent evidence for other stock commands. The Rust inventory retains
each event, declared access string, and ordered `Boolean`, `Integer`, `Float`
or `String` mandatory and optional parameters. All projected forms are
`Unsupported` with reason `native submission unproved`.

`docs/commands.md` was inspected for extra documented command words.
`/debugperformance` and `/worldreset` appear outside the GM section and patch
table. The former has a client event in reverse-engineering notes, but no
stock XML `Access` attribute or GM method binding here. The latter has a
GM-gated inherited method, `onWorldInstanceReset` (92), but its stock XML
`Access` attribute and slash-event binding are unknown. Both remain unresolved
stock candidates, excluded only from the *verified GM* addendum. Their
presence in the client and eligibility for a GM palette are not established.
Existing GM command guide rows are already in patch 012.

The checkout does not contain stock
`Common/xml/slash_commands/SlashCommands.xml` or
`Common/xml/slash_commands/FinalSlashCommands.xml`. The stock inventory is
therefore **unknown**, with zero *verified* extras. That is not a complete
empty addendum. The machine-readable snapshot is
`crates/lab/src/workbench/fixtures/stock_slash_addendum.json`.
DW-27C must obtain inspectable stock files, audit their command words and
access masks, and add descriptors for every verified GM extra before DW-32
can assert complete slash coverage. No client launch or native submission
occurred in DW-24A.

The dispatch table's explicit indices and DONE/ADAPT/NEW statuses supply
method candidates. Event-suffix name matches are direct candidates. The table
explicitly names `/gmsetfly` and `/gmsetghost` → `onPhysics` (221), and
`/gmsendgmshout` → `sendGMShout` (222). It names stock `/GiveNaqahdah` →
`gmGiveCash` (134) and `/Spawn` → `gmSpawnByCmd` (185); patch 012 instead
declares `/gmgivecash` with `Event_SlashCmd_GiveNaqahdah` and `/gmspawnbycmd`
with `Event_SlashCmd_Spawn`. Those two patch-word candidates are semantic
inferences, not explicit dispatch-table aliases. Every candidate remains
`binding_verified=false`; none proves the patch event's client binding.

The guide at `docs/commands.md` shows three arguments for
`/debugminigamecomplete` (`gameId`, `winnerId`, `loserId`), while patch 012
declares one mandatory `Integer ResultCode` for that word. The inventory
follows the maintained patch TOML. DW-24B must reconcile this guide drift
against the native parser before enabling a form; no event binding or result
semantics were inferred here.
