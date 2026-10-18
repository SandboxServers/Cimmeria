# Draft reply to #1797 (owner posts, or OKs ops to post)

Draft only. The ai-harness-docs campaign (D-AH17, D-AH18, D-AH19) depends on it. Post it as a PR comment on #1797.

---

Thanks @aurablacklight, this is a really useful write-up. The "What breaks" list, the ops `blocked` reason and the headless-launch note are all accurate, and we want them in.

One change before it lands: the status line. The maintainer's position is now:

> **Windows with PowerShell 7 is fully supported. WSL, Linux, and Git Bash on Windows are not validated, but not unsupported.**

So "not supported for interactive sessions" becomes "not validated". Nothing else in the section has to change for that; "not a supported configuration" further down would read "not a validated configuration".

If you can amend that and land it, we'll stack our platform-wording pass on top of it, so we don't edit the same section in parallel. If you'd rather not, say so and we'll carry your content in our PR with a `Co-authored-by` line for you.

Your three questions:

1. **WSL support.** As above: not validated, but not unsupported. Reports of what breaks are welcome, and your list is the start of that record.
2. **A `.sh` path for `ops`.** Not for now. ops is a small, low-effort agent, and two shell paths would double the rules it has to follow. Humans can use the `.sh` twins in `tools/build-lane/`; agents stay on PowerShell and return `blocked` without it, as your change already does. We'll revisit after a WSL smoke run.
3. **Scoping the orchestrator default to interactive sessions.** We're keeping the default for every launch: there's no setting that scopes `agent` by interactivity, and headless runs can already pass `--agent <name>` or `--setting-sources user` (as Senpai does). To cut the confusion instead, we're adding a session banner that names the persona and points at `claude --agent hands-on`, plus a one-line hands-on reminder in the orchestrator prompt.

The kickoff guide note is right too. We'll word it as "not a validated host", with CLAUDE.md's lane rule winning until that guide is revised.
