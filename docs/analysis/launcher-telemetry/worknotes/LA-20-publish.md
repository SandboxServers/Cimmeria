# LA-20: the LT-37 file index publish, re-run (done)

Date: 2026-10-22. The owner authorized one signed publish of the file index. PR I2 (#1982) is merged, and CI was green.

**Outcome: done.** `content-current` now serves a manifest whose `file_index` points at the release `content-file-index-20261022`. The manifest was signed once, and no other key was used.

## Steps

1. The live pair from `content-current` had not changed since LT-37 (`68c9b343…eb34d`, `.sig` `146fdfe9…c7815`). It was backed up to `B:\lt37-publish\la20-backup\`.
2. A fresh worktree from origin/main at 1928182c6 (contains f2cde8244, #1982). Tools built through the lane (exit 0). `prepare-file-index.ps1` ran with `-BlobPath https://github.com/SandboxServers/Cimmeria/releases/download/content-file-index-20261022/file-index.json` and passed all 11 checks, including `index_blob_form`, `append_only` and `parity`. The rebuilt index is byte-identical to LT-37's.
3. The public-key secret in the vault was compared in a script with the release key: it matches. Only the secret `manifest-signing-key` was read, into a temp file inside an ACL-restricted scratch folder. It signed once. The file was then overwritten three times with random bytes and deleted, along with its folder. Nothing was printed. `cimmeria-patchset verify` reported `signature OK`.
4. The release `content-file-index-20261022` was created (pre-release, not latest) with `file-index.json`. The hosted copy's sha256 was checked before the swap. `content-current` was swapped at 2026-10-22 11:06:58Z.
5. Post-swap, I fetched what the launcher fetches: `content-current/manifest.json`, its `.sig`, and `file_index.blob`. Results: `signature OK`. The index sha256 and size match the manifest. `verify-file-index --previous <backup> --seed --patch…` passed all 11 checks.
6. Compared with the backup, `schema`, `seed`, `patches` (the `after` chain included) and `min_launcher` are unchanged. The only addition is `file_index`. No patch was added, so no lab load was needed.

## Digests (sha256, bytes)

| File | sha256 | Size |
|---|---|---|
| new `manifest.json` | `0b4eedf9441e0538eac11b040f8e40cf18a11190d470e2ccea7acfef6e083254` | 8028 |
| new `manifest.json.sig` | `9fb9e65b0d73045c477eb89d96381880b37b447b1eaa565f86b5f15d7751f12f` | 128 |
| `file-index.json` | `c0bed73e213ab937a766375f91484346c1a3bf4cf6dbb4af9e6af2d82a993a8a` | 1273979 |

## Rollback

```powershell
gh release upload content-current -R SandboxServers/Cimmeria --clobber B:\lt37-publish\la20-backup\manifest.json B:\lt37-publish\la20-backup\manifest.json.sig
```

The old manifest has no `file_index` entry, so after a rollback the release `content-file-index-20261022` is simply unused. Leave it in place.

## Cleanup and what is left

- Worktree `la-20-publish` and `B:\targets\la-20-publish` are retired. Nothing was committed.
- LT-27 U7 (adoption on the laptop that would not adopt) can now run.
