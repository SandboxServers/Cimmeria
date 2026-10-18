# HA-00c seal: static audit half

> Type: packet seal (docs-only audit). Sealed 2026-10-18 against campaign tip `9455f2af2` by the mission-systems advisor. Nothing here was run. This note is the contract; the deliverable is a later worknote that replaces it. Parent contract: [four-zone HA-00c](../../four-zone-planning/harset-work-packets.md). Precedent: [HA-00b](HA-00b.md). Ledger: [work-packets](../work-packets.md), [id-allocation](../id-allocation.md).

## 1. Scope split

HA-00c reserves authored and repeatable mission IDs. It cannot finish without the lab, because the first authored quest needs a client proof. The part that needs no client is sealed here. No ID is allocated by the static half: no numeric-gap picks, no census, carry-in or 1664 reuse, and no row is added to the claim tables.

| Part | Status after this seal |
|---|---|
| Stock catalog inspection (committed pak) | Sealed, executable now |
| Current active-branch collision audit | Sealed, executable now |
| Mission graph signoff list (nine graphs plus shared claims) | Sealed as a list; signatures stay with the signers |
| First authored quest client proof (journal, dialog, advance, relog) | BlockedEvidence: owner-run lab UAT |
| Premise, reward and reset decisions within O14-04, O14-10, O14-13 | Owner-manual |
| HA-17 and HA-18 | BlockedDependency on HA-00c |

## 2. Packet contract (static half)

| Item | Value |
|---|---|
| Executor | Sonnet or mission advisor. Not Haiku: it reads a binary container and interprets collisions |
| Branch / worktree | `harset/ha-00c-authored-ids` / `harset-ha-00c` |
| Commit subject | `Harset HA-00c: seal authored and repeatable IDs (static half)` |
| Base | campaign tip at dispatch, recorded in the worknote |
| Deliverable | `docs/analysis/harset-continuation/worknotes/HA-00c.md` (replace this seal with the audit result; keep the contract table) |

### Allowlist (docs only)

`docs/analysis/harset-continuation/worknotes/HA-00c.md`, `.../id-allocation.md`, `.../work-packets.md`, `.../README.md`, `.../RESUME.md`. Scratch scripts live under `.claude/reports/` (gitignored) and are not committed. No cargo, no lab or client, no seed or SQL edit, no `db/scripts`.

### Method

1. **Reader check first.** Read `data/cache/CookedDataMissions.pak` per [cooked-data-pak-format.md](../../../engine/cooked-data-pak-format.md) with a scratch reader. Before trusting it, reproduce the frozen [CB-N05a](../../four-zone-planning/cb-n05a-seal.md) numbers: SHA-256 `9A96ED8278957CF37E337367AE592C8D8155A0901F1061CA0D3873D3AAA570EA`, 749028 bytes, 1040 numeric entries, entry IDs 38 to 1826, MetaData 7538, 12,906 stock ID attributes, and schema witness `_1624` (1093 raw bytes, SHA-256 `9AFFDF20C17B238A8DE76B27001E7EFEA31D0EFE1B5AB5E141D723B859B0DE21`). Any mismatch stops the packet as `blocked`; do not tune the reader to fit.
2. **Stock census.** Per kind (mission, step, objective, task) list the attribute-ID min, max and gaps from the pak, and the seed counts for comparison (1041 missions, 3480 steps, 4037 objectives, 4358 tasks as of CB-N05a; recount from `db/resources/`, do not copy). Include dialog, DSM, item and chain-ID maxima from the seeds with the query or grep used.
3. **Claim tables.** Collect every ID already claimed: the 77 reviewed IDs, the 1664 interface, the 61000 to 61003 test reservation (1139-byte fixture), and the four-zone shared [id-allocation](../../four-zone-planning/id-allocation.md) rows. Output one merged table with owner, source and status.
4. **Collision scan.** Freeze the SHAs first and write them in the note. Scan every registered branch (baseline from HA-00b: 84 refs, 72 differing, 0 collisions; bible integration heads Castle `1ebdff94f`, Cellblock `457b4f75a`, Harset `37ffd858d`, Tollana `b105967cc`) with a **named-column** parser: quoted VALUES respected, `point_id` taken from the second column, never regex over whole lines. Reuse the scratch validator `.claude/reports/four-zone/validate-harset.ps1` (present in the zone-harset worktree) after reading it. Report the new ref count, differing count and collisions; zero is not assumed.
5. **Duplicate-key proof.** A scratch-only fixture with one deliberate duplicate must fail the scan and a clean copy must pass it. Record both outputs in the note.
6. **Signoff list.** Write a table of the nine Harset mission graphs (mission, steps, objective primitives used, reward path, repeatable flag, claimed IDs, open question) with a blank signature cell for the mission advisor, and one row for the coordinator's shared-claims signature. The static half fills everything except the signatures.

### Checks

Relative links and anchors resolve; CRLF line endings on every touched `docs/analysis` file; scoped `npx --yes markdownlint-cli2 --no-globs <files>`; `git -c core.whitespace=cr-at-eol diff --check`; file under the 500-line soft cap.

### Stop conditions

Reader check mismatch; a collision found (report it, allocate nothing, escalate to the coordinator); a branch that cannot be read; any need to edit outside the allowlist.

## 3. What this does not prove

It does not prove a client can display, accept or advance an authored quest; that is the BlockedEvidence item above and belongs to the owner's manual UAT. It does not decide any premise, reward or reset rule. The signoff list is a list: until the named signers sign, HA-00c stays open and HA-17 and HA-18 stay blocked.
