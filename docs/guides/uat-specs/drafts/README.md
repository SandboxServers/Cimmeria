# Recorded UAT drafts

This folder holds draft row specs written by `lab record` from a person
playing a section in the lab (the lab-record campaign,
[README](../../../analysis/lab-record/README.md), decision D-LR10). Each file is
`<section>-<yyyyMMdd-HHmm>.toml`, with section id `rec-<section>-<yyyyMMdd-HHmm>`
and rows `REC-01`, `REC-02`, and so on.

## What a draft is

- **A plain `lab uat` never runs it.** Specs load from
  `docs/guides/uat-specs/` only, not from folders under it. Run a draft by
  pointing the command at this folder:
  `lab uat rec-first-session-20261012-1430 -SpecsDir docs/guides/uat-specs/drafts`.
- **It is UNCONFIRMED** until `lab golden record` agrees on it 5 times
  (D-LR12). Its header says so. One passing run proves nothing about the
  next one.
- **Nothing in it is guessed** (D-LR8). A fact the transform's table does not
  cover is a `# TODO` line where it happened. A row that cannot be written
  validly (a pose outside 1 to 8 m, a target with no spawn tag, no server
  change to check) is written whole as comments under
  `# TODO REC-0n not runnable: <reason>`.
- **SQL names no real character.** The recorded name is replaced by
  `${character}`, or by the `-CharacterExpr` given to `lab record`.

A draft is committed only when a person chooses to. Every committed draft
must parse, validate and name only seeded spawn tags:
`committed_drafts_parse_and_validate` in `crates/lab/src/uat/mod.rs` checks
that on every build.

## Promoting a draft

1. Resolve its `# TODO` lines: add the missing steps by hand, or delete the
   lines once you know they don't matter.
2. Run `lab golden record <draft section> -Rows REC-01,... -ServerVersion <sha> -SpecsDir docs/guides/uat-specs/drafts`
   until its 5 runs agree. Fix the draft, not the golden, when it doesn't.
3. Move the rows into the real spec (`docs/guides/uat-specs/<section>.toml`),
   giving them that spec's row ids, and record that spec's golden.
4. Delete the draft.
