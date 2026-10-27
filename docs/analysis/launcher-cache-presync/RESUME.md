# launcher-cache-presync: resume card

> **Type:** Reference · **Audience:** The orchestrator resuming this campaign · **Updated:** 2026-10-26
> **Companions:** [Ledger](README.md), [work packets](work-packets.md), [wave 2](packets-wave2.md), [waves 3-5](packets-wave3.md), [worknotes](worknotes/)

**State (2026-10-26): campaign done, pending the owner UATs.** Waves 1 (#2039), 2 (#2043) and 3+4 (#2045) are merged. The campaign review ended `ship` at pass 2. The close-out (LP-30) updated the ledger, `docs/project-status.md` (once; `gap-analysis*` has no row for this), [unified UAT](../../guides/unified-uat.md#launcher-cooked-data-presync) and the doc-update map. **A live Play against a real server and client has never run.**

**What is left (none of it is code):**

1. Owner UATs LP-U1..U4 ([ledger](README.md#owner-uat)). **Ask the owner before any lab use.** LP-U1 must check SigNoz for `cooked_data.version_reply` `up_to_date` per category and no `cooked_data.sync_start`. LP-U2 needs the next colo deploy; LP-U3 and LP-U4 need a local server.
2. D-LP28 (owner, non-blocking): failure code in the anonymous launcher summary. Yes adds LP-F4.
3. Open issues for the follow-ups the close-out names but does not open: LP-F2 (signed index or HTTPS-only; state the threat, link D-LP10, coordinate with client-tls-oidc CT-51/52) and LP-F6 (server body copies, quota 60 per IP per minute is the bound). LP-F1, LP-F3, LP-F5 are listed in the ledger.
4. Retire every `lcp-*` worktree and empty `.claude/state/active-campaign` (`ops`, `rm-worktree.ps1 --merged-into <PR> <names>`), once the close-out PR merges. Orphaned targets (~163 GB: cto-40/41/45/46/w3a/w3b) were reported to the user, not deleted.
5. D-LP15 stays unverified and non-blocking (an optional `re-lookup` of `UpdateFromSourceArchives` `0x00479fa0`, no lab).

**Hazards:**

- PR #2031 (client-tls-oidc) is open and also edits `worker.rs`: expect a rebase conflict there.
- The main checkout has uncommitted `frontend/ui/index.html` edits that aren't ours; never commit them from the root.
- No CME bytes, paths, IPs or account names in the repo or worknotes.
- `Profile::Firesky` (a real Documents folder) has no automated test; only LP-U1 covers it.
- Side finding for the owner: served dialog 60110 shows a markup parse error after "I can add" (seed text; candidate issue).

Board: c/cimmeria/launcher-cache-presync/43.
