# launcher-consolidation

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
| total | $291.64 |
| attributed | $291.64 |
| unattributed in campaign | $0.00 |

By model tier:

| model tier | USD |
|---|---:|
| fable | $0.00 |
| haiku | $0.25 |
| opus | $280.69 |
| other | $0.00 |
| sonnet | $10.71 |

By role:

| role | USD |
|---|---:|
| coordinator | $56.05 |
| implementer-domain | $190.26 |
| implementer-haiku | $0.25 |
| other | $34.38 |
| reviewer | $10.71 |

### PRs

- PRs: 22, merged 22
- Cost per PR: mean $13.26, p50 $12.10, p90 $26.20

| PR | USD | CI first pass | CI runs |
|---|---:|---:|---:|
| 1353 | $0.50 | - | 0 |
| 1355 | $0.07 | - | 0 |
| 1356 | $3.47 | - | 0 |
| 1361 | $10.06 | - | 0 |
| 1379 | $2.44 | - | 0 |
| 1386 | $14.74 | - | 0 |
| 1405 | $1.52 | - | 0 |
| 1409 | $11.15 | - | 0 |
| 1414 | $10.79 | - | 0 |
| 1416 | $15.37 | - | 0 |
| 1418 | $9.03 | - | 0 |
| 1425 | $16.84 | - | 0 |
| 1431 | $13.05 | - | 0 |
| 1438 | $20.80 | - | 0 |
| 1442 | $5.44 | - | 0 |
| 1443 | $8.72 | - | 0 |
| 1446 | $26.80 | - | 0 |
| 1482 | $16.75 | - | 0 |
| 1534 | $17.51 | - | 0 |
| 1535 | $18.81 | - | 0 |
| 1619 | $27.49 | - | 0 |
| 1641 | $40.28 | yes | 1 |

### Accepted

- Accepted packets: 21
- Cost per accepted change: $13.89

### First pass

- First-pass rate: 0.0%
- Eligible packets: 15

| id | implementer sessions | reviewer sessions | fix commits | first pass |
|---|---:|---:|---:|---:|
| LX-01 | 0 | 1 | 2 | no |
| LX-02 | 0 | 1 | 2 | no |
| LX-03 | 2 | 1 | 1 | no |
| LX-04 | 0 | 1 | 1 | no |
| LX-05 | 1 | 0 | - | no |
| LX-06 | 1 | 1 | 2 | no |
| LX-07 | 1 | 1 | 2 | no |
| LX-08a | 0 | 1 | 1 | no |
| LX-08b | 0 | 1 | 2 | no |
| LX-08c | 0 | 1 | 1 | no |
| LX-09 | 1 | 1 | 1 | no |
| LX-10 | 0 | 0 | - | - |
| LX-11 | 1 | 1 | 2 | no |
| LX-12 | 0 | 0 | - | - |
| LX-13 | 0 | 0 | - | - |
| LX-14 | 1 | 1 | 1 | no |
| LX-15 | 0 | 0 | - | - |
| LX-17 | 0 | 1 | 5 | no |
| LX-19 | 0 | 0 | - | - |
| LX-25 | 0 | 1 | 1 | no |
| LX-29 | 0 | 0 | - | - |

### Retries

- Retries: 1

| reason | count |
|---|---:|
| context-exhausted | 1 |

### CI

- First-attempt rate: 100.0%
- Overall rate: 100.0%
- Code PRs first-attempt rate: 100.0%
- Queue first-attempt rate: 100.0%
- PR CI runs, total: 1

### Escaped defects

- Escaped defects: 0
- Rate per merged PR: 0

### Escalation

- Dispatched to Haiku: 7
- Escalated: 4
- Escalation rate: 57.0%

| packet | escalated to | why |
|---|---|---|
| LX-03 | implementer-domain | other |
| LX-05 | implementer-domain | context-exhausted |
| LX-06 | implementer-domain | context-exhausted |
| LX-07 | implementer-domain | context-exhausted |

### Stop context

- Workers: 65
- Context at stop, p50 / p90 / max: 152,877 / 260,928 / 456,643
- Share above 90k: 80.0%

| stop | count |
|---|---:|
| end_turn | 65 |
| tool_use | 0 |
| error | 0 |
| no_record | 0 |

No error kinds recorded.

90k is the Haiku worker context cap.

### Cycle time

- Campaign hours: n/a (no close-out commit in the ledger window)
- Packet hours: p50 1.9, p90 6.5 over 19 packets

### Overstep

- Opus coding: $2.06
- Opus research: $0.00
- Sonnet implementing: $0.00
- Recoverable: $1.03 to $1.58
- Wasted (repriced at intended tier): $1.58

| rule | count | USD | wasted USD |
|---|---:|---:|---:|
| O1 | 7 | $2.06 | $1.58 |

| rule | USD | tool | path class | model |
|---|---|---|---|---|
| O1 | $0.23 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.27 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.72 | Edit | crates | claude-opus-5-5 |
| O1 | $0.40 | Edit | crates | claude-opus-5-5 |
| O1 | $0.05 | Edit | tools | claude-opus-5-5 |
| O1 | $0.19 | Edit | tools | claude-opus-5-5 |
| O1 | $0.20 | Edit | tools | claude-opus-5-5 |
