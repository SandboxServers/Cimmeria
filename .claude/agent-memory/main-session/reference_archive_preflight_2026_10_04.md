---
name: reference_archive_preflight_2026_10_04
description: RAR and FDI name preflight before reference extraction
metadata:
  type: reference
---

The adoption reference must not silently choose one of conflicting archive
entries. Shared RAR extraction now completes listing and a Windows-name inventory
before writing files. FDI uses a header-only pass across all cabinets; COPY_FILE
returns zero in this pass, so file starts are counted once and continuations are
ignored. Count must equal the INF index, and source handles deny mutation across
passes. The unchanged helper protocol reports these failures as Archive.

Mac shared extraction tests passed (30 passed, one real-client fixture ignored).
Native Windows FDI tests and a rebuilt helper remain required before claiming
cabinet compatibility. Existing debug helper binaries predate this change.
See [the extraction contract](../../../crates/launcher/desktop/docs/wine-validation.md#rar-and-cabinet-entry-preflight).
