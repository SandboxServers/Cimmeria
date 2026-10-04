# Mac game graphics preparation

- 2026-10-04: Game-only `VK_DRIVER_FILES` selects the pinned runtime's MoltenVK
  descriptor after runtime verification and cache ownership; missing or redirected
  descriptors refuse preparation. See `crates/launcher/desktop/docs/launch.md`
  and `engine/src/storage/launch/wine_tests.rs` under the desktop workspace.
- Environment unit tests do not prove rendering, login, or gameplay. Those require
  separate retained native UAT evidence.
