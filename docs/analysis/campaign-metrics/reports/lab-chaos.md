# lab-chaos

> Regime: mixed

### Stamp

- Regime: mixed
- Generated: 2026-10-12T18:51:11Z
- Window: 2026-09-21T00:00:00Z to 2026-10-13T00:00:00Z
- Price table: 2026-10-12
- Profiler schema: 6
- Unattributed share: 12.2%
- Unmapped share: 38.0%

Dollars are list-price estimates of plan usage, a floor, never a bill.

### Cost

|  | USD |
|---|---:|
| total | $120.00 |
| attributed | $88.19 |
| unattributed in campaign | $31.80 |

By model tier:

| model tier | USD |
|---|---:|
| fable | $0.00 |
| haiku | $0.15 |
| opus | $113.32 |
| other | $0.00 |
| sonnet | $6.52 |

By role:

| role | USD |
|---|---:|
| coordinator | $25.00 |
| implementer-domain | $76.03 |
| implementer-haiku | $0.15 |
| other | $12.85 |
| reviewer | $5.97 |

### PRs

- PRs: 7, merged 7
- Cost per PR: mean $12.60, p50 $9.76, p90 $24.84

| PR | USD | CI first pass | CI runs |
|---|---:|---:|---:|
| 1506 | $1.09 | - | 0 |
| 1517 | $1.11 | - | 0 |
| 1644 | $9.76 | yes | 2 |
| 1650 | $17.42 | yes | 1 |
| 1651 | $23.52 | yes | 1 |
| 1653 | $8.48 | yes | 1 |
| 1656 | $26.81 | yes | 1 |

### Accepted

- Accepted packets: 11
- Cost per accepted change: $10.91

### First pass

- First-pass rate: 0.0%
- Eligible packets: 1

| id | implementer sessions | reviewer sessions | fix commits | first pass |
|---|---:|---:|---:|---:|
| CH-01 | 1 | 1 | - | - |
| CH-02 | 1 | 1 | - | - |
| CH-03 | 1 | 1 | - | - |
| CH-04 | 0 | 2 | - | - |
| CH-05 | 1 | 1 | - | no |
| CH-06 | 0 | 1 | - | - |
| CH-07 | 0 | 1 | - | - |
| CH-08 | 0 | 0 | - | - |
| CH-09 | 0 | 0 | - | - |
| CH-13 | 0 | 3 | - | - |
| CH-14 | 0 | 2 | - | - |

### Retries

No retries.

### CI

- First-attempt rate: 100.0%
- Overall rate: 100.0%
- Code PRs first-attempt rate: 100.0%
- Queue first-attempt rate: 50.0%
- PR CI runs, total: 6

### Escaped defects

- Escaped defects: 0
- Rate per merged PR: 0

### Escalation

- Dispatched to Haiku: 4
- Escalated: 1
- Escalation rate: 25.0%

| packet | escalated to | why |
|---|---|---|
| CH-05 | implementer-domain | context-exhausted |

### Stop context

- Workers: 48
- Context at stop, p50 / p90 / max: 133,159 / 295,824 / 343,439
- Share above 90k: 79.0%

| stop | count |
|---|---:|
| end_turn | 47 |
| tool_use | 0 |
| error | 0 |
| no_record | 1 |

No error kinds recorded.

90k is the Haiku worker context cap.

### Cycle time

- Campaign hours: n/a (no close-out commit in the ledger window)
- Packet hours: p50 -, p90 - over 0 packets

### Overstep

- Opus coding: $2.91
- Opus research: $0.00
- Sonnet implementing: $0.00
- Recoverable: $4.81 to $7.77
- Wasted (repriced at intended tier): $7.77

| rule | count | USD | wasted USD |
|---|---:|---:|---:|
| O1 | 19 | $2.91 | $2.35 |
| O3b | 71 | $6.71 | $5.42 |

| rule | USD | tool | path class | model |
|---|---|---|---|---|
| O1 | $0.17 | Edit | crates | claude-opus-5-5 |
| O1 | $0.11 | Edit | crates | claude-opus-5-5 |
| O1 | $0.18 | Edit | crates | claude-opus-5-5 |
| O1 | $0.09 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.25 | Edit | tools | claude-opus-5-5 |
| O1 | $0.11 | Edit | tools | claude-opus-5-5 |
| O1 | $0.08 | Edit | tools | claude-opus-5-5 |
| O1 | $0.14 | Edit | tools | claude-opus-5-5 |
| O1 | $0.15 | Edit | tools | claude-opus-5-5 |
| O1 | $0.11 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.04 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.09 | Edit | crates | claude-opus-5-5 |
| O1 | $0.05 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.18 | Edit | .github | claude-opus-5-5 |
| O1 | $0.22 | Edit | crates | claude-opus-5-5 |
| O1 | $0.24 | Edit | crates | claude-opus-5-5 |
| O1 | $0.07 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.31 | Edit | crates | claude-opus-5-5 |
| O1 | $0.30 | Edit | crates | claude-opus-5-5 |
| O3b | $0.17 | Write | crates | claude-opus-5-5 |
