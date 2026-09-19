# Issue tracker: GitHub

Issues and specs for this repo live as GitHub issues. Use the `gh` CLI for all operations.

## Conventions

- **Create an issue**: `gh issue create --title "..." --body "..."`. Use a heredoc for multi-line bodies.
- **Read an issue**: `gh issue view <number> --comments`, filtering comments by `jq` and also fetching labels.
- **List issues**: `gh issue list --state open --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'` with appropriate `--label` and `--state` filters.
- **Comment on an issue**: `gh issue comment <number> --body "..."`
- **Apply / remove labels**: `gh issue edit <number> --add-label "..."` / `--remove-label "..."`
- **Close**: `gh issue close <number> --comment "..."`

Infer the repo from `git remote -v`; `gh` does this automatically when run inside a clone.

## Repo specifics

- **The maintainer's GitHub handle is `@Cadacious`.** Use it for @-mentions on issues and PRs. The git author name is not a GitHub handle.
- **Leading-slash arguments get mangled in Git Bash on Windows.** `gh pr comment <n> --body "/release"` posts `C:/Program Files/Git/release`, and the ChatOps workflow then skips silently. The same conversion breaks `git show <ref>:.github/...`. Use `--body-file`, run from PowerShell, or prefix the command with `MSYS_NO_PATHCONV=1`. Verify a posted body with `gh pr view <n> --json comments --jq '.comments[-1].body'`.
- **`/release` on a merged PR deploys `main` to the colo.** It is a maintainer action. Agents never post it.
- **GitHub starts no CI run on a PR that is `CONFLICTING`.** If checks never appear, merge `main` into the branch first.
- **CodeRabbit does not review automatically here** (the repo is under its star threshold, and it skips PRs over 100 files). A green CodeRabbit status means "skipped", not "reviewed". Copilot code review does run automatically.
- PRs are squash-merged. Title format and the body checklist are in [`CONTRIBUTING.md`](../../CONTRIBUTING.md), and the PR template pre-fills them.

## Ticket body contract

When a skill writes a ticket (`/to-tickets`, `/to-spec`, `/triage` rewriting a report), the body carries these sections so that whoever picks it up, human or agent, starts with what this repo's reviewers will ask for:

```markdown
## Problem
What is wrong or missing, in glossary terms (docs/spec/glossary.md).

## Evidence
Doc links, Ghidra addresses, log lines, or capture offsets that support the premise.
State what docs/protocol/ and the RE findings already say about it, including disagreement.

## Acceptance criteria
Observable outcomes: a byte string, a DB row, a log field, a client-visible behaviour.

## Test type
One or more of the types in TESTING.md, with the bug shape the guard must reproduce.

## Docs to update
The rows of the CLAUDE.md doc-update map this change touches.

## Client impact
"Free" (server-authoritative, reuses messages the client already speaks) or
"needs a client patch" (new opcode, wire-crypto change, UI the client lacks).

## Domain advisor
The .claude/agents/ advisor to consult first (see docs/agents/development-workflow.md).

## Needs a human for
RE, in-game UAT, colo access, or "nothing".
```

A ticket with an empty **Evidence** section is `needs-triage`, not `ready-for-agent`. See [`triage-labels.md`](triage-labels.md).

## Pull requests as a triage surface

**PRs as a request surface: no.** _(Set to `yes` if this repo treats external PRs as feature requests; `/triage` reads this flag.)_

When set to `yes`, PRs run through the same labels and states as issues, using the `gh pr` equivalents:

- **Read a PR**: `gh pr view <number> --comments` and `gh pr diff <number>` for the diff.
- **List external PRs for triage**: `gh pr list --state open --json number,title,body,labels,author,authorAssociation,comments` then keep only `authorAssociation` of `CONTRIBUTOR`, `FIRST_TIME_CONTRIBUTOR`, or `NONE` (drop `OWNER`/`MEMBER`/`COLLABORATOR`).
- **Comment / label / close**: `gh pr comment`, `gh pr edit --add-label`/`--remove-label`, `gh pr close`.

GitHub shares one number space across issues and PRs, so a bare `#42` may be either: resolve with `gh pr view 42` and fall back to `gh issue view 42`.

## When a skill says "publish to the issue tracker"

Create a GitHub issue.

## When a skill says "fetch the relevant ticket"

Run `gh issue view <number> --comments`.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a single issue with **child** issues as tickets.

- **Map**: a single issue labelled `wayfinder:map`, holding the Notes / Decisions-so-far / Fog body. `gh issue create --label wayfinder:map`.
- **Child ticket**: an issue linked to the map as a GitHub sub-issue (`gh api` on the sub-issues endpoint). Where sub-issues aren't enabled, add the child to a task list in the map body and put `Part of #<map>` at the top of the child body. Labels: `wayfinder:<type>` (`research`/`prototype`/`grilling`/`task`). Once claimed, the ticket is assigned to the driving dev.
- **Blocking**: GitHub's **native issue dependencies**, the canonical, UI-visible representation. Add an edge with `gh api --method POST repos/<owner>/<repo>/issues/<child>/dependencies/blocked_by -F issue_id=<blocker-db-id>`, where `<blocker-db-id>` is the blocker's numeric **database id** (`gh api repos/<owner>/<repo>/issues/<n> --jq .id`, _not_ the `#number` or `node_id`). GitHub reports `issue_dependencies_summary.blocked_by` (open blockers only, the live gate). Where dependencies aren't available, fall back to a `Blocked by: #<n>, #<n>` line at the top of the child body. A ticket is unblocked when every blocker is closed.
- **Frontier query**: list the map's open children (`gh issue list --state open`, scoped to the map's sub-issues / task list), drop any with an open blocker (`issue_dependencies_summary.blocked_by > 0`, or an open issue in the `Blocked by` line) or an assignee; first in map order wins.
- **Claim**: `gh issue edit <n> --add-assignee @me`, the session's first write.
- **Resolve**: `gh issue comment <n> --body "<answer>"`, then `gh issue close <n>`, then append a context pointer (gist + link) to the map's Decisions-so-far.
