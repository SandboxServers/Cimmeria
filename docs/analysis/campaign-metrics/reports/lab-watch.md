# lab-watch

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
| total | $4.11 |
| attributed | $4.11 |
| unattributed in campaign | $0.00 |

By model tier:

| model tier | USD |
|---|---:|
| fable | $0.00 |
| haiku | $0.04 |
| opus | $3.84 |
| other | $0.00 |
| sonnet | $0.23 |

By role:

| role | USD |
|---|---:|
| coordinator | $1.83 |
| implementer-domain | $2.01 |
| implementer-haiku | $0.04 |
| reviewer | $0.23 |

### PRs

- PRs: 4, merged 4
- Cost per PR: mean $1.03, p50 $0.28, p90 $2.52

| PR | USD | CI first pass | CI runs |
|---|---:|---:|---:|
| 1395 | $0.25 | - | 0 |
| 1511 | $3.47 | - | 0 |
| 1520 | $0.31 | - | 0 |
| 1536 | $0.07 | - | 0 |

### Accepted

- Accepted packets: 4
- Cost per accepted change: $1.03

### First pass

- First-pass rate: 0.0%
- Eligible packets: 2

| id | implementer sessions | reviewer sessions | fix commits | first pass |
|---|---:|---:|---:|---:|
| LW-01 | 1 | 1 | - | no |
| LW-02 | 1 | 1 | 1 | no |
| LW-03 | 1 | 1 | - | - |
| LW-04 | 0 | 1 | - | - |

### Retries

No retries.

### CI

- First-attempt rate: n/a (no CI data)
- Overall rate: n/a (no CI data)
- Code PRs first-attempt rate: n/a (no CI data)
- Queue first-attempt rate: n/a (no CI data)
- PR CI runs, total: 0

### Escaped defects

- Escaped defects: 0
- Rate per merged PR: 0

### Escalation

- Dispatched to Haiku: 3
- Escalated: 1
- Escalation rate: 33.0%

| packet | escalated to | why |
|---|---|---|
| LW-01 | implementer-domain | context-exhausted |

### Stop context

- Workers: 9
- Context at stop, p50 / p90 / max: 90,991 / 165,820 / 196,113
- Share above 90k: 67.0%

| stop | count |
|---|---:|
| end_turn | 9 |
| tool_use | 0 |
| error | 0 |
| no_record | 0 |

No error kinds recorded.

90k is the Haiku worker context cap.

### Cycle time

- Campaign hours: n/a (no close-out commit in the ledger window)
- Packet hours: p50 1.0, p90 1.0 over 1 packets

### Overstep

- Opus coding: $1.96
- Opus research: $0.00
- Sonnet implementing: $0.00
- Recoverable: $1.36 to $2.20
- Wasted (repriced at intended tier): $2.20

| rule | count | USD | wasted USD |
|---|---:|---:|---:|
| O1 | 11 | $1.96 | $1.56 |
| O3b | 6 | $0.76 | $0.65 |

| rule | USD | tool | path class | model |
|---|---|---|---|---|
| O1 | $0.16 | Edit | tools | claude-opus-5-5 |
| O1 | $0.21 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.11 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.07 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.17 | Edit | crates | claude-opus-5-5 |
| O1 | $0.24 | Edit | tools | claude-opus-5-5 |
| O1 | $0.18 | Edit | tools | claude-opus-5-5 |
| O1 | $0.22 | Edit | tools | claude-opus-5-5 |
| O1 | $0.13 | Edit | tools | claude-opus-5-5 |
| O1 | $0.23 | Edit | tools | claude-opus-5-5 |
| O1 | $0.23 | Edit | tools | claude-opus-5-5 |
| O3b | $0.24 | Write | tools | claude-opus-5-5 |
| O3b | $0.17 | Write | tools | claude-opus-5-5 |
| O3b | $0.15 | Write | tools | claude-opus-5-5 |
| O3b | $0.06 | Edit | tools | claude-opus-5-5 |
| O3b | $0.10 | Edit | tools | claude-opus-5-5 |
| O3b | $0.04 | Edit | tools | claude-opus-5-5 |
