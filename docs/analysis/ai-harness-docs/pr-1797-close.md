# Closing reply on #1797 (owner posts, AH-11i)

Post after AH-11g and AH-11h merge (D-AH32). Replace `#<a>` with the AH-11a PR number and `#<b>`..`#<h>` with the AH-11b to AH-11h PR numbers, then run `gh pr comment 1797 --body-file <this file, without this header>` and `gh pr close 1797`. Post only the text below the line.

---

Thanks again, @aurablacklight. We took this over rather than ask for another round, and it has landed in #<a>:

- Your commit is in #<a> unchanged, cherry-picked onto main, so you are its author. The squash merge carries a `Co-authored-by` line for you.
- A second commit trims it to the maintainer's platform position: **Windows with PowerShell 7 is fully supported; WSL, Linux, and Git Bash on Windows are not validated, but not unsupported.** It also drops the open-question sentence, states the WSL2 loopback fact inline (a dead endpoint at `[::1]:9` is refused at once) instead of linking into agent memory, and points the by-hand `external/` link at the existing worktree section.
- One correction: `rm-worktree.sh` does run on Linux. What stopped you was `ship.sh merge --retire`, which died at `python` before it got there.

On your questions:

2. **A `.sh` path for `ops`: now yes.** This reverses the earlier "not for now". Your list became the fixes:
   - `mk-worktree.sh` symlinks `external/` off Windows (#<b>);
   - `ship.sh` and `rebase-pr.sh` find `python3` (#<c>);
   - `ship.py` falls back to `gh pr view --json statusCheckRollup` and `gh api` on an old `gh` (#<d>);
   - a Linux CI job runs `bash -n`, shellcheck and the scripts' tests (#<e>).

   `ops` now picks the shell from its platform and runs the `.sh` twins on Linux and WSL (#<g>), and the WSL and Linux section says what is fixed (#<h>). The packet workers still need PowerShell, so Linux stays "not validated" until a full session has run end to end there.
3. **Orchestrator default for interactive sessions only: unchanged.** No setting scopes `agent` by interactivity, so the default applies to every launch. Headless and CI runs pass `--agent <name>` or `--setting-sources user`, which your headless subsection documents, and the session banner names the persona at the start of every session.

Closing as superseded by #<a>. Thank you for the write-up; it saved us rediscovering all of it.
