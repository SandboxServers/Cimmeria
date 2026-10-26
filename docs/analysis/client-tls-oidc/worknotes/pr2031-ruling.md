# PR #2031 review: architect ruling

> **Type:** Reference (ruling note)
> **Audience:** The orchestrator, the owner, and the agents running waves 6-9
> **Last updated:** 2026-10-26
> **Companions:** [ledger](../README.md), [work packets](../work-packets.md), [contract C16-C18](../contract.md#c16-revocation-model-pr-2031-review-waves-6-and-7-ruling-2026-10-26), waves [6](../packets-wave6.md) [7](../packets-wave7.md) [8](../packets-wave8.md) [9](../packets-wave9.md)

The review of PR #2031 (head `8225135ef`, integration branch `client-tls-oidc/campaign`, worktree `cto-camp`) found 19 defects, F01-F19, and listed six owner items. This note rules on each: what gets fixed, by which packet, and what goes to the owner. It writes no code. The F-numbers are the review's, not the design's facts.

**Ruling in one line:** every finding is fixed before the PR merges. The fixes are 22 new packets in four waves (6-9) plus a tail, all merging into `cto-camp`, so the campaign still ships as **one PR, #2031**. One owner item (stored email authority) is a BlockedDecision. The live client, native keyring and rollout items stay release gates, as the PR already says.

## Findings

| # | Sev | Finding | Ruling | Packets |
|---|---|---|---|---|
| F01 | High | Rebuild over the old schema fails counting a table it lacks | Fix: a table the old schema lacks counts as 0. Add an upgrade-from-base case. | CT-70 |
| F02 | High | Concurrent refresh reuse can leave a successor token live | Fix: one advisory lock per refresh family, and access-token fences checked at use (C16.1, C16.3). | CT-71, CT-72 |
| F03 | High | Reuse detection leaves a minted launch ticket alive | Fix: `Rotation::Reused` revokes the family's access tokens and the account's launch tickets **before** it audits (C16.6). | CT-74 |
| F04 | High | A pending Add method restores access after revocation | Fix: a new family is bound to its identity (C16.2), and Add method carries its family, so a revoked family cannot add one (C16.5). | CT-73, CT-75 |
| F05 | High | Revocation misses SOAP and BaseApp handoffs already issued | Fix: tickets carry a guard checked at the SOAP handoff and again at BaseApp admission (C16.4). An established game connection is not cut (documented by CT-92). | CT-77, CT-78, CT-79 |
| F06 | High | A late browser or token completion signs the launcher back in after Sign out | Fix: an operation epoch, bumped by Sign out and Cancel under the state lock and checked before every store write and install (C17.2). The UI guard is CT-91. | CT-87, CT-91 |
| F07 | High | Adopted installs sign in to one server and launch another | Fix: sign-in and Play read one effective list (C18.2). D-CT21 is amended. | CT-90 |
| F08 | Med | Two sessions for one auth host rotate one refresh token | Fix: one `HostSlot` per auth host. Every `SignIn` for that host shares its lock (C17.1). Sign out forgets before it posts `/logout`. | CT-86 |
| F09 | Med | The manual-browser fallback shows a URL after closing its listener | Fix: a failed browser open parks the flow, and `resume` finishes it (C18.1). | CT-89, CT-91 |
| F10 | Med | Offline discovery hides Sign out | Fix: `available: false` with `signed_in` keeps Sign out (C18.1). | CT-91 |
| F11 | Med | An offline server switch keeps the previous host's account | Fix: offline always resolves the target's own host. A target with no auth base drops the session. | CT-86 |
| F12 | Med | `needs_link` has no way back | Fix: a **Back to sign in** button that sends `sign_out`. | CT-91 |
| F13 | Med | Case-insensitive recovery can pick the wrong account | Fix: (a) a folded-name lookup answers one account or none; (b) ticket verify checks every candidate. | CT-80, CT-77 |
| F14 | Med | Guard triggers have concurrent-write and UPDATE holes | Fix: the triggers lock the account row and cover UPDATE. Both reproduced shapes become live-DB tests. | CT-81 |
| F15 | Med | Generic OIDC always uses `client_secret_post` | Fix: a configurable `token_endpoint_auth_method`, Basic by default, checked against discovery (C18.3). | CT-83 |
| F16 | Low | The ledger and contract are not in the PR | Fix: commit this folder into the PR and repair the guide's links. | CT-92 |
| F17 | Med | Swallowed keyring failures lose rotated tokens and undo Sign out | Fix: a fallible store API, a session-only fallback and a tombstone, each shown as a panel warning (C17.3, D-CT25). Native proof stays CT-U6. | CT-88, CT-91 |
| F18 | Med | The relay's 14 tests never run in CI | Fix: add `cimmeria-launcher-relay` tests and all-target clippy to the desktop workflow. The report quotes the 14 names from the CI log. | CT-85 |
| F19 | Med | Live-DB cleanup deletes by broad predicates | Fix: registered sentinel slots, and delete by exact inserted id. Covers the three files the review names. | CT-82 |

## Owner items

| # | Item | Ruling |
|---|---|---|
| 1 | Historical email authority | **BlockedDecision D-CT23** (CT-94). The architect recommends refreshing or clearing the stored email and `email_verified_at` on every sign-in, and accepting only identities whose provider is trusted in the current config. Nothing in waves 6-9 depends on it. |
| 2 | OIDC claim policy | Decided by the architect as **D-CT24** (C18.3): `iat` is required and at most 60 s in the future; `azp` is required with several audiences and must equal the client id when present. Pinned with signed-token tests in CT-84. |
| 3 | Live client acceptance | A release gate, as the PR says: CT-04, CT-58 and CT-U1..U6 stay owed and lab-gated. It does not block the fix waves. |
| 4 | Native credential stores | A release gate: CT-U6 on packaged Windows and macOS, now including the CT-88 refusal paths. CI does not stub `keyring`. |
| 5 | Rollout and operation | CT-85 forwards every provider secret the example config names. The proxy route and peer narrowing are CT-52 (D-CT22). Registrations and secrets are CT-53 (owner). The privacy page, D-CT16 deletion and crash-left ticket files stay owner items. |
| 6 | CI narrower than its badge | Accepted as the repo's PR policy. PR #2031 must carry the `full-ci` label before it merges, so workspace nextest, live-DB and Python contracts run on its head. |

## Contract changes

All are in [contract.md](../contract.md).

- **C16** (waves 6-7): the revocation model. It has family locks, identity-bound families, access-token fences, launch-ticket guards checked at both handoffs, Add method carrying its family, and unlink revoking inside its transaction.
- **C17** (wave 9): launcher host sessions.
  - C17.1 is one slot per auth host.
  - C17.2 is the operation epoch. A server switch is not a supersede: an old-base completion is stored under its own host, but installed only while the target's auth base still matches.
  - C17.3 is the fallible store, D-CT25.
- **C18**: C18.1 adds `storage_warning`, `resume`, `no_pending_flow`, offline Sign out and Back to sign in. C18.2 is the effective server (D-CT21 amended). C18.3 covers the OIDC token-endpoint method and claims (D-CT24).

## Waves

Every packet merges into `client-tls-oidc/campaign` (`cto-camp`) with `merge-noff.ps1`. There is no wave PR and no packet PR: **one PR, #2031**.

| Wave | Packets | Opens | Merge order | Extra review |
|---|---|---|---|---|
| 6 Revocation, server half | CT-70..CT-76 | now | CT-70, 71, 72, 74, 73, 75, 76 | - |
| 7 Ticket guards, handoffs, lookup, triggers, cleanup | CT-77..CT-82 | CT-76 merged | CT-77, 80, 81, 82, 78, 79 | one `network-security-auth` pass over waves 6 and 7 together |
| 8 OIDC interop, relay CI | CT-83..CT-85 | now (parallel with 6) | CT-85, 83, 84 | - |
| 9 Launcher sessions and controls | CT-86..CT-91 | now (parallel with 6 and 8) | CT-90, 86, 87, 88, 89, 91 | one `network-security-auth` pass over wave 9 |
| Tail | CT-92 | waves 6-9 merged | CT-92, then the campaign review loop | - |

Waves 6 and 7 edit the identity, auth, base and db crates. Wave 8 edits identity's `oidc.rs`, the relay CI workflow and compose. Wave 9 edits only `crates/launcher/desktop/`. No two parallel packets edit one file, and every chain inside a wave is named in its file. No packet edits `worker.rs`, `login_port.rs` or `crates/server/src/main.rs`, which launcher-cache-presync owns.

Sizing:

- Every packet-coder packet is one module plus one new test file, about 40k working tokens.
- CT-86, CT-87 and CT-89 go to `rust-gameserver-dev`. They each restructure shared async state and are not safe for a Haiku session. CT-92 goes to `documentation-writer`.

## Decisions recorded

| ID | Decision | Status |
|---|---|---|
| D-CT21 | Sign-in uses the first server of the effective list Play writes | Amended 2026-10-26 (architect) |
| D-CT22 / D-MR6 | The malformed ledger row is split into its two decisions; both are unchanged | Ledger fix |
| D-CT23 | Stored email authority | **BlockedDecision (owner)** |
| D-CT24 | ID-token `iat` and `azp` policy | Decided (architect) |
| D-CT25 | Credential-store failure semantics | Decided (architect) |

## BlockedDecision and owner list

1. D-CT23: stored email authority (CT-94).
2. Live client UAT: CT-04, CT-58 and CT-U1..U6 (lab, owner go).
3. Native keyring proof on packaged Windows and macOS (CT-U6).
4. CT-53: provider registrations, secrets in `cimmeria-kv`, which providers are `trusted_email`, and the age gate.
5. CT-52: applying the proxy route and the D-CT22 narrowing on the colo (no new owner gate; pick a quiet time), then OPS-LT1.
6. The privacy policy page for open sign-up.
7. D-CT16: full account deletion.
8. Ticket files a crashed launcher leaves behind (`LoginInternal.lua` holding a still-valid ticket until it expires): accept the ticket lifetime as the bound, or add a startup sweep.

## Accepted, no packet

- The launcher summary's `first_login_server` copy in `summary.rs` stays. It feeds the anonymous adoption summary, not auth, so F07 does not apply to it.
- Live-DB tests outside the three files F19 names keep their cleanup. CT-82 fixes those three. The sentinel lint (`cimmeria-test-support`) already guards new tests.
- CT-93, a provider chooser for Sign in again, is Deferred. CT-91 gives the no-provider press visible feedback, which meets the button rule.
- The review's "What held up" list (proxy peer check, ticket digest, GM boundary, password clear on link, last-method lock, OIDC algorithm and nonce checks, log hygiene) needs no change.

## For the orchestrator

- **Dispatch now:** CT-70, 71, 72 and 74; CT-85 and 83; CT-90 and 91. Then follow each wave file's chain.
- The ledger has the details: [README](../README.md) packet rows CT-70..CT-94, the [work-packets](../work-packets.md) Waves table, and [RESUME](../RESUME.md).
- Ask the owner D-CT23 at any time. Nothing waits on it except CT-94.
