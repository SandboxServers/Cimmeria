# Custom map first client result (2026-10-06)

- `CimmeriaLab` world 1301 loaded in the QA client. The avatar stood and jumped on an invisible floor and fell off its edge after walking; the viewport was black. This is collision evidence for at least one loaded surface, not proof that its mesh rendered.
- The renamed Tollana_Curia persistent package has `SkyLight` export 374/component 375 but the Level actor list has only `WorldInfo` after the strip. The SGC floor mesh components in the sublevel had baked lightmaps removed.
- A local `upk_patch clone-objects` candidate cloned SkyLight export 374 into the persistent level at UE `(0,0,200)`, changing Level actor count 1→2. The patcher reopened and audited the candidate. It was not installed while SGW.exe held the target package open; in-client lighting remains untested.
- See `docs/analysis/custom-debug-map/client-load-test.md` for the test and evidence boundary.
