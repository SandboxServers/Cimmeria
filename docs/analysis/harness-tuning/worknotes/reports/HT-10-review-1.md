## Verdict
ship

## Fix notes
None.

## Not fixed
None. Checked, no findings:
- `prefix_owners()` run at 5d44d6100 lists 26 multi-folder prefixes. `GRANDFATHERED` has exactly those 26, each with its exact folders. `NOT_PACKETS` is empty because the scan found no non-packet tokens.
- Probe: copied `docs/analysis/harness-tuning/README.md` to `docs/analysis/zz-probe/README.md`. The new test failed with "D-HT is used by harness-tuning, zz-probe; HT is used by harness-tuning, zz-probe". The probe folder is deleted and `git status` is clean.
- `python -m unittest discover -s tools/agent-skills -p "test_*.py"`: 91 tests OK (1 skipped), exit 0.
- Line endings: `test_ledgers.py` is 95 lines, all CRLF. `agent-skills.yml` has no CR. The `docs/analysis/**/*.md` line is added after the `RESUME.md` line in both `pull_request.paths` and `push.paths`.
- The regexes match C7 verbatim. A prefix passes only when its folders are a subset of its GRANDFATHERED entry. The failure message names the prefix and its folders and cites campaign-packet skill § 1.
- Scan of the main checkout, including its untracked campaign folders, reports no un-grandfathered collision.