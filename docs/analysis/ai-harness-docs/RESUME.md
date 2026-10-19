# ai-harness-docs: resume card

Ledger: README.md (F1-F56, D-AH1-D-AH58). Packets: work-packets-w4.md (dispatch rule 0, R1-R11, C12-C16, AH-W40, W4a, ship procedure), work-packets-w4b.md (order, AH-23a/b, W5, S2-S6), work-packets-s4.md (C19, S4a-d), work-packets-w4c.md (C17, AH-25..27), work-packets-w3.md (W3c AH-11g/h/i, AH-15, AH-16). Drafts: w4-drafts.md, w4b-drafts.md. Owner answers 2026-10-18: w4-owner-answers-report.md, w4-s1s4-report.md.

## State (2026-10-18)

- W1, W2, W3, W3b, W3d merged (#1805-#1843); AH-11p merged as #1850 (retire `ah-11p`). Wave 4 ledger written in the main checkout, uncommitted; nothing of W4 dispatched.
- Owner answered: README holds the newcomer explanation (D-AH50); architect writes no code or fix (D-AH51); S2/S3/S5/S6 approved (D-AH52); Bash on Linux/WSL only (D-AH53, W4c); S1 vetoed (D-AH57); S4 approved, brief tag design (D-AH58).
- **Gate (rule 0, D-AH55):** nothing merges while `main` is red. #1844 closed via #1849; recheck before each merge.
- **Review loop (C12):** never dispatch `packet-review-fixer`; read-only `packet-reviewer`, fresh `packet-coder` per fix pass, up to 3 passes.

## Next action (order D-AH56, amended)

1. AH-W40: `ops` makes `ah-w4-ledger`; a packet-coder copies the 18 ledger files; `ops` ships and merges it (docs only).
2. Tell persistent-player-data's coordinator its waves use C12, not "one review-fixer each" (F39).
3. W3c: AH-11g, AH-11h, each with its packet loop. Then AH-11i (owner closes #1797).
4. W4a: one PR from `ah-w4a`. Then tell the user to relaunch.
5. In parallel: W4b (AH-23a then AH-23b, one PR), S6, S5a, S5b, S4 (S4a-d in `ah-s4`, sequential, one PR).
6. W4c after W4b (one PR from `ah-w4c`; relaunch after); S5c after S5a+S5b; S3 after W4b.
7. W5: AH-24 after W4b, W4c, AH-11i, S3, S5c, S6, S4; then AH-15 (owner), AH-16.
8. Retire `ah-17..21` and `ah-25..27` with `--merged-into <PR>` once S5c merged; else owner `--force`.

## Owner questions open

- D-AH33: AH-15 Linux/WSL leg required (recommended) or best effort. (S1 vetoed; S4 approved.)

## Loop rules (C12)

- Coder `blocked` with reason `context`: fresh packet-coder, "continuation N of 3"; then one Sonnet implementer.
- Reviewer `verdict=fix` at pass 1 or 2: fresh packet-coder, "review fix, pass N of 3"; then a fresh reviewer.
- `fix` at pass 3, or `decision`: architect ruling note only (writes no code or fix), one Sonnet implementer, one fresh reviewer.
- S4 briefs: never write the fix-pass phrase with a digit except in a real fix pass (C19 item 6).
