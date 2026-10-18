# PP-06 report

Wrote docker/db-rebuild.sh (only file). Verified locally: LF only, no `DROP DATABASE` of the source, no bashisms. Shell NOT run (CI only: PP-08 scenarios 3-6).

Deviations from the packet, all forced by the actual wave 1 files:
- PLAYER_SCHEMAS is empty: persistent-public-tables.txt lists public tables only and has no schema list.
- Step 3: reconcile.sql has no pre-check phase, so "none".
- Step 9: cimmeria_meta.db_state has no applied_at; script sets state, image_id, db_source_sha256 (from db-source.sha256 if present), first_seeded_at, last_rebuild_at, rebuild_count (old count from source + 1).
- seed-accounts-stage.sql and seed-accounts.sql (PP-17) are absent: steps 4a/5a run them if present, else print a "skipped" line. Once PP-17 lands they run unchanged.
- Reseed list uses bare names; read_list normalises bare names to public.<name>.
- Step 6 sequence fix-up runs after reconcile block 4 (which already does it); harmless.
- Count check uses SELECT ... FROM ONLY (sgw_inventory inherits sgw_inventory_base) and logged deletes keyed by reconcile_log.table_name; account top-up counted from action insert_seed_account.
- Step 4 truncates all carried tables in one TRUNCATE (no CASCADE) before the data restore.
