--
-- resources.ammo_item_types (ammo campaign AM-F, issue #1026).
--
-- Which inventory item holds the reserve rounds of each special EAmmoType
-- (D-AM01: the reserve lives in the bags). Every packet that needs an ammo
-- item goes through this table, never a hardcoded item id, so a re-seed of
-- the ids (AM-07's spike fallback) touches only the two seed files.
--
-- Loaded from the seed section of db/database.sql, after the items seed and
-- resources/_primary_keys.sql, because the foreign key needs items' primary
-- key to exist already.
--

SET search_path = resources, pg_catalog;

CREATE TABLE ammo_item_types (
    ammo_type "EAmmoType" NOT NULL,
    item_id integer NOT NULL,
    CONSTRAINT ammo_item_types_pkey PRIMARY KEY (ammo_type),
    CONSTRAINT ammo_item_types_item_id_key UNIQUE (item_id),
    CONSTRAINT ammo_item_types_item_id_fkey FOREIGN KEY (item_id) REFERENCES items(item_id),
    -- Default ammo reloads for free (D-AM02) and has no reserve item.
    CONSTRAINT ammo_item_types_special_only_chk CHECK (
        ammo_type NOT IN ('AMMO_NONE', 'Bullet_Default', 'Dart_Default', 'Dagger_Default')
    )
);
