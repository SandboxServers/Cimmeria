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

## Running Frontend Logic UAT Checklist

Keep this running list current as features are added:

1. Rename card
2. Move card
3. Rename sequence thread
4. Add input ports
5. Add output ports
6. Delete card
7. Add new library card
8. Draft save payload update
9. Content-engine save payload update
10. Scope filtering integrity
11. Typed field selection and database-picker filtering
12. Validation issue generation and focus transitions

When a new frontend capability is added, append the relevant REPL-UAT item(s) here if they are not already covered.

## Notes

- REPL-style logic UAT supplements browser/manual UAT; it does not replace visual verification.
- If a feature cannot be meaningfully exercised in the JS REPL, state that clearly and explain why.
