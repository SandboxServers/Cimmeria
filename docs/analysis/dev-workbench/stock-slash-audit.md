# DW-24A stock slash audit

The patch 012 TOML is the maintained source for its 162 words: 161 `/gm*` and
`/debugminigamecomplete`. The generated `InternalSlashCommands.xml` is checked
against that TOML by `tools/client-patches/gm_slash_commands.py --check`; it is
not independent evidence for other stock commands. The Rust inventory retains
each event, declared access string, and ordered `Boolean`, `Integer`, `Float`
or `String` mandatory and optional parameters. All projected forms are
`Unsupported` with reason `native submission unproved`.

`docs/commands.md` was inspected for extra documented command words. Its
`/debugperformance` row is outside the GM command section and the patch table.
The client event appears in reverse-engineering notes, but neither the stock
XML `Access` attribute nor a GM method binding is available here. It is an
exclusion from the *verified GM* addendum, not evidence that the word is
unavailable in the client. Existing GM command guide rows are already in
patch 012.

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

The dispatch table's explicit indices and DONE/ADAPT/NEW statuses are used
only when an event suffix exactly matches a method name. Such matches are
reported as `binding_verified=false`; the protocol note says some slash
events bind by different names and require client or packet evidence.
