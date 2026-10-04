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

**Exception, 2026-09-29: small UI files ship whole.** A delta only applies to the exact stock file, so it fails on the clients many players already have (Project Giza, or a colo client with the portrait fix installed by hand), and before #1114 that one failure stopped every later patch. `008-dialog-portraits` therefore carries its five small UI files whole, as the Black Market overlay (`crates/client-patches/overlay/`) already does. Large binaries (maps, cooked-data PAKs) stay deltas. This was a maintainer-side call that accepts the distribution risk for small UI files.

## The patches

| Id | What it changes | Files | Zip |
|---|---|---|---|
| `001-dialog-portraits` | **Superseded by 008; remove it from the manifest.** Speaker portraits in NPC dialog windows (`TaharezLook.scheme`, `Dialog.layout/.lua`, `Blurb.layout/.lua`) | 5 deltas | 4.7 KB |
| `002-castle-ring-transport` | **Retired, superseded by 007 (see below).** The ring station on the CellBlock stasis-hall pad and the ring rig on the Armory pad (mission 688), built by `upk_patch` | 2 map deltas against the normalized stock maps | 4.4 KB |
| `003-cooked-data` | The merged Kismet sequence, Kismet set-event and interaction-set PAKs the server loads from `data/cache/`, plus its `CookedBehaviorEvents.pak` | 3 deltas + 1 file | 86 KB |
| `004-log-config` | `SGWLogConfig.xml`, so `SGW.exe` writes `SGWDebugLog.log` (log upload and telemetry read it) | 1 file, ours | 1 KB |
| `005-login-delay` | `eula.lua` waits 19 s before showing the login screen, so the gate-dialing animation finishes first | 1 delta | 1.5 KB |
| `006-gate-sound-bank` | Copies the stock `audio/genprp/prp_gen.fev` and `prp_gen_gate.fsb` into `Audio/UI/`, where the known-good client has them (byte-identical); the sources are the player's own stock files | 2 near-empty deltas | 1.5 KB |
| `007-castle-armory-ring` | The ring rig on the CellBlock Armory pad (mission 688). This is 002's Armory op with the same source, delta and result, so installs that applied 002 skip it | 1 map delta against the normalized stock map | 2.6 KB |
| `008-dialog-portraits` | **Supersedes 001.** The same five dialog-portrait files, shipped whole instead of as deltas, so it applies to any client: stock, Project Giza, or one that already carries a hand-installed portrait fix | 5 whole files | 93 KB |
| `009-starter-hotbar` | Puts a new character's starting abilities on its action bar at first login (see below) | 1 delta (`ActionProfileDefault1.lua`) | 4.1 KB |

`002-castle-ring-transport` was **removed from the signed content
manifest on 2026-09-29**, and `007-castle-armory-ring` supersedes it.
The maintainer decided to keep the Armory ring rig and drop the
stasis-hall ring station, so 007 repeats only 002's Armory op. 002's
spec and zip stay here unchanged (append-only, below).

Installs that applied 002 while it was in the manifest keep its
stasis-hall map (`Castle_CellBlock-fffdfffc.umap`, with the ring
station) until the launcher can restore a stock file from the player's
own seed. That restore is follow-up work and has not been built. A delta
can't do the restore: 002 wrote that map uncompressed, and the stock map
is LZO-compressed. A delta from 002's map back to the stock bytes is
389 KB, and 383 KB of that is stock map bytes stored verbatim, which the
no-CME-bytes rule forbids. Fresh installs never get the station.

Applied to the stock client, the rebuilt files of 001-008 are
byte-identical to a known-good QA client's.

### 009-starter-hotbar

The action bar is client state (`GActionProfiles`, saved per character in
`ActionButtons - Saved Vars.lua`), so the server can grant abilities but
cannot put them on the bar, and a new character starts with 100 empty
buttons. 009 appends [StarterHotbar.lua](009-starter-hotbar/StarterHotbar.lua)
to the stock `ActionProfileDefault1.lua`, byte for byte. The ActionButtons
module loads that file after `ActionProfiles.lua`.

- **What it does.** On the profile the client creates at a character's
  first login (when `GActionProfiles` is empty), it binds each known
  starting ability (Pistol Shot 592, Strike 594, Heal Focus 597, Health
  Heal 1646, Recuperation 1218, in that order) to the next empty
  layer-bound button from 11 to 20 (default keys Alt+1 to Alt+0), using the
  same calls as dropping an ability from the Ability window. It reads the
  known abilities with the native `getAbilityList(UIAbilityGroup.KnownAbility)`
  and tops up on `Events.AbilityUpdate` (and, throttled, on
  `Events.PropertyUpdated`) until the first session ends. The profile then
  records `cimmeriaStarterHotbar = 'done'` and is never touched again. A
  profile that existed before the patch, or one made with the editor's
  New Profile button, has no such mark and is left alone.
- **Compatible with UI packs.** It never creates, moves or resizes a
  button, and it patches no file that the WQHD v26 UI pack replaces
  (`ActionProfiles.lua`, `ActionButton.layout`). It wraps
  `ActionProfileMod.createProfile` and `ActionProfileMod.loadProfile`,
  which the stock file and the v26 replacement both define.
- **Rebuilding.** Build a patched tree whose `ActionProfileDefault1.lua` is
  the stock file (sha256 `a09eb055...`) followed by `StarterHotbar.lua`, then
  run `cimmeria-patchset build` as below. `starter_hotbar_tests.rs` in
  `crates/patchset` fails when the hook changes without a rebuilt zip.
- **Logic UAT.** `lua5.1 data/client-patches/009-starter-hotbar/test/run.lua`,
  or `python .../test/run_lupa.py` on Windows. CI runs it against a
  clean-room model of the ActionButtons module. Set `SGW_UI_DIR` to a stock
  client's `Working/SGWGame/Content/UI`, and `SGW_V26_ACTIONPROFILES` to
  the v26 pack's `ActionProfiles.lua`, to also run it against the real
  scripts.

Each spec carries a `title` and `description` for the launcher's
**Changes to your client** list. `cimmeria-patchset build` copies them
into the manifest entry; they are not in the zip, so adding them to a
published patch changes nothing a player downloads. The launcher keeps
the same text as a fallback for manifests published without it
(`builtin_description` in `crates/launcher/src/client_changes.rs`), and
the test `builtin_catalog_matches_every_patch_spec` keeps the two in step.

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

**Spec paths use the stock spelling.** Every `target`, source `path` and
`files` path must be spelled exactly as the stock client spells it (the
2009 cabinets' `Data\DATA.INF` file list is the reference), down to the
case: the game looks some files up case-sensitively even on Windows.
`005-login-delay` shipped with `eula.lua` for the stock `EULA.lua`, and
the launcher that applied it renamed the file, so the game never showed
its login screen. `cimmeria-patchset build` now refuses a path the stock
tree spells in another case, `apply` keeps whatever name the file
already has on disk, the launcher renames `eula.lua` back on existing
installs, and the launcher test `every_patch_target_is_listed` checks
every op target against its stock-spelled `PATCH_TARGETS` list.

The specs for `004-log-config` (`Working/binaries`), `005-login-delay`
(`EULA.lua`) and `006-gate-sound-bank` (`Content/audio/ui`) were
corrected to the stock spelling on 2026-09-29, after their zips were
published. The published zips keep the old spelling, which `apply` now
resolves to the files on disk, so they install correctly and stay as
they are (append-only, below). A rebuild from a corrected spec gives
different bytes; publish it under a new patch id, if ever.

Once published, a patch is append-only: fix it with a new patch id, never
by rebuilding a published zip (see the launcher guide's
[append-only invariants](../../docs/client/launcher-guide.md#append-only-invariants)).
