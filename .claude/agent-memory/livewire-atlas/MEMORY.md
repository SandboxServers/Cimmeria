# Livewire atlas packet (2026-10-13)

- The installed `Livewire.upk` has one `GFxMovieInfo` export named `Livewire`; its serial data contains a GFX blob after a short object prefix. `tools/upk_parser.py` resolves the export and handles package compression.
- The movie exports 56 numbered wire libraries and six electrode/art libraries used by the hit atlas. The 5-stage-unit raster yields local hit points and shape bounds for each. The generated JSON is private to the lab install and must not be committed.
- `docs/analysis/cellblock-autoplay/livewire-spike/test_atlas.py` keeps one hit point and a coarse shape-bounds golden per library. Set `CIMMERIA_LIVEWIRE_UPK` to run against a local client copy.
- The lab's server clones per routed request and instance. Keep the validated atlas as an `Arc` field on `LabServer`; loading it only as a local inside daemon router setup drops it before a solver can use it.
- `CIMMERIA_LAB_HOME` or the installer `-LabHome` can relocate the lab cache. The installed `labd/daemon.ps1` resolves its parent folder and exports that path to the Rust daemon.
