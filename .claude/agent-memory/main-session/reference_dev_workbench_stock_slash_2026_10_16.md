---
name: DW-24A-S stock slash XML audit
description: Source hashes, access role evidence and remaining GM eligibility uncertainty
type: reference
---

The operator-supplied stock `SlashCommands.xml` has 98 parsed command rows
and SHA-256 `6e34355a628bd3b5f95e31ef58e7a2e9700dcce6b9798c751bda2431ba0dcd66`.
`FinalSlashCommands.xml` has 4 rows and SHA-256
`7d166f89d3b6829d2fe6c891c90104c5562d1de653864b10d7dd06fbf97263db`.
The separate QA files have identical hashes. The first file's sole explicit
access mask is `/reload Access=qd`; the role legend in the second defines
`q` as QA and `d` as Designer. Mask evaluation is unverified; this
establishes a role-marked client row but neither Cimmeria GM authorization
nor an event-to-server binding. See
`docs/analysis/dev-workbench/stock-slash-audit.md` for the evidence boundary.
