---
name: project-named-telemetry-campaign
description: "Named-telemetry campaign (Rule 6) closed 2026-10-05: every logged ID paired with its name, unpaired fields 7,039 to 0, 795 nt:id-only marks; not yet read in a live SigNoz session. Read before adding log fields or Discord events."
metadata:
  type: project
---

**Closed 2026-10-05** (plan #1189 on 2026-10-04, packets #1191 to #1238, close-out NT-50b). The ledger, with the final count, lessons and follow-ups, is `docs/analysis/named-telemetry/README.md`; the rule itself is Rule 6 in `docs/architecture/instrumentation-discipline.md`.

- **End state.** NT-03's scan (`crates/server/src/logging/unpaired_id_tests/`) blocks CI on any new unpaired ID field; the baseline file reads `# total 0` (started at 7,039 of 7,109 ID fields). 795 production fields carry `// nt:id-only <reason>`; `git grep -c nt:id-only` reports 810 because the scanner's own fixtures and docs (14) and one Discord doc comment also contain the string.
- **Lookups.** Content IDs: the `cimmeria-names` NameBook. Runtime entity IDs: `SpaceManager::entity_label` / `entity_names` plus departed-entity rings, never the NameBook (slots are recycled). Players, accounts, orgs by serial ID: `cimmeria_entity::known_names`.
- **Not observed live.** SigNoz was unreachable for the whole campaign (2026-10-04/05), so every packet was proven by capture-layer tests. The first session on a build at or after `1d032a500` is the first live check; the saved view **Named telemetry — Missing names** (`docs/operations/signoz/missing-names.view.json`) shows seed holes. Neither saved view existed on the colo SigNoz when committed.
- **Discord** now renders `Name (#id)` and strips non-allowlisted links (#1199, #1208). The `trace_id` footer stays empty until a server site logs `trace_id` from span context (needs `tracing-opentelemetry`).

**Why:** names in logs and Discord save humans and agents from manual ID lookups; the restoration team reads Discord, not SigNoz.

**How to apply:** a new log field with an ID needs its `_name` pair or an `nt:id-only` reason, or CI fails. Before trusting a renamed key in an old SigNoz query, check the renamed-keys table in `docs/architecture/negative-logging-convention.md`.
