# class-start-v6

> Regime: old

### Stamp

- Regime: old
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
| total | $177.94 |
| attributed | $177.94 |
| unattributed in campaign | $0.00 |

By model tier:

| model tier | USD |
|---|---:|
| fable | $0.00 |
| haiku | $0.00 |
| opus | $177.94 |
| other | $0.00 |
| sonnet | $0.00 |

By role:

| role | USD |
|---|---:|
| coordinator | $21.46 |
| implementer-domain | $101.90 |
| other | $54.58 |

### PRs

- PRs: 8, merged 8
- Cost per PR: mean $22.24, p50 $21.64, p90 $36.83

| PR | USD | CI first pass | CI runs |
|---|---:|---:|---:|
| 1261 | $7.79 | - | 0 |
| 1263 | $19.27 | - | 0 |
| 1264 | $25.74 | - | 0 |
| 1270 | $15.32 | - | 0 |
| 1271 | $36.37 | - | 0 |
| 1273 | $24.01 | - | 0 |
| 1313 | $11.53 | - | 0 |
| 1314 | $37.91 | - | 0 |

### Accepted

- Accepted packets: 6
- Cost per accepted change: $29.66

### First pass

- First-pass rate: -
- Eligible packets: 0

| id | implementer sessions | reviewer sessions | fix commits | first pass |
|---|---:|---:|---:|---:|
| CS-00 | 0 | 0 | - | - |
| CS-01a | 0 | 0 | - | - |
| CS-01b | 0 | 0 | - | - |
| CS-02 | 0 | 0 | - | - |
| CS-03 | 0 | 0 | - | - |
| CS-04 | 0 | 0 | - | - |

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

- Dispatched to Haiku: 0
- Escalated: 0
- Escalation rate: -

### Stop context

- Workers: 12
- Context at stop, p50 / p90 / max: 291,417 / 514,761 / 536,712
- Share above 90k: 100.0%

| stop | count |
|---|---:|
| end_turn | 11 |
| tool_use | 1 |
| error | 0 |
| no_record | 0 |

No error kinds recorded.

90k is the Haiku worker context cap.

### Cycle time

- Campaign hours: n/a (no close-out commit in the ledger window)
- Packet hours: p50 4.4, p90 90.4 over 5 packets

### Overstep

predates the regime switch; the rules apply to the Haiku regime
