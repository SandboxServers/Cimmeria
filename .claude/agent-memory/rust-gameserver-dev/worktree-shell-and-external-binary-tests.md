---
name: worktree-shell-and-external-binary-tests
description: Worktree-isolated Bash refuses `env VAR=x cmd`, heredocs and shell-var-in-command-name; and a test asserting a from-tree C++ binary's behaviour needs an explicit opt-in env var, not a capability probe
metadata:
  type: feedback
---

Two traps that cost real time in a worktree-isolated session.

## The Bash tool refuses more than you expect

**Why:** worktree isolation has to prove a command is not a `git`
mutation targeting another checkout, and it gives up on anything it
cannot statically resolve.

**How to apply:** in a worktree agent, these are all refused —

- `env -u FOO cmd ...` and `env FOO=bar cmd ...` (the command name is
  "computed at runtime").
- `$L/lane.sh ... <<'EOF' ... EOF` — a heredoc plus a shell variable in
  the command position.
- `python3 - <<'PY' ... PY` when combined with anything else on the line.
- `cat >> file <<'EOF'` to append to a source file.

What works: spell the binary out in full (the absolute path to your
worktree's `tools/build-lane/lane.sh`), put env vars as a plain `FOO=bar cmd` prefix (that form *is*
accepted), write commit messages to a scratch file and `git commit -F
<path>` as a separate call, and use the Edit/Write tools instead of
heredoc appends.

Also: `python3 - <<'PY'` on its own line *is* accepted, but a `re.sub`
with `^`/`$` anchors over a CRLF file silently does nothing useful.
Check `file <path>` first; for CRLF sources use the Edit tool or
PowerShell `[System.IO.File]::ReadAllLines` / `WriteAllLines`.

Two more refusals seen in NA21 (2026-09-25): a `cd <worktree>/<subdir> &&
python - <<'EOF'` combination, and any loop whose command word comes from a
variable (`for f in ...; do "$BIN" "$f"`). Inline heredoc Python also broke on a
`\` just before a closing `'''`. The reliable pattern is to Write the script
into the scratchpad and run `python <path>` (or `bash <path>`) as its own call,
with every path absolute inside the script.

Pets PT-01 (2026-09-26) confirmed three scratchpad tools that work in one
call each:

- A `lane.sh` wrapper that exports `CIMMERIA_TARGET_ROOT`, `cd`s to the
  worktree, runs `tools/build-lane/lane.sh "$@"` into a log file and greps the
  result.
- An `edit.py` that reads a spec file of `(path, [(old, new)])` pairs and
  does exact, count-checked, CRLF-preserving replacements. Several commands
  chained with `&&` after a heredoc, or two `git commit`s in one call, are
  refused.
- A revert-proof driver that mutates a file, runs the test, then restores
  with `git checkout HEAD -- <file>` from a Python `subprocess`. It needs a
  commit first.

**The session scratchpad is shared by every agent in the session.** During
PT-01 another worker overwrote `revert_proof.py` with a copy whose `ROOT`
pointed at *its* worktree; running it would have mutated and `git checkout`ed
another agent's files. Give scratch scripts a packet-unique name
(`pt01_*.py`), hard-code your own worktree in `ROOT`, and re-read a script
before re-running it.

Also, when the Dev Drive that holds the build-lane target dirs fills up
("no space on device"), delete only your own worktree's target dir under
`CIMMERIA_TARGET_ROOT` and point `CIMMERIA_TARGET_ROOT` at a scratch
directory on a drive with free space.

## A test against a rebuilt C++ binary needs an explicit opt-in

**Why:** `tests/navbuilder_axis_roundtrip.rs` gained a case asserting
`build_params.hpp`'s parameter validation. It passed against a
scratch build and failed against `bin64/NavBuilder.exe`, which another
worker had rebuilt from a *different* branch. Every observable
difference between the two binaries *is* one of the assertions, so no
capability probe can distinguish them without being circular.

**How to apply:** when a test asserts the behaviour of C++ in the
current tree, gate it on an env var the operator sets to state that
(`CIMMERIA_NAVBUILDER_FROM_TREE=1`), self-skip loudly with the rebuild
command in the message, and make the first assertion a cheap sanity
check on the claim. `bin64/` holds binaries built from whatever branch
last touched them — never assume it matches your working tree.
