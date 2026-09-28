# Client patches

The client-side changes Cimmeria ships to players, as launcher patch
sets. Each directory holds a `patch.json` spec (and any files that are
entirely ours); the `.zip` next to it is the built patch the launcher
downloads.

## The rule: no CME bytes

The project never hosts CME's files. The launcher installs the stock 2009
client from archive.org, and every change to a stock file ships as a
bsdiff delta that the launcher applies to the player's own copy. A patch
zip holds a `cimmeria-patch.json` recipe, the deltas, and files we wrote
ourselves. See [docs/client/sgw-launcher.md](../../docs/client/sgw-launcher.md#patch-sets-cimmeria-patchset)
for the format and [crates/patchset](../../crates/patchset/) for the code.

## The patches

| Id | What it changes | Files | Zip |
|---|---|---|---|
| `001-dialog-portraits` | Speaker portraits in NPC dialog windows (`TaharezLook.scheme`, `Dialog.layout/.lua`, `Blurb.layout/.lua`) | 5 deltas | 4.7 KB |
| `002-castle-ring-transport` | The ring station on the CellBlock stasis-hall pad and the ring rig on the Armory pad (mission 688), built by `upk_patch` | 2 map deltas against the normalized stock maps | 4.4 KB |
| `003-cooked-data` | The merged Kismet sequence, Kismet set-event and interaction-set PAKs the server loads from `data/cache/`, plus its `CookedBehaviorEvents.pak` | 3 deltas + 1 file | 86 KB |
| `004-log-config` | `SGWLogConfig.xml`, so `SGW.exe` writes `SGWDebugLog.log` (log upload and telemetry read it) | 1 file, ours | 1 KB |

Applied to the stock client, the rebuilt files are byte-identical to a
known-good QA client's.

Not here, on purpose:

- **The 2008 Agnos content** (`Maps\Agnos`, `Maps\Agnos_Library` and the
  packages that came with them). Agnos is a seeded world that missions send
  players to, so a client without these maps can't load it. They are whole
  CME files, which a delta against the stock client can't carry, so they
  can't ship as a patch set. The open question is sourcing them from an
  archived build on archive.org rather than hosting them.
- **The historical CellBlock worlds**: GM-only destinations, installed by
  hand.
- **`LoginInternal.lua` and ASLR**: the launcher writes those itself on every
  install and launch.

## Rebuilding a patch

You need a stock client (the seed unpacked, with
`Working\SGWGame\Cache.en-US` renamed to `SourceCache.en-us`, which is what
the launcher leaves behind) and a client with the change:

```bash
cargo run -p cimmeria-patchset -- build data/client-patches/<id>/patch.json \
  --stock <stock client> --patched <patched client> \
  --out data/client-patches/<id>.zip \
  --blob-url https://raw.githubusercontent.com/SandboxServers/Cimmeria/<commit>/data/client-patches/<id>.zip
```

Builds are reproducible: an unchanged spec and unchanged inputs give the
same zip bytes. The command prints the manifest entry; the `sha256` must
match the committed zip. To try a patch on a copy of a client:
`cargo run -p cimmeria-patchset -- apply data/client-patches/<id>.zip --install <client>`.

Once published, a patch is append-only: fix it with a new patch id, never
by rebuilding a published zip (see the launcher guide's
[append-only invariants](../../docs/client/launcher-guide.md#append-only-invariants)).
