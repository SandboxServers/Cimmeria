-- ============================================================
-- Harset Goa'uld mission chains (chain ids 6101-6300)
-- ============================================================
-- Campaign: docs/analysis/harset-rebuild/ (ledger packets H40-H47).
-- Authoring rules: work-packets.md "Worker Input And Ownership";
-- canonical tags: worknotes/harset-tags.md; dialog-set-map ids 120201-120300.
-- Every coordinate in this file must be recovered from the 2009 Python
-- or pinned in M0; mission-scoped hostiles are spawned into the player's
-- own Market/Storage instance, never into world 57 or 68.
-- Sub-allocation per mission is fixed in the ledger table; never reuse a
-- 1xxx-5xxx id.
-- ============================================================

SET search_path = resources, pg_catalog;

