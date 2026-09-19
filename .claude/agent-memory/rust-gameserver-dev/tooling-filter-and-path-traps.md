---
name: tooling-filter-and-path-traps
description: Two silent-failure traps — live-db-test.sh forwards args as nextest POSITIONAL substrings (a test() filterset matches nothing, exit 4), and gh -F body=@FILE in Git Bash needs a Windows-style path even under MSYS_NO_PATHCONV
metadata:
  type: feedback
---

Two tooling traps that fail in ways that look like something else.

**1. `live-db-test.sh <filter>` forwards its arguments as nextest *positional*
substring filters, not as a `-E` filterset.** A nextest filterset expression
passed positionally is treated as a literal test-name substring, matches zero
tests, and the run ends `Starting 0 tests across 1 binary` / `error: no tests to
run` / exit 4 — *after* burning a ~30 s database reload and a lane slot.

```bash
# WRONG — matches nothing, exit 4
$L/live-db-test.sh 'test(livewire_pairs) or test(start_minigame_difficulty)'
# RIGHT — bare substrings, space-separated
$L/live-db-test.sh livewire_pairs start_minigame_difficulty minigame_result
```

**Why:** the wrapper's last line is `cargo nextest run --profile=ci-live-db -p
cimmeria-services --lib "$@"`. Positional args are plain substring filters;
filterset syntax only works behind `-E`.

**How to apply:** always pass bare substrings, and treat a `0 tests run` summary
as a failed run (WORKER-RULES.md says so explicitly). Same trap applies to
`lane.sh cargo test` — that's libtest, which has no filterset syntax at all.

**2. `gh api -F body=@<file>` in Git Bash needs a Windows-style path.** `gh` is
a native Windows binary, so an MSYS path (`/c/Users/...`) fails with `error
parsing "body" value: ... The system cannot find the path specified`. Setting
`MSYS_NO_PATHCONV=1` does *not* fix it — that only stops MSYS from rewriting the
path, which is the opposite of what's needed.

```bash
D="C:/Users/Steve/AppData/Local/Temp/.../bodies"   # Windows path, forward slashes
MSYS_NO_PATHCONV=1 gh api -X POST "$R/$id/replies" -F "body=@$D/reply.md"
```

Keep `MSYS_NO_PATHCONV=1` for the *URL* argument (which contains `/` segments
MSYS would mangle into a drive path) and give the `@file` a Windows path. This is
the reliable way to post multi-line PR review replies — a heredoc through
`-f body=...` mangles markdown tables and backticks.

Related: [[revert-verification-loses-uncommitted-fmt]],
[[cargo-test-vs-nextest-flakiness]].
