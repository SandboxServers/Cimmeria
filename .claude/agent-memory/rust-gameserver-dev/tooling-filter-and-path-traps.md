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

**3. Editing CRLF docs from a python script: never put `\r\n` in the
replacement text.** `docs/**/*.md` and `crates/**/README.md` are CRLF. The
working pattern is to author the new text with plain `\n` and normalise once:

```python
s = open(p, encoding="utf-8", newline="").read()   # newline="" preserves CRLF
def sub(old, new):
    o, n = old.replace("\n", "\r\n"), new.replace("\n", "\r\n")
    assert o in s, old[:150]                        # assert, or a silent no-op
    ...
open(p, "w", encoding="utf-8", newline="").write(s)
```

The trap: if the replacement string already contains a literal `\r\n`, that
final `.replace("\n", "\r\n")` turns it into `\r\r\n`. One bare CR makes `file`
report "CRLF, CR line terminators" and `git diff --stat` shows the **whole file**
rewritten (611 insertions / 449 deletions on a 449-line README) — which looks
exactly like an accidental line-ending flip and is easy to "fix" by reverting
good work. Check with `file <path>` after every scripted doc edit; find the
offender with `re.finditer(rb"\r(?!\n)", open(p,"rb").read())`.

**3b. Never put a Windows path — or any backslash — in scripted replacement
text.** Writing `` `$C\\navmesh\\harset\\Harset\\mse13.nav` `` inside a
`python - <<'PY'` heredoc produced `$C` + a real newline + `avmesh\harset\...`
in the output markdown: the heredoc/tool layer collapsed `\\` to `\`, so Python
saw `\n` and turned it into a line break **mid-word, inside a sentence**. The
only warning was a `SyntaxWarning: invalid escape sequence '\h'` that scrolled
past above a cheerful `ok`, and the file's line-ending check still came back
clean — CRLF 110 / bare LF 0 — because a newline *is* legal, just not there.

Prose with a path in it is exactly where this bites. Either spell the path with
forward slashes, describe it instead of quoting it, or **use the Edit tool**,
which takes the string literally and needs no escaping. And read back the
rendered paragraph after any scripted prose edit — `git diff --stat` cannot see
a newline that landed in the middle of a sentence, because the line count is
still plausible.

Two smaller ones from the same session:

- The Bash tool **refuses** `lane.sh <cmd>` when any argument is computed at
  runtime (`$VAR`, `$(...)`) or the command is a complex heredoc, because it
  can't prove the command isn't `git`. Write the payload to a file first and
  pass a plain literal path — including for list arguments, which means a bin
  that takes `<list-file>` beats one that takes `<item>...`.
- A python `.replace()` against a CRLF file **silently does nothing** when the
  pattern has LF newlines. Always `assert old in s` before writing, or the
  script prints "ok" having changed nothing.

Related: [[revert-verification-loses-uncommitted-fmt]],
[[cargo-test-vs-nextest-flakiness]].
