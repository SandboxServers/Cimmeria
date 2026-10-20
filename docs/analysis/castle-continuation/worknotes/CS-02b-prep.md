# CS-02b-prep — Extract internal message reference prose

> **Type:** Reference. **Audience:** Castle packet implementers and reviewers.
> **Last updated:** 2026-10-14. **Status:** Integrated after independent diff and documentation review.
> **Companions:** [Castle ledger](../README.md), [receipt packet](CS-02b1.md), [packet gates](../work-packets.md#cs-0203-conditional-graph-candidates-not-runtime-seals), [doc-update map](../../../agents/doc-update-map.md), [new message reference](../../../protocol/internal-cell-to-base.md).

## Purpose and boundary

`crates/wire/src/cell/messages/cell_to_base.rs` is a 909-line, single-enum
file, including about 485 Rustdoc lines. CS-02b1 must narrow its internal
`MinigameResult` variant to a ticket, but the repository's 700-line hard cap
applies to every touched Rust file. This preparatory packet moves the long
message-contract explanations into one canonical reference document before
the b1 edit. It makes **no** runtime, enum-variant, field, type, visibility,
client Mercury, SmartFox, or internal channel behavior change. No gameplay,
actor binding, result validation, or player UAT is claimed.

The enum cannot be split across Rust files while preserving flat
`CellToBaseMsg::Variant` call sites. A nested inventory/contact enum would
rewrite dozens of unrelated callers. Keep the enum and its public shape;
split the substantial explanatory Rustdoc from the declaration along
message-family seams.

## Exact edit allowlist

| File | Permitted change |
|---|---|
| `crates/wire/src/cell/messages/cell_to_base.rs` | Move at least 240 lines of long-form Rustdoc to the new reference, for a net reduction of at least 220 lines after concise replacement comments and the link. Keep a summary and any field-level ownership or safety constraint needed beside each variant. After editing, this file must be at or below 700 lines. Do not alter Rust tokens that define the enum or its fields. |
| `docs/protocol/internal-cell-to-base.md` | New reference: message-family sections preserving the moved field meanings, ownership rules, ordering and failure limits. State prominently that these are in-process `tokio::mpsc` messages, not client wire opcodes. Include title, type, audience, last updated and companion links. |
| `docs/protocol/README.md` | Add the new reference to its document table. |
| `docs/readme.md` | Add the new reference under the protocol section. |
| `docs/analysis/castle-continuation/README.md`, `RESUME.md`, `work-packets.md`, `worknotes/CS-02b-prep.md`, `worknotes/CS-02b1.md` | Record checks, review, integration and the b1 dependency without claiming b1 is ready or implemented. |

No other Rust, seed, schema, client, gameplay, or generated-marker file may
change. If moving the prose reveals a new source-contract conflict or needs
an extra file, stop and amend this allowlist for review. The existing
[doc-update map](../../../agents/doc-update-map.md) already routes protocol
documentation; update it only if a new kind of change requires a row, after
an explicit packet amendment.

## Extraction shape

Use three coherent sections rather than removing arbitrary comments to hit
the cap:

1. Space, world-entry, Area of Interest, persistence and progression
   messages near the start of the enum. Preserve why the Base or Cell owns
   each value, including world/instance selection and mission repeat counts.
2. Position, appearance, equipment, currency and ability-related messages
   in the middle. Preserve the stated update and recipient constraints.
3. GM, authoring, teleport and witness messages before `StartMinigame`.
   Preserve authorization, previous-position and observer semantics.

The current file has enough long-form comments in these families to move at
least 240 lines while keeping short, useful variant and field documentation.
The new reference names each moved variant and points to the source enum;
the enum-level Rustdoc links back with the durable repository URL
`https://github.com/SandboxServers/Cimmeria/blob/main/docs/protocol/internal-cell-to-base.md`.
Do not use a relative Markdown URL from generated Rustdoc HTML. Keep
minigame variant documentation short and accurate, ready for b1's
ticket-only notification.
Do not present an internal message as a recovered 2009 wire layout. The
change must remain a documentation extraction, not a new source of gameplay
claims competing with the Cimmeria Bible.

## Acceptance and checks

- Compare the enum declaration before and after with a token-aware review:
  its variant names, order, field names/types, derives and visibility must be
  identical. The only Rust edits are comments or documentation attributes.
  A semantic Rust edit fails this packet even when tests pass.
- The new reference preserves the moved constraints and distinguishes
  server-internal messages from Mercury messages sent to the client.
- `cell_to_base.rs` is at most 700 lines; the reference and both indexes link
  to one another. All edited Markdown stays CRLF.
- Run `git diff --check`, local-link and anchor checks, and scoped
  `pwsh tools/lint-md.ps1` over the new reference and changed Markdown.
- Run format and the Windows build-lane check, followed by Rustdoc compilation.
  Separately inspect the generated enum HTML for that exact repository URL and
  confirm its `docs/protocol/internal-cell-to-base.md` target exists in this
  checkout; Rustdoc does not validate Markdown URL targets:

```powershell
cargo fmt --all -- --check
pwsh tools/build-lane/lane.ps1 cargo check -p cimmeria-wire --all-targets
pwsh tools/build-lane/lane.ps1 cargo doc -p cimmeria-wire --no-deps
```

No new runtime regression test or live client UAT is credited to this
documentation-only packet. Review the full source-to-reference move
independently before integration. CS-02b1 may change the enum only after this
packet is integrated and the at-most-700-line count is rechecked.

## Independent review and dispatch seal

The independent packet reviewer found one link-check gap: Rustdoc compiles
without validating an external Markdown target. The durable source URL and
generated-HTML target check above close it. No other dispatch blocker was
verified. The coordinator records the exact Castle feature base SHA, clean packet
branch/worktree, matching merge-base, isolated test database and reviewer
approval before a worker starts. Proposed commit subject:
`docs(protocol): extract CellToBase message contract reference`.

## Implementation and checks

The 29 message-contract explanations are now in the three families of the
[internal Cell-to-Base reference](../../../protocol/internal-cell-to-base.md).
The source enum fell from 909 to 623 lines after 302 long-form Rustdoc lines
were replaced with short summaries and the durable source link was added.
A non-comment declaration comparison against the packet base found all 423
lines identical, including enum variants, order, fields, derives and visibility.
No runtime or client behavior changed. At this packet's integration, CS-02b1
remained a draft; its later seal follows the independent review and
integration recorded here.

- `git diff --check`, using Git's `cr-at-eol` whitespace setting for CRLF Markdown: passed.
- Local file links: 527 checked, none missing; nine local anchors resolved; the source URL appears in generated `enum.CellToBaseMsg.html`, and its Markdown target exists in the checkout.
- Scoped `tools/lint-md.ps1`: unavailable. No local `markdownlint-cli2` is installed; `npx` failed with registry DNS `ENOTFOUND`. No package was installed.
- Lane `cargo fmt -p cimmeria-wire -- --check`: `status=ok`.
- Lane `cargo check -p cimmeria-wire --all-targets`: `status=ok`. The first attempt failed in `cimmeria-entity` because the worktree lacked its ignored `external/recast` setup link; the coordinator restored that link before this passing rerun.
- Lane `cargo doc -p cimmeria-wire --no-deps`: `status=ok`; generated enum HTML contains the exact repository source URL.
- Lane `cargo clippy -p cimmeria-wire --all-targets -- -D warnings`: `status=ok`.
- Lane `cargo test -p cimmeria-wire`: `status=ok`; 325 passed, 0 failed.

## Independent review and integration

The packet diff reviewer verified that the enum API and source constraints
were preserved, then found one Markdown list missing a preceding blank line.
Commit `702226fcc` fixed that formatting finding. The documentation writer
independently signed the new reference, source link and both indexes. Merge
`d33a04041` integrates the packet into Castle. On the combined branch,
`cargo fmt --all -- --check` passed and the build-lane
`cargo check -p cimmeria-wire --all-targets` passed
(`20261014-181222-5352.log`). The enum remains 623 lines. CS-02b1 may now
seal its ticket-only internal variant; its runtime and client gates remain.
