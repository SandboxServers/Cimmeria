# Campaign metrics summary

## Stamp

- Generated: 2026-10-12T18:51:11Z
- Window: 2026-09-21T00:00:00Z to 2026-10-13T00:00:00Z
- Price table: 2026-10-12
- Dollars are a list-price estimate, a floor.

## Comparison

| metric | old | Haiku | delta |
|---|---:|---:|---:|
| Cost per merged PR, p50 | $18.18 | $0.91 | -$17.26 (better) |
| Cost per merged PR, mean | $22.01 | $2.67 | -$19.35 (better) |
| Cost per accepted change | $18.65 | $2.35 | -$16.29 (better) |
| First-pass rate | n/a | 5.0% | n/a |
| Retries per accepted packet | n/a | 0.60 | n/a |
| CI first-attempt rate | n/a | 100.0% | n/a |
| Escaped defects per merged PR | 0.04 | 0.01 | -0.03 |
| Packet cycle p50 | 1.1 h | 1.1 h | +0.0 h |
| Pure campaigns | 28 | 7 | n/a |
| Merged PRs | 248 | 185 | n/a |
| Total USD | $5,727.50 | $594.06 | n/a |
| Escalation rate | n/a | 17.0% | n/a |

Sample: old 28 campaigns and 174 packets; Haiku 7 campaigns and 223 packets.

Note: haiku minus old; for cost and retries a negative delta is better, for first-pass and CI a positive one is.

Per-packet and per-PR costs compare units of different size: Haiku packets are sliced smaller, so they overstate the saving. Read them next to the spend share outside campaigns in Caveats.

## Campaigns

| campaign | regime | USD | PRs | $/PR p50 | $/accepted | first pass | CI first | escaped |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| [npc-ai-restoration](npc-ai-restoration.md) | old | $816.85 | 36 | $16.05 | $22.69 | n/a | n/a | 2 |
| [debug-area](debug-area.md) | old | $601.69 | 16 | $35.03 | $28.65 | n/a | n/a | 0 |
| [crafting](crafting.md) | old | $522.24 | 18 | $23.48 | $29.01 | n/a | n/a | 1 |
| [named-telemetry](named-telemetry.md) | old | $505.71 | 23 | $19.05 | $21.07 | n/a | n/a | 0 |
| [social-systems](social-systems.md) | old | $456.44 | 18 | $24.71 | $25.36 | n/a | n/a | 1 |
| [pets](pets.md) | old | $426.52 | 12 | $35.36 | $35.54 | n/a | n/a | 0 |
| [bank-vault](bank-vault.md) | old | $343.55 | 12 | $25.30 | $28.63 | n/a | n/a | 0 |
| [black-market](black-market.md) | old | $296.97 | 10 | $23.88 | $29.70 | n/a | n/a | 3 |
| [launcher-consolidation](launcher-consolidation.md) | mixed | $291.64 | 22 | $12.10 | $13.89 | 0.0% | 100.0% | 0 |
| [telemetry-gaps](telemetry-gaps.md) | haiku | $287.71 | 150 | $0.78 | $1.81 | 9.0% | 100.0% | 0 |
| [ability-mechanics](ability-mechanics.md) | old | $275.44 | 13 | $15.97 | $21.19 | n/a | n/a | 0 |
| [organizations](organizations.md) | old | $274.64 | 11 | $21.53 | $24.97 | n/a | n/a | 0 |
| [dialog-ui-redesign](dialog-ui-redesign.md) | old | $201.82 | 8 | $23.26 | $25.23 | n/a | n/a | 0 |
| [pr-825](pr-825.md) | old | $185.01 | 1 | $7.13 | $185.01 | n/a | n/a | 0 |
| [class-start-v6](class-start-v6.md) | old | $177.94 | 8 | $21.64 | $29.66 | n/a | n/a | 0 |
| [ability-trees](ability-trees.md) | old | $159.71 | 9 | $19.58 | $17.75 | n/a | n/a | 0 |
| [ammo](ammo.md) | old | $147.46 | 19 | $7.45 | $8.19 | n/a | n/a | 1 |
| [token-usage](token-usage.md) | old | $133.60 | 22 | $5.28 | $8.91 | n/a | n/a | 1 |
| [lab-cli-rust](lab-cli-rust.md) | haiku | $123.32 | 2 | $53.38 | $6.17 | 0.0% | 100.0% | 0 |
| [lab-chaos](lab-chaos.md) | mixed | $120.00 | 7 | $9.76 | $10.91 | 0.0% | 100.0% | 0 |
| [lab-spec-vocab](lab-spec-vocab.md) | mixed | $97.91 | 15 | $3.78 | $6.12 | 0.0% | n/a | 0 |
| [lab-record](lab-record.md) | mixed | $95.40 | 8 | $8.10 | $11.93 | 0.0% | 100.0% | 0 |
| [lab-fixtures](lab-fixtures.md) | mixed | $90.26 | 11 | $10.77 | $30.09 | 0.0% | n/a | 3 |
| [pr-1164](pr-1164.md) | old | $65.08 | 1 | $8.51 | $65.08 | n/a | n/a | 0 |
| [wireclient-ci](wireclient-ci.md) | haiku | $56.77 | 16 | $1.17 | $3.78 | 11.0% | n/a | 0 |
| [any-install-adoption](any-install-adoption.md) | haiku | $50.14 | 1 | $33.84 | n/a | n/a | 100.0% | 1 |
| [lab-harness](lab-harness.md) | haiku | $48.91 | 1 | $34.55 | $2.45 | 0.0% | 100.0% | 0 |
| [lab-golden](lab-golden.md) | mixed | $44.60 | 10 | $4.96 | $4.46 | 0.0% | n/a | 0 |
| [campaign-metrics](campaign-metrics.md) | mixed | $44.32 | 0 | n/a | n/a | n/a | n/a | 0 |
| [castle-population](castle-population.md) | old | $44.30 | 1 | $44.30 | $44.30 | n/a | n/a | 0 |
| [pr-1302](pr-1302.md) | old | $44.19 | 2 | $22.09 | $22.09 | n/a | n/a | 0 |
| [pr-1178](pr-1178.md) | old | $22.36 | 2 | $11.18 | $11.18 | n/a | n/a | 0 |
| [client-limits](client-limits.md) | mixed | $21.81 | 9 | $2.14 | $2.42 | 0.0% | n/a | 0 |
| [lab-parallel-clients](lab-parallel-clients.md) | haiku | $19.28 | 10 | $1.68 | $1.93 | 0.0% | n/a | 0 |
| [historical-cellblocks](historical-cellblocks.md) | old | $18.67 | 1 | $18.67 | n/a | n/a | n/a | 1 |
| [lab-cli](lab-cli.md) | haiku | $7.92 | 6 | $0.89 | $1.13 | 0.0% | n/a | 0 |
| [lab-watch](lab-watch.md) | mixed | $4.11 | 4 | $0.28 | $1.03 | 0.0% | n/a | 0 |
| [pr-5](pr-5.md) | old | $1.62 | 1 | $1.62 | $1.62 | n/a | n/a | 0 |
| [legacy-command-parity](legacy-command-parity.md) | old | $1.55 | 2 | $0.78 | $0.78 | n/a | n/a | 0 |
| [pr-1241](pr-1241.md) | old | $1.21 | 1 | $1.21 | $1.21 | n/a | n/a | 0 |
| [pr-673](pr-673.md) | old | $1.08 | 1 | $1.08 | $1.08 | n/a | n/a | 0 |
| [castle-rebuild](castle-rebuild.md) | old | $0.96 | 2 | $0.48 | $0.48 | n/a | n/a | 0 |
| [harset-rebuild](harset-rebuild.md) | old | $0.48 | 1 | $0.48 | $0.48 | n/a | n/a | 0 |
| [pr-751](pr-751.md) | old | $0.43 | 1 | $0.43 | $0.43 | n/a | n/a | 0 |

## Overstep

- Old regime, recoverable overstep: n/a (no data).
- Haiku regime, recoverable overstep: $40.07 to $63.29.
- Fixer requests: 1,122.

## Caveats

- Unattributed share of spend: 12.2%; unmapped session share: 38.0%.
- Spend on PRs with no campaign: 26.5%.
- USD is a floor: list-price estimates of plan usage, not a bill.
- Mixed campaigns are excluded from the cost comparison: campaign-metrics, client-limits, lab-chaos, lab-fixtures, lab-golden, lab-record, lab-spec-vocab, lab-watch, launcher-consolidation.
- The overstep range covers every post-switch campaign, mixed ones included; cost comparisons use pure campaigns only.
- Untagged PRs: 280.
- Missing inputs: none.
- Small samples: no.

## Packet sizing vs the 90k Haiku cap

Packet sizing against the 90k Haiku context cap is summarized in the [README findings](../README.md#findings-for-the-report).

## Cost of this campaign

| role | USD |
|---|---:|
| coordinator | $44.32 |

Total $44.32 over 2,030 requests in 1 session.

Measured at close-out; excludes the close-out session's own tail.
