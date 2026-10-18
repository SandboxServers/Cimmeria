# DW-24B worknote

The old `client_console` RPC accepted arbitrary caller-supplied addresses and
passed a raw string pointer. Its known `0x00539850` entry takes an `FFrame`,
and no string-taking ABI is verified. The endpoint now refuses every native
call until a fingerprinted address and ABI are established.

The hub-pass UAT ledger has one independent server receipt for
`/gmsendgmshout 1 HPUshout1` through stock `ChatMod.onTextAccepted`. A bounded
candidate accepts only that plain three-token shape for offline planning.
Its stock UI adapter builds an atomic input set/readback/submit Lua call, but
there is no callable submission method until the supervisor can derive and
recheck its own process fingerprint and generation. The receipt model keeps
client submission, acknowledgement, outgoing method and server receipt as
separate unknown states. Caller-built fingerprint/generation/ack/server rows
cannot promote them; a private cross-source binding gate remains false
because the native method does not carry the local attempt ID. Full
same-line concurrent attribution and provenance tests are deferred until a
trusted producer exists. Every patch-table descriptor remains disabled.

`crates/lab/src/supervisor/mod.rs` was already 644 lines; the required module
registration made it 645. Splitting this established host is outside DW-24B.

**Status: BlockedDependency.** The installed client binary and a separately
approved lab probe are needed to prove fingerprint, tokenizer behavior for
spaces/quotes/backslashes and optional values, and independently correlated
acknowledgement/outgoing/server receipts. No lab lease or client was used.
