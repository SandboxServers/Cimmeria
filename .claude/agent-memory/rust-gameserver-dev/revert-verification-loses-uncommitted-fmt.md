# Revert-verification workflow: `git checkout --` silently undoes an uncommitted `cargo fmt`

When proving a regression guard by temporarily reverting the fix, the safe
restore is `git checkout -- <file>` against a WIP checkpoint commit. The trap:
**anything done to the working tree *after* that checkpoint — most often a
`cargo fmt --all` pass — is thrown away by the restore, and nothing warns you.**
The tests still pass (formatting isn't a test), so the loss is invisible until
the final `cargo fmt --all -- --check` fails right before the commit — or, if
you skip that, until CI's `fmt` job does.

Working order that avoids it:

1. Implement.
2. `cargo fmt --all` **first**, then `clippy`, then tests.
3. *Then* make the WIP checkpoint commit.
4. Run the revert experiments, restoring with `git checkout -- <file>`.
5. Re-run `cargo fmt --all -- --check` once more before the real commit anyway.

Also: prefer a WIP commit over `git stash push` for the checkpoint. `git stash
push` reverts the working tree as a side effect, so you immediately have to
`git stash apply <sha>` to get your work back, and the stash stack is shared
with other worktrees and concurrent Claude sessions.

## Same trap when splitting one working tree into several commits

Splitting a finished change into per-finding commits hits this harder, because
the obvious technique for "commit only part of file X" is
`git checkout -- X` → re-apply just the subset → `git add X` → commit. If X
carries edits belonging to *two* commits (e.g. one doc file touched by two
findings), the revert drops **both** subsets and you only re-apply one. The
second is gone silently: it is no longer in the working tree, so nothing
flags it and `git status` looks clean.

Before splitting, copy every multi-owner file aside (`cp X /tmp/full/`) and
after the last commit `diff` the working tree against those copies — **not**
`git show HEAD:X`, which comes back LF-normalised against a CRLF file and
reports the whole file as changed. Recovery without an interactive rebase:
`git reset --soft HEAD~1` → `git restore --staged .` → re-apply → `git add` →
`git commit --amend --no-edit` → re-stage and re-commit the undone commit.

One more gotcha in this repo: `git add crates/services` staged files under the
gitignored `crates/services/logs/`. Check `git status --short` after staging
and `git restore --staged crates/services/logs` if they appear — the
legacy-command-parity README explicitly requires keeping that directory out of
every packet.
