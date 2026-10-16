# DW-23B-E worknote

Source-only audit of the 14 `registry/commands/net_debug.rs` specs is in `evidence/DW-23B-E.md`. No network command or lab action was invoked.

The next metadata packet should keep the 14 names covered but disabled/hidden as recorded. Key gates: accepted raw args reach Discord GM audit; all three sequence forms need being-only destination/target validation; `.net_timer` requires an unrelated selection; `.net_mapinfo` wraps negative signed values onto unsigned wire fields; `.net_dialog` mutates the offered-dialog authorization set before best-effort client send and can evict an unanswered offer; `.net_challenge` returns to an unimplemented response handler; `.debug_follow` has no expiry; exact server readback is missing for threat, aggression override and caller aggro switch. `debug_controller` is an audited no-op. A command feedback line is not a wire or client-applied receipt.

Check: exact 14-heading set matched the source `SPECS`; positive `.aggro off` and refused `.aggro maybe`/`.aggression 9` branches inspected in source and tests; `git diff --check` passed. Runtime tests were not run because this packet changes only evidence documentation. Network-security and server-authority review remains for the coordinator.
