# DW-26A worknote — 2026-10-16

The third patch-012 slice now has 27 typed disabled forms from
`/gmreloadinventory` through `/debugminigamecomplete`. Each form includes the
exact TOML word, event and argument order, handler availability, target scope,
impact when a handler exists, method-specific readback limits, compensation and
a source-linked unsupported reason. The forms remain standalone until DW-27B
merges all six slices into the catalog. No native line builder or slash-to-dot
execution path was added.

The non-`/gm` debug word is still server-GM-gated at candidate index 191 and
has no handler. Its one `ResultCode` argument is not a ticketed SmartFox game
outcome. The `/gmrechargeitem` GM method is also unhandled; paid vendor recharge
is separate. Despawn, kill and mob attribute forms explain their patch/server
signature mismatches, and spawn's candidate binding remains semantic only.

All TOML String arguments are flagged sensitive and audit previews redact their
values. Typed Boolean, integer and finite float previews do not build slash
lines. Source-drift tests mutate names, events, access and argument shapes and
require fail-closed rejection. The packet has no rendered frontend; JS REPL
logic UAT and visual UAT are deferred to the companion-window packet. No live
client or lab was used.

Checks: `cargo fmt -p cimmeria-lab` status=ok;
`cargo clippy -p cimmeria-lab --all-targets -- -D warnings` status=ok;
`cargo test -p cimmeria-lab` status=ok (1,564 passed).
