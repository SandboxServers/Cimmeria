# wireclient-ci

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
| total | $56.77 |
| attributed | $56.77 |
| unattributed in campaign | $0.00 |

By model tier:

| model tier | USD |
|---|---:|
| fable | $0.00 |
| haiku | $0.36 |
| opus | $51.55 |
| other | $0.00 |
| sonnet | $4.86 |

By role:

| role | USD |
|---|---:|
| coordinator | $30.69 |
| implementer-domain | $14.78 |
| implementer-haiku | $0.36 |
| other | $6.08 |
| reviewer | $4.86 |

### PRs

- PRs: 16, merged 16
- Cost per PR: mean $3.55, p50 $1.17, p90 $9.04

| PR | USD | CI first pass | CI runs |
|---|---:|---:|---:|
| 1358 | $0.39 | - | 0 |
| 1359 | $0.83 | - | 0 |
| 1360 | $0.89 | - | 0 |
| 1369 | $0.65 | - | 0 |
| 1373 | $2.51 | - | 0 |
| 1399 | $0.64 | - | 0 |
| 1400 | $0.02 | - | 0 |
| 1401 | $0.01 | - | 0 |
| 1402 | $0.02 | - | 0 |
| 1403 | $2.01 | - | 0 |
| 1404 | $1.46 | - | 0 |
| 1447 | $8.58 | - | 0 |
| 1496 | $2.23 | - | 0 |
| 1507 | $5.65 | - | 0 |
| 1555 | $21.39 | - | 0 |
| 1625 | $9.50 | - | 0 |

### Accepted

- Accepted packets: 15
- Cost per accepted change: $3.78

### First pass

- First-pass rate: 11.0%
- Eligible packets: 9

| id | implementer sessions | reviewer sessions | fix commits | first pass |
|---|---:|---:|---:|---:|
| WC-01 | 1 | 1 | 0 | yes |
| WC-02 | 1 | 1 | 1 | no |
| WC-03 | 1 | 1 | 1 | no |
| WC-04 | 1 | 1 | 1 | no |
| WC-05 | 1 | 1 | 1 | no |
| WC-06 | 1 | 0 | - | - |
| WC-07 | 1 | 0 | - | - |
| WC-08 | 1 | 0 | - | - |
| WC-09 | 1 | 0 | - | - |
| WC-10 | 1 | 1 | 1 | no |
| WC-11 | 1 | 1 | - | - |
| WC-12 | 0 | 1 | - | - |
| WC-13 | 1 | 1 | 1 | no |
| WC-14 | 0 | 1 | 2 | no |
| WC-18 | 1 | 1 | 1 | no |

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

- Dispatched to Haiku: 13
- Escalated: 0
- Escalation rate: 0.0%

### Stop context

- Workers: 33
- Context at stop, p50 / p90 / max: 79,832 / 204,586 / 338,391
- Share above 90k: 33.0%

| stop | count |
|---|---:|
| end_turn | 33 |
| tool_use | 0 |
| error | 0 |
| no_record | 0 |

No error kinds recorded.

90k is the Haiku worker context cap.

### Cycle time

- Campaign hours: n/a (no close-out commit in the ledger window)
- Packet hours: p50 0.7, p90 2.8 over 13 packets

### Overstep

- Opus coding: $8.44
- Opus research: $1.50
- Sonnet implementing: $0.00
- Recoverable: $6.53 to $9.95
- Wasted (repriced at intended tier): $9.95

| rule | count | USD | wasted USD |
|---|---:|---:|---:|
| O1 | 57 | $8.44 | $6.65 |
| O2 | 21 | $1.50 | $0.75 |
| O3b | 32 | $3.12 | $2.55 |

| rule | USD | tool | path class | model |
|---|---|---|---|---|
| O1 | $0.09 | Edit | crates | claude-opus-5-5 |
| O1 | $0.08 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.09 | Edit | crates | claude-opus-5-5 |
| O1 | $0.15 | Edit | crates | claude-opus-5-5 |
| O1 | $0.09 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.15 | Edit | crates | claude-opus-5-5 |
| O1 | $0.16 | Write | crates | claude-opus-5-5 |
| O1 | $0.22 | Edit | crates | claude-opus-5-5 |
| O1 | $0.24 | Edit | crates | claude-opus-5-5 |
| O1 | $0.31 | Write | crates | claude-opus-5-5 |
| O1 | $0.33 | Edit | crates | claude-opus-5-5 |
| O1 | $0.09 | Grep | tools | claude-opus-5-5 |
| O1 | $0.31 | Edit | crates | claude-opus-5-5 |
| O1 | $0.29 | Edit | crates | claude-opus-5-5 |
| O1 | $0.28 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.19 | Edit | crates | claude-opus-5-5 |
| O1 | $0.20 | Edit | crates | claude-opus-5-5 |
| O1 | $0.32 | Edit | crates | claude-opus-5-5 |
| O1 | $0.42 | Edit | crates | claude-opus-5-5 |
| O1 | $0.30 | Edit | crates | claude-opus-5-5 |
