# any-install-adoption

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
| total | $50.14 |
| attributed | $33.84 |
| unattributed in campaign | $16.29 |

By model tier:

| model tier | USD |
|---|---:|
| fable | $0.00 |
| haiku | $3.08 |
| opus | $43.05 |
| other | $0.00 |
| sonnet | $4.01 |

By role:

| role | USD |
|---|---:|
| coordinator | $29.99 |
| implementer-domain | $6.89 |
| implementer-haiku | $3.06 |
| other | $6.19 |
| reviewer | $4.01 |

### PRs

- PRs: 1, merged 1
- Cost per PR: mean $33.84, p50 $33.84, p90 $33.84

| PR | USD | CI first pass | CI runs |
|---|---:|---:|---:|
| 1658 | $33.84 | yes | 3 |

### Accepted

- Accepted packets: 0
- Cost per accepted change: n/a (the ledger counts no accepted packets)

### First pass

- First-pass rate: -
- Eligible packets: 0

| id | implementer sessions | reviewer sessions | fix commits | first pass |
|---|---:|---:|---:|---:|

### Retries

- Retries: 31

| reason | count |
|---|---:|
| context-exhausted | 17 |
| other | 14 |

### CI

- First-attempt rate: 100.0%
- Overall rate: 100.0%
- Code PRs first-attempt rate: 100.0%
- Queue first-attempt rate: 100.0%
- PR CI runs, total: 3

### Escaped defects

- Escaped defects: 1
- Rate per merged PR: 1

| kind | citing item | cites PR |
|---|---:|---:|
| pr | 1664 | 1658 |

### Escalation

- Dispatched to Haiku: 14
- Escalated: 0
- Escalation rate: 0.0%

### Stop context

- Workers: 48
- Context at stop, p50 / p90 / max: 90,652 / 95,304 / 286,866
- Share above 90k: 52.0%

| stop | count |
|---|---:|
| end_turn | 48 |
| tool_use | 0 |
| error | 0 |
| no_record | 0 |

No error kinds recorded.

90k is the Haiku worker context cap.

### Cycle time

- Campaign hours: n/a (no close-out commit in the ledger window)
- Packet hours: p50 -, p90 - over 0 packets

### Overstep

- Opus coding: $1.59
- Opus research: $0.00
- Sonnet implementing: $0.00
- Recoverable: $0.80 to $1.26
- Wasted (repriced at intended tier): $1.26

| rule | count | USD | wasted USD |
|---|---:|---:|---:|
| O1 | 11 | $1.59 | $1.26 |

| rule | USD | tool | path class | model |
|---|---|---|---|---|
| O1 | $0.30 | Edit | crates | claude-opus-5-5 |
| O1 | $0.34 | Edit | crates | claude-opus-5-5 |
| O1 | $0.44 | Edit | crates | claude-opus-5-5 |
| O1 | $0.04 | Edit | crates | claude-opus-5-5 |
| O1 | $0.06 | Edit | crates | claude-opus-5-5 |
| O1 | $0.16 | Edit | crates | claude-opus-5-5 |
| O1 | $0.04 | Edit | crates | claude-opus-5-5 |
| O1 | $0.04 | Edit | crates | claude-opus-5-5 |
| O1 | $0.03 | Edit | crates | claude-opus-5-5 |
| O1 | $0.02 | PowerShell | - | claude-opus-5-5 |
| O1 | $0.12 | Edit | crates | claude-opus-5-5 |
