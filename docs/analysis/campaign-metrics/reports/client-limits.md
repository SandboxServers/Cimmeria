# client-limits

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
| total | $21.81 |
| attributed | $21.81 |
| unattributed in campaign | $0.00 |

By model tier:

| model tier | USD |
|---|---:|
| fable | $0.00 |
| haiku | $0.05 |
| opus | $15.55 |
| other | $0.00 |
| sonnet | $6.21 |

By role:

| role | USD |
|---|---:|
| coordinator | $7.50 |
| implementer-domain | $3.38 |
| implementer-haiku | $0.05 |
| other | $9.90 |
| reviewer | $0.98 |

### PRs

- PRs: 9, merged 9
- Cost per PR: mean $2.42, p50 $2.14, p90 $5.49

| PR | USD | CI first pass | CI runs |
|---|---:|---:|---:|
| 1502 | $0.50 | - | 0 |
| 1503 | $0.14 | - | 0 |
| 1504 | $1.58 | - | 0 |
| 1505 | $3.27 | - | 0 |
| 1525 | $5.30 | - | 0 |
| 1532 | $2.43 | - | 0 |
| 1533 | $2.14 | - | 0 |
| 1539 | $0.18 | - | 0 |
| 1550 | $6.26 | - | 0 |

### Accepted

- Accepted packets: 9
- Cost per accepted change: $2.42

### First pass

- First-pass rate: 0.0%
- Eligible packets: 3

| id | implementer sessions | reviewer sessions | fix commits | first pass |
|---|---:|---:|---:|---:|
| CL-01 | 0 | 1 | 1 | no |
| CL-02 | 0 | 0 | - | - |
| CL-03 | 0 | 0 | - | - |
| CL-04 | 0 | 0 | - | - |
| CL-05 | 0 | 0 | - | - |
| CL-06 | 1 | 1 | 1 | no |
| CL-07 | 1 | 0 | - | - |
| CL-11 | 1 | 0 | - | - |
| CL-13 | 1 | 1 | 1 | no |

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

- Dispatched to Haiku: 4
- Escalated: 0
- Escalation rate: 0.0%

### Stop context

- Workers: 18
- Context at stop, p50 / p90 / max: 124,284 / 219,941 / 286,804
- Share above 90k: 67.0%

| stop | count |
|---|---:|
| end_turn | 17 |
| tool_use | 0 |
| error | 0 |
| no_record | 1 |

No error kinds recorded.

90k is the Haiku worker context cap.

### Cycle time

- Campaign hours: n/a (no close-out commit in the ledger window)
- Packet hours: p50 0.5, p90 0.9 over 8 packets

### Overstep

- Opus coding: $1.92
- Opus research: $0.00
- Sonnet implementing: $0.00
- Recoverable: $1.26 to $2.01
- Wasted (repriced at intended tier): $2.01

| rule | count | USD | wasted USD |
|---|---:|---:|---:|
| O1 | 9 | $1.92 | $1.51 |
| O3b | 7 | $0.60 | $0.50 |

| rule | USD | tool | path class | model |
|---|---|---|---|---|
| O1 | $0.21 | Edit | crates | claude-opus-5-5 |
| O1 | $0.13 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.18 | Edit | crates | claude-opus-5-5 |
| O1 | $0.13 | Edit | crates | claude-opus-5-5 |
| O1 | $0.18 | Edit | crates | claude-opus-5-5 |
| O1 | $0.25 | Edit | crates | claude-opus-5-5 |
| O1 | $0.20 | Edit | crates | claude-opus-5-5 |
| O1 | $0.30 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.34 | PowerShell | - | claude-opus-5-5 |
| O3b | $0.17 | Write | crates | claude-opus-5-5 |
| O3b | $0.04 | Edit | crates | claude-opus-5-5 |
| O3b | $0.05 | Edit | crates | claude-opus-5-5 |
| O3b | $0.04 | Edit | crates | claude-opus-5-5 |
| O3b | $0.14 | Write | crates | claude-opus-5-5 |
| O3b | $0.10 | Edit | crates | claude-opus-5-5 |
| O3b | $0.06 | Edit | tools | claude-opus-5-5 |
