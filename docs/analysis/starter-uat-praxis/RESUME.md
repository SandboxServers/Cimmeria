# starter-uat-praxis: resume card

> **Type:** reference. **Audience:** the orchestrator resuming this campaign. **Updated:** 2026-10-27
> **Companions:** [ledger](README.md), [work packets](work-packets.md). Siblings: [union](../starter-uat-union/RESUME.md), [shops](../starter-uat-shops/RESUME.md), [debug-armory](../debug-armory/RESUME.md).

**State (2026-11-02):** SP-00 (row inventory) in progress in `C:/Users/Steve/source/projects/Cimmeria/.claude/worktrees/sp-integration` (branch `starter-uat-praxis/integration`, based on `origin/main` `e8973395f`). Wave 1 not dispatched.

**Goal:** a graded lab playthrough of every owed client row from Castle_CellBlock to Tollana, an issue per failure, and a TOML spec per pass.

**Start here:**

1. [Campaign-packet § 0](../../../.claude/skills/campaign-packet/SKILL.md#0-make-sure-nobody-else-is-running-it): check no peer runs it; write `.claude/state/active-campaign`.
2. `ops`: create `sp-integration` (branch `starter-uat-praxis/integration`), commit the three campaign files.
3. Dispatch SP-00 (documentation-writer) in `sp-integration`. Packet review loop.
4. Wave 1: worktrees branched at the `sp-integration` head; SP-01, 02a, 02b, 03a, 03b, 04, 05 at once. Packet loops, merge `--no-ff`, wave loop, one docs PR.
5. Before wave 2: D-SP1 and D-SP2 are decided (below); the D-SP5 authorization covers local-server waves ([contract § C1](work-packets.md#c1-ask-first)).
6. Per drive packet: lab-driver (brief), verifier (verify file), up to 3 attempts plus a Sonnet takeover. Keep the per-instance order of the [waves table](work-packets.md#waves-2-to-5-lab-drive-packets-ask-first) ("A then B"). Results PR per wave.
7. Wave 6: SP-91, SP-92 (packet-coder, the two-test lane check in SP-91), SP-98, then SP-99 after FZ-99.

**Hazards:**

- The lab-server MCP refused connections on 2026-10-27; verifiers need it or ClickHouse over ssh.
- A fresh client logging straight into Castle hangs (#756): fixtures land at character select, then Play.
- Never `client_wait_for` on a world load (#1677); lanes never share a plaza (dispatch rule 4); Tollana runs one packet at a time.
- Status docs change only in SP-99, after FZ-99 (D-SP7).
- Other campaigns' uncommitted folders in the main checkout are not ours; never commit from the root.

**Decisions set by user instruction 2026-11-02 (owner may override):** D-SP1 yes, D-SP2 local server at `e8973395f`, D-SP3 yes (scheduling only), D-SP5 blanket lab authorization (local server only). No open decision blocks the briefs.

**Next ledger update:** when SP-00 is integrated.
