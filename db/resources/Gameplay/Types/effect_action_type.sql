--
-- TOC entry 1160 (class 1247 OID 62754)
-- Name: effect_action_type; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE effect_action_type AS ENUM (
    'SetStateFlag',
    'StatChange',
    'PermanentStatChange',
    'RemoveEffect',
    'RemoveMoniker',
    'QRCombatDamage'
);

