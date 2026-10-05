---
name: Verified-copy adoption boundary
description: Reference reconstruction, durable ownership, and remaining published-client compatibility gates
type: project
---

2026-10-04. `crates/launcher/desktop/engine/src/storage/adoption/` compares
whole-file hashes with a reconstructed signed release, not the imported ledger.
Schema-2 installed-content records carry adoption provenance, independent legacy
identity, operation ID and desktop owner ID. Source JSON remains in the existing
migration archive and source files remain untouched. This phase gates Play.

The original ZIP extractor skips unsafe entries and permits duplicate overwrite;
adoption adds strict preflight before reusing `unpack::unpack`. Its bounded local
ZIP path does not support the published RAR/MakeCAB seed. Existing integration
seams are `install::SeedExtractor`, `install::SeedExtraction`,
`install::install_all_with_seed_extractor`, `mac_wine::WineSeedExtractor`, and
`storage::extraction_work`. Next phase must extend durable extraction ownership
for an adoption reference and validate RAR/CAB names before extraction. It must
retain helper/prefix ownership and handle uncertain exit, not call the helper as
an untracked subprocess. See the adoption worknote for the complete API boundary.
