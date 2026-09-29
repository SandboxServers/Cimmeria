---
name: shepherd-merge-traps
description: Landing a stack of PRs by merging main into each branch — clean merges can duplicate a section or an index line; const-slice ptr::eq tests fail on i686.
metadata:
  type: project
---

Found while landing #1090, #1084, #1089 and #1086 in a row (2026-09-29).

- **A "clean" merge can duplicate text.** A branch that already carried a
  sibling PR's section (pre-merged from that PR's tip) got the same
  section again when main's squash of that PR was merged in: git saw two
  adds at different offsets, reported no conflict, and the doc had
  `## Volume control: the governor` twice. Same for agent-memory
  `MEMORY.md` index lines edited on two branches. **How to apply:** after
  every merge of main, `grep "^## " | sort | uniq -d` the touched docs,
  check memory indexes for repeated links, and compare
  `git diff --stat origin/main` with the PR's original diff size.
- **Pre-merging the next PR on the previous PR's tip** saves a round:
  when the squash lands, the squash's files are byte-identical to the tip,
  so add/add conflicts resolve to `--ours` (verify with
  `git diff --stat <tip> origin/main -- <paths>` being empty first).
- **`std::ptr::eq` on rows of a `pub const X: &[T]` is not stable.** Two
  uses of a const slice need not share one allocation; the i686 test
  build returned a different copy and `no_rule_is_shadowed_by_an_earlier_one`
  failed only on the i686 CI nextest. Compare positions (or make it a
  `static`).
- **Local `cimmeria-client-launch` start32 tests fail in the agent
  sandbox**: they run SysWOW64 `PING.EXE 127.0.0.1`, which exits 1 when
  ICMP is blocked. Environmental; CI runs them for real.
