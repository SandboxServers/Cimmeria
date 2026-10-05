---
name: Adoption reference ownership
description: Native reference preparation has its own retained operation and cleanup checkpoint, independent of installed ownership.
type: project
---

2026-10-04. Source: `crates/launcher/desktop/engine/src/storage/adoption/preparation.rs` and its fixture tests.

The preview UUID owns a nonterminal Adopt operation before reference extraction.
Its saved `PreparationRecord` carries an extraction descriptor only: no Install
operation, installed intent file or installed receipt is created. The existing
ExtractionWork and WineSeedExtractor adapters bind that descriptor to the helper.
Confirmation finishes preparation and admits a distinct copy operation under the
same state mutex. The original legacy launcher lock stays held through review.

Interrupted preparation is never resumed by extracting again. Explicit cleanup
checks the saved directory inode and independent owner lease, verifies any Wine
helper host is absent, and stops only its bound prefix. A cleanup checkpoint
makes interrupted deletion retryable without treating unexplained absence as
success. Retained records can be enumerated after an orphaned handoff. Wine
prefix/runtime evidence is retained, not deleted by reference cleanup.

The real pinned helper passed isolated signed RAR and RAR-with-CAB reference,
whole-file comparison and publication fixtures. It predates the coordinator's
new archive preflight; validation of a freshly rebuilt preflight-enabled helper
remains separate. No actual game execution, published multi-cabinet seed run,
Windows-native adoption or power-loss proof is claimed.

See `docs/analysis/playtests/2026-10-03-macos-wine/worknotes/published-adoption-preparation.md` for API and exact commands.
