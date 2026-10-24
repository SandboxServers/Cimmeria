# Light adoption: accept any install with SGW.exe (wave 8, v2)

Owner direction, 2026-10-21: "Just accept any install for adoption that has SGW.exe at the right path relative to the game's root directory as we understand it today. Most users will download with the launcher the first time; this is only for existing users who already have an install."

**v2, 2026-10-21 (owner answers to D-LA8..D-LA13).** Light adoption is the only in-place mode (D-LA8). A pending patch whose input differs is applied as far as it can be, and never blocks adoption or Play (D-LA9). The first Update after adoption does not need the 4.1 GB seed, and that work is in wave 8 (D-LA10). The seedless Update is for light-adopted installs only; downloaded installs update exactly as today (D-LA14, owner 2026-10-21). LT-37 is finished, and adoption compares the folder against the published index without blocking (D-LA11). D-LA12 and D-LA13 follow the recommendations. **The per-file `policy` idea (file index schema v2: required/optional/ignore) is superseded** by D-LA9 and D-LA11, and no packet implements it.

Packets: [work-packets-light-adoption.md](work-packets-light-adoption.md). Anchors are file:line at the launcher integration tip `e6f16395a` (main plus PR L #1981). Engine paths are under `crates/launcher/desktop/engine/src/storage/`, shell under `crates/launcher/desktop/shell/src/`, frontend under `crates/launcher/desktop/frontend/`. Every packet re-greps its anchors at its base tip.

## Findings

| # | Finding | Evidence | Packet |
|---|---|---|---|
| F1 | The client root holds `Working`; `SGW.exe` is in `<root>/Working/Binaries`. The picker takes the root, `Working` or `Working/Binaries`, case-blind. | `adoption/source.rs:93` (`resolve_folder`), `:142` (`holds_sgw`), `:152`, `:161` (`refuse_links`); `in_place_preview.rs:12` (`full_layout`) | LA-14 |
| F2 | A flat folder (`SGW.exe` at the root) stays `exe_at_root` (D-AI20). | `launcher-core/src/install_layout.rs:30-42` | — |
| F3 | `adopt_existing_install` has no caller outside its tests and writes `seed_adopted = true`, which Play refuses. | `launcher-core/src/install.rs:508` (`AdoptError`), `:546`; `install_tests.rs:273-328` | LA-07 |
| F4 | Today's adoption hashes every file and compares it with the index. With no published index it builds one from the 4.1 GB seed, which is the "validating, then downloading the whole game" the owner saw. | `adoption/index_source.rs:47-80` (`acquire_index`), `:199-250` (`built`); `verification.rs:103-115` (`NoReusableFiles`) | LA-11 |
| F5 | A recipe op pins its sources and `result_sha256`. The applier skips a target already at its result, runs the whole compute phase in memory first, and returns at once on `SourceMissing`/`SourceMismatch`, a delta error or a result mismatch. Writes are temp-plus-rename; overlays go last and need no source. `ApplyReport` has no skipped list. | `patchset/src/apply.rs:11-19`, `:58-106` (early returns `:86-88`, `:93-96`, `:98-104`), `:108-141`, `:219-228`; `lib.rs:50-103` | LA-02 |
| F6 | `unpack` throws the `ApplyReport` away. A plain zip is extracted with `File::create`, which truncates an existing file in place. | `launcher-core/src/unpack/zip.rs:45-61`, `:83`; `unpack/mod.rs:127` | LA-03, LA-08 |
| F7 | `InstalledState` is `{applied_patches, seed_sha256, seed_adopted}`, with no `deny_unknown_fields`. `install_all_into` loads it **from the install dir**, re-seeds only when the seed sha differs, skips applied patches, and records a patch only on `Ok`. Play's `content_valid` reads only this ledger and the layout. | `launcher-core/src/state.rs:45-99`; `install.rs:197-278`; `install_worker/mod.rs:470-497`; `launch/mod.rs:155-160` | LA-03, LA-04 |
| F8 | Update always rebuilds: an empty stage `<dest>/.cimmeria-update-<id>/game`, then `install_all` (seed + every patch), then swap `game` → backup, stage → `game`. **A stage pre-seeded with `game/` (ledger included) makes `install_all` skip the seed and apply only the missing patches**, when the seed sha is unchanged. | `update/preparation/mod.rs:187-311` (stage `:205-215`, `install_all` `:217`/`:236`); `update/commit/mod.rs:96-216`; `seed_archive/mod.rs:65-80` | LA-13 |
| F9 | No tree copy or hard-link helper exists. `staged_content::digest` refuses `nlink != 1` on unix, and the adoption inventory refuses it everywhere. Stage and backup sit beside `game/` on one volume. | `update/staged_content/mod.rs:69-75`; `adoption/inventory.rs:108,147` | LA-08 |
| F10 | The index is `{seed: {sha256, files}, patches: [{state_key, sha256, inputs, outputs}], setup}`; `IndexedFile = {path, size, sha256}`; `histories()` maps a lower-cased path to its states. | `launcher-core/src/file_index/mod.rs:45-96`, `:223-234` | LA-06 |
| F11 | `blob_forms_match` accepts only the seed's https host, or relative under a relative seed. The seed is on archive.org, so no hostable index blob passes. The launcher itself takes any absolute https URL, or a relative one resolved against the manifest URL. | `file_index/check.rs:117-126`, `:135-146`; tests `:271`, `:342`; `manifest.rs:356` (`blob_url`) | LA-05 |
| F12 | `prepare-file-index.ps1` **is tracked** on `origin/main` (`tools/content-release/prepare-file-index.ps1`, 88 lines, #1977). The search that missed it ran in the root checkout, whose local `main` is behind at `5079be48f`. The runbook (`docs/client/launcher-distribution-setup.md` § File index, `:136-151`) never names the host or the blob rule. | worknotes/LT-37.md; `prepare-file-index.ps1:19,66,73` | LA-05, LA-20 |
| F13 | `publication.rs` is 697 lines and `in_place.rs` 688 (hard cap 700); `verification.rs` is 514, shell `main.rs` 635. New logic goes in new files. | line counts at `e6f16395a` | all |

## 1. Acceptance rule (unchanged, D-LA1)

Light adoption accepts a folder when **all** of these hold:

1. **Root.** `resolve_folder` resolves the pick, or, when nothing matches, exactly one child folder one level down. Two or more candidates: `several_game_folders`. None: `not_a_game_folder`.
2. **Exe.** `<root>/Working/Binaries/SGW.exe` is a regular file. No links or junctions on the root, `Working` or `Binaries`.
3. **Not owned.** There is no installed content (`identity_conflict`) and no live launcher names (`leftover_launcher_files`).
4. **Safe to move.** The `unsafe_file` survey stays (AU, D-LA5).
5. **Writable, not running.**

## 2. Reaching the current patch level, never blocking (D-LA2, D-LA9)

1. **Summarize and detect.** Fetch every patch blob (never the seed) and summarize its sources and outputs (LA-01). A patch is *current* when every output path already holds that patch's output or a later one; otherwise it is *pending*.
2. **Replay, tolerant.** Replay the pending patches in signed order in `<prep>/scratch`, with the patchset applier in **tolerant mode** (LA-02):
   - Overlays and plain-zip entries always apply. They are whole files and need no source.
   - A recipe op applies when its source matches its pin or an alternative, or when its target is already at the result.
   - **Anything else is skipped, not refused, and recorded:** `source_missing`, `source_mismatch`, `delta_failed` or `result_mismatch`. That target keeps the bytes the player had. A later op that reads the skipped target's result fails its own source pin, so it is skipped and recorded too.
3. **Store and record.** Every scratch file that differs from the folder becomes a `Replacement{NeedsPatching}`. The synthesized ledger lists every patch in `applied_patches`, plus a new `incomplete_patches` entry per patch with skipped ops (LA-03). A patch listed there is **not fully applied**. `content_valid` reads only `applied_patches`, so Play stays enabled.
4. **Retry.** Each later Update re-runs only the skipped ops of each incomplete patch, in signed order, with overlays off (`only_targets`, LA-02 and LA-04). If the player has restored the file, the op now applies and the entry clears. Repair rebuilds from the seed and clears the list.
5. **Provenance.** `Provenance.check = light`. `reference_digest` is the post-setup light tree, skipped files at their found bytes. Publication, re-hash and recovery run unchanged (LA-11).

**What cannot be forced, and what the player sees (flag to the owner).** A bsdiff delta encodes the difference from one exact source file. Applied to other bytes it produces garbage, which the result hash catches, so **no code can force a recipe op onto a changed or missing source**. The launcher therefore keeps the player's file as found, and every op chained on it from later patches stays unapplied too. Overlays and whole-file replacements always land. The player sees:

- **Review:** "Some patch changes can't be applied: N files differ from the originals the patches expect. They stay as they are. You can still use this folder." Then an expandable list of those files, by path and patch.
- **The confirm button stays enabled.**
- **After adoption:** a persistent line on the main view, "N patch changes could not be applied. If the game crashes or misbehaves, use Repair (downloads the full game, 4.1 GB)." Play is enabled.
- The client may crash or desync where it loads one of those files. Telemetry tags launches from such installs (LA-17).

## 3. Incremental update for light-adopted installs only (D-LA10, D-LA14)

**Owner constraint, 2026-10-21 (D-LA14).** A player with no install, who downloads the seed from the Internet Archive, must keep working exactly as today. So the seedless Update below applies **only** to an install adopted through light adoption. A launcher-downloaded install keeps today's seed rebuild on Update, byte for byte: same requests, same staged tree, same ledger bytes. The only addition on that path is telemetry: `update_staged` with `mode = full`, `ops_skipped = 0`, `stage_linked = 0` and `stage_copied = 0`.

**Discriminator.** The launcher's installed-content record (the engine's `Record`, `installed_content/mod.rs:61-68`) carries `adoption: Option<Provenance>`. Light adoption publishes it with `check = light` (LA-11), and `publish_update_index` keeps it across Updates. Update takes the seedless path only when that provenance exists and has `check = light`. These cases all take today's full rebuild:

- no provenance: a launcher download;
- `check = full`: a verified copy;
- a legacy import.

`InstalledState.seed_adopted` is **not** the discriminator: it is a legacy flag that Play refuses and that migration imports from old ledgers. Nothing else in `game/launcher-installed.json` is either, because a downloaded install writes the same shape. Extending the seedless path to every install with a qualifying ledger is a later option (D-LA14b), not part of wave 8.

When the install is light-adopted, and the installed ledger's `seed_sha256` equals the target manifest's seed, and its `applied_patches` are a prefix of the target's patch list in order (the append-only chain rule), Update takes the **incremental** path:

1. **Write set.** The write set is the outputs of every patch not yet applied (LA-01), the skipped targets of each incomplete patch, `launcher-installed.json`, and the setup-rule paths.
2. **Seed the stage from `game/`** (LA-08). Write-set files are real copies. Every other file is a hard link on Windows, falling back to a copy when linking fails. Off Windows everything is copied, because `staged_content::digest` refuses `nlink != 1` there by design. Hard links are safe because the applier writes temp-plus-rename, and every file a patch could truncate in place (F6) is in the write set.
3. **Apply.** Call `install_all` in tolerant mode (LA-04). The seed is skipped because the ledger's seed sha matches (F7). Incomplete patches are retried, then the new patches are applied tolerantly.
4. **Unchanged after that:** `content_valid(stage)`, `staged_content::digest`, the prepared record, and the commit's atomic swap with backup, recovery and cleanup (`update/commit/mod.rs:96-216`).

If the install is not light-adopted, or either condition fails (a new seed, or a rewritten chain), Update takes today's full rebuild, which needs the seed. **Repair stays the full rebuild**: strict apply, from the seed, the escape hatch. `update_staged` gains `mode` (`incremental`/`full`), `ops_skipped` and `stage_copied`/`stage_linked` counts.

## 4. Index comparison at adoption, non-blocking (D-LA11)

After the scan (every file is hashed anyway, D-LA13), light preview fetches the published index (`index_source.rs:99-152`, `published` only). It **never builds one from the seed**, and compares (LA-06, LA-12):

| Class | Meaning |
|---|---|
| `stock` | bytes equal the seed's file |
| `patched` | bytes equal some patch's output for that path |
| `prepared` | a setup-rule file in its prepared form (ASLR-off `SGW.exe`, the login file) |
| `changed` | the index names the path; the bytes match none of its states |
| `missing` | in the seed layer, absent from the folder |
| `extra` | the index names no state for the path |

The stock 2009 `Cache.en-US` spelling is mapped to `SourceCache.en-us` first (`install_layout::bundled_cooked_alias`, LA-06).

- **User-visible.** The review shows the class counts and an expandable list of every non-`stock` file with its class, sorted and capped at 500 rows in the status (the count says how many more). The comparison never blocks.
- **Index unavailable.** It is unpublished (`index_not_published`), the fetch failed (`index_unreachable`), or the digest or bind failed (`index_invalid`). Adoption goes ahead, and the review says "No published file list to compare against yet."
- **Telemetry (R5).** One `adoption_index_compared` info event per preview, target `adoption`, with these fields:
  - `state`
  - the six class counts
  - `changed_sample` and `missing_sample`: each at most 10 paths, comma-joined, at most 600 characters.

  **The samples hold only paths copied from the signed public index** (stock game file names, identical for every player), never a name that exists only in the player's folder. `extra` is a count only, because a player's own file names can identify them. No absolute path, URL, host, IP or user name appears (D-LT14 R5a).

## 5. LT-37 completion (D-LA11)

- **LA-05 (PR I2).**
  - `blob_forms_match` accepts any absolute `https://` URL or a relative blob, and refuses anything else (`http://`, `file://`, other schemes). The check is renamed `index_blob_form`, and its tests are updated.
  - The runbook names the host: a new GitHub release per publication, `content-file-index-<yyyymmdd>`, whose asset URL is the blob. It also covers the tracked script, the backup and the rollback (`gh release upload content-current --clobber <backup pair>`).
- **LA-20 (hands-on, after PR I2 merges with green CI).**
  - Re-run the publish under the 2026-10-21 authorization, reaffirmed by D-LA11: prepare, verify (`--previous`, `--seed`), create the release, upload `file-index.json`, sign, verify the signature, back up, swap `content-current`, then check after the upload.
  - Reuse `B:\lt37-publish`. `append_only` and `parity` fail if the live manifest moved since 2026-10-21; then rebuild.
  - Record the digests in `worknotes/LT-37.md`.

## 6. Decisions

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-LA1 | Acceptance = § 1; flat folder refused; one level down, exactly one candidate | Decided | F1, F2 |
| D-LA2 | Light replay of pending patches (option E) | Decided | F5 |
| D-LA3 | `AdoptionMode::Light` (serde `light`); verified `InPlace` leaves the UI; modes `[Light, Copy]` | Decided | One literal on the wire |
| D-LA4 | `Provenance.check: Full \| Light`, `#[serde(default)]`, skipped when `Full` | Decided | Old records and digests stay byte-identical |
| D-LA5 | `unsafe_file` and `leftover_launcher_files` stay refusals | Decided | Data safety for the moves |
| D-LA6 | The old ledger's `applied_patches` is never trusted | Decided | `adoption/mod.rs:1-2` |
| D-LA7 | Remove `adopt_existing_install` and `AdoptError` | Decided | F3 |
| D-LA8 | Light is the default and only in-place mode; the verified copy stays | **Decided (owner 2026-10-21)** | Owner: yes |
| D-LA9 | Tolerant apply: overlays and matching ops apply; other ops are skipped and recorded in `incomplete_patches`; adoption and Play are never blocked by a changed input; Update retries, Repair clears | **Decided (owner 2026-10-21)** | "Apply the patch anyway"; § 2 is the honest reading of what a delta can do |
| D-LA10 | Incremental update (stage seeded from `game/`, tolerant apply, same swap and backup) is in wave 8; Repair stays the full rebuild | **Decided (owner 2026-10-21)** | "Adopting is meant to avoid downloading the source archive" |
| D-LA11 | Finish LT-37 (LA-05, LA-20) and add the non-blocking index comparison with counts-only telemetry (§ 4) | **Decided (owner 2026-10-21)** | Owner wants per-install drift known, shown and reported |
| D-LA12 | Keep the verified in-place code until after the LA UAT; delete or keep at close-out | Decided (owner: recommendation); the delete question returns at close-out | Low risk while hidden |
| D-LA13 | Accept the full hash scan at preview, with progress (hashed bytes of total) | Decided (owner: recommendation) | Also feeds § 4 |
| D-LA14 | The seedless incremental Update serves **only light-adopted installs**: the installed-content record has adoption provenance with `check = light`. Downloaded installs, copy adoptions and legacy imports keep today's seed rebuild byte for byte (only `update_staged` gains `mode = full` and zero counts). LA-02/03/04/08/13 carry this as hard rule R-LA 10, and LA-04 and LA-13 add guard tests that fail if a downloaded install takes the new path | **Decided (owner 2026-10-21)**: replaces the architect's earlier every-install default | "The user who has no install and downloads the seed from the internet archive must keep working exactly as today." The provenance is engine-owned and survives Updates; `seed_adopted` is a legacy ledger flag that Play refuses |
| D-LA14b | Extend the seedless Update to every install with a qualifying ledger | Later option, not scheduled | Saves the 4.1 GB rebuild for downloaded installs too. It needs its own owner call and a follow-up packet that only widens `seedless_allowed` and flips guard test 6 |
| D-LA15 | Hard links on Windows for files outside the write set, copy fallback; copy-only elsewhere | Decided | F6, F9: safe because of temp-plus-rename and the write set |
| D-LA16 | Retry incomplete patches with `only_targets` and overlays off | Decided | Re-running an old patch's overlays would regress newer files |
| D-LA17 | Index-comparison telemetry: counts plus at most 10 index-sourced paths per sample; `extra` count only | Decided (architect, within the owner's "counts only, plus non-identifying names") | Index paths are public stock names; folder-only names are not |
| D-LA18 | `incomplete_patches` keeps the patch in `applied_patches` | Decided | Keeps `content_valid`, `missing_patches` and the `after` chain unchanged; older launchers ignore the new field |

## 7. Packets and shipping

Full specs: [work-packets-light-adoption.md](work-packets-light-adoption.md).

- **PR LA** ships waves 8a (except LA-05), 8b, 8c and LA-19 from the integration worktree `lt-light-integration` (branch `launcher-telemetry/light-adoption`). Its base is the `launcher-telemetry/launcher` tip; `ops` restacks it onto `origin/main` once PR L merges.
- **PR I2** is LA-05 alone, from `origin/main`.
- **LA-20** runs after PR I2 merges.

| Wave | Packets |
|---|---|
| 8a | LA-01, LA-02, LA-05 (PR I2), LA-06, LA-07, LA-08 (all parallel) |
| 8b | LA-03 → LA-04; LA-09 → LA-10 → LA-11; LA-12 (parallel to the chains; LA-11 after LA-12) |
| 8c | LA-13 (after LA-11; light-adopted installs only); LA-14 → LA-15 → LA-16 → LA-18; LA-17 |
| close | LA-19 docs (PR LA), LA-20 publish (after PR I2) |

Sonnet-sized (`rust-gameserver-dev`): **LA-10, LA-11, LA-13**. LA-19 goes to `documentation-writer`, LA-20 to `hands-on`. The rest are `packet-coder`, about 40k working tokens each.

**After PR LA:** owner UAT on the second laptop's install (ask first): adopt, Play, then a later Update without the seed. In SigNoz, check `adoption_light_planned`, `adoption_index_compared` and `update_staged{mode=incremental}`. The close-out adds the rows to `docs/guides/unified-uat.md`.
