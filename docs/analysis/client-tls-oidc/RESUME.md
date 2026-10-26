# client-tls-oidc: resume card

> **Type:** Reference · **Audience:** The orchestrator resuming this campaign · **Updated:** 2026-10-26
> **Companions:** [Ledger](README.md), [work packets](work-packets.md), [wave 10](packets-wave10.md), [contract](contract.md), [PR #2031 ruling](worknotes/pr2031-ruling.md)

**Authority:** the consolidated architect ruling, written out in [wave 10](packets-wave10.md), governs waves 6-10 and the tail. Where an older wave file disagrees, the ruling and [packets-wave10.md](packets-wave10.md) win.

**State (updated at the wave 8/9 boundary, 2026-10-26):** `cto-camp` head `a7dccefe4` (campaign review fixes on top of merge `5ab9c6af3`). Merged: wave 6 CT-70..75; wave 8 CT-83, 84, 85 (wave review `ship`, pass 2); wave 9 CT-86..91 (wave review `ship`, pass 2; CT-90 Inspect ruling in `.claude/reports/client-tls-oidc/wave9-ruling-ct90.md` applied as N6); wave 10 CT-95, 97, 98, 99. In flight: **CT-76** (`cto-76`, fix 1 `200d9e29b`, pass-2 review running). Next: wave 7 opens when CT-76 merges (CT-77, 80, 81, 82, then 78, 79); wave 10 CT-96 (after CT-73 and CT-82), CT-94, CT-101 (after CT-95), CT-102 (after CT-77); wave 6 and wave 7 and wave 10 reviews, `network-security-auth` passes (waves 6/7/10 together, wave 9 alone), CT-92 docs (add the CT-90 ruling notes to the sign-in.md brief), CT-100 deferred, campaign review, PR #2031 `full-ci`. Worktrees to retire (user approves `--force`): cto-73, 75, 87, 88, 89, 91 and later ones. One PR, **#2031**, open from it. Wave 5 gates unchanged (CT-51 in the queue as #2030; CT-52, 53, 56, 57, 58 gated). The "Start here" list below is the original plan; the State line wins.

**Owner instruction (2026-10-26):** PR #2031 is already OPEN (opened 2026-10-25, head `client-tls-oidc/campaign` at `8225135ef` on origin, no labels, CodeRabbit skipped: 203 files). Before the last push into the PR, bring current `main` into the campaign branch (rebase, per the owner). Do it once all packets and reviews are done and no packet worktree is in flight (a rebase rewrites SHAs the packet branches were cut from); the push then needs `--force-with-lease`, so ask the user before pushing.

**Start here:**

1. Campaign-packet § 0: check no peer session works `cto-camp`.
2. Dispatch at once, each in its own `cto-NN` worktree at the campaign head: wave 6 CT-70, 71, 72, 74; wave 8 CT-85, 83; wave 9 CT-90, 91; **wave 10 CT-95, 97, 98**; tail CT-94. Parallel by crate (identity/auth/server vs desktop launcher).
3. Chains, each branching at the campaign head after the previous packet merged:
   - wave 6: CT-73 (after 71 and 72), CT-75, CT-76;
   - wave 7 (opens when CT-76 merged): CT-77, 80, 81, 82, then 78, 79;
   - wave 8: CT-84 (after 83);
   - wave 9: CT-86 (after 90), 87, 88, 89; CT-91 merges last;
   - wave 10: CT-99 after CT-97; CT-96 after CT-73 and CT-82; merge order CT-95, 97, 98, 99, 96.
4. Tail, merged before CT-92: CT-94 (D-CT23 = a), CT-101 (D-CT26 = b, after CT-95), CT-102 (D-CT27 = b, after CT-77). Then CT-92 (`documentation-writer`, amended), then CT-100 when `main.rs` is free (Deferred).
5. Reviews: packet loops, a wave loop each; **one `network-security-auth` pass over waves 6, 7 and 10 together** (after CT-96) and one over wave 9; then the campaign review loop; then PR #2031 with the `full-ci` label.

**Merge into `cto-camp` with `merge-noff.ps1`** in each wave's order. Retire each `cto-NN` once merged in; never `git worktree prune`.

**Owner calls:** decided 2026-10-26: D-CT23 = a, D-CT26 = b, D-CT27 = b. Still open: **D-CT29** (OIDC and the age gate; joins CT-53), CT-53 registrations and secrets, D-CT16, the privacy page, crash-left ticket files, the live client UAT (CT-04, CT-58, CT-U1..U6), the native keyring proof (CT-U6), OPS-LT1 after CT-52.

**Hazards:**

- launcher-cache-presync edits `worker.rs`, `login_port.rs` and `crates/server/src/main.rs`; no packet of waves 6-10 edits those (CT-100 waits for it).
- The main checkout has someone else's uncommitted `frontend/ui/index.html`; CT-91 runs every check in `cto-91` only.
- `docs/**` is CRLF. This folder is untracked until CT-92; workers read it from the main checkout path.
- Live-DB packets use the sentinel slots their packet names (next free `0x7000_EC00`); never broad cleanup predicates (F19).

**Next ledger update:** when the first packet of waves 6, 8, 9 or 10 merges into `cto-camp`. Next free ids: CT-103, D-CT31.