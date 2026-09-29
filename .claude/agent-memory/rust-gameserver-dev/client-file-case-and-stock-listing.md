---
name: client-file-case-and-stock-listing
description: SGW client UI resources are looked up case-sensitively even on Windows (eula.lua broke login, 2026-09-29); DATA.INF in the seed RAR is the stock-spelling reference
metadata:
  type: reference
---

Learned 2026-09-29 fixing the no-login-screen bug in launcher-20260929-f518b57.

- **Case matters on Windows.** The game's CEGUI `SGWResourceProvider` finds UI Lua
  by the name the stock `.toc` gives it. `EULA.lua` renamed to `eula.lua` =
  "'EULA.lua' does not exist in group lua", no EULA screen, so no login screen.
- **How the rename happened.** A write-temp-then-`std::fs::rename` onto an existing
  file gives it the rename target's spelling. `std::fs::write` / `File::create`
  onto an existing file keep the on-disk name. So any "atomic write" of a client
  file must resolve the on-disk spelling first
  (`cimmeria_patchset::resolve_existing_case`).
- **Stock spelling reference.** The seed RAR (`Stargate Worlds (0.8348.1.4046)
  (2009-06-30) (beta).rar`) holds `Data\DATA.INF`, whose `[file list]` has every
  installed path. Extract just it with `7z e -o<dir> <rar> "Data\DATA.INF" -r`
  (526 KB, seconds, the RAR is stored). The stock tree mixes case itself:
  `Working\binaries\SGW.exe`, `Content\audio\ui\`, `Content\audio\genprp\`.
- **Guards now in place.** `patchset::build` refuses a spec path whose existing
  part the stock tree spells differently (`CaseMismatch`); `apply` keeps on-disk
  names; launcher `client_setup::stock_case::PATCH_TARGETS` renames case-only
  mismatches back every launch, and `every_patch_target_is_listed` pins every
  spec op target to that list. A new patch spec target must be added there.
- Published zips 004/005/006 still carry the old spellings (append-only); the
  specs were corrected, so those three no longer rebuild byte-identically.

Related: [[build-environment]].
