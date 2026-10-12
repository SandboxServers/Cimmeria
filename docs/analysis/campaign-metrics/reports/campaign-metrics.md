# campaign-metrics

> Regime: mixed

### Stamp

- Regime: mixed
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
| total | $44.32 |
| attributed | $0.00 |
| unattributed in campaign | $44.32 |

By model tier:

| model tier | USD |
|---|---:|
| opus | $26.05 |
| sonnet | $16.64 |
| haiku | $1.63 |
| fable | $0.00 |
| other | $0.00 |

By role:

| role | USD |
|---|---:|
| coordinator | $44.32 |

### PRs

- PRs: 0, merged 0
- Cost per PR: mean -, p50 -, p90 -

| PR | USD | CI first pass | CI runs |
|---|---:|---:|---:|

### Accepted

- Accepted packets: 0
- Cost per accepted change: n/a (no packet table)

### First pass

- First-pass rate: -
- Eligible packets: 0

| id | implementer sessions | reviewer sessions | fix commits | first pass |
|---|---:|---:|---:|---:|

### Retries

- Retries: 16

| reason | count |
|---|---:|
| context-exhausted | 5 |
| other | 11 |

### CI

- First-attempt rate: n/a (no CI data)
- Overall rate: n/a (no CI data)
- Code PRs first-attempt rate: n/a (no CI data)
- Queue first-attempt rate: n/a (no CI data)
- PR CI runs, total: 0

### Escaped defects

- Escaped defects: 0
- Rate per merged PR: n/a (no merged PRs)

### Escalation

- Dispatched to Haiku: 21
- Escalated: 0
- Escalation rate: 0.0%

### Stop context

- Workers: 81
- Context at stop, p50 / p90 / max: 57,104 / 99,415 / 184,788
- Share above 90k: 21.0%

| stop | count |
|---|---:|
| end_turn | 79 |
| tool_use | 1 |
| error | 0 |
| no_record | 1 |

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
