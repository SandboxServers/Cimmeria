# lab-cli-rust

> Regime: haiku

### Stamp

- Regime: haiku
- Generated: 2026-10-12T18:51:11Z
- Window: 2026-09-21T00:00:00Z to 2026-10-13T00:00:00Z
- Price table: 2026-10-12
- Profiler schema: 6
- Unattributed share: 12.2%
- Unmapped share: 38.0%

Dollars are list-price estimates of plan usage, a floor, never a bill.

### Cost

|  | USD |
|---|---:|
| total | $123.32 |
| attributed | $69.17 |
| unattributed in campaign | $54.15 |

By model tier:

| model tier | USD |
|---|---:|
| fable | $0.00 |
| haiku | $2.08 |
| opus | $51.06 |
| other | $0.00 |
| sonnet | $70.18 |

By role:

| role | USD |
|---|---:|
| coordinator | $51.06 |
| fixer | $26.56 |
| implementer-haiku | $2.08 |
| other | $37.94 |
| reviewer | $5.67 |

### PRs

- PRs: 2, merged 1
- Cost per PR: mean $53.38, p50 $53.38, p90 $53.38

| PR | USD | CI first pass | CI runs |
|---|---:|---:|---:|
| 1654 | $15.79 | yes | 2 |
| 1659 | $53.38 | yes | 4 |

### Accepted

- Accepted packets: 20
- Cost per accepted change: $6.17

### First pass

- First-pass rate: 0.0%
- Eligible packets: 18

| id | implementer sessions | reviewer sessions | fix commits | first pass |
|---|---:|---:|---:|---:|
| LX-01 | 3 | 0 | - | no |
| LX-02 | 4 | 0 | - | no |
| LX-03 | 0 | 0 | - | - |
| LX-04 | 3 | 0 | - | no |
| LX-05 | 2 | 0 | - | no |
| LX-06 | 4 | 0 | - | no |
| LX-07 | 1 | 0 | - | no |
| LX-08 | 2 | 0 | - | no |
| LX-09 | 1 | 0 | - | no |
| LX-10 | 3 | 0 | - | no |
| LX-11 | 4 | 0 | - | no |
| LX-12 | 4 | 0 | - | no |
| LX-13 | 4 | 0 | - | no |
| LX-14 | 1 | 0 | - | no |
| LX-15 | 4 | 0 | - | no |
| LX-16 | 5 | 0 | - | no |
| LX-17 | 7 | 0 | - | no |
| LX-18 | 10 | 0 | - | no |
| LX-19 | 3 | 1 | - | no |
| LX-20 | 0 | 0 | - | - |

### Retries

- Retries: 47

| reason | count |
|---|---:|
| context-exhausted | 9 |
| other | 38 |

### CI

- First-attempt rate: 100.0%
- Overall rate: 100.0%
- Code PRs first-attempt rate: 100.0%
- Queue first-attempt rate: 67.0%
- PR CI runs, total: 7

### Escaped defects

- Escaped defects: 0
- Rate per merged PR: 0

### Escalation

- Dispatched to Haiku: 18
- Escalated: 2
- Escalation rate: 11.0%

| packet | escalated to | why |
|---|---|---|
| LX-01 | general-purpose | other |
| LX-17 | general-purpose | other |

### Stop context

- Workers: 91
- Context at stop, p50 / p90 / max: 76,941 / 117,559 / 222,715
- Share above 90k: 29.0%

| stop | count |
|---|---:|
| end_turn | 91 |
| tool_use | 0 |
| error | 0 |
| no_record | 0 |

No error kinds recorded.

90k is the Haiku worker context cap.

### Cycle time

- Campaign hours: n/a (no close-out commit in the ledger window)
- Packet hours: p50 -, p90 - over 0 packets

### Overstep

- Opus coding: $0.00
- Opus research: $0.00
- Sonnet implementing: $0.00
- Recoverable: $0.00 to $0.00
- Wasted (repriced at intended tier): $0.00
