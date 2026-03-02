--
-- TOC entry 1157 (class 1247 OID 62743)
-- Name: effect_action; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE effect_action AS ENUM (
    'SetStateFlag',
    'PermanentStatChange',
    'TemporaryStatChange',
    'RemoveEffect',
    'RemoveMoniker'
);

