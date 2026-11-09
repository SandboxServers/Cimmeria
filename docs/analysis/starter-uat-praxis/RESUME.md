# starter-uat-praxis: resume card

**Blocker (2026-11-09):** main CI is red (#2072): `container build + smoke` fails on the CI runners at the s6-overlay download (`curl` HTTP error, reruns fail; the URL works from the dev workstation, so suspect the runners' egress path). Fix PR #2074 (`curl --fail` + retries) is red for the same reason. PR #2073 (`client_interact`, review ship, CI green) waits on main going green; its worktree `lab-interact` is not retired. Needs an infra look at runner egress to github release downloads. Tried on #2074: `curl --fail` (surfaced a real 504), then proxy build-args with the name `egress` (not resolvable inside BuildKit RUN), then the resolved IP (still 504 from the proxy path, run 42582817563). Next: host check from a node (`curl -v` of the s6 URL via egress:3128 and direct), or give `buildkitd` proxy env in `tools/ci-runners/compose.yml`; or vendor the s6 tarballs. Owner/infra action.

**Stopped overnight 2026-11-09 (all remaining parts hit the same two blockers):** (1) native NPC/prop right-click fails (Kaerlia, Marsh, Prisoner 329, small panels): `client_interact` (PR #2073, review ship, CI green) is built and the lab daemon already runs it (backup `cimmeria-lab.exe.bak-20261109`), but agents need a Claude Code `/mcp` reconnect or restart to see the tool; then live-test it on Kaerlia (100205), Marsh (100199), Prisoner 329. (2) `/gmkilltarget` skipped the death event so kill-count chains never advance: PR #2075 (review ship, CI green) fixes it; the lab server (dr-lab) needs main merged in and a rebuild after it lands. Merges of #2073, #2074, #2075 all wait on main going green (#2072, container build). Not run: DCB3-p11..p13, DCB4-p7, p8, DCB5-p1..p4, p6..p9, DCB6-p1..p3, DCB7-p1..p9, DTO4 U7 objective text, DCB3-p9 Marsh clause, DCB3-p10 re-drive, DTO2-p1 P9 re-run, wave 6, brief packet for SP-DCA8..DCA24. Use `/gmmissionabandon <id>` as the abandon route (not /gmmissionclear: rename ParamName DesignID to MissionDesignId in `data/client-patches/012-gm-slash-commands/commands.toml` is a maintainer call).

**Progress (2026-11-09):** lab drives run on instance p2 with Haiku drivers (`lab-driver`), Sonnet verifiers, runner batches DCA-a..p. Results are committed through DTO3-p1 (commit 3973733). Next unrun parts: SP-DTO3-p2..p4, SP-DTO4-p1..p3, then SP-DTO2-p1 (two clients, alone), wave 6, and a brief packet for SP-DCA8..DCA24. Kaerlia right-click is a harness gap (BLOCKED-known) until `client_interact` (#2073) ships and is tested live. Open issue: `/gmmissionclear` empty argument (results/issues/SP-DHA1-p1-1.md). Local main is not fast-forwarded: run `git merge --ff-only origin/main`.

> **Type:** reference. **Audience:** the orchestrator resuming this campaign. **Updated:** 2026-11-02 (SP-L1)
> **Companions:** [ledger](README.md), [work packets](work-packets.md). Siblings: [union](../starter-uat-union/RESUME.md), [shops](../starter-uat-shops/RESUME.md), [debug-armory](../debug-armory/RESUME.md).

**State (2026-11-02):** SP-00 and all seven wave-1 packets (SP-01, 02a, 02b, 03a, 03b, 04, 05) are merged into `sp-integration` (branch `starter-uat-praxis/integration`, head `10a2ef530` before SP-L1). SP-L1 synced the contract, waves table, `rows.md` and README rows to the 116 brief parts on disk. **Next: one docs PR for waves 0 and 1 and the ledger sync, then wave 2 lab drives** (D-SP5 covers the ask, local server only).

**Goal:** a graded lab playthrough of every owed client row from Castle_CellBlock to Tollana, an issue per failure, and a TOML spec per pass.

**Start here:**

1. [Campaign-packet § 0](../../../.claude/skills/campaign-packet/SKILL.md#0-make-sure-nobody-else-is-running-it): check no peer runs it; write `.claude/state/active-campaign`.
2. Ship the docs PR from `sp-integration` (`ship pr -C <wt>`), CI green, merge, as in the [ship-pr skill](../../../.claude/skills/ship-pr/SKILL.md). Waves 0 and 1 are done; do not redispatch them.
3. Wave 2 lab drives, then waves 3 to 5, in the lane order of the [waves table](work-packets.md#waves-2-to-5-lab-drive-packets-ask-first): one sequential lane, one client at a time ([rule 4](work-packets.md#dispatch-rules)). The two-client part SP-DTO2-p1 is the only concurrent run.
4. Per drive part: lab-driver (the part's brief), verifier (its verify file), up to 3 attempts (a brief re-run as `ATTEMPT: 2` or `3` spends one) plus a Sonnet takeover ([rule 5](work-packets.md#dispatch-rules)). Results PR per wave.
5. Wave 6: SP-91, SP-92 (packet-coder, the two-test lane check in SP-91), SP-98, then SP-99 after FZ-99.

**Hazards:**

- SP-DCA8 to SP-DCA24 (81 end-uat rows) have no brief and are in no wave; a brief packet must cover them before they can be driven.
- The wave plan is long: wave 2 65 parts, about 780 minutes, wave 3 16 parts, about 192 minutes, wave 4 24 parts, about 288 minutes, wave 5 11 parts, about 132 minutes.
- The lab-server MCP refused connections on 2026-10-27; verifiers need it or ClickHouse over ssh.
- A fresh client logging straight into Castle hangs (#756): fixtures land at character select, then Play.
- Never `client_wait_for` on a world load (#1677); one sequential lane, one client at a time (dispatch rule 4); Tollana runs one packet at a time.
- Status docs change only in SP-99, after FZ-99 (D-SP7).
- Other campaigns' uncommitted folders in the main checkout are not ours; never commit from the root.

**Decisions set by user instruction 2026-11-02 (owner may override):** D-SP1 yes, D-SP2 local server at `e8973395f`, D-SP3 yes (scheduling only), D-SP5 blanket lab authorization (local server only). No open decision blocks the briefs.

**Next ledger update:** when the docs PR merges, then after each lab wave (README status rows, `rows.md` grades, `results/`).
