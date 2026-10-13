# Lab UAT reliability: 13 October 2026

This campaign fixes the lab MCP failures observed during the hub-pass and
Cellblock runs on 2026-10-13. It covers the lab harness, not the separate
product defects found by those runs. Each code packet has one worktree, one
Luna implementer, a read-only adversarial review, a local lane test and a PR.
The [packet contracts](work-packets.md) are the implementation brief.

## Scope and evidence

| Finding | Issue | Packet | Boundary |
|---|---|---|---|
| Typed prompt reader missing | [#1673](https://github.com/SandboxServers/Cimmeria/issues/1673) | LR-01 | Read-only UI |
| Covered button reports native success | [#1676](https://github.com/SandboxServers/Cimmeria/issues/1676) | LR-02 | Window click |
| World-load bridge failure aborts wait | [#1677](https://github.com/SandboxServers/Cimmeria/issues/1677) | LR-03 | Polling |
| Feedback metadata and relog welcome absent | [#1679](https://github.com/SandboxServers/Cimmeria/issues/1679) | LR-04a/b | Chat capture |
| Dead player confuses ensure; logout crashes | [#1680](https://github.com/SandboxServers/Cimmeria/issues/1680) | LR-05a/b | Session flows |
| Targeting stalls on mouse-over read | [#1684](https://github.com/SandboxServers/Cimmeria/issues/1684) | LR-06 | World hover |
| Ring map native click mismatch | [#1686](https://github.com/SandboxServers/Cimmeria/issues/1686) | LR-07 | Input provenance |
| Livewire board and real-input solve | [#1685](https://github.com/SandboxServers/Cimmeria/issues/1685) | CA-MG-L0..L5 | [Cellblock autoplay](../cellblock-autoplay/README.md) |
| Routable Cellblock movement | [#1689](https://github.com/SandboxServers/Cimmeria/issues/1689) | CA-AP-11 | [Cellblock autoplay](../cellblock-autoplay/README.md) |

The earlier [#1687](https://github.com/SandboxServers/Cimmeria/pull/1687)
merged a graded ring-map Lua fallback; it does not establish a native click
pass, so #1686 remains open. The Cellblock autoplay campaign already owns
the Livewire solver and route-design packets; this campaign will not fork
those implementations. Product issues #1674, #1675, #1678, #1681, #1682 and
#1688 have their own domain owners and are outside this lab-harness ledger.

## Decisions

| ID | Status | Decision | Reason |
|---|---|---|---|
| D-LR1 | Accepted | A tool must report an effect before claiming `native_pass`. | The covered-button and ring-map probes showed that dispatch is not effect. |
| D-LR2 | Accepted | A failed bridge poll during loading is retried only inside the caller's deadline. | A world load may block the client; a permanent bridge outage must still time out. |
| D-LR3 | Accepted | An unknown hover is never silently turned into a target success. | The target timeout fix must preserve UAT evidence. |
| D-LR4 | Accepted | Livewire and route packets stay in Cellblock autoplay. | Their existing design includes server state, board geometry and route evidence. |

## Packet ledger

Status vocabulary: Ready, BlockedDependency, BlockedDecision, Writing, Review,
Integrated, UATPending, Done. A packet is Done only after its PR and required
live validation; an integration test alone leaves it UATPending.

| Packet | Issue | Status | PR | Live proof |
|---|---|---|---|---|
| LR-01 prompt read | #1673 | Ready | | Hub prompt text/buttons |
| LR-02 covered click | #1676 | Ready | | Mail compose over inbox |
| LR-03 bridge wait | #1677 | Ready | | World load through `client_wait_for` |
| LR-04a feedback metadata | #1679 | Ready | | Channel 9 colour and tab |
| LR-04b relog capture | #1679 | BlockedDependency: LR-04a | | Welcome after relog |
| LR-05a dead player observation | #1680 | Ready | | Defeat window state |
| LR-05b safe logout | #1680 | BlockedDependency: LR-05a | | Logout after death |
| LR-06 target timeout | #1684 | Ready | | Hostile target while in combat |
| LR-07 native ring click | #1686 | Ready | | Fresh client mouse vs handler |
| CA-MG-L0..L5 | #1685 | BlockedDependency: autoplay prerequisites | | Livewire N1 solve |
| CA-AP-11 | #1689 | BlockedDependency: autoplay prerequisites | | Route to Mess Hall |

## Validation rules

All code packets run `cargo fmt -p cimmeria-lab --check`, then lane clippy and
lane tests for `cimmeria-lab`; focused tests must fail against the pre-fix
behavior. The coordinator checks the server log and packet tap for live claims
under the [lab UAT guide](../../guides/live-research-lab.md). Luna implementers
do not take a lab lease or deploy the daemon. One coordinator owns the live
client and releases its lease after each probe.
