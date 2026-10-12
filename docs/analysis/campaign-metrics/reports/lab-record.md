# lab-record

> Regime: mixed

### Stamp

- Regime: mixed
- Generated: 2026-10-12T19:03:48Z
- Window: 2026-09-21T00:00:00Z to 2026-10-13T00:00:00Z
- Price table: 2026-10-12
- Profiler schema: 6
- Unattributed share: 12.2%
- Unmapped share: 38.0%

Dollars are list-price estimates of plan usage, a floor, never a bill.

### Cost

|  | USD |
|---|---:|
| total | $95.40 |
| attributed | $95.40 |
| unattributed in campaign | $0.00 |

By model tier:

| model tier | USD |
|---|---:|
| fable | $0.00 |
| haiku | $0.13 |
| opus | $87.98 |
| other | $0.00 |
| sonnet | $7.29 |

By role:

| role | USD |
|---|---:|
| coordinator | $15.36 |
| implementer-domain | $65.85 |
| implementer-haiku | $0.13 |
| other | $7.68 |
| reviewer | $6.38 |

### PRs

- PRs: 8, merged 8
- Cost per PR: mean $11.93, p50 $8.10, p90 $25.05

| PR | USD | CI first pass | CI runs |
|---|---:|---:|---:|
| 1521 | $5.61 | - | 0 |
| 1527 | $2.38 | - | 0 |
| 1626 | $1.44 | - | 0 |
| 1627 | $9.16 | - | 0 |
| 1634 | $18.07 | - | 0 |
| 1637 | $7.04 | - | 0 |
| 1643 | $41.31 | yes | 1 |
| 1649 | $10.38 | yes | 1 |

### Accepted

- Accepted packets: 8
- Cost per accepted change: $11.93

### First pass

- First-pass rate: 0.0%
- Eligible packets: 5

| id | implementer sessions | reviewer sessions | fix commits | first pass |
|---|---:|---:|---:|---:|
| LR-01 | 1 | 0 | - | - |
| LR-02 | 1 | 1 | 1 | no |
| LR-03 | 1 | 1 | 1 | no |
| LR-04 | 0 | 1 | 1 | no |
| LR-06 | 0 | 1 | 1 | no |
| LR-10 | 0 | 2 | 1 | no |
| LR-13 | 0 | 0 | - | - |
| LR-16 | 0 | 0 | - | - |

### Retries

No retries.

### CI

- First-attempt rate: 100.0%
- Overall rate: 100.0%
- Code PRs first-attempt rate: 100.0%
- Queue first-attempt rate: 67.0%
- PR CI runs, total: 3

### Escaped defects

- Escaped defects: 0
- Rate per merged PR: 0

### Escalation

- Dispatched to Haiku: 3
- Escalated: 2
- Escalation rate: 67.0%

| packet | escalated to | why |
|---|---|---|
| LR-02 | implementer-domain | context-exhausted |
| LR-03 | implementer-domain | other |

### Stop context

- Workers: 39
- Context at stop, p50 / p90 / max: 162,796 / 260,130 / 330,366
- Share above 90k: 85.0%

| stop | count |
|---|---:|
| end_turn | 39 |
| tool_use | 0 |
| error | 0 |
| no_record | 0 |

No error kinds recorded.

90k is the Haiku worker context cap.

### Cycle time

- Campaign hours: n/a (no close-out commit in the ledger window)
- Packet hours: p50 1.8, p90 3.8 over 6 packets

### Overstep

- Opus coding: $2.44
- Opus research: $0.00
- Sonnet implementing: $0.00
- Recoverable: $2.53 to $4.04
- Wasted (repriced at intended tier): $4.04

| rule | count | USD | wasted USD |
|---|---:|---:|---:|
| O1 | 15 | $2.44 | $1.92 |
| O3b | 36 | $2.61 | $2.12 |

| rule | USD | tool | path class | model |
|---|---|---|---|---|
| O1 | $0.19 | Edit | crates | claude-opus-5-5 |
| O1 | $0.13 | Edit | crates | claude-opus-5-5 |
| O1 | $0.13 | Edit | crates | claude-opus-5-5 |
| O1 | $0.10 | Edit | crates | claude-opus-5-5 |
| O1 | $0.09 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.14 | Edit | crates | claude-opus-5-5 |
| O1 | $0.09 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.16 | Edit | crates | claude-opus-5-5 |
| O1 | $0.15 | Edit | crates | claude-opus-5-5 |
| O1 | $0.11 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.13 | Edit | crates | claude-opus-5-5 |
| O1 | $0.09 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.21 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.35 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.37 | PowerShell | - | claude-opus-5-5 |
| O3b | $0.17 | Edit | crates | claude-opus-5-5 |
| O3b | $0.11 | Edit | crates | claude-opus-5-5 |
| O3b | $0.07 | Edit | crates | claude-opus-5-5 |
| O3b | $0.07 | Edit | crates | claude-opus-5-5 |
| O3b | $0.08 | Edit | crates | claude-opus-5-5 |
