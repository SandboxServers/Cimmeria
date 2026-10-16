# DW-24B worknote

The old `client_console` RPC accepted arbitrary caller-supplied addresses and
passed a raw string pointer. Its known `0x00539850` entry takes an `FFrame`,
and no string-taking ABI is verified. The endpoint now refuses every native
call until a fingerprinted address and ABI are established.

The hub-pass UAT ledger has one independent server receipt for
`/gmsendgmshout 1 HPUshout1` through stock `ChatMod.onTextAccepted`. A bounded
candidate accepts only that plain three-token shape for offline planning.
Its stock UI adapter builds an atomic input set/readback/submit Lua call, but
the absent verified client fingerprint stops it before any bridge mutation.
The receipt model separates client submission, acknowledgement, outgoing
method and server receipt, with exact attempt/generation matching. A private
cross-source binding gate remains false because the native method does not
carry the local attempt ID; synthetic observations never produce acceptance. Every
patch-table descriptor remains disabled.

**Status: BlockedDependency.** The installed client binary and a separately
approved lab probe are needed to prove fingerprint, tokenizer behavior for
spaces/quotes/backslashes and optional values, and independently correlated
acknowledgement/outgoing/server receipts. No lab lease or client was used.
