# Organizations: Session Resume

> Type: how-to. Audience: any later session and the owner.
> Updated: 2026-09-27 (ORG-11 close-out). Companions: [decisions, open questions and known gaps](../README.md), [work packets](../work-packets.md), [audit](../audit.md), [UAT guide](../../../guides/organizations-uat.md).

## State: feature-complete server-side, awaiting the release and the owner's UAT

The coordinator session was **cimmeria-1f**. The campaign's id block is entity templates 330-349 and spawns 430-449, and its branches were `org/*`. Every packet has merged; nothing is in flight.

| Packet | Status | PR |
|---|---|---|
| ORG-E1 | Integrated | #861 |
| ORG-01 | Integrated | #871 |
| ORG-02 | Integrated | #881 |
| ORG-03 | Integrated | #886 |
| ORG-04 | Integrated | #922 |
| ORG-05 | Integrated | #942 |
| ORG-06 | Integrated | #941 |
| ORG-07 | Integrated | #945 |
| ORG-08 | Integrated | #954 |
| ORG-09 | Integrated | #951 |
| ORG-10 | Integrated | #952 |
| ORG-11 | Integrated | the close-out PR (branch `docs/org-closeout`) |
| ORG-UAT | **Ready** | the owner runs it on the colo after the release |

The ledger PRs were #855, #857, #878 and #936. Squads, Teams and Commands work on the server and are covered by unit, wire-format, live-DB, fan-out and negative-log tests, plus two wireclient tests (`two_client_squad` and `two_client_command_invite`). None of it has been run in a real client yet, so every status doc marks it "not yet client-verified".

What the coordinator still does after this PR merges: comment `/release` on it (from PowerShell, or with `MSYS_NO_PATHCONV=1`), close #568 and #584 with a pointer to the [README](../README.md), and retire the `org-11` worktree with `bash tools/build-lane/rm-worktree.sh org-11`.

## After the owner's UAT: reading SigNoz

The owner follows [organizations-uat.md](../../../guides/organizations-uat.md), which gives the expected rows and the query for each step. To read a UAT run back, open the Logs explorer, filter `service.name = cimmeria-server`, and then:

| To see | Filter |
|---|---|
| Everything the campaign logged | `scope_name IN ('org','squad')` |
| One player's history | add `AND player_id = <id>`, ordered by time |
| Every refusal, and why | add `AND outcome = 'rejected'`, grouped by `event`, `reason` |
| One squad or organization | `squad_id = <id>` or `org_id = <id>` |
| Login restore | `event = 'org.login_restore' AND player_id = <id>` |
| GM test actions | `event = 'org.gm_action'` |
| Moments the owner flagged with `.bug <note>` | `scope_name = 'playtest.bookmark'` |

A step that shows no row at all is a bug in its own right: every handled action ends in exactly one outcome row (D-ORG24). Record each finding as a GitHub issue that cites the step, the row and the worknote.

## What a follow-up session picks up

1. **The owner's answers** to [README § Open questions for the owner](../README.md#open-questions-for-the-owner): the Leader renaming rank 8, refusing "Leader" as a lower rank's name, an officer channel for Teams, and inviting offline characters. Each answer becomes a new D-ORG row (the next free number is D-ORG29), then a small packet.
2. **UAT findings**, especially the client-behaviour questions in [README § Known gaps and follow-ups](../README.md#known-gaps-and-follow-ups): the registrar right-click and naming window, a doubled squad or org chat line, the invite commands' wire path, the rank editor's mask, and `/ReloadOrganizations`.
3. **Server-side follow-ups** from the same list: one read snapshot for the login push, a rate limit on CM 13 and 14, officer chat reading `OfficerChat` under the lock, `/squadpromote` once 0xD2 is confirmed, gate-transit gaps, the squad loot mode being applied to loot, and the splits of `entity_struct.rs` and `helpers/mod.rs`.

Open a GitHub issue per item before starting it, and keep this campaign's id block and sentinel map ([work-packets.md § Schema](../work-packets.md#contract-fixed-by-this-ledger)) for any new tests.

## Other campaigns

- **Bank / Vault** (cimmeria-79): owns the Team and Command vaults (BV-07: storage and open merged as #948; moves are #949, open at close-out) and the treasury (BV-08, which replaces the CM 19 "not available yet" arm). It builds on the [ORG-API](../work-packets.md#bank-campaign-api-org-api) and follows D-ORG28's lock order. It also replaces the `org_vault_is_empty` and `org_vault_is_empty_sql` stubs, after which the memberless-organization branch can get its live-DB test.
- **Social** (cimmeria-3d): owns the chat channel ids and `CHAN_*` (SS-C4, D-ORG26).
- **Black market and the stasis debug hub** (cimmeria-11): the registrar NPCs sit in the hub's slots (`docs/content/debug-hub.md`).
