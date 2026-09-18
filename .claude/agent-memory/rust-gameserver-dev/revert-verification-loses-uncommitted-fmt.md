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

Scripting the temporary revert: **`crates/**/*.rs` are stored CRLF in this
repo, not just `docs/`.** A Python/sed patch script that matches exact
multi-line strings with `\n` silently misses every hunk (and `cat -A` through
Git Bash can still print bare `$`, so it looks LF). Read the file as bytes,
`replace('\r\n', '\n')` before matching, and restore the original convention on
write — otherwise the "revert" is a no-op and the guard looks like it passes
when reverted.

Another gotcha in this repo: `git add crates/services` staged files under the
gitignored `crates/services/logs/`. Check `git status --short` after staging
and `git restore --staged crates/services/logs` if they appear — the
legacy-command-parity README explicitly requires keeping that directory out of
every packet.
