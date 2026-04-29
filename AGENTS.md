# Repo Instructions

## Frontend UAT Rule

For every new frontend feature, milestone, or meaningful behavior change:

- perform a JS REPL-style logic UAT in addition to normal tests/builds
- verify the underlying state/persistence behavior, not just compile success
- summarize what was exercised and what passed
- explicitly call out what was **not** covered by the REPL pass

This applies especially to:

- card create/update/delete flows
- sequence/thread edits
- card movement and layout state
- input/output port changes
- persistence and dirty-state transitions
- validation-state changes
- content-engine serialization/deserialization changes

## Notes

- REPL-style logic UAT supplements browser/manual UAT; it does not replace visual verification.
- If a feature cannot be meaningfully exercised in the JS REPL, state that clearly and explain why.
