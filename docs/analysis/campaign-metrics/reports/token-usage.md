# token-usage

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
| total | $133.60 |
| attributed | $133.60 |
| unattributed in campaign | $0.00 |

By model tier:

| model tier | USD |
|---|---:|
| fable | $0.00 |
| haiku | $0.14 |
| opus | $133.46 |
| other | $0.00 |
| sonnet | $0.00 |

By role:

| role | USD |
|---|---:|
| coordinator | $24.20 |
| other | $109.40 |

### PRs

- PRs: 22, merged 22
- Cost per PR: mean $6.07, p50 $5.28, p90 $9.61

| PR | USD | CI first pass | CI runs |
|---|---:|---:|---:|
| 1122 | $4.65 | - | 0 |
| 1123 | $0.89 | - | 0 |
| 1124 | $4.38 | - | 0 |
| 1125 | $3.19 | - | 0 |
| 1126 | $5.63 | - | 0 |
| 1127 | $6.91 | - | 0 |
| 1128 | $11.01 | - | 0 |
| 1129 | $0.67 | - | 0 |
| 1130 | $3.60 | - | 0 |
| 1131 | $7.71 | - | 0 |
| 1132 | $8.42 | - | 0 |
| 1133 | $9.72 | - | 0 |
| 1134 | $8.58 | - | 0 |
| 1135 | $4.96 | - | 0 |
| 1136 | $0.43 | - | 0 |
| 1137 | $2.66 | - | 0 |
| 1138 | $26.97 | - | 0 |
| 1140 | $5.75 | - | 0 |
| 1142 | $5.60 | - | 0 |
| 1143 | $4.49 | - | 0 |
| 1145 | $1.50 | - | 0 |
| 1146 | $5.89 | - | 0 |

### Accepted

- Accepted packets: 15
- Cost per accepted change: $8.91

### First pass

- First-pass rate: -
- Eligible packets: 0

| id | implementer sessions | reviewer sessions | fix commits | first pass |
|---|---:|---:|---:|---:|
| TP-00 | 0 | 0 | - | - |
| TP-01a | 0 | 0 | - | - |
| TP-01b | 0 | 0 | - | - |
| TP-02 | 0 | 0 | - | - |
| TP-03 | 0 | 0 | - | - |
| TP-04 | 0 | 0 | - | - |
| TP-05 | 0 | 0 | - | - |
| TP-06 | 0 | 0 | - | - |
| TP-07 | 0 | 0 | - | - |
| TP-08 | 0 | 0 | - | - |
| TP-09 | 0 | 0 | - | - |
| TP-10 | 0 | 0 | - | - |
| TP-05b | 0 | 0 | - | - |
| TP-11 | 0 | 0 | - | - |
| TP-12 | 0 | 0 | - | - |

### Retries

No retries.

### CI

- First-attempt rate: n/a (no CI data)
- Overall rate: n/a (no CI data)
- Code PRs first-attempt rate: n/a (no CI data)
- Queue first-attempt rate: n/a (no CI data)
- PR CI runs, total: 0

### Escaped defects

- Escaped defects: 1
- Rate per merged PR: 0.05

| kind | citing item | cites PR |
|---|---:|---:|
| pr | 1139 | 1130 |

### Escalation

- Dispatched to Haiku: 0
- Escalated: 0
- Escalation rate: -

### Stop context

- Workers: 20
- Context at stop, p50 / p90 / max: 207,924 / 330,798 / 447,259
- Share above 90k: 100.0%

| stop | count |
|---|---:|
| end_turn | 20 |
| tool_use | 0 |
| error | 0 |
| no_record | 0 |

No error kinds recorded.

90k is the Haiku worker context cap.

### Cycle time

- Campaign hours: 5.9
- Packet hours: p50 0.3, p90 0.9 over 14 packets

### Overstep

predates the regime switch; the rules apply to the Haiku regime
