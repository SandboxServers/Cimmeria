# DW-24A worknote

Parsed patch 012 TOML into ordered typed slash parameters and 162 disabled
workbench descriptors. A roster digest pins the exact command-name set; unit
tests cover mandatory/optional type and order, duplicate/malformed rejection,
disabled execution state, method candidate uncertainty, and the explicit
unknown stock fixture.

The stock files named by the client loader are absent from this checkout.
`docs/commands.md` adds no verified GM word. `/debugperformance` lacks a stock
access mask; `/worldreset` has GM-gated method 92 but no stock `Access` or
slash binding evidence. Method candidates include exact-name matches, three
explicit slash-word mappings, and two semantic inferences from patch event
names and stock command rows. Native submission, client outcome, all patch
event bindings and complete stock coverage remain unproved.
`docs/commands.md` describes three
`/debugminigamecomplete` arguments, but patch 012 declares only one mandatory
`Integer ResultCode`; the palette inventory follows the TOML and native
parser reconciliation remains for DW-24B. No lab client was used.
