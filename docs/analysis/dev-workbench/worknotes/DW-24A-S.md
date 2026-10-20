# DW-24A-S stock XML audit worknote

The operator-supplied stock `SlashCommands.xml` and `FinalSlashCommands.xml`
were read without copying them into the repository. Their respective SHA-256
hashes are `6e34355a628bd3b5f95e31ef58e7a2e9700dcce6b9798c751bda2431ba0dcd66`
and `7d166f89d3b6829d2fe6c891c90104c5562d1de653864b10d7dd06fbf97263db`;
the QA copy matches. Parsed rows: 98 plus 4. Only `/reload` has explicit
`Access=qd`, with a required Integer `reloadType`; the stock role legend
calls `q` QA and `d` Designer. Its GM authorization, event binding and
native submission remain unproved. The fixture's `complete` state covers
only inspection of those two XML files; the verified GM addendum is empty.

Changed the stock fixture, its regression guard and stock audit document.
The guard pins both copies' hash/count records and the unresolved
`/reload` exclusion; omission or hash drift fails it. No runtime execution
path or client file changed. DW-27C must resolve whether `/reload` belongs
in a disabled stock form and cannot claim full GM coverage from this audit
alone. DW-24B still owns native transport proof.

Branch: `dev-workbench/dw-24a-stock-audit`.
Worktree: `.claude/worktrees/dw-24a-stock-audit`.
Checks passed: `python tools/client-patches/gm_slash_commands.py --check`;
`cargo fmt -p cimmeria-lab -- --check`; `cargo clippy -p cimmeria-lab
--all-targets -- -D warnings`; focused `cargo test -p cimmeria-lab
workbench::slash_inventory_tests` (7 passed). Cargo commands ran through
the PowerShell build lane. Removing the `/reload` exclusion caused the
new guard to fail, and restoring it passed all seven focused tests. A first
Clippy attempt could not compile without the worktree's standard ignored
`external/` junction; a first test command incorrectly selected a
nonexistent library target. Both were corrected before final checks.
