---
name: launcher-extracted-mtimes-and-ue3-ini-version
description: UE3 compares Default*.ini mtimes against [INIVersion] in the player's SGW*.ini; launcher extraction must keep archive DOS times (cab/zip), UnRAR already does; fixture and test traps.
metadata:
  type: project
---

UE3 records each `Default*.ini` timestamp in `[INIVersion]` of
`Documents\My Games\Firesky\SGWGame\Config\SGW*.ini`; a changed mtime
shows the "Your ini ... file is outdated" dialog. The 2009 cabinets date
the client 2009-06-30. Fixed 2026-09-29 (branch
fix/launcher-preserve-cab-mtimes): `crates/launcher/src/unpack/dos_time.rs`
converts DOS date/time with `DosDateTimeToFileTime` +
`LocalFileTimeToFileTime` (current-bias rule, same as expand.exe) and is
applied in FDI `CLOSE_FILE_INFO` and zip extraction.

**Why:** the stock installer and Windows extractors stamp files; the
launcher previously left install time on every file.

**How to apply:**
- UnRAR (`unrar` crate `extract_to`) restores RAR mtimes itself, and
  `move_tree` is a rename, so the RAR path needs nothing.
- zip crate writes 1980-01-01 00:00 (`DateTime::DEFAULT`, datepart 0x21)
  when no time is set; treat it as "no timestamp", not a real date.
- MakeCAB stamps entries from the SOURCE file mtime, so `make_cab_set`
  sets each source file's mtime to `fixture_mtime()` before running it.
- Patch-set (bsdiff) outputs keep write time by design.
- Launcher is a bin crate: `cargo test -p sgw-launcher --bin sgw-launcher`
  (`--lib` fails with "no library targets").
