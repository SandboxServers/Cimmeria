---
name: project-final-re-bundles
description: "2026-10-02 external SGW final RE archive inventory, evidence limits, and useful research paths"
metadata:
  type: project
---

Reviewed 2026-10-02, read-only from three external ZIPs:

- `SGW_FINAL_OVERVIEW_EXPORT.zip` (SHA-256 `a12060f202cb219921a6f9d9292fc67cc47e0025a2ffea58ed7a986b76b4b6b2`): 14 CSV summaries, including the corrected build dossier, map-presence matrix and protocol-feature persistence matrix.
- `SGW_FINAL_DEVELOPER_HANDOFF.zip` (SHA-256 `0317e0d4445f1c851bb60ee910fc8ea77b5661bfd589d315015dfcadc1cfb28a`): 615 entries, mainly 24 recovered-world folders with mission, Kismet, runtime binding, cinematic, transport, trigger and candidate asset CSVs. Its 87 missing-world folders are references and lexical leads, not recovered maps.
- `SGW_FINAL_COMPLETE_RE_REVIEW_BUNDLE.zip` (SHA-256 `35a77982664541e9c61cee5314d827d326bc5fe44b29857543cebd490c768fcc`): 155 entries, including cross-build file manifest, entity-definition extracts, world links and coverage reports. All 144 shared paths with the developer handoff matched by length and ZIP CRC.

The handoff README names `SGW_FINAL_RE.sqlite`, `SGW_ALL_FINDINGS_ARCHIVE.sqlite`, `SOURCE_INDEX/` and per-build/per-patch raw tables, but none of those are in these ZIPs. `REPORTS/00_FINAL_SUMMARY.txt` reports 46,410 recovered file rows representing 52.48 GB; the archives contain report rows and hashes, not those recovered files. `REPORTS/16_FINAL_VALIDATION.csv` records one failed archive source import. Treat the files as research indexes, not a self-contained reconstruction or proof of native/server behavior.

Evidence boundaries are explicit in `HANDOFFS/OPEN_QUESTIONS_AND_LIMITS.md`: live wire layout, exact VO-to-dialog links, placement for unrecovered maps, and untraced native semantics remain open. `REPORTS/09_ENTITY_DEF_PARSE_FAILURES.csv` lists malformed `SGWEntity.def` for seven builds from 55124 through QA4046. The QA metadata report's 35 unique `.def` paths all SHA-256 match `entities/defs/` in this repo, so those QA definitions are already available locally; older-build deltas are the novel part. The build history has 12 nodes, seven proven patch transitions, and four material/snapshot gaps. Absence in partial builds 45032-49486 is unknown.

Best next uses: cross-check the historical CellBlock build states and ring rigs with the per-build world CSVs; compare the Castle and Harset mission/transport/runtime links with the existing campaign audits; use the overview map-presence matrix to select a build before asset archaeology. Keep `docs/protocol/` and address-cited client findings authoritative for wire claims, and verify an archive lead against the actual package, client, or repo seed before implementing it.
