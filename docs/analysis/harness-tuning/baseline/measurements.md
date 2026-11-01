# Harness-tuning baseline: measurements

> **Type:** Reference · **Audience:** The harness-tuning orchestrator and HT-19 · **Window:** 2026-10-03 to 2026-10-31 (--until 2026-11-01 excludes the 1st) · **Base:** 5f93b75d6

HT-05 ran the three `metrics` subcommands over the window, with `--repo` set to the main checkout. Each printed `status=ok`:

- `status=ok returns contract=1692 writers_no_line=19.4 writers_no_report=8.1 coder_no_line_uncapped=34.6 coder_capped=240`
- `status=ok coder-context coders=1209 stops=249 stop_rate=0.206 read_share=0.485 grep_share=0.246 continuations=86`
- `status=ok resume-cards compactions=45 firings=32 wrong=13 wrong_pct=40.6`

The raw outputs are `returns.json`, `coder-context.json` and `resume-cards.json` in this folder. The ledger values are README F1-F7 and F3a. "Within 10%" means the measured value is within 10% of the ledger value, relative to the ledger value. The ledger README is not edited; the orchestrator copies the numbers. `returns.json` counts an agent-type label that has no agent file and no built-in or retired name under `other` (HT-02a); on real transcripts the `other` group holds 528 of 4,192 dispatches (12.6%), none of them contract dispatches, with 32 context stops; the labels were not listed or examined.

| metric | ledger value | measured | within 10%? | note |
|---|---|---|---|---|
| contract dispatches | 1,692 (F1) | 1,692 | yes | exact |
| strict one-line returns | 1,109 (65.5%, F2) | 1,103 (65.2%) | yes | 6 fewer, 0.5% |
| no usable line, all contract dispatches | 257 (15.2%, F2) | 404 (23.9%) | no | Expected gap, SHA rule: C1 accepts only hex or `-` as the SHA. Buried lines fall from 169 to 27, with strict and line-first changing by -5 net. The 147 extra non-usable lines are lines the ledger's LINE accepted with a non-hex SHA token. |
| no report write, all contract dispatches | 226 (13.4%, F3) | 267 (15.8%) | no | Not one of the two expected gaps. The ledger's rule is the same in words, so the cause is open. See the findings below. |
| report writers: contract dispatches | 691 (F3a) | 691 | yes | exact |
| report writers: no usable line | 101 (14.6%, F3a) | 134 (19.4%) | no | Same SHA rule as the row above, inferred; not split per writer type. |
| report writers: no report write | 38 (5.5%, F3a) | 56 (8.1%) | no | Not explained by the expected gaps. Open, as the no-report row above. |
| packet-coder no usable line, cap not fired | 34/696 (4.9%, F3a) | 335/969 (34.6%) | no | Expected gap 2. The ledger counted contract coders outside context stops (696). `coder_lines` counts every packet-coder dispatch without a cap denial (1,209 minus 240), so both the numerator and the denominator widen. |
| packet-coder context stops | 167/863 (19.4%, F4) | 249/1,209 (20.6%) | count no, rate yes | Same scope as expected gap 2: `coder-context` counts every packet-coder dispatch (1,209), not the 863 contract coders, and a stop includes a cap denial. The rate is within 10%; the count is not comparable. |
| cap denials | 240 (F5) | 240 | yes | exact; `coder_lines.capped` |
| continuations | 87 (F4) | 86 | yes | 1.1% |
| continuation fresh tokens | 6.9M (F4) | 6.67M | yes | 3.3% |
| median fresh tokens per continuation | mean about 79k (F4; the README asks HT-05 for the median) | 75.2k (75,240.5) | n/a | No ledger median to compare. This is the baseline HT-16 and HT-19 use. |
| wrong-campaign resume cards | 14/32 (44%, F7) | 13/32 (40.6%) | yes | one card (7%); all firings are tier `active` |
| Read share | 50.9% (F6) | 50.6% | yes | stopped group (stops and cap denials) |
| Grep share | 26.4% (F6) | 26.7% | yes | stopped group |
| median output | 37.1k (F6) | 37.0k (37,001) | yes | stopped group |

## Findings

- **No report write is higher than the ledger, in both groups.** All contract dispatches measure 267 (15.8%) against 226 (13.4%), and report writers measure 56 (8.1%) against 38 (5.5%). Neither gap is one of the two expected causes, and the metric was not changed. The next step is to check a few of the 267 against their transcripts, which the owner or the HT-16 ruling can do. By agent type the rise is packet-coder +15 (148 to 163), packet-reviewer +11 (8 to 19, more than double), architect +4 and ops +3. The reviewer rise is the largest in relative terms and is where to look first. Not examined: why; no transcript was read.
- **Non-usable return lines are higher than the ledger, from the SHA rule.** The gap is expected, and the ledger's figures are not comparable with C1 on this point. The README's F2 and F3a percentages should be restated on the C1 rule before HT-06 is judged against them.
- **Resume cards on real transcripts (HT-04's open item).** Measured: the 45 compactions in the window gave 32 firings, all from the active-campaign attachment (tier `active`; tier `inferred` fired 0 times). 41 compactions were followed by an Agent dispatch, and 2 read a RESUME.md before it. Those counts equal the ledger's 32 firings, 41 compactions and "2 of 41", so the real attachment shape and the real `Agent` tool name are matched by the code. The verdicts split 15 matched, 13 wrong, 3 mixed and 1 undetermined, against the ledger's 15, 14, 2 and 1: one card moved from wrong to mixed, and the cause is not examined. Not measured: the tier-2 (inferred) path has never fired on real data, so it is tested only on fixtures; the 13 compactions with no firing (45 minus 32) are not broken down (no card, or no dispatch after it); no card's verdict was checked by hand against its transcript.