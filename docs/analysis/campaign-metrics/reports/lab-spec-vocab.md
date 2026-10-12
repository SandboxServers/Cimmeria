# lab-spec-vocab

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
| total | $97.91 |
| attributed | $97.91 |
| unattributed in campaign | $0.00 |

By model tier:

| model tier | USD |
|---|---:|
| fable | $0.00 |
| haiku | $0.35 |
| opus | $91.25 |
| other | $0.00 |
| sonnet | $6.30 |

By role:

| role | USD |
|---|---:|
| coordinator | $37.11 |
| implementer-domain | $43.43 |
| implementer-haiku | $0.35 |
| other | $10.71 |
| reviewer | $6.30 |

### PRs

- PRs: 15, merged 15
- Cost per PR: mean $6.53, p50 $3.78, p90 $13.43

| PR | USD | CI first pass | CI runs |
|---|---:|---:|---:|
| 1391 | $1.89 | - | 0 |
| 1392 | $0.72 | - | 0 |
| 1393 | $1.87 | - | 0 |
| 1396 | $3.56 | - | 0 |
| 1415 | $3.23 | - | 0 |
| 1417 | $2.72 | - | 0 |
| 1422 | $12.00 | - | 0 |
| 1423 | $9.72 | - | 0 |
| 1449 | $8.45 | - | 0 |
| 1452 | $3.03 | - | 0 |
| 1453 | $6.02 | - | 0 |
| 1456 | $14.39 | - | 0 |
| 1460 | $15.24 | - | 0 |
| 1466 | $11.28 | - | 0 |
| 1491 | $3.78 | - | 0 |

### Accepted

- Accepted packets: 16
- Cost per accepted change: $6.12

### First pass

- First-pass rate: 0.0%
- Eligible packets: 13

| id | implementer sessions | reviewer sessions | fix commits | first pass |
|---|---:|---:|---:|---:|
| SV-01 | 1 | 1 | 1 | no |
| SV-02 | 1 | 1 | 1 | no |
| SV-03 | 2 | 1 | 2 | no |
| SV-04 | 1 | 1 | 2 | no |
| SV-05 | 1 | 1 | 2 | no |
| SV-06 | 1 | 1 | 1 | no |
| SV-07 | 0 | 1 | 1 | no |
| SV-08 | 0 | 1 | 2 | no |
| SV-09 | 1 | 1 | 1 | no |
| SV-10 | 0 | 1 | 1 | no |
| SV-11 | 1 | 1 | 1 | no |
| SV-12 | 0 | 1 | 1 | no |
| SV-13 | 0 | 1 | 1 | no |
| SV-14 | 0 | 0 | - | - |
| SV-15 | 0 | 0 | - | - |
| SV-16 | 0 | 0 | - | - |

### Retries

- Retries: 1

| reason | count |
|---|---:|
| context-exhausted | 1 |

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

- Dispatched to Haiku: 8
- Escalated: 2
- Escalation rate: 25.0%

| packet | escalated to | why |
|---|---|---|
| SV-09 | implementer-domain | context-exhausted |
| SV-11 | implementer-domain | other |

### Stop context

- Workers: 41
- Context at stop, p50 / p90 / max: 118,482 / 217,107 / 323,367
- Share above 90k: 73.0%

| stop | count |
|---|---:|
| end_turn | 40 |
| tool_use | 0 |
| error | 0 |
| no_record | 1 |

No error kinds recorded.

90k is the Haiku worker context cap.

### Cycle time

- Campaign hours: 7.9
- Packet hours: p50 1.3, p90 2.2 over 14 packets

### Overstep

- Opus coding: $8.88
- Opus research: $0.00
- Sonnet implementing: $0.00
- Recoverable: $5.07 to $7.93
- Wasted (repriced at intended tier): $7.93

| rule | count | USD | wasted USD |
|---|---:|---:|---:|
| O1 | 48 | $8.88 | $6.87 |
| O3b | 18 | $1.27 | $1.06 |

| rule | USD | tool | path class | model |
|---|---|---|---|---|
| O1 | $0.20 | Edit | crates | claude-opus-5-5 |
| O1 | $0.06 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.04 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.04 | Edit | crates | claude-opus-5-5 |
| O1 | $0.04 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.09 | Edit | crates | claude-opus-5-5 |
| O1 | $0.04 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.13 | Edit | crates | claude-opus-5-5 |
| O1 | $0.09 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.09 | Edit | crates | claude-opus-5-5 |
| O1 | $0.15 | Edit | crates | claude-opus-5-5 |
| O1 | $0.11 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.08 | Edit | crates | claude-opus-5-5 |
| O1 | $0.08 | Edit | crates | claude-opus-5-5 |
| O1 | $0.06 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.17 | Edit | crates | claude-opus-5-5 |
| O1 | $0.10 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.11 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.15 | Edit | crates | claude-opus-5-5 |
| O1 | $0.09 | PowerShell | - | claude-opus-5-5 |
