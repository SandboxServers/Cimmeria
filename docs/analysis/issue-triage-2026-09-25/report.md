# Open-issue triage — consolidated report

Scope: all 123 open issues in SandboxServers/Cimmeria, researched read-only against origin/main
@ 059d6038 on 2026-09-25. Nothing has been posted, labelled or closed on GitHub. Per-issue
evidence and the ready-to-post text (closing comments, status comments, full replacement bodies)
are in [findings/](findings/); this file is the index, the owner's answers, the reconciliation of
cross-batch conflicts, and the execution checklist.

**Execution log:** **2026-09-25: the 39 CLOSE verdicts in §7 were executed** at the owner's request.
Each got an evidence comment, then a close with the §7 reason. The §3.1 reconciliation was applied
(speed-cap residual pointed at #461), and #30's comment was adjusted per §1.1. Fold-in comments
were posted first on #72 (from #466), #571 (from #468), #568 and #569 (from #472). All 39 were
unchanged since the snapshot. #684-#689 were deliberately **not** closed (§1.7). Open issues:
123 → 84.

**2026-09-25: 49 of the 50 REWRITE verdicts were executed.** Each got an explanatory comment, then
its body was replaced. Before posting, the drafts were reconciled with §1 and §3:

- **#439 / #25:** re-rated P3 per §1.1; the #439 body now also asks to delete the dead editor
  routes.
- **#459:** umbrella boxes synced with the sec-b verdicts and the closures.
- **#461:** the speed-cap residual was added (§3.1).
- **#690:** the children stay open; validation runs on the colo (§1.7, §1.9).
- **#534:** the SigNoz result and the research-first steps were added (§1.4).
- **#72:** an RE-before-implementing section was added (§1.5).
- **#569:** #278/#279 are now closed.
- **#462 / #464:** owner approval of the fixes is recorded.

Fold comments went to #570 (from #462, C-11) and #567 (from #465, F-02..07). **#265 was held**,
because rewrite vs. close is an open owner decision (§4). Proposed label changes in the findings
were **not** applied.

**2026-09-25: seven new agent-ready tickets filed** from §5, each researched against `main` @
acbcc22e to the body contract in `docs/agents/issue-tracker.md`:

| Issue | Ticket | From |
|---|---|---|
| #798 | Buyback MoveItem dupe | §5.1 |
| #799 | Respawn / callForAid on a living player, or to a respawner not offered | §5.2 |
| #800 | Four live-DB sentinel collisions, plus a uniqueness lint | §5.3 |
| #801 | method_idx ↔ entities/defs conformance test | §5.7 |
| #802 | Enforce `content_triggers.once` | §5.4 |
| #803 | Delete dead `crates/game` commands/ and missions/ | §5.8 |
| #804 | Docs drift: nine stale claims | §5.10 |

- Cross-link comments were posted on #464, #462, #458, #213, #171, #265 and #310, and the verified
  scope additions were posted on #295.
- `ready-for-agent` was **not** applied (maintainer-only, per `docs/agents/triage-labels.md`).
- Not filed: §5.5 (waits on #265), §5.6 (now in the #690 body), §5.9 (Cover Stance magnitude
  undecided).
- Found during this research, not yet filed:
  - The `EF_*` flag values in `crates/entity/src/abilities/defs.rs:56-62` disagree with the game
    enum. For example, `EF_DONT_USE_QR = 32` where `Atrea/enums.py:772` has 16.
  - `UseInventoryItem` and `remove_by_type` accept items sitting in buyback (16).
- **#784 was closed by PR #797** (NA27) after the triage; its §4 question is moot.

**Status:** research done; owner review in progress. §1 records the owner's answers so far. §4
(owner decisions) has not been reviewed yet. **No GitHub action until the whole review is
finished**; a fresh session then executes per [README.md](README.md).

**Totals at research time:** 45 CLOSE · 50 REWRITE · 16 KEEP · 12 NEEDS-OWNER. After the owner's
answers: 39 CLOSE · 50 REWRITE · 22 KEEP · 12 NEEDS-OWNER (#684-#689 moved from CLOSE to KEEP, see
§1.7).

Batches: `sec-a` (#459-468), `sec-b` (#469-477, 443, 439, 447, 448, 434, 532, 294), `wire`,
`content`, `npc-combat`, `features`, `tooling`, `debt`. To find an issue's section:
`grep -n "^## #<N> " findings/*.md`.

Where this file and a findings file disagree, **this file wins**.

---

## 1. Owner answers and follow-up research (2026-09-25)

### 1.1 Admin port 8443 on the colo (#439, #25) — not internet-exposed

**Owner:** 8443 is not exposed to the internet from the colo; it is fine.

**Effect:** the P0 rating came from reading `docker/compose.yml:64` (`8443:8443`, all interfaces)
without the colo's network edge, which the repo cannot see. The owner's answer overrides it.

- **#439:** still REWRITE, now **P3**. Threat model: reachable only from the colo's private side.
  Remaining work is defense in depth: publish 8443 on `127.0.0.1` (or the WireGuard address) in
  compose so the protection lives in the repo and not only at the edge, and delete the dead
  `/api/editor/*` routes (no caller since 62a6c0ee; that also closes #30). Rewrite the findings
  text accordingly; drop the "live P0" wording.
- **#25** (admin JWT): back to **P3**.

### 1.2 Buyback money dupe (#464) — approved: fix

**Owner:** fix it.

- **Bug:** MoveItem validates only the destination container
  (`crates/services/src/base/world_entry/methods/inventory/move_/mod.rs:162-164`). A sold item sits
  in the buyback container (16); moving it back into the bag skips the buyback price, so the
  player keeps both the item and the sale money. Found by code reading; not yet reproduced in game.
- **Fix shape:** reject any MoveItem whose *source* is the buyback container (and any other
  vendor-owned container). Buyback must go through the vendor buyback handler, which charges the
  price. Send the client visible feedback on reject (first-press rule).
- **Tests:** a regression guard that fails with the fix reverted: sell, then MoveItem 16 → bag, then
  assert the item stayed in 16 and the cash is unchanged. Use a live-DB test if the move persists
  through SQL.
- **Tracking:** file a standalone `bug`,`security`,`ready-for-agent` issue and link it from the
  #464 rewrite.

### 1.3 Respawn / callForAid on a living player (#462) — approved: fix

**Owner:** fix it.

- **Bug:** `callForAid` (`crates/services/src/cell/cell_methods/player/combat/mod.rs:35-76`)
  and `respawn` (`:160-164`) call `respawn::handle_respawn` with no `BSF_DEAD` check. A living
  player gets a full heal and a teleport to any respawner.
- **Fix shape:** gate both dispatch arms on the server-side `BSF_DEAD` state. The check goes in the
  arms, not in `handle_respawn`, because GM `gmRespawn` (`combat/respawn.rs:74-76`) shares it. Restrict the respawner choice to the respawners the player is allowed to
  use (world-scoped today; #233 adds `known_respawners`). Log a warn on a reject.
- **Tests:** a regression guard with a living player calling each method: assert no heal, no move.
- **Tracking:** fold into the #462 rewrite (C-residual) or file standalone; standalone is preferred
  so it can ship alone.

### 1.4 Ammo mismatch (#534, #448, PR #602) — approved: fix after further research

**Owner:** fix it after further research.

- **Claim (decompile, this triage):** the client sends the weapon's **instance** id as
  `requestAmmoChange.ItemId`; the server matches it against the **design** id
  (`cell/cell_methods/inventory/bandolier/ammo_change.rs:84-120`), so ammo-type swaps are
  probably silently ignored.
- **Research done since:** SigNoz, colo, last 30 days: **zero** log lines mentioning
  `requestAmmoChange` or `not in bandolier`. The miss path logs at `warn` (`ammo_change.rs:106`),
  so a failed swap would have shown up. The most likely reading is that nobody has tried an
  ammo-type swap on the colo, so there is **no field evidence either way**.
- **Research still needed before the fix:**
  1. A local in-game repro: equip a weapon with 2+ allowed ammo types and switch ammo. Watch the
     server log for `requestAmmoChange` and `item not in bandolier`, and note whether the ammo
     actually changes.
  2. `/re-verify` the two decompile anchors: `0x00e1ee10` (sends `item+0x0C`) and the item ctor
     `0x00d21750` (`+0x0C = id`).
  3. Check that the Castle/Cellblock loadouts include a weapon with more than one ammo type. If
     none does, the bug is real but currently unreachable, and the priority drops.
- **Fix shape once confirmed:** match on `BandolierItem.instance_id` (exists since #520); validate
  against the weapon def of *that* item; fail closed on a def-cache miss (this absorbs #448 /
  #463 D-06). Close PR #602 as superseded; it keeps the wrong key.

### 1.5 Mail (#72, #466) — approved if we know enough

**Owner:** fix mail if we know enough about it.

**Assessment: almost.** What we know:

- **Wire formats are documented** (`docs/reverse-engineering/findings/mail-wire-formats.md`):
  `sendMailMessage(RecipientFlags, Recipients, Subject, Body, Cash, bCOD, ItemId, ItemQuantity)`,
  `sendMailResult(ResultCode UINT8, FailedRecipients, FailedRecipientFlags)`, and the
  `MessageAttachment` layout (id, itemId, stackSize, durability, charges; 20 bytes).
- The read path (headers, body, archive, delete) works, with live-DB tests.
- The `sgw_gate_mail` table has `cash`, `item_id` and `flags`.
- PR #586 (unmerged) has a reusable `send_mail_to_player` helper.

What we don't know:

- **What the `ResultCode` values mean.** The client handler is `onSendMailResult` @ `0x00d7d060`
  (`docs/analysis/event-net-mapping.md:571`); the client's mail UI Lua probably holds the message
  strings (same approach that settled the contact-list eventIds via `Social.lua`).
- **What the header `flags` bits mean** for COD / has-cash / has-item, and **what `RecipientFlags`
  means**.
- Whether a `MessageAttachment` must be sent before `takeItemFromMailMessage` is enabled in the UI.
- **There is no behavioral reference:** the 2009 Python `sendMailMessage` is `pass` and the other
  handlers only `print`.

**Plan:** a short RE packet first (decompile `0x00d7d060` and the header-flags consumer; read the
client mail Lua). Then implement, server-authoritative with no client patch:

1. send, with `sendMailResult` on every outcome (first-press feedback) and `onNewMail` to online
   recipients;
2. take cash, take item, return, and pay COD, each as an atomic SQL transaction;
3. the #466 CAT-G findings as acceptance criteria.

Also correct `docs/gap-analysis.md` §24 and the false 2026-05-27 triage comment on #72.

### 1.6 Radio item (#334, PR #605) — owner unsure; explanation and recommendation

**What it is.** SGC_W1 mission 1561 starts when the player *uses* the Radio (item 5168); the
chains are 3020/3021/3025 in `sgc_w1_chains.sql`. #334 assumed the client offers "Use" only when
our `items_event_sets` table has a row, and PR #605 adds that row. But the client never sees that
table. It decides "Use" from its own local item data (`CookedDataItems.pak`), where the Radio has
`<ItemEventSet AbilityID="0" EventID="5">`. Other quest radios that work (1937 "Banged-up Radio",
1893, 2819) have `AbilityID="597"`. So PR #605 on its own changes nothing the player sees.

**Options, cheapest first:**

1. **Verify first (5 minutes in game):** give yourself item 5168 and right-click it. If "Use" is
   already offered, `AbilityID=0` does not hide it: the bug is elsewhere, and #334 should be
   rewritten around what actually fails. 507 cooked items have `EventID=5, AbilityID=0`, so this
   is a genuinely open question.
2. **If "Use" is missing:** push a server-side cooked-item override that sets `_5168`'s
   `AbilityID` to 597. The server already delivers per-key cooked-data overrides to clients
   (#755). `base/item_overrides.rs` only patches icon and stack size today, so it needs extending
   to `ItemEventSet`. No client patch is needed.
3. **Alternative:** re-trigger mission 1561 from something other than Use (item acquired, region
   entered, NPC dialog). This is a content change, but it departs from the original design.

**Recommendation:** do 1; if Use is missing, do 2 and ship PR #605's row alongside for
consistency. Otherwise close PR #605. Priority follows SGC_W1 (#335), which is P2 but not in an
active campaign.

### 1.7 Live research lab (#684-#689, #690) — not tested; keep open

**Owner:** #684-#689 haven't been tested yet; can we wire in the server side of the MCP server and
see if it works?

**Change of verdict:** #684-#689 move from CLOSE to **KEEP** until each phase's exit criterion
passes live. Post a status comment on each one ("code merged in PR #692/#696/#706/#693/#695/#703,
awaiting live validation"), then close each after validation. #690 stays REWRITE.

**The server side is already wired**, but it is off by default:

- `crates/server/src/main.rs:216-222` calls `cimmeria_lab_mcp::spawn_if_configured(...)`. It is
  fail-closed: it starts only if `CIMMERIA_LAB_MCP_BIND` **and** `CIMMERIA_LAB_MCP_TOKEN`
  (≥ 32 bytes) are set. It runs on its own listener, not the admin router.
- **Local blockers found:**
  - The root `cimmeria-server.exe` is dated 2026-06-18, three months older than lab-mcp
    (PR #693, 2026-09-19), so it needs a rebuild.
  - The local `.mcp.json` has no `lab-server` entry (only cimmeria-rag, ghidra, x64dbg, signoz).
  - No server is running locally.

**Local test recipe** (for the owner or the fresh session):

1. Rebuild via `setup.ps1` (the owner runs builds).
2. Set user env vars `CIMMERIA_LAB_MCP_BIND=127.0.0.1:8444` and
   `CIMMERIA_LAB_MCP_TOKEN=<64 hex chars>` so that the server launched by setup inherits them.
3. Add the `lab-server` http entry from `.mcp.json.example:74-80`, with the same token, to
   `.mcp.json`.
4. Start the server, then start a **new** Claude Code session (MCP servers load at session start).
5. Run the #687 exit checks: `server_sessions`; a `.`-console passthrough (for example a GM
   `.where`); `server_logs`; `server_db_query` with a SELECT that succeeds and an UPDATE that is
   rejected; a bad token that gets a 401.
6. Then run #688 (LabQuery entity/witness reads with a client logged in; the Cellblock corpse
   check).
7. The client-side phases #684/#685/#686 also need the telemetry DLL built `--features lab-bridge`
   and `cargo build -p cimmeria-lab`. Those are a second step.

The colo alternative is the `docker/compose.lab.yml` overlay over WireGuard
(`docs/operations/colo-deploy.md:150-167`).

**Known defect to fix during validation:** `drain_event_ring()`
(`crates/lab/src/timeline/client_events.rs:68`) returns empty, so the merged timeline only shows
heartbeats (#690).

### 1.8 Mercury window size (#353, #298) — owner unsure; explanation

**What it is.** Mercury (the game's UDP protocol) resends "reliable" packets until the other side
acknowledges them. The server lets at most **32** reliable packets be in flight unacknowledged
(`TX_WINDOW_SIZE = 32`, `crates/mercury/src/lib.rs:73-90`). Anything beyond that waits in a
deferred-send queue (#357). #353 assumed the *client* can only track 32, and proposed patching
`SGW.exe` to raise it to 64.

**What the triage found.** The decompile shows the client's channel constructor sets its window to
**512** (`Channel+0x2c = 0x200` @ `0x01576bf0`). The proposed patch would therefore *shrink* the
client from 512 to 64. The "32-bit ack bitmap" the 32 was based on cites a function that is really
a length decoder.

**Why it matters.** During bursts (zone-in, inventory and mission dumps) the server is throttled to
32 packets per round trip. On the colo's latency that stretches load time. The data still arrives
(nothing is lost since #357); it just queues.

**What to do.** Re-scope #353 to "raise the server's `TX_WINDOW_SIZE` (for example to 128 or 256),
no client patch":

1. Confirm by RE that nothing overwrites `Channel+0x2c` after construction.
2. Update the pinned test `tx_window_size_pinned_until_client_patch_widens_slot_store`.
3. Correct the draft-chapter ack-bitmap claims.

Priority **P3** (downgraded from the agent's P2), because nothing is broken today. Close #298 as
the same finding.

### 1.9 Lab MCP on the colo (#687/#688 validation) — owner: run it from the colo

**Owner:** run the lab MCP from the colo instance. Can we connect securely over the internet, or
should we use the colo's private IP?

**Answer: use the existing VPN; no internet exposure is needed.** The owner's dev box already
has a WireGuard tunnel to the colo. It routes the colo's private subnets, and the colo side of the
tunnel answered ping on 2026-09-25. Addresses, subnets and adapter names are deliberately kept out
of this public repo; the operator has them.

That matches the settled ADR decision (colo lab port reachable over WireGuard only;
`docs/architecture/live-research-lab.md`). Plain `http://` is acceptable inside the tunnel because
WireGuard encrypts it, and the bearer token is a second gate.

**Over the internet** would be possible the way SigNoz is exposed: a Cloudflare Tunnel plus a
Cloudflare Access service token (`docs/operations/signoz-remote-access.md`), giving HTTPS and two
secrets. It is **not recommended**: it reverses the ADR decision (an amendment would be needed),
and it adds a public hostname for a tool that can run GM console commands, for no gain while the
VPN works.

**Colo Docker host:** the owner supplied its private, VPN-side address on 2026-09-25 (written
`<colo-vpn-ip>` below), and it answered ping over the tunnel. So on the colo,
`CIMMERIA_WG_IP=<colo-vpn-ip>`, and the dev-box URL is `http://<colo-vpn-ip>:8444/mcp`. The
owner's local `.mcp.json` defaults to that URL
(`${CIMMERIA_LAB_MCP_URL:-http://<colo-vpn-ip>:8444/mcp}`), so only the token env var is required
on the dev box. Port 8444 did not answer at that time, as expected before the lab overlay is
deployed.

**What the colo compose needs.** The overlay `docker/compose.lab.yml` already exists and needs no
structural change. The token-generation commands were added to its header comment on this branch.

1. **Image:** a release that contains lab-mcp (PR #693/#695, merged 2026-09-19). The 2026-09-25
   release (#790) qualifies.
2. **`CIMMERIA_WG_IP`:** an address the colo host itself owns, reachable over the VPN. Docker
   refuses to publish on an IP the host doesn't have. Never `0.0.0.0` or the public IP; the `:?`
   guard refuses an unset value.
3. **`CIMMERIA_LAB_MCP_TOKEN`:** 64 hex characters from the generator command. Put both variables
   in `/opt/cimmeria/.env` (`chmod 0600`), not only in a shell `export`, so that a later manual
   `docker compose up -d` still has them. Compose reads `.env` automatically. Watchtower image
   swaps keep the container's existing env.
4. **Deploy command:**
   `docker compose -f compose.yml [-f compose.discord.yml] -f compose.lab.yml up -d`. Any later
   `up` must repeat `-f compose.lab.yml`, or the port is unpublished again (fail-closed).
5. **Host firewall:** allow TCP 8444 from the VPN subnet only.
6. **Audit:** every tool call emits a `lab.tool_call` log line to SigNoz. Touch only the lab
   character and what it spawns (colo-deploy.md). The colo DB is rebuilt from the seed on each
   deploy, so accidental state changes don't persist.

**Token generation** (PowerShell, dev box; also in the `compose.lab.yml` header):

```powershell
$b = New-Object byte[] 32; [System.Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($b); -join ($b | ForEach-Object { $_.ToString('x2') })
```

This was verified on 2026-09-25 to print 64 lowercase hex characters in both Windows PowerShell
5.1 and pwsh 7. Never commit the token.

**Dev-box side (done 2026-09-25):**

- The local `.mcp.json` (gitignored) now has a `lab-server` entry.
- `.mcp.json.example` on this branch uses the same shape:
  `"url": "${CIMMERIA_LAB_MCP_URL:-http://127.0.0.1:8444/mcp}"` and
  `"Authorization": "Bearer ${CIMMERIA_LAB_MCP_TOKEN}"`. Claude Code expands these from the
  environment, so the token never sits in a file.
- The owner sets the token user env var (the URL defaults to the colo host locally):

  ```powershell
  [Environment]::SetEnvironmentVariable('CIMMERIA_LAB_MCP_TOKEN', '<token>', 'User')
  ```

- Then restart the terminal and Claude Code (both env vars and MCP servers are read at startup).
- A quick reachability check before starting Claude Code:
  `Test-NetConnection <colo-vpn-ip> -Port 8444`.

**Then** run the #687 and #688 exit checks from §1.7 against the colo, and record the results on
each issue before closing it.

---

## 2. Remaining high-priority items (after §1)

| # | Verdict | Priority | Why |
|---|---|---|---|
| 464 | REWRITE | P1 | Buyback dupe (§1.2) plus the six vendor findings; E-01 is free repair via an omitted field. |
| 462 | REWRITE | P1 | Respawn exploit (§1.3); casting while stunned is now reachable. |
| 534 | REWRITE | P1 → confirm | §1.4; priority depends on the repro. |
| 72 | REWRITE | P2 | Mail Send silently no-ops (§1.5). |

## 3. Cross-batch reconciliations (apply these over the per-batch text)

1. **Speed-cap enforcement had no home.** npc-combat closes #63 → #443; sec-b closes #443 →
   #63/#461. **Resolution:** close both #63 and #443 as completed, and add this checklist line to
   the #461 new body: "Speed layer is warn-only (`cell/space_manager/client_move.rs:351-388`);
   calibrate `SPEED_WARN_TOLERANCE` from SigNoz and switch to enforce." Edit both closing comments
   to say "tracked in #461".
2. **#534, #448 and PR #602 are one fix** (§1.4).
3. **#466 → #72.** Close #466 and post its correction on #72 (§1.5).
4. **Umbrella #459** (sec-a draft) leaves the #469-#474 boxes unchecked. Update it from the sec-b
   verdicts: #472 closed into #568/#569, the others rewritten with residual lists. Also note that
   the #460-#474 bodies link the old audit branch; the audit files are on main now.
5. **Folds:**
   - #465 F-02..07 → #567; #468 → #571; #472 → #568/#569; #470 K-07 → #532
   - #476, and #477's zero-IV item → #434
   - #69 → #570
   - #30 route deletion → #439
   - #277 → #723; #276 → #727
   - #298 is the same finding as #353 (§1.8)
6. **#165:** npc-combat closes it (fixed by #520/#445); sec-a agrees.
7. **#269:** the features batch speculated partial; the content batch confirms it is fully done.
   CLOSE.
8. **#439 and #25** priorities are overridden by §1.1.
9. **#684-#689** verdicts are overridden by §1.7; colo validation setup is in §1.9.
10. **#353** priority is overridden by §1.8.

## 4. Owner decisions needed — NOT YET REVIEWED

Standalone NEEDS-OWNER issues:

| # | Question |
|---|---|
| ~~784~~ | Resolved: closed by PR #797 (NA27) on 2026-09-25. |
| 294 | Is msg 0x01 a static ticket or a rotating token? If static, close into #477. |
| 610 | How do effect chains bind: by effect id, by script class, or disable the demo chains? Unblocks PR #745. |
| 587 | Ship the Black Market client patch? Which vehicle: launcher, or the client-launch/telemetry DLL? Note that "server already sends onBMOpen" is only true on the unmerged #586 branch, and a method-90-only patch opens an empty window. |
| 480 | Close as superseded by the lab, or rewrite as a lab follow-up? (The tls-shim base was never built.) |
| 393 | Should the Atrea-editor MCP fold into the lab bridge? May it depend on AtreaRL? It also needs a new port (8765 is taken). |
| 27 | Drop the WebSocket entity-property stream? Nothing emits and nothing consumes; the lab covers the use case. |
| 261 | Is per-request trace propagation across the base↔cell mpsc still wanted, given OTLP and identity fields? |
| 264 / 262 / 282 | Continue or shelve the Bible program? One answer settles all three (the recommendation is to close #262 and #282 as not planned). #295 and #318 relate; #318 closes regardless. |
| 213 | Formally adopt "client cooked data is canonical, with conformance tests pinning Rust constants"? |

Questions embedded in other verdicts:

- **Stale PRs:** rebase, re-cut or drop PR #586 (BM, #571) and PR #584 (orgs, #568)? Both have
  conflicted since June. PR #585 (ignore enforcement) is also stale.
- #470 K-01: placeholder minigames win on the client's say-so. Is that acceptable?
- #463 D-03: what are the loot ownership rules?
- #464 E-01: did the original 2009 game have a free-repair path?
- #460: when should IP binding (#738) move from warn to reject?
- #310 (+ Castle GC3): what is the mission XP formula? Every seeded `reward_xp`/`reward_naq` is 0.
- #265: rewrite with the updated checklist, or close as not planned?
- #62: what value should `TimeToAid` (auto-release timer, currently a hardcoded 30 s) take?
- NA follow-up: Cover Stance hit modifier, +100 or +200?

## 5. New tickets worth filing (found during research, not covered by any open issue)

1. **MoveItem buyback dupe** (P1, approved for fix, §1.2).
2. **Respawn / callForAid on a living player** (P1, approved for fix, §1.3), if not folded into
   #462.
3. **Live-DB sentinel collision `0x7000_0500`** in `inventory/core/resync_tests.rs:26` and
   `mail/tests.rs:17`. Latent; masked by serialized execution. P3, `ready-for-agent`.
4. **`content_triggers.once` is loaded but never enforced**, so fire-once chains can re-fire. It
   is in the #265 checklist (C3); worth a standalone P2 bug.
5. **Eight content actions have no executor** (StartTimer, CancelTimer, PlayAnimation, PlaySound,
   ModifyProperty, RollLootTable, SpawnLootBag, ExecuteCustom); they are logged and skipped. In
   #265 section E.
6. **Lab timeline `drain_event_ring()` returns empty** (§1.7). It is in the #690 rewrite.
7. **method_idx ↔ `.def` conformance test.** Both #458 (G12) and #213 want it; track it once.
8. **Dead code in `crates/game`**: `commands/` is unregistered, and `missions/{rewards,manager}.rs`
   have `todo!()`. A cleanup like #699.
9. **Cover Stance hit-resolution effect**, and the owner's crouch-pose experiment (NA hand-off).
10. **Docs-drift PR** (one docs PR):
    - `docs/gap-analysis.md`: :268 claims `EF_ClearOnDeath` is wired (it isn't); :436-438 spawn
      caps marked working (they aren't); §24 says mail Send is implemented (it's a stub).
    - `docs/gameplay/npc-ai.md:626` still says the movement-type byte broadcast is live (NA10
      suppressed it).
    - `sgwplayer-base-method-dispatch-table.md:42-43` is missing the `aFlag`/`aPlayerNick` args
      for chatIgnore/chatFriend.
    - `pet-wire-formats.md`: INT32 should be INT8.
    - `black-market-client-window-patch.md`: "all six are sent" is true only on #586.
    - `docs/content/content-engine.md` ("11 seeded system_message rows"; there is 1) and
      `mission-chains.md` (SGC_W1 list).
    - Mercury draft chapter: flag bits, sub-slot threshold, createEntity size, length escape
      (#295), the ack-bitmap citation `0x0158b2d0` (#353, §1.8); entity-property-sync
      createBasePlayer filter (#314).

## 6. Open PRs touched

| PR | State | Recommendation |
|---|---|---|
| #604 | mergeable, CI green | Merge; closes #447. |
| #602 | conflicting | Supersede with the #534 + #448 fix (§1.4). |
| #605 | open | Depends on the §1.6 in-game check. |
| #745 | held | Blocked on the #610 decision. |
| #718 | awaiting review/UAT | Implements #271; land it before #720. |
| #586 / #584 / #585 | stale since June | Owner call (§4). |

## 7. Per-issue verdict index (after §1)

**CLOSE — completed (23):** #63, #165, #190, #191, #209, #242, #269, #276, #277, #278, #279,
#293, #318, #406, #407, #443, #476, #48, #616, #64, #74, #76, #205.

**CLOSE — not planned (16):** #466, #468, #472 (folds), #69 (dup #570), #359, #298, #302, #301,
#300, #299, #297, #266, #75, #30, #18, #166.

**REWRITE (50):** #459, #460, #461, #462, #463, #464, #465, #467, #439, #434, #477, #532, #469,
#470, #471, #473, #474, #353, #295, #171, #727, #534, #314, #334, #310, #268, #265, #233, #330,
#62, #46, #572, #569, #72, #66, #690, #417, #29, #28, #26, #25, #16, #531, #529, #458, #416, #304,
#281, #246, #167.

**KEEP (22):**

- #447, #448, #735, #733, #316, #720, #271
- #715 (+`ready-for-human`), #612, #335, #723 (`needs-triage`→`needs-info`), #567
- #571, #570, #568, #484
- #684, #685, #686, #687, #688, #689: status comment only, until live validation (§1.7)

**NEEDS-OWNER (12):** #784, #294, #610, #587, #480, #393, #27, #261, #282, #264, #262, #213.

The one-line reasons per issue are in each findings file's `## Batch summary` table.
