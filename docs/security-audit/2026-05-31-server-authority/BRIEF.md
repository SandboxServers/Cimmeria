# Server-Authority Audit — Agent Brief

You are auditing the Cimmeria SGW server emulator for server-authority / anti-cheat /
anti-replay gaps. This brief is shared across all per-category agents.

## Hard rules for evidence

A finding may ONLY cite as authoritative:

1. **Ghidra MCP** — decompiled SGW.exe / STBC.exe / AtreaRL.dll. See the inventory of
   client-side `Event_NetOut_*` classes in `.scratch/audit/surface.md`.
2. **Client install behavioral evidence** at `C:\Users\Steve\source\projects\sgw\Stargate Worlds-QA\`:
   - `Working/binaries/SGWDebugLog.log` — most recent client-run debug log
   - `Working/binaries/CrashDumps/` — crash dump tree (currently mostly empty)
   - `Working/SGWGame/Logs/` — runtime log dir (mostly empty)
   - `Common/xml/SGWShared/CookedData/*.xsd` — schema of cooked client data
3. **Direct client code references** — the SGW client source isn't checked in, only the
   compiled binary. So this means Ghidra-decompiled C++.

A finding MAY NOT cite as authoritative:

- The deprecated Python in `deprecated/python/`
- The Rust server code in `crates/`
- The PostgreSQL schema in `db/`
- Any markdown docs in `docs/`

You can REFERENCE these to identify where to look or what's currently implemented,
but the *truth* about what the client sends / accepts must come from Ghidra or
client-side behavioral evidence.

## What "demonstrable + likely-exploitable theoretical" means

A finding is reportable if either:

- (a) **Demonstrable**: Ghidra shows the client constructing field X from local state
  Y with no client-side bounds check, AND the Rust server reads X and acts on it
  without server-side validation. The attack is: a modified or replayed client
  sends X = `<adversary value>` and the server obeys.

- (b) **Likely-exploitable theoretical**: The Rust handler trusts a client-supplied
  field that obviously should be authoritative server-side (e.g. price, target
  entity ID, currency type, slot index, GM flag). You may not have a full
  Ghidra trace, but the trust violation is clear from the wire shape and
  the absence of validation. Flag with "needs live debugger to confirm at
  triage time."

Don't file:

- Speculative holes with no observable client trigger and no obvious wire shape.
- Generic "what if attacker sends malformed bytes" without a specific bad value.
- Code-quality issues that aren't security relevant.

## What to produce per finding

A markdown block in this format (write to `.scratch/audit/findings/<category>.md`,
one per category):

```
### CAT-X-NN — <short title>

**Severity**: Critical | High | Medium | Low
**Class**: <attack class — e.g. "GM auth bypass", "TOCTOU", "missing range check">
**Wire surface**: <client message name(s) — e.g. `Event_NetOut_UseAbility`>
**Demonstrable / Likely-theoretical**: <one of the two>

**Trust violation**
<one paragraph — what the client sends, what the server trusts, why it's wrong>

**Evidence**
- Ghidra: `<addr>` `<symbol>` — <decoded payload shape OR observed client behavior>
- Client behavioral log: <file:line OR "n/a">
- Cross-ref to Rust handler (for the fix author, NOT as truth): `<file>:<line>`

**Attack scenario**
1. <step>
2. <step>
3. Observable effect on the server

**Suggested remediation (one line)**
<terse: "validate server-side X" or "drop client-supplied Y and recompute from Z">

**Would benefit from x64dbg trace?**
Yes / No — <one-line reason if yes>
```

## Output discipline

- ONE markdown file per category at `.scratch/audit/findings/<category>.md`
- File starts with a one-paragraph summary of the category's overall trust posture
- Then lists findings in the format above, numbered CAT-X-01, CAT-X-02, etc.
- At the END of the file, a "Not Filed" section listing things you considered but
  decided didn't meet the bar, with a one-line "why not filed" each. This is the
  user's "let me know what you decide not to do" deliverable.
- DO NOT write any commits, edits, or PR comments. Findings are local-only.
- DO NOT file GitHub issues — that batch step happens later in main thread.

## Tools you have

- Read / Glob / Grep / Bash — for surveying Rust code + ../sgw/ binaries
- Ghidra MCP — `mcp__ghidra__*` tools. SGW.exe is open as the active program at
  `C:/Users/Steve/source/projects/SGW/Stargate Worlds-QA/Working/binaries/SGW.exe`.
  173,223 functions, 634,156 symbols available.

Tools you should AVOID:
- Edit / Write to source files (audit is read-only; only `.scratch/audit/**` writes)
- Any git operations
- Any GitHub gh commands

## Scope of this audit

Categories (one agent per category):

- CAT-A: Auth / Session / Character lifecycle / Disconnect-LogOff
- CAT-B: Movement / Teleport / Position / Crouched / WeaponState / Unstuck
- CAT-C: Combat / Abilities / UseAbility / Pet abilities / Respawn / SetTarget
- CAT-D: Inventory / Items / MoveItem / RequestAmmoChange / Bandolier / UseItem
- CAT-E: Vendor / PurchaseItems / SellItems / BuybackItems / RepairItems / RechargeItems
- CAT-F: Crafting / Craft / Alloy / Research / ReverseEngineer / TrainAbility
- CAT-G: Mail / Send / Take / PayCOD / Archive / Delete
- CAT-H: Trade P2P / TradeProposal / TradeLockState
- CAT-I: Black Market / BMCreateAuction / BMPlaceBid / BMSearch
- CAT-J: Mission / Dialog / Interact / DialogButtonChoice / MissionAdvance/Reset/Complete
- CAT-K: Minigame / Start / End / Complete / Spectate
- CAT-L: Chat / Contact list / sendPlayerCommunication / Petition / Who
- CAT-M: Organization / Squad / Duel
- CAT-N: GM debug commands (Set*/Give*/Show*/Spawn/Despawn/Kill/Goto*/Summon/etc.)
- CAT-O: World / Space / GateTravel / RingTransporter / DHD / WorldInstanceReset

Each agent gets its category. CAT-N (GM commands) is the largest and most critical —
verify each Set*/Give*/Show*/Kill/Spawn/Summon path is gated by server-side GM check,
NOT by hiding the UI affordance.
