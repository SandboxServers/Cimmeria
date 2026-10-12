# lab-parallel-clients

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
| total | $19.28 |
| attributed | $19.28 |
| unattributed in campaign | $0.00 |

By model tier:

| model tier | USD |
|---|---:|
| fable | $0.00 |
| haiku | $0.25 |
| opus | $16.32 |
| other | $0.00 |
| sonnet | $2.71 |

By role:

| role | USD |
|---|---:|
| coordinator | $13.08 |
| implementer-haiku | $0.25 |
| other | $3.24 |
| reviewer | $2.71 |

### PRs

- PRs: 10, merged 10
- Cost per PR: mean $1.93, p50 $1.68, p90 $3.54

| PR | USD | CI first pass | CI runs |
|---|---:|---:|---:|
| 1316 | $1.36 | - | 0 |
| 1320 | $0.77 | - | 0 |
| 1321 | $2.22 | - | 0 |
| 1322 | $0.68 | - | 0 |
| 1323 | $3.53 | - | 0 |
| 1325 | $2.51 | - | 0 |
| 1327 | $1.43 | - | 0 |
| 1328 | $3.68 | - | 0 |
| 1329 | $1.93 | - | 0 |
| 1331 | $1.18 | - | 0 |

### Accepted

- Accepted packets: 10
- Cost per accepted change: $1.93

### First pass

- First-pass rate: 0.0%
- Eligible packets: 3

| id | implementer sessions | reviewer sessions | fix commits | first pass |
|---|---:|---:|---:|---:|
| LP-01 | 1 | 1 | 1 | no |
| LP-02 | 1 | 1 | - | - |
| LP-03 | 1 | 1 | 1 | no |
| LP-04 | 0 | 0 | - | - |
| LP-05a | 1 | 1 | - | - |
| LP-05b1 | 1 | 0 | - | - |
| LP-05b2 | 1 | 1 | - | - |
| LP-06 | 1 | 1 | 1 | no |
| LP-08 | 1 | 1 | - | - |
| LP-07 | 0 | 0 | - | - |

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

- Dispatched to Haiku: 8
- Escalated: 0
- Escalation rate: 0.0%

### Stop context

- Workers: 16
- Context at stop, p50 / p90 / max: 63,696 / 91,852 / 203,733
- Share above 90k: 12.0%

| stop | count |
|---|---:|
| end_turn | 16 |
| tool_use | 0 |
| error | 0 |
| no_record | 0 |

No error kinds recorded.

90k is the Haiku worker context cap.

### Cycle time

- Campaign hours: n/a (no close-out commit in the ledger window)
- Packet hours: p50 0.1, p90 0.2 over 3 packets

### Overstep

- Opus coding: $2.36
- Opus research: $0.00
- Sonnet implementing: $0.00
- Recoverable: $1.18 to $1.86
- Wasted (repriced at intended tier): $1.86

| rule | count | USD | wasted USD |
|---|---:|---:|---:|
| O1 | 15 | $2.36 | $1.86 |

| rule | USD | tool | path class | model |
|---|---|---|---|---|
| O1 | $0.15 | Edit | crates | claude-opus-5-5 |
| O1 | $0.16 | Edit | crates | claude-opus-5-5 |
| O1 | $0.09 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.05 | Edit | crates | claude-opus-5-5 |
| O1 | $0.17 | Edit | crates | claude-opus-5-5 |
| O1 | $0.12 | Edit | crates | claude-opus-5-5 |
| O1 | $0.09 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.21 | Edit | crates | claude-opus-5-5 |
| O1 | $0.20 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.16 | Edit | crates | claude-opus-5-5 |
| O1 | $0.20 | Edit | crates | claude-opus-5-5 |
| O1 | $0.16 | Edit | crates | claude-opus-5-5 |
| O1 | $0.11 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.25 | Edit | crates | claude-opus-5-5 |
| O1 | $0.25 | PowerShell | - | claude-opus-5-5 |
