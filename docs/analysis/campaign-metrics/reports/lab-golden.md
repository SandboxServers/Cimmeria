# lab-golden

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
| total | $44.60 |
| attributed | $44.60 |
| unattributed in campaign | $0.00 |

By model tier:

| model tier | USD |
|---|---:|
| fable | $0.00 |
| haiku | $0.18 |
| opus | $41.05 |
| other | $0.00 |
| sonnet | $3.37 |

By role:

| role | USD |
|---|---:|
| coordinator | $12.29 |
| implementer-domain | $17.56 |
| implementer-haiku | $0.18 |
| other | $11.20 |
| reviewer | $3.37 |

### PRs

- PRs: 10, merged 10
- Cost per PR: mean $4.46, p50 $4.96, p90 $6.47

| PR | USD | CI first pass | CI runs |
|---|---:|---:|---:|
| 1397 | $1.84 | - | 0 |
| 1406 | $0.99 | - | 0 |
| 1413 | $1.46 | - | 0 |
| 1426 | $5.49 | - | 0 |
| 1428 | $9.01 | - | 0 |
| 1429 | $5.71 | - | 0 |
| 1435 | $6.18 | - | 0 |
| 1445 | $6.07 | - | 0 |
| 1451 | $4.42 | - | 0 |
| 1454 | $3.42 | - | 0 |

### Accepted

- Accepted packets: 10
- Cost per accepted change: $4.46

### First pass

- First-pass rate: 0.0%
- Eligible packets: 1

| id | implementer sessions | reviewer sessions | fix commits | first pass |
|---|---:|---:|---:|---:|
| GD-01 | 1 | 1 | - | - |
| GD-02 | 1 | 1 | - | - |
| GD-03 | 1 | 1 | - | no |
| GD-04 | 0 | 1 | - | - |
| GD-05 | 1 | 1 | - | - |
| GD-06 | 0 | 1 | - | - |
| GD-07 | 0 | 1 | - | - |
| GD-08 | 0 | 1 | - | - |
| GD-09 | 0 | 1 | - | - |
| GD-11 | 0 | 0 | - | - |

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
- Escalated: 1
- Escalation rate: 25.0%

| packet | escalated to | why |
|---|---|---|
| GD-03 | implementer-domain | context-exhausted |

### Stop context

- Workers: 28
- Context at stop, p50 / p90 / max: 116,587 / 188,701 / 222,144
- Share above 90k: 64.0%

| stop | count |
|---|---:|
| end_turn | 28 |
| tool_use | 0 |
| error | 0 |
| no_record | 0 |

No error kinds recorded.

90k is the Haiku worker context cap.

### Cycle time

- Campaign hours: 4.9
- Packet hours: p50 -, p90 - over 0 packets

### Overstep

- Opus coding: $2.64
- Opus research: $0.00
- Sonnet implementing: $0.00
- Recoverable: $1.32 to $2.06
- Wasted (repriced at intended tier): $2.06

| rule | count | USD | wasted USD |
|---|---:|---:|---:|
| O1 | 15 | $2.64 | $2.06 |

| rule | USD | tool | path class | model |
|---|---|---|---|---|
| O1 | $0.26 | Edit | crates | claude-opus-5-5 |
| O1 | $0.13 | Edit | crates | claude-opus-5-5 |
| O1 | $0.10 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.18 | Edit | crates | claude-opus-5-5 |
| O1 | $0.14 | Edit | crates | claude-opus-5-5 |
| O1 | $0.10 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.21 | Edit | crates | claude-opus-5-5 |
| O1 | $0.16 | Edit | crates | claude-opus-5-5 |
| O1 | $0.11 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.09 | Edit | crates | claude-opus-5-5 |
| O1 | $0.24 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.19 | Edit | crates | claude-opus-5-5 |
| O1 | $0.12 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.26 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.34 | Edit | tools | claude-opus-5-5 |
