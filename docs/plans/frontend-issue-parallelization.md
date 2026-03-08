# Frontend Issue Parallelization

This document records the recommended dependency order, parallelization boundaries, and worktree strategy for the current frontend backlog.

## Worktree Callout

Use dedicated `git worktree` checkouts for active issue branches.

Reason:
- several issues converge on the same hot files
- it reduces accidental cross-issue edits
- it keeps integration work separate from implementation work

Recommended pattern:
- keep `/home/derek/Cimmeria` as the integration workspace
- create one worktree per active issue branch
- keep one owner per hot file at a time

Hot files that need strict ownership:
- `frontend/src/react/ChainFlowWorkbench.react.tsx`
- `frontend/src/react/chainDraftPersistence.ts`
- `crates/admin-api/src/routes/content.rs`

Recommended first-wave worktrees:
- `issue/10-content-persistence`
- `issue/13-editor-state-safety`
- `issue/14-library-search`
- `issue/17-admin-shell-live-data`

## Dependency Matrix

| Issue | Can Run In Parallel? | Blocked By | Best Parallel With | Avoid Parallel With | Shared Hot Files | Recommended Owner |
|---|---|---:|---|---|---|---|
| `#10` Real content-engine persistence | Limited | none | `#13`, `#14`, `#17` | `#11`, `#12`, `#18`, `#19` | `ChainFlowWorkbench.react.tsx`, `chainDraftPersistence.ts`, `content.rs` | Backend/frontend contract owner |
| `#11` Typed forms + DB pickers | Partially | `#10` contract should stabilize first | `#13`, `#14`, `#17` | `#10`, `#12`, `#15`, `#16` | `ChainFlowWorkbench.react.tsx`, `content.rs` | Chain editor form/UX owner |
| `#12` Validation panel | Partially | best after `#10`, cleaner after `#11` | `#13`, `#14` | `#10`, `#11`, `#18` | `ChainFlowWorkbench.react.tsx`, `ContentEditor.tsx`, `content.rs` | Validation/error UX owner |
| `#13` Redo/revert/autosave | Yes | none | `#10`, `#11`, `#12`, `#14`, `#17` | `#15`, `#16` | `ChainFlowWorkbench.react.tsx`, `chainDraftPersistence.ts` | Editor state/history owner |
| `#14` Library search/filter/favorites | Yes | none | almost everything | minor overlap with `#11` | `ChainFlowWorkbench.react.tsx` | Library/catalog UX owner |
| `#15` Duplicate/copy-paste/templates | Somewhat | none, but better after `#13` | `#10`, `#12`, `#14`, `#17` | `#13`, `#16` | `ChainFlowWorkbench.react.tsx`, layout/serialization helpers | Interaction workflow owner |
| `#16` Multi-select/bulk actions | Somewhat | none, but better after `#13` | `#10`, `#12`, `#14`, `#17` | `#13`, `#15` | `ChainFlowWorkbench.react.tsx`, layout/selection helpers | Selection/interaction owner |
| `#17` Replace mock admin-shell data | Yes | none | `#10`, `#13`, `#14`, `#11`, `#12` | only backend coordination overlap with `#10` | `Dashboard.tsx`, `Players.tsx`, `Logs.tsx`, `Config.tsx`, `SpaceViewer.tsx`, admin-api route files | Admin-shell/data owner |
| `#18` Publish/review/diff/rollback | Low | `#10`, ideally `#12` | `#17` only if separated | `#10`, `#12`, `#19` | `ContentEditor.tsx`, `ChainFlowWorkbench.react.tsx`, `content.rs` | Workflow/revision owner |
| `#19` Collaboration affordances | Medium-Low | best after `#10`, partly after `#18` | `#14`, `#17` | `#10`, `#18` | `ChainFlowWorkbench.react.tsx`, `ContentEditor.tsx`, backend locking/revision support | Collaboration/status UX owner |

## Hard Dependencies

- `#11` depends on the payload/data contract from `#10`
- `#12` depends on real validation sources from `#10`
- `#18` depends on real persistence from `#10`
- `#19` depends on at least basic real persistence from `#10`

## Soft Dependencies

- `#15` is safer after `#13`
- `#16` is safer after `#13`
- `#12` is cleaner after `#11`
- `#19` is better after `#18`

## Parallel Waves

### Wave 1

- `#10`
- `#13`
- `#14`
- `#17`

### Wave 2

- `#11`
- `#12`

### Wave 3

- `#15` or `#16`

Do not run `#15` and `#16` in parallel unless ownership is split very tightly.

### Wave 4

- `#18`
- `#19`

## Rules To Minimize Git Conflict Pain

- Keep `/home/derek/Cimmeria` as the integration workspace.
- Rebase issue branches frequently onto the integration branch.
- Merge smaller PRs quickly instead of letting branches drift.
- Extract helpers before parallelizing work inside the same surface.
- Avoid multiple simultaneous owners of the same hot file.
