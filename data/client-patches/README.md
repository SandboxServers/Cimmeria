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
| `009-starter-hotbar` | Puts a new character's starting abilities on its action bar at first login (see below) | 1 delta (`ActionProfileDefault1.lua`) | 4.7 KB |
| `010-debug-area-rings` | **Retired 2026-10-05, superseded by 011: it hangs the client.** Eight ring transport rigs for the Debug Area, on the Ihpet_Crater_Light map (see below). | 1 map delta against the normalized stock map, with 007's Armory map as donor | 18 KB |
| `011-debug-area-rings-fix` | **Supersedes 010.** The same eight rigs, rebuilt so the client loads them, with the arena station moved off the pit's water plane (see below). Needs 007 applied first, unless 010 already applied | 1 map op: a delta from the normalized stock map plus 007's Armory map, and an alternative delta from 010's output | 19 KB |
| `012-gm-slash-commands` | The `/gm` slash commands (`/gmdhd`, `/gmgivexp`, ...): adds the `InternalSlashCommands.xml` the stock client lacks (see below) | 1 whole new file, written by this project (162 commands) | 48.7 KB |

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

- **What it does.** The client creates a profile when a character has no
  saved UI variables: its first login on this machine (a new character, or
  an existing one on a new machine or Windows profile), or after the stock
  version-2 wipe. On that profile it binds each known starting ability
  (Pistol Shot 592, Strike 594, Heal Focus 597, Health Heal 1646,
  Recuperation 1218, in that order) to the next empty layer-bound button
  from 11 to 20 (default keys Alt+1 to Alt+0), using the same calls as
  dropping an ability from the Ability window. It reads the known abilities
  with the zero-argument native `getAbilityList()` and, while there is
  something to place, listens to `Events.AbilityUpdate` and (the player's
  own updates, at most once a second) `Events.PropertyUpdated`.
- **Only in the first session.** The profile stores the module load (login)
  that created it. At any later login the profile is marked
  `cimmeriaStarterHotbar = 'done'`, whatever was placed, and the patch
  never touches it again, even if the starting abilities become known
  later. It also stops listening as soon as it is done. A profile that
  existed before the patch, or one made with the editor's New Profile
  button, has no mark and is left alone.
- **Compatible with UI packs.** It never creates, moves or resizes a
  button, and it patches no file that the WQHD v26 UI pack replaces
  (`ActionProfiles.lua`, `ActionButton.layout`). It wraps
  `ActionProfileMod.refreshProfileTemplateCombo` (called once per module
  load, from `onModLoaded`), `createProfile` and `loadProfile`, which the
  stock file and the v26 replacement both define.
- **Rebuilding.** Build a patched tree whose `ActionProfileDefault1.lua` is
  the stock file (sha256 `a09eb055...`) followed by `StarterHotbar.lua`, then
  run `cimmeria-patchset build` as below. `starter_hotbar_tests.rs` in
  `crates/patchset` decodes the committed delta and fails when the bytes it
  adds differ from `StarterHotbar.lua`.
- **Logic UAT.** `lua5.1 data/client-patches/009-starter-hotbar/test/run.lua`,
  or `python .../test/run_lupa.py` on Windows. CI runs it against a
  clean-room model of the ActionButtons module. Set `SGW_UI_DIR` to a stock
  client's `Working/SGWGame/Content/UI`, and `SGW_V26_ACTIONPROFILES` to
  the v26 pack's `ActionProfiles.lua`, to also run it against the real
  scripts. Its stub natives raise on a wrong argument count, as the client's
  tolua shims do.

### 010-debug-area-rings

> **Retired 2026-10-05: it freezes the client.** It was in the signed
> manifest for about half an hour and was pulled. Installs that applied it
> keep the broken Ihpet chunk (sha256 `62ef4acd...`) until
> [011](#011-debug-area-rings-fix) repairs it. Never put it back in a
> manifest. The section below is what 010 was meant to be; the cause of the
> crash and the fix are in the 011 section.

The ring stations of the GM-only Debug Area (DA-08,
[debug-area.md § Ring transports](../../docs/content/debug-area.md#ring-transports)).
One op rebuilds `Ihpet_Crater_Light-fff80002.umap`, the chunk in the
middle of the crater, with eight copies of region 3's Castle CellBlock
ring rig: base platform, five rings, the particle emitter and the whole
Kismet sequence (rings rise, flash, sound, rings drop). Each copy's
sequence took its own instance number, so the server plays one station
at a time by object path. The console mesh is not cloned: the server's
ring-switch entity (template 3) renders one.

- **Depends on 007, and only on 007.** The rig's bytes come from
  `Castle_CellBlock-fffeffff.umap`, which 007 rewrites, so the op's second
  source is pinned to **007's result hash**, not the stock map's, and
  marked `"output_of": "007-castle-armory-ring"` in the spec and recipe.
  Publish the manifest entry with `"after": "007-castle-armory-ring"`,
  never "the previous entry": the launcher's `blocked_by_failure` checks
  only the one id named.
  - If 007 failed or was skipped on an install, the launcher skips 010 too
    (`skipped, it builds on 007-castle-armory-ring, which did not apply`)
    and carries on with the rest.
  - If 007 is recorded as applied but its map was replaced since, 010
    fails with "... does not match patch 007-castle-armory-ring's output
    ... it cannot apply until 007-castle-armory-ring has applied", leaves
    the Ihpet map untouched, and the install reports the failure.
  - Keep 010 terminal: chain no later entry `after` 010, so a failed 010
    (a GM-only world most players never enter) skips nothing else.
  - `debug_area_rings_tests.rs` in `crates/patchset` fails if the pin,
    the `output_of` marker and 007's result ever disagree.
  - **Superseding 007 means rebuilding the rig patch.** 010 (and 011)
    freeze 007's result hash. A future patch that changes the Armory map
    (as 007 superseded 002) changes the donor, so the rig patch needs a new
    patch id rebuilt against the new result and chained after the new patch.
- **What the delta carries.** 17,057 bytes for a 2.26 MB map, rebuilt
  from the player's own stock Ihpet chunk and 007's Armory map. Its extra
  block, the only bytes that reach the map verbatim, is 487 bytes
  compressed (4,163 raw). Most of it is binary glue we chose (coordinates,
  export and name indices). About 463 bytes are ASCII: short CME
  identifier strings that also exist in the donor (`ring5ring4ring3ring2ring1`
  eight times, `Bool`, `ource`); the longest run is 138 bytes. 007's extra
  block is 31 bytes. `committed_010_delta_ships_no_verbatim_map_bytes`
  fails above 520 compressed bytes, so growth is noticed.
- **World 73 sees it too.** The live Ihpet Crater (world 73) loads the
  same map file, so it shows the eight platforms as scenery. Nothing is
  seeded for world 73, so none of them does anything there.
- **Without the patch** the Debug Area consoles and trips still work, with
  no ring hardware on the pads and no animation.
- **Rebuild.** `upk_patch` takes `--first-at` once per station (UE units:
  `X = game z * 100`, `Y = game x * 100`, `Z = floor y * 100`), in the
  station order the server's sequence ids assume:

  ```bash
  upk_patch clone-objects <stock>/.../Ihpet_Crater_Light-fff80002.umap \
      <007-applied>/.../Castle_CellBlock-fffeffff.umap <patched>/.../Ihpet_Crater_Light-fff80002.umap \
      --roots 772,1192,216,218,219,220,227,228 --map 764:104 \
      --first-at -93800,22400,690   --first-at -73800,39400,-1113 \
      --first-at -78200,8100,5      --first-at -70200,17600,-719 \
      --first-at -72500,21000,-3328 --first-at -55900,12700,2306 \
      --first-at -56600,43600,2309  --first-at -93700,43700,1130
  ```

  The output's SHA-256 is
  `62ef4acdc08eb784816e4d9ca0cfd1e9790e8b24a421c0615b2711885eaa4946`.
  Then run `cimmeria-patchset build` as below with a `--stock` tree whose
  `Castle_CellBlock-fffeffff.umap` is 007's result (sha256 `2f41a7e1…`).
- **Check on a real client.** With `SGW_PATCHED_CLIENT` set to a client
  that has 007 applied:
  `cargo test -p cimmeria-patchset real_client_debug_area_rings -- --ignored`.
- **Publishing** (coordinator): add the printed manifest entry with
  `"after": "007-castle-armory-ring"`, keep it the last link of its chain
  (no entry `after` 010), re-check the `after` chain, sign the manifest
  offline and upload it with the zip.

### 011-debug-area-rings-fix

Replaces [010](#010-debug-area-rings), which hung every client that loaded
the Ihpet Crater map (world 1300, the Debug Area, and world 73, the live
Ihpet Crater, which streams the same chunk).

- **Symptom (DA-06, 2026-10-05).** With 010, the client took a first-chance
  `ACCESS_VIOLATION` (read) at `SGW.exe+0xbc6a0` (0x004bc6a0) right after the
  streaming loader reached `Ihpet_Crater_Light-fff80001/fff80002`, then the
  main thread stopped ticking and the watchdog killed it 30 s later. Three of
  three tries; no other client log in 30 days has that address.
- **Where.** The AV is in `ULinkerLoad`'s `operator<<(FName&)` (0x4bc660,
  AV at 0x4bc6a0): it reads an int from the stream and indexes the linker's
  name map with no bounds check. An x32dbg capture of the AV (names.Num 349,
  which is Ihpet with the rig's names added) gave the index `0xB5000000`,
  where `0xB5` is the name `None`: the reader stood three bytes before a
  `None` terminator. The stack was the object `Serialize` path
  (`UParticleSystemComponent` and `UStaticMeshComponent` ->
  `UPrimitiveComponent::Serialize` -> the tagged property loop at 0x4b56e0,
  whose `FPropertyTag` reader is 0x4b6a60), not the export-table parse. That
  function does not look at flags; it is only where the misread surfaces.
- **Cause.** Every name-table entry carries `RF_LoadForClient`,
  `RF_LoadForServer` and `RF_LoadForEdit` bits (`0x0007_0000_0000_0000`;
  client is `0x0001_...`). The name-table loader (`FUN_004bad20`, the
  "serializing name map" step of the linker tick `FUN_004beca0`) ANDs each
  entry's flags with the linker's load-context mask and, when nothing is
  left, stores FName `(0, 0)`, which is `None`; the FName reader then returns
  it and still consumes the 4-byte number. A property tag named `None` ends
  the property list. The clones' component `LightingChannels` struct names
  its second property `Dynamic`. In Castle's table `Dynamic` is
  `0x0007_0010_0000_0000`; in Ihpet's it is `0x0004_0010_0000_0000`
  (editor-only). The cloner reused Ihpet's entry by name, so the 48
  components that name it (40 InterpActor components, 8
  `ParticleSystemComponent`s) read as `None` mid-struct and every tag after
  it was read from the wrong offset. Of the 326 stock names, `Dynamic` is the
  only one whose flags differ from the donor's. That a nested `None` ends the
  outer list's parse rests on the captures and the 48-versus-0 split below,
  not on a decompile of the struct tag loop.
- **Evidence that this is the whole story.** Client-loaded exports (object
  flags with `RF_LoadForClient`) that name an entry the client does not load:
  **48 of 765 in 010's chunk, 0 in 011's, 0 in stock Castle (1,571), 0 in
  stock Ihpet (237), 0 in 007's Castle map.** Stock packages do name
  editor-only entries (90 tags in stock Castle, 3 in stock Ihpet, for
  example a `Dynamic` in Ihpet's `Brush_3.BrushComponent_7`), but only on
  objects without `RF_LoadForClient` (a `Brush`, a `DrawLightConeComponent`,
  an `InterpCurveEdSetup`), which the client never serializes.
  `upk_patch audit-names <package>` reproduces the counts, and is empty on
  any unmodified package.
- **Bisect (lab, same chunk built from fewer roots).** The ring base
  `StaticMeshActor` alone loads; the Kismet sequence alone loads; the five
  `InterpActor`s, the `Emitter`, one full rig, and every mix that includes
  either of those fail. Swapping meshes did not change that (a `StaticMeshActor`
  with the ring-00 mesh plus an `InterpActor` with the base mesh still
  fails): the base mesh component's `LightingChannels` struct names a
  different property, so it never tripped.
- **Why 007 worked.** 007 clones inside Castle, whose table is the donor's:
  the names and their flags are the same entries.
- **Fix.** `PatchSession::ensure_name_with_flags`
  (`crates/upk/src/patcher/mod.rs`): a name is reused only when the
  target's entry has every load bit the source entry had; otherwise the table
  gets a second entry for the same string, with the source's bits, and the
  clones use it. Stock objects keep reading the original entry. The chunk
  has two `Dynamic` entries (index 67, editor-only; and a new one at the end)
  and loads. `a_cloned_property_name_keeps_the_load_bits_it_had_in_the_source`
  fails when the flag check is removed.
- **One op, two starting points.** The op rebuilds the chunk from the normalized
  stock Ihpet chunk plus 007's Armory map (clean installs), or, through the
  recipe's new `alternatives` field, from 010's output alone (installs that
  applied 010; sha256 `62ef4acd...`, pinned `output_of: 010-debug-area-rings`).
  Both give the same bytes, sha256
  `52b4f3adc5cb7beb8f3e2728e90ea764603de0515e013b600f30a1c63199ede0`.
  The 010 path's delta is 402 bytes, and needs no Armory map. `apply` tries
  the primary sources first and the alternatives in order; when none match it
  reports the primary's error (the stock file is the one a player can
  restore).
- **Which launchers do what.** The 010 -> 011 upgrade path needs a launcher
  with `alternatives` support, which no release has yet (the newest,
  `launcher-20260929-0d71e26`, predates it). A launcher without it parses
  the recipe (`Op` is not `deny_unknown_fields`), ignores `alternatives` and
  applies the primary, which is 010's own stock + 007 source: **that is
  correct on every clean install**, so fresh installs and everyone who never
  applied 010 are served by every launcher. On a 010 install it fails with
  the usual source mismatch and leaves the broken chunk untouched. As of
  2026-10-05 the only install that applied 010 was repaired by hand, so no
  launcher release, `min_launcher` gate or patch split was made for the
  upgrade path; if another 010 install turns up, either release a launcher
  with this field (and set `min_launcher`), or repair by hand with
  `cimmeria-patchset apply 011-debug-area-rings-fix.zip --install <dir>` from
  this repo's build (which does support it).
- **Arena station moved.** The first arena station stood on the pit's water
  plane, where the client draws water and players who step off sink to y -52.
  Region 39 now sits on the east shelf at (331, -11.12, -693), the largest
  clear disc on it (DA-F2 surveyed the real occluder and navmesh data). The
  rig is the fifth `--first-at` copy, so it keeps instance `_Seq_3`; only the
  seed coordinates and the copy's position changed.
- **No new CME bytes.** The primary delta is 17,049 bytes (010's was 17,057)
  and the alternative 402. `committed_011_deltas_ship_no_verbatim_map_bytes`
  in `crates/patchset/src/debug_area_rings_fix_tests.rs` pins both, and
  `the_readme_states_the_committed_011_zip_and_result_hashes` fails when the
  zip and this file stop agreeing.
- **Rebuild.** Use the cloner built from this repo (it adds the name entry):

  ```bash
  upk_patch clone-objects <stock>/.../Ihpet_Crater_Light-fff80002.umap \
      <007-applied>/.../Castle_CellBlock-fffeffff.umap <patched>/.../Ihpet_Crater_Light-fff80002.umap \
      --roots 772,1192,216,218,219,220,227,228 --map 764:104 \
      --first-at -93800,22400,690   --first-at -73800,39400,-1113 \
      --first-at -78200,8100,5      --first-at -70200,17600,-719 \
      --first-at -69300,33100,-1112 --first-at -55900,12700,2306 \
      --first-at -56600,43600,2309  --first-at -93700,43700,1130
  cimmeria-patchset build data/client-patches/011-debug-area-rings-fix/patch.json \
      --stock <stock tree: stock Ihpet chunk + 007's Castle_CellBlock-fffeffff.umap> \
      --alt-stock <tree holding 010's Ihpet chunk, sha256 62ef4acd...> \
      --patched <tree with the rebuilt chunk> --out data/client-patches/011-debug-area-rings-fix.zip \
      --blob-url https://raw.githubusercontent.com/SandboxServers/Cimmeria/<commit>/data/client-patches/011-debug-area-rings-fix.zip
  ```

  `--alt-stock` is given once per alternative, in spec order.
- **Publishing** (coordinator): add the manifest entry below with
  `"after": "009-starter-hotbar"`, **remove 010 from the manifest** (and keep
  it out: no test or file guards that, and a manifest that lists both leaves
  a launcher without `alternatives` on the broken chunk), re-check the
  `after` chain, sign offline, upload with the zip. The launcher applies
  each manifest patch once, by id, so 011 also runs on installs that already
  recorded 010 as applied.

  ```json
  {"id": "011-debug-area-rings-fix", "after": "009-starter-hotbar",
   "size": 19033,
   "sha256": "34fa0127a61a6955469be811dc2e3ab8c9564117dc7930c1a64a79d9c58f4621"}
  ```

  (plus `blob`, `title` and `description` as `cimmeria-patchset build`
  prints them). The chunk 011 writes is sha256
  `52b4f3adc5cb7beb8f3e2728e90ea764603de0515e013b600f30a1c63199ede0`.

### 012-gm-slash-commands

The client builds its slash-command map from three XML files in
`Common\xml\slash_commands\` (`SlashCommands.xml`, `InternalSlashCommands.xml`,
`FinalSlashCommands.xml`; the directory is `SlashCommandXMLPath` in
`GameplayEngine.ini`, `..\..\Common\xml\slash_commands`, read by
`LaunchMisc__InitContentSystems` at `0x0041f9c0`). The launcher's seed has the
first and the last but not `InternalSlashCommands.xml`, the one that defines
the `/gm*` commands, so a launcher install has 104 commands in its map and
every native `/gm*` command answers "Invalid command." (confirmed in the lab,
DA-06, 2026-10-05). 012 ships that file, new, whole.

- **Written by this project, no CME text.** The file is generated by
  [tools/client-patches/gm_slash_commands.py](../../tools/client-patches/gm_slash_commands.py)
  from the table [commands.toml](012-gm-slash-commands/commands.toml), which
  holds only identifiers: the command word, the `Event_SlashCmd_*` class it
  fires (a name in `SGW.exe`) and the typed parameters in the order the
  client parses them. Every `Description` and `ParamDescription` is empty,
  every `Usage` is built from the word and the parameter names
  (`/gmdhd [DestinationId]`: square brackets for a required parameter, round for an optional one), and the one comment says the file is generated.
  The QA client's file was read as a reverse-engineering reference for the
  command set and the schema only. Tests in
  `crates/patchset/src/gm_slash_commands_tests.rs` keep it that way: the
  XML may hold no prose field (checked structurally: that is the real guard,
  because the file has no free-text channel left). As a tripwire on top, no
  multi-word string in it may exactly match a SHA-256 in
  [cme-prose.sha256](012-gm-slash-commands/cme-prose.sha256), the hashes of
  every description, parameter description and comment line in the client's
  three files (CI has no client, and the repository does not hold the
  sentences; `tools/client-patches/cme_prose_hashes.py` writes the list).
  Exact hashes catch a verbatim paste only, never a reworded copy, so a green
  run of that test is not proof of anything on its own.
  `cargo test -p cimmeria-patchset gm_slash_commands -- --include-ignored`
  with `SGW_QA_CLIENT` set also compares the sentences themselves.
- **The schema the client's parser demands** (a gSOAP-generated reader,
  `FUN_00a4e0a0` over `CMETextCmds::cmd::CommandList`; element reader
  `0x00a62bb0`, command builder `FUN_00a4d470`): root `CommandList`;
  `Command` elements in the namespace
  `http://www.cheyenneme.com/common/slash_commands` (an identifier the parser
  matches, nothing is fetched) with the required attributes `CommandName`,
  `EventName`, `Description` and `Usage` and the optional `Access`; then
  `MandatoryParam` elements followed by `OptionalParam` elements, each with
  `PType` (`Boolean`, `Integer`, `Float` or `String` here), `ParamName` and
  `ParamDescription`. The client keeps one entry per command word in a
  `std::map`, a later word replacing an earlier one. The command's event
  class must exist in `SGW.exe` (all 162 do; checked against the 256
  `Event_SlashCmd_*` names in the binary).
- **What `Access` does.** The client turns the attribute into a mask with
  `0x00a4c530`: one bit per role letter in `gcCqQadDpm`, set when
  `std::wstring::find` returns non-zero. A letter at position 0 returns 0 and
  a letter that is absent returns `npos`, so the mask is every role except
  the first letter: `"p"` gives `0x2FF`. A command is usable when its mask is
  0 or shares a bit with the player's access level (`0x00c789e0` against the
  global `0x01df2d44`, written by `onEntityProperty(GENERICPROPERTY_AccessLevel
  = 7, level)`). `0x2FF` passes levels 1 to 4 and refuses level 0, so a
  normal player gets "Invalid command." and `/help` hides the command. This
  is only the client's own filter: the server's gate
  ([gm-cell-method-gating.md](../../docs/architecture/gm-cell-method-gating.md))
  still requires GameMaster (2).
- **What it registers.** 162 words: the 164 entries of the QA client's file
  minus two words it defines twice (`/gmaddbehavioreventset` and
  `/gmremovebehavioreventset`; the later entry wins in the client's map, so
  that is the one kept: class `...EventSet`, no parameters; the earlier names
  `...Event_Set` classes `SGW.exe` does not have). With the 104 the stock
  files and the client's own commands give, the map holds **266**, the number
  `/help` reports. Every `/gm*` row in [commands.md](../../docs/commands.md) is
  among them. One deliberate difference from the QA file: `/gmspawnbycmd` has
  no `Access` there. A command with no `Access` attribute gets mask 0
  (`0x00a4c530` returns 0 for a null attribute), and mask 0 means usable at
  every access level, so the client would let a level-0 player type it. Giving
  it `"p"` like the rest closes that client-side hole and grants nothing: the
  server gate (cell method 185 is above the GM index range) already refused it
  for anyone below GM.
- **Parameter types matter: a wrong one crashes the client.** `PType` decides
  how the typed text becomes the event's value, and the handler reads that
  value as the type it expects. The QA file declares `/gmspawnbycmd`'s two
  offsets `Integer`, while `gmSpawnByCmd` in `SGWGmPlayer.def` takes
  `FLOAT, FLOAT`; in the lab (DA-06, 2026-10-05) `/gmspawnbycmd 1310 5 5` with
  the Integer file raised an access violation reading `0x00000008` at
  `SGW.exe` `0x00561778` before any cell method was sent, and the typed line
  stayed in the chat box. Patch 012 declares them `Float`, like the def and
  like `/gmgotoxyz` (lab: the server then spawned the NPC at the GM's position
  plus the offsets). `/gmsetnoxp` is declared with no parameter, as its def
  takes none: the client sends an empty payload with or without an argument
  (lab), so a declared Boolean only made the bare command answer "Not enough
  parameters.". The other commands carry the QA file's types. The test
  `declared_types_feed_the_def_arguments_of_the_same_method` compares every
  command whose word is a `gm*` method of `SGWGmPlayer.def` and whose parameter
  count equals some variant's argument count, so a mismatch like the spawn
  offsets fails CI. Commands that declare fewer parameters than the method has
  arguments (the client supplies the rest) cannot be compared, and two
  (`/gmgiveminigamecontact`, `/gmremoveminigamecontact`: QA `Integer, Integer`
  against a def of `WSTRING, INT64`) disagree and were not traced; they keep
  the QA types, listed in the test as unverified. Beyond those and the
  commands the lab ran (`/gmdhd`, `/gmgotolocation`, `/gmgotoxyz`, `/gmgivexp`,
  `/gmspawnbycmd`, `/gmsetnoxp`) the types are the QA file's, unexercised.
- **The client never shows `Usage`.** A command with too few parameters prints
  only "Not enough parameters." (lab), so the `[name]` brackets in the file are
  cosmetic. They are there because the field is required.
- **`/gmdespawn` needs a server-side fallback.** The client sends target id 0
  and does not fill in its selection, so the server treats 0 as the GM's
  selected target (`gm/world.rs`, `handle_despawn`).
- **Verified in the lab (da06, 2026-10-05, colo build 86fe5dcab, GM
  character, world 1300).** With the file installed: map size `0x10A` (266);
  `/help` ends "266 commands found."; `/gmdhd 3` sends `gmDHD` and the server
  dials, `/gmdhd 29` sends it and the server refuses; `/gmgotolocation`,
  `/gmgotoxyz` and `/gmgivexp` reach the server with the right argument types;
  chat and the UI are unaffected and `SGWDebugLog.log` has no slash-command
  or parse errors. A non-GM account was not available in the lab, so the
  level-0 refusal is from the binary reading above and the unit test of the
  mask, not from a run.
- **No dependency, no delta.** The file is new, so there is nothing to diff
  against: it applies to any client and replaces an existing
  `InternalSlashCommands.xml` (a QA install already has one).
  It touches no other file and needs no other patch.
- **Rebuild.** `python tools/client-patches/gm_slash_commands.py` rewrites the
  XML from the table (`--check` fails when it is stale, which CI runs;
  `--summary` prints counts). Then run `cimmeria-patchset build` as below with
  any `--stock` tree whose `Common/xml/slash_commands/` directory exists (a
  folder with one empty file is enough) and any `--patched` directory.
  `committed_zip_carries_exactly_the_generated_xml` fails when the zip and
  the XML differ.
- **Publishing** (coordinator): add the printed manifest entry with
  `"after": "009-starter-hotbar"` (or the newest entry of the chain at that
  time), re-check the `after` chain, sign the manifest offline and upload the
  zip. Keep 012 terminal: nothing builds on it.

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
