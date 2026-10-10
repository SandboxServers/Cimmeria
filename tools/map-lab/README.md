# Local map editor bootstrap experiment

`patch_editor.py` turns the seven `Group="Editor"` byte-patch definitions in
the user's `AtreaLoader.config.xml` into a separate executable. It checks the
original bytes, understands `XX` wildcard bytes and refuses to overwrite its
input or an existing output. It does not write a map or replace AtreaRL's
runtime symbol hooks. Keep all executable output in a disposable client copy;
never commit it.

```powershell
$python = '<python executable>'
$bin = '<disposable QA Working\binaries>'
& $python tools/map-lab/patch_editor.py "$bin\SGW.exe" `
  "$bin\AtreaLoader.config.xml" --output "$bin\SGWEditorLab.exe"
& $python -m unittest discover -s tools/map-lab -p 'test_*.py' -v
```

On the 2026-10-06 QA executable the check matched all 11 patch chunks and
changed 21 bytes; input SHA-256 `109f3077…e4925783`, output
`75086412…170c58732`. This is a **bootstrap test**, not a working editor.
The separate executable and the unmodified one both hit an early `Message`
modal when launched from the current automated shell. No editor command file
ran or map was saved. The native Computer Use service also returned
`GetCursorPos failed: Access is denied` during launch, so no visual diagnosis
was possible in this session. See the [CM-00 log](../../docs/analysis/custom-debug-map/automation-probe.md).

The direct package route has one guarded primitive in `upk_patch`:

```powershell
# Use a disposable client copy. Output is a research artifact, not playable yet.
bash tools/build-lane/lane.sh cargo run -p cimmeria-upk --bin upk_patch -- `
  retain-level-actors '<source sublevel.umap>' '<scratch output.umap>' `
  --classes WorldInfo
```

On the 5,459-byte stock `Tollana_Curia-00000000.umap`, this reduced the
Level's placed-actor refs from 27 to 1 while preserving all 61 exports. The
uncompressed output reopens and passes structural verification. Its remaining
`WorldInfo` is a technical scaffold; no new floor or wall has been inserted,
the package still has its stock identity, and there is no game-client load
evidence. The experimental `alias_package.py` gives an uncompressed package
a same-length new map name and deterministic GUID, and rejects a compressed
input or existing output. It was used on the persistent level, sublevel and
MapData scaffold to form a `Cimmeria_Lab1` scratch package set. This changes
identity, not map content; cross-package references have not been fully
audited and no client has loaded it.

`upk_patch clone-objects` can then place individual existing assets into the
stripped sublevel. A test cloned `GA-Props.GA-Cover_INT_Med03` from
`Harset_CmdCenter-00000000.umap` at UE `(0, 0, 0)`, increasing the Level
actor list from one to two. Parser reopening and property-name audit passed;
appearance, collision and cover behavior still require a client test. The
`upk_info --mesh-actors` inventory flag lists donor StaticMeshActor export
indices, locations and mesh references.

The SGC floor candidate (`SGC-RoundRoom_Floor00`, export 1132 in
`SGC-00000002.umap`) carries a 276-byte baked 2D lightmap tail. The
[CM-00b byte study](../../docs/analysis/custom-debug-map/lightmap-tail.md)
identifies its three package-local texture refs. `clone-objects
--strip-lightmaps` is an explicit opt-in that accepts the exact verified
shape, discards the baked lighting and writes the known unlit LOD form.
Four floor actors cloned into scratch and passed structural verification.
There is still no client rendering or collision evidence.

`clone-objects --first-at X,Y,Z --yaw-degrees 0|90|180|270` sets an
absolute cardinal yaw on the first root actor, rotating any actors in its
cloned group around that placement. The map shell probe uses it to align
Ancient elbow and three-way corridor meshes into a closed 3 × 3 interior.
The option has a serialized transform regression test in `cimmeria-upk`.
Run `assemble_worldforge_probe.ps1` with explicit `-ClientRoot`, `-InputMap`,
`-PreviousProbe` and `-OutputMap` paths to reproduce the package assembly.
It refuses an existing output, and its result still requires client testing.
