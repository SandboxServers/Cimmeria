# lab-cli

> Regime: haiku

### Stamp

- Regime: haiku
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
| total | $7.92 |
| attributed | $7.92 |
| unattributed in campaign | $0.00 |

By model tier:

| model tier | USD |
|---|---:|
| fable | $0.00 |
| haiku | $0.06 |
| opus | $7.16 |
| other | $0.00 |
| sonnet | $0.69 |

By role:

| role | USD |
|---|---:|
| coordinator | $4.32 |
| implementer-haiku | $0.06 |
| other | $2.84 |
| reviewer | $0.69 |

### PRs

- PRs: 6, merged 6
- Cost per PR: mean $1.32, p50 $0.89, p90 $2.78

| PR | USD | CI first pass | CI runs |
|---|---:|---:|---:|
| 1334 | $2.50 | - | 0 |
| 1335 | $0.75 | - | 0 |
| 1336 | $0.32 | - | 0 |
| 1337 | $1.02 | - | 0 |
| 1338 | $0.26 | - | 0 |
| 1339 | $3.06 | - | 0 |

### Accepted

- Accepted packets: 7
- Cost per accepted change: $1.13

### First pass

- First-pass rate: 0.0%
- Eligible packets: 5

| id | implementer sessions | reviewer sessions | fix commits | first pass |
|---|---:|---:|---:|---:|
| LC-01 | 1 | 1 | 1 | no |
| LC-02 | 1 | 1 | 1 | no |
| LC-03 | 1 | 1 | 1 | no |
| LC-04 | 1 | 1 | 1 | no |
| LC-05 | 1 | 1 | 1 | no |
| LC-06 | 0 | 0 | - | - |
| LC-07 | 0 | 0 | - | - |

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

- Dispatched to Haiku: 5
- Escalated: 0
- Escalation rate: 0.0%

### Stop context

- Workers: 11
- Context at stop, p50 / p90 / max: 72,820 / 92,522 / 186,610
- Share above 90k: 27.0%

| stop | count |
|---|---:|
| end_turn | 11 |
| tool_use | 0 |
| error | 0 |
| no_record | 0 |

No error kinds recorded.

90k is the Haiku worker context cap.

### Cycle time

- Campaign hours: 1.5
- Packet hours: p50 0.3, p90 0.5 over 6 packets

### Overstep

- Opus coding: $3.76
- Opus research: $0.00
- Sonnet implementing: $0.00
- Recoverable: $1.88 to $3.00
- Wasted (repriced at intended tier): $3.00

| rule | count | USD | wasted USD |
|---|---:|---:|---:|
| O1 | 25 | $3.76 | $3.00 |

| rule | USD | tool | path class | model |
|---|---|---|---|---|
| O1 | $0.14 | Edit | crates | claude-opus-5-5 |
| O1 | $0.08 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.08 | Edit | tools | claude-opus-5-5 |
| O1 | $0.07 | Edit | tools | claude-opus-5-5 |
| O1 | $0.11 | Edit | tools | claude-opus-5-5 |
| O1 | $0.10 | Edit | tools | claude-opus-5-5 |
| O1 | $0.10 | Edit | tools | claude-opus-5-5 |
| O1 | $0.11 | Edit | tools | claude-opus-5-5 |
| O1 | $0.08 | Edit | tools | claude-opus-5-5 |
| O1 | $0.15 | Edit | tools | claude-opus-5-5 |
| O1 | $0.10 | Edit | tools | claude-opus-5-5 |
| O1 | $0.19 | Edit | tools | claude-opus-5-5 |
| O1 | $0.16 | Edit | tools | claude-opus-5-5 |
| O1 | $0.19 | Edit | tools | claude-opus-5-5 |
| O1 | $0.18 | Edit | tools | claude-opus-5-5 |
| O1 | $0.11 | Edit | tools | claude-opus-5-5 |
| O1 | $0.11 | Edit | tools | claude-opus-5-5 |
| O1 | $0.15 | Edit | tools | claude-opus-5-5 |
| O1 | $0.12 | Edit | tools | claude-opus-5-5 |
| O1 | $0.18 | Edit | tools | claude-opus-5-5 |
