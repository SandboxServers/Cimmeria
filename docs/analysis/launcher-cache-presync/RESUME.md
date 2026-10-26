# launcher-cache-presync: resume card

> **Type:** Reference · **Audience:** The orchestrator resuming this campaign · **Updated:** 2026-10-26
> **Companions:** [Ledger](README.md), [work packets](work-packets.md), [wave 2](packets-wave2.md), [waves 3-5](packets-wave3.md)

**State (2026-10-26):** wave 1 code-complete in `lcp-w1`, head `48e9712d8`: LP-01, 02, 03, 04, 07 merged, LP-05 worknote says **compose works** (D-LP3 resolved, no ruling). Wave review fixes landed (head `887f339e6`) and the wave review passed `ship` at pass 3. The wave 1 PR is next. Then ask the user about the lab for LP-06, and open wave 2 (LP-10, LP-11). Board: c/cimmeria/launcher-cache-presync/43. Orphaned targets (~163 GB: cto-40/41/45/46/w3a/w3b) reported to the user, not deleted.

**Goal:** at Play, the launcher brings the Documents `Cache.en-US` up to the server's cooked-data versions, so the ~1 min resync at character select never fires. **Owner 2026-10-26:** D-LP1 yes (anonymous routes on 8081, quota 60/min, kill switch), D-LP2 yes (changed entries only), D-LP4 reversed: a failed presync **blocks Play** with the exact failure and a **Retry** button; no fallback to `cooked_sync` from the launcher. Edge cases: D-LP18..D-LP29.

**Start here:**

1. Campaign-packet § 0: check no peer runs it. Have `ops` create the board subcategory.
2. Wave 1 (one PR from `lcp-w1`): LP-01 and LP-07 at once (both Ready). Chain LP-02, LP-03, LP-04, each at the previous reviewed head; then LP-05 (`rust-gameserver-dev`, `<client-copy>` in the brief, never committed). Merge order LP-01, 02, 03, 04, 07, 05.
3. LP-05 verdict "ruling needed" (base mismatch, `source_newer`, or a body over 16 MB) → dispatch `architect` for D-LP3 first.
4. After wave 1 merges: ask the user about the lab, then LP-06 (UATPending).
5. Wave 2 (one PR from `lcp-w2`): LP-10 and LP-11 at once, then LP-12. Opens on wave 1 merged + LP-05 "compose works".
6. Wave 3 into `lcp-w3`: LP-06 worknote, LP-20 (and LP-24 from `main` in parallel), then the chain LP-21, LP-26, LP-22, LP-27. Opens on wave 1 merged + LP-06 pass. Wave 3 loop.
7. Wave 4 into the same `lcp-w3`: LP-23, LP-24, LP-28 (at LP-24's head), LP-25. Wave 4 loop, `network-security-auth` pass, then ONE PR for waves 3+4. Never ship wave 3 alone (it blocks Play without telling the player why).
8. Campaign review, then LP-30 (wave 5), then owner UATs LP-U1..U4.

**Hazards:**

- client-tls-oidc (`cto-camp`, one PR) edits `worker.rs`, `login_port.rs`, `crates/server/src/main.rs`. Check whether it merged before waves 2 and 3; keep those hunks small.
- The main checkout has uncommitted `frontend/ui/index.html` edits that aren't ours; never commit them from the root.
- No CME bytes in the repo or tests (R9). No paths, IPs or account names in worknotes (R7).
- LP-27's revert-proof test (a failed presync never reaches client setup) is the campaign's key guard; reviewers check it fails with the gate removed.

**Open decisions:** D-LP28 (owner, non-blocking: failure code in the anonymous launcher summary → LP-F4). D-LP3 is resolved: LP-05 found compose works.

**Next ledger update:** when the wave 2 PR opens.
