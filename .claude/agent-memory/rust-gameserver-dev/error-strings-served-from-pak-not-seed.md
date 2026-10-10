---
name: error-strings-served-from-pak-not-seed
description: client error text comes from served ErrorStrings.pak (216 entries), not the error_texts seed; check the PAK, then patch Text or add the entry in attribute_patches
metadata:
  type: project
---

Any "send an error id the client renders" fix (onCharacterCreateFailed, onErrorCode) must check `data/cache/ErrorStrings.pak`, not `db/resources/Texts/Seed/error_texts.sql`. The base serves cooked category 11 from the PAK plus `crates/resources/src/base/attribute_patches/`, and a metadata change triggers a full resync of those entries.

- The seed has 218 rows; the shipped PAK has 216. 20001 `ERROR_InvalidCharacterName` and 20002 `ERROR_CellLoginFailed` came from the Giza dump and never shipped. Since PR #1314, 20001 is added (`attribute_patches/additions.rs`, `ERROR_STRING_ADDITIONS`); 20002 still has no client text.
- `_10000`-`_10004` and `_10006`-`_10009` ship their moniker as Text (several in quotes) and render as raw tokens unless patched. PR #1314 patches 10000-10003; 10004 and 10006-10009 are still raw.
- `ATTRIBUTE_PATCHES` only patches shipped entries; `ERROR_STRING_ADDITIONS` adds missing ones in the shipped `COOKED_ERROR_TEXT` shape. Both feed the one category-11 bump (`bump_for`), whose value is pinned in `attribute_patches/tests.rs`, so any change re-pins it. The seed rows must carry the same text (`seed_rows_match_the_patches`).

Found reviewing PR #1314 (CS-08 F1), 2026-10-10. Check with `zipfile.ZipFile('data/cache/ErrorStrings.pak').read('_<id>')`.

**How to apply:** for any error code you send, confirm the PAK has the id and that its Text is human-readable. If not, add an attribute patch or an addition, and confirm the result with a lab screenshot. Related: [[client-error-codes-and-ammo-chatter]], [[cooked-item-additions-shape]].
