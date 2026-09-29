--
-- resources.ammo_modifiers (ammo campaign AM-F, issue #1026).
--
-- What a shot fired with a special ammo type loaded does differently. Per
-- D-AM07 the server applies the row directly on every such shot: no cast, no
-- cooldown, and the toggle abilities (715, 719, ...) are never launched.
-- toggle_ability_id is provenance only: the ability whose description or
-- effect the reconstructed numbers came from.
--
-- Empty at AM-F. Each family packet seeds its own rows in its own file
-- (Abilities/Seed/ammo_modifiers_<family>.sql): AM-04 Hollow Point and Armor
-- Piercing, AM-08 Incendiary, AM-09 EMP, AM-10 Explosive, AM-11a/b/c darts.
--
-- No foreign keys: on_hit_effect_id and toggle_ability_id point at
-- resources.effects / resources.abilities, whose seeds load after this
-- table, and a family seed row is checked by its packet's live-DB test.
--

SET search_path = resources, pg_catalog;

CREATE TABLE ammo_modifiers (
    ammo_type "EAmmoType" NOT NULL,
    damage_mult real DEFAULT 1.0 NOT NULL,
    penetration_mult real DEFAULT 1.0 NOT NULL,
    -- Overrides the damage type of the shot; NULL keeps the ability's own.
    damage_type "EDamageType",
    on_hit_effect_id integer,
    toggle_ability_id integer NOT NULL,
    CONSTRAINT ammo_modifiers_pkey PRIMARY KEY (ammo_type),
    CONSTRAINT ammo_modifiers_mults_positive_chk CHECK (damage_mult > 0 AND penetration_mult > 0),
    CONSTRAINT ammo_modifiers_special_only_chk CHECK (
        ammo_type NOT IN ('AMMO_NONE', 'Bullet_Default', 'Dart_Default', 'Dagger_Default')
    )
);
