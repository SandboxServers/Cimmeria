---
name: patchset-supersede-and-restore-to-stock
description: Superseding a launcher patch set - repeat kept ops with the same pins (apply skips target==result before checking sources); restoring an upk_normalize'd map to stock ships CME bytes
metadata:
  type: project
---

Facts from building `007-castle-armory-ring` (superseding the retired `002-castle-ring-transport`, 2026-09-29):

- `cimmeria_patchset::apply` checks `target sha == result_sha256` and skips the op **before** it reads or hashes any source. A new patch that repeats an older patch's op (same sources, delta, result) is therefore a no-op on installs that already applied the old one, and still works on fresh installs.
- `build` reads every source from `--stock` and every result from `--patched`, so the trees can be mixed per file when an op's "source" is not stock.
- **Restoring a map to stock with a delta does not work under the no-CME-bytes rule.** Patched maps are written uncompressed (upk patcher); stock maps are LZO-compressed. A bsdiff from 002's fffdfffc (2.19 MB) back to stock (770 KB) was 389 KB, and its extra block held 383 KB of verbatim stock bytes. The maintainer chose to drop the restore op. Restoring needs a launcher re-extract from the player's own seed (not built yet).
- Retiring a manifest entry never undoes it on installs that already applied it.

**How to apply:** before promising "restore to stock" via a recipe op, decode the delta's bsdiff header (ctrl/diff/extra sizes). The extra block is the target bytes the delta ships verbatim.
