# starter-uat-praxis: resume card

> **Type:** reference. **Audience:** the orchestrator resuming this campaign. **Updated:** 2026-10-27
> **Companions:** [ledger](README.md), [work packets](work-packets.md). Siblings: [union](../starter-uat-union/RESUME.md), [shops](../starter-uat-shops/RESUME.md), [debug-armory](../debug-armory/RESUME.md).

**State (2026-10-27):** planned, nothing dispatched. The campaign files live on branch `lab-uat-plans` until ops commits them into `sp-integration`.

**Goal:** a graded lab playthrough of every owed client row from Castle_CellBlock to Tollana, an issue per failure, and a TOML spec per pass.

**Start here:**

1. Campaign-packet § 0: check no peer runs it; write `.claude/state/active-campaign`.
2. `ops`: create `sp-integration` (branch `starter-uat-praxis/integration`), commit the three campaign files.
3. Dispatch SP-00 (documentation-writer) in `sp-integration`. Packet review loop.
4. Wave 1: worktrees branched at the `sp-integration` head; SP-01, 02a, 02b, 03, 04, 05 at once. Packet loops, merge `--no-ff`, wave loop, one docs PR.
5. Before wave 2: D-SP1 and D-SP2 must be answered (relay to the owner; never decide). Then ask the user for the lab (contract § C1), wave by wave.
6. Per drive packet: lab-driver (brief), verifier (verify file), up to 3 attempts plus a Sonnet takeover. Results PR per wave.
7. Wave 6: SP-91, SP-92 (packet-coder, lane check `committed_specs_parse_and_validate`), SP-98, then SP-99 after FZ-99.

**Hazards:**

- The lab-server MCP refused connections on 2026-10-27; verifiers need it or ClickHouse over ssh.
- A fresh client logging straight into Castle hangs (#756): fixtures land at character select, then Play.
- Never `client_wait_for` on a world load (#1677); lanes never share a plaza.
- Status docs change only in SP-99, after FZ-99 (D-SP7).
- Other campaigns' uncommitted folders in the main checkout are not ours; never commit from the root.

**Open decisions:** D-SP1 (agents may run owner-reserved rows), D-SP2 (colo or local server), D-SP3 (adopt lomiada's priority; scheduling only).

**Next ledger update:** when SP-00 is integrated.
