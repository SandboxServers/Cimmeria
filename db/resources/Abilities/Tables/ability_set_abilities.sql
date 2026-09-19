--
-- TOC entry 183 (class 1259 OID 62791)
-- Name: ability_set_abilities; Type: TABLE; Schema: resources; Owner: -; Tablespace: 
--

-- The primary key is `(ability_set_id, ability_id)` -- a set holds N abilities.
-- It is declared in db/resources/_primary_keys.sql, not here: every primary key
-- in the resources schema lives in that file, and db/database.sql applies it at
-- line 264, after this DDL (line 190) and before the seed (line 273). Harset
-- packet H09 widened it from the single-column `(ability_set_id)` import
-- artefact; see docs/analysis/harset-rebuild/worknotes/H09.md.
CREATE TABLE ability_set_abilities (
    ability_set_id integer NOT NULL,
    ability_id integer NOT NULL
);

