# lab-harness

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
| total | $48.91 |
| attributed | $34.55 |
| unattributed in campaign | $14.37 |

By model tier:

| model tier | USD |
|---|---:|
| fable | $0.00 |
| haiku | $0.76 |
| opus | $17.15 |
| other | $0.00 |
| sonnet | $31.00 |

By role:

| role | USD |
|---|---:|
| coordinator | $13.83 |
| implementer-haiku | $0.66 |
| other | $32.45 |
| reviewer | $1.97 |

### PRs

- PRs: 1, merged 1
- Cost per PR: mean $34.55, p50 $34.55, p90 $34.55

| PR | USD | CI first pass | CI runs |
|---|---:|---:|---:|
| 1663 | $34.55 | yes | 2 |

### Accepted

- Accepted packets: 20
- Cost per accepted change: $2.45

### First pass

- First-pass rate: 0.0%
- Eligible packets: 18

| id | implementer sessions | reviewer sessions | fix commits | first pass |
|---|---:|---:|---:|---:|
| LH-01 | 1 | 0 | - | no |
| LH-02 | 1 | 0 | - | no |
| LH-03 | 1 | 0 | - | no |
| LH-04 | 1 | 0 | - | no |
| LH-05 | 1 | 0 | - | no |
| LH-06 | 1 | 0 | - | no |
| LH-07 | 1 | 0 | - | no |
| LH-08 | 1 | 0 | - | no |
| LH-09 | 1 | 0 | - | no |
| LH-10 | 1 | 0 | - | no |
| LH-11 | 1 | 0 | - | no |
| LH-12 | 1 | 0 | - | no |
| LH-13 | 1 | 0 | - | no |
| LH-14 | 1 | 0 | - | no |
| LH-15 | 1 | 0 | - | no |
| LH-16 | 1 | 0 | - | no |
| LH-17 | 1 | 0 | - | - |
| LH-19 | 2 | 0 | - | no |
| LH-20 | 1 | 0 | - | no |
| LH-18 | 0 | 0 | - | - |

### Retries

- Retries: 1

| reason | count |
|---|---:|
| other | 1 |

### CI

- First-attempt rate: 100.0%
- Overall rate: 100.0%
- Code PRs first-attempt rate: 100.0%
- Queue first-attempt rate: 100.0%
- PR CI runs, total: 2

### Escaped defects

- Escaped defects: 0
- Rate per merged PR: 0

### Escalation

- Dispatched to Haiku: 19
- Escalated: 18
- Escalation rate: 95.0%

| packet | escalated to | why |
|---|---|---|
| LH-01 | general-purpose | other |
| LH-02 | general-purpose | other |
| LH-03 | general-purpose | other |
| LH-04 | general-purpose | context-exhausted |
| LH-05 | general-purpose | other |
| LH-06 | general-purpose | other |
| LH-07 | general-purpose | other |
| LH-08 | general-purpose | other |
| LH-09 | general-purpose | other |
| LH-10 | general-purpose | other |
| LH-11 | general-purpose | other |
| LH-12 | general-purpose | other |
| LH-13 | general-purpose | other |
| LH-14 | general-purpose | other |
| LH-15 | general-purpose | other |
| LH-16 | general-purpose | other |
| LH-19 | general-purpose | other |
| LH-20 | general-purpose | other |

### Stop context

- Workers: 39
- Context at stop, p50 / p90 / max: 77,953 / 96,069 / 121,860
- Share above 90k: 26.0%

| stop | count |
|---|---:|
| end_turn | 39 |
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
