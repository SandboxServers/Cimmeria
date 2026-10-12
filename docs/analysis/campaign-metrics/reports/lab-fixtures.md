# lab-fixtures

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
| total | $90.26 |
| attributed | $90.26 |
| unattributed in campaign | $0.00 |

By model tier:

| model tier | USD |
|---|---:|
| fable | $0.00 |
| haiku | $0.19 |
| opus | $85.58 |
| other | $0.00 |
| sonnet | $4.49 |

By role:

| role | USD |
|---|---:|
| coordinator | $29.36 |
| implementer-domain | $46.60 |
| implementer-haiku | $0.19 |
| other | $10.25 |
| reviewer | $3.86 |

### PRs

- PRs: 11, merged 11
- Cost per PR: mean $8.21, p50 $10.77, p90 $14.88

| PR | USD | CI first pass | CI runs |
|---|---:|---:|---:|
| 1408 | $15.39 | - | 0 |
| 1420 | $1.26 | - | 0 |
| 1421 | $14.88 | - | 0 |
| 1424 | $0.10 | - | 0 |
| 1430 | $11.28 | - | 0 |
| 1432 | $10.77 | - | 0 |
| 1434 | $11.38 | - | 0 |
| 1436 | $0.49 | - | 0 |
| 1440 | $8.81 | - | 0 |
| 1441 | $4.00 | - | 0 |
| 1448 | $11.91 | - | 0 |

### Accepted

- Accepted packets: 3
- Cost per accepted change: $30.09

### First pass

- First-pass rate: 0.0%
- Eligible packets: 3

| id | implementer sessions | reviewer sessions | fix commits | first pass |
|---|---:|---:|---:|---:|
| FX-01 | 2 | 1 | 1 | no |
| FX-02 | 0 | 1 | 1 | no |
| FX-05 | 2 | 1 | 2 | no |

### Retries

- Retries: 2

| reason | count |
|---|---:|
| other | 2 |

### CI

- First-attempt rate: n/a (no CI data)
- Overall rate: n/a (no CI data)
- Code PRs first-attempt rate: n/a (no CI data)
- Queue first-attempt rate: n/a (no CI data)
- PR CI runs, total: 0

### Escaped defects

- Escaped defects: 3
- Rate per merged PR: 0.27

| kind | citing item | cites PR |
|---|---:|---:|
| pr | 1433 | 1421 |
| pr | 1457 | 1434 |
| pr | 1471 | 1421 |

### Escalation

- Dispatched to Haiku: 5
- Escalated: 2
- Escalation rate: 40.0%

| packet | escalated to | why |
|---|---|---|
| FX-06 | implementer-domain | other |
| FX-07 | implementer-domain | context-exhausted |

### Stop context

- Workers: 36
- Context at stop, p50 / p90 / max: 116,236 / 220,236 / 254,002
- Share above 90k: 72.0%

| stop | count |
|---|---:|
| end_turn | 36 |
| tool_use | 0 |
| error | 0 |
| no_record | 0 |

No error kinds recorded.

90k is the Haiku worker context cap.

### Cycle time

- Campaign hours: n/a (no close-out commit in the ledger window)
- Packet hours: p50 0.8, p90 0.9 over 3 packets

### Overstep

- Opus coding: $3.06
- Opus research: $0.00
- Sonnet implementing: $0.00
- Recoverable: $7.71 to $12.58
- Wasted (repriced at intended tier): $12.58

| rule | count | USD | wasted USD |
|---|---:|---:|---:|
| O1 | 11 | $3.06 | $2.36 |
| O3b | 139 | $12.36 | $10.22 |

| rule | USD | tool | path class | model |
|---|---|---|---|---|
| O1 | $0.31 | Edit | crates | claude-opus-5-5 |
| O1 | $0.24 | Edit | crates | claude-opus-5-5 |
| O1 | $0.25 | Edit | crates | claude-opus-5-5 |
| O1 | $0.18 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.23 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.20 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.13 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.31 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.35 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.42 | Edit | tools | claude-opus-5-5 |
| O1 | $0.45 | PowerShell | - | claude-opus-5-5 |
| O3b | $0.11 | Write | crates | claude-opus-5-5 |
| O3b | $0.08 | Edit | crates | claude-opus-5-5 |
| O3b | $0.11 | Write | crates | claude-opus-5-5 |
| O3b | $0.13 | Write | crates | claude-opus-5-5 |
| O3b | $0.11 | Edit | crates | claude-opus-5-5 |
| O3b | $0.08 | Edit | crates | claude-opus-5-5 |
| O3b | $0.08 | Edit | crates | claude-opus-5-5 |
| O3b | $0.07 | Edit | crates | claude-opus-5-5 |
| O3b | $0.23 | Write | crates | claude-opus-5-5 |
