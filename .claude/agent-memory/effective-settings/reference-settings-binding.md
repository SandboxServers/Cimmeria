---
name: Adopted Play settings binding
description: The records that must agree before an adopted copy's imported patch setting is used, and the file adoption must keep
type: reference
---

Verified 2026-10-04 on `launcher/effective-settings` by tamper tests
(`storage::effective_settings::adopted_tests`), each proven to fail with its
check removed.

An adopted copy's patch setting is not read from the published adoption record
alone. `effective_settings::launch_binding` requires all of:

- `installed-content.json` provenance (work id, legacy install id, import digest);
- the Published `adoption-<work>.json` plan, equal to the admission checkpoint
  `adoption-plan-<work>.json`;
- the Adopt journal digest, only while Adopt is still the latest operation;
- `legacy-import.json`, whose `validate()` re-parses the retained exact source
  JSON and recomputes the confirmation digest on every read. This is the record
  that independently restates the settings.

The checkpoint comparison is the only cover for plan fields that neither the
import nor the owner intent restates (`choices`, `report`) once another
operation has superseded Adopt.

Consequence for adoption work: `adoption-plan-<work>.json` must survive
publication. Deleting it after success makes every adopted copy unplayable
(fail closed), with no error surfaced beyond Play being unavailable.

The signed launcher minimum is deliberately outside this binding. It comes from
the owner's retained release evidence, so a copy with refused settings still
reports it.
